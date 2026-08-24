use parking_lot::Mutex;
use regex::RegexBuilder;
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, Result, Row, ToSql};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::external_import::ExternalTextClip;

mod activity;
mod analysis_activity;
mod analytics;
mod bins;
mod capture;
mod classifiers;
mod clip_collections;
mod clip_concealment;
mod clip_mutations;
mod clip_names;
mod clip_search;
mod content_type_registry;
mod contracts;
mod extractors;
mod full_backups;
mod intelligence_connections;
mod lifecycle;
mod maintenance;
mod operations;
mod search_indexes;
mod source_queries;
pub use clip_names::clip_name_input_limit;
mod clip_protection;
mod clip_queries;
mod clip_records;
mod clip_revisions;
mod retention;
mod schema;
mod settings;
mod stored_analysis;
mod timestamps;
mod transfers;
mod transforms;

pub use activity::{ActivityArchive, ActivityArchiveEntry, ActivityImportReport, ActivityLog};
pub use clip_collections::ClipCollectionSummary;
use clip_concealment::{
    append_clip_concealment, configure_content_type_schema, create_effective_view,
};
use clip_names::append_clip_names;
use clip_records::{
    append_clip_content_types, append_clip_file_formats, append_clip_protection,
    append_smart_bin_memberships, clip_item_from_row, normalize_imported_clip_types,
    push_smart_condition, replace_imported_content_types, smart_bin_feature_policy,
    SmartBinFeaturePolicy, MAX_CLIP_SEARCH_FILTERS, MAX_CLIP_SEARCH_OFFSET,
    MAX_CLIP_SEARCH_QUERY_BYTES, MAX_CLIP_SEARCH_TERMS,
};
pub use clip_records::{
    ClipItem, ClipSearchRequest, ClipSearchResult, DEFAULT_CLIP_SEARCH_PAGE_SIZE,
    MAX_CLIP_SEARCH_PAGE_SIZE,
};
#[cfg(test)]
use clip_search::parse_clip_search;
pub use contracts::{
    AnalysisClassification, AnalyticsSummary, BackupPayload, Bin, BinTransformBinding,
    ClipImportReport, ClipMutationSummary, ClipSearchableText, ClipTypeStat, ClipVersion,
    ContentClassificationRescanReport, DailyStat, DbState, FactoryResetReport,
    FileFormatRescanReport, FileFormatStat, FullBackupInspection, FullBackupReport,
    FullRestoreReport, LibraryArchiveInspection, OcrBackfillStatus, OcrBackupMetadata,
    OcrCandidate, OcrExtractorProvenance, SourceStat, StoredExtractionAttempt,
    StoredExtractionObservation, TypeStat,
};
use contracts::{ClipRevisionContext, ClipRevisionOrganization, ClipSaveInput, FullBackupManifest};
pub use intelligence_connections::{IntelligenceConnection, IntelligenceConnectionUpdate};
pub use lifecycle::open_pasted_database;
use lifecycle::open_pasted_database_read_only;
pub use operations::{Operation, ResolvedCustomOperation};
use schema::{
    add_column_if_missing, column_exists, insert_default_bins,
    retire_structural_content_type_entries, table_exists,
};
#[cfg(test)]
use schema::{migrate_legacy_semantic_clip_types, migrate_pipelines_to_saved_transforms};
pub use search_indexes::{SearchIndexEntry, SearchIndexStatus};
use timestamps::{
    canonical_utc_timestamp, migrate_analysis_transform_timestamps, migrate_canonical_timestamps,
    normalize_library_archive_timestamps,
};
pub use transforms::{
    ClipTransformationProvenance, Pipeline, PipelineStep, PipelineStepInput, SavedTransform,
    TransformAuthoringKind, TransformClipApplication, TransformDefinition, TransformationExecution,
    TransformationExecutionStart,
};

const BACKUP_SCHEMA_VERSION: u32 = 13;
const FULL_BACKUP_FORMAT_VERSION: i64 = 1;
const PENDING_CLIENT_STATE_SETTING: &str = "pendingFullBackupClientState";
const MAX_BACKUP_INTERFACE_STATE_BYTES: usize = 1024 * 1024;
#[cfg(test)]
use analytics::MAX_ANALYTICS_FILE_FORMATS;

fn invalid_extractor_input(error: impl Into<String>) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        error.into(),
    )))
}

fn sqlite_count(row: &rusqlite::Row<'_>) -> Result<usize> {
    let count = row.get::<_, i64>(0)?;
    usize::try_from(count).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}

fn ensure_resource_size(value: &str, maximum: usize, label: &str) -> Result<()> {
    if value.len() <= maximum {
        return Ok(());
    }
    Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "{label} exceeds Pasted's {} MB safety limit",
                maximum / 1024 / 1024
            ),
        ),
    )))
}

fn ensure_safe_raster_data_url(value: &str, label: &str) -> Result<()> {
    crate::resource_limits::validate_raster_data_url(value).map_err(|error| {
        rusqlite::Error::InvalidParameterName(format!("{label} is invalid: {error}"))
    })
}

fn validate_backup_json(value: Option<&str>, label: &str) -> Result<()> {
    let Some(value) = value else {
        return Ok(());
    };
    ensure_resource_size(value, MAX_BACKUP_INTERFACE_STATE_BYTES, label)?;
    serde_json::from_str::<serde_json::Value>(value)
        .map(|_| ())
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

fn escape_like_literal(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn derived_origin_kind(content_type: &str, source: &str) -> &'static str {
    crate::content_inspection::origin_kind(content_type, Some(source)).stable_name()
}

fn content_classifier_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<crate::content_classification::Classifier> {
    let patterns_json: String = row.get(5)?;
    let patterns = serde_json::from_str(&patterns_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let stable_ref: String = row.get(1)?;
    let is_builtin: bool = row.get(9)?;
    Ok(crate::content_classification::Classifier {
        id: row.get(0)?,
        defaults: is_builtin
            .then(|| crate::content_classification::classifier_defaults(&stable_ref))
            .flatten(),
        stable_ref,
        name: row.get(2)?,
        content_type: row.get(3)?,
        description: row.get(4)?,
        patterns,
        validator: row.get(6)?,
        enabled: row.get(7)?,
        priority: row.get(8)?,
        is_builtin,
        is_deleted: row.get(10)?,
    })
}

fn describe_clip_ids(ids: &[i64]) -> String {
    if ids.len() == 1 {
        return format!("clip #{}", ids[0]);
    }
    let mut shown = ids
        .iter()
        .take(5)
        .map(|id| format!("#{id}"))
        .collect::<Vec<_>>()
        .join(", ");
    if ids.len() > 5 {
        shown.push_str(&format!(", +{} more", ids.len() - 5));
    }
    format!("{} clips ({shown})", ids.len())
}

#[cfg(test)]
#[path = "db/tests/mod.rs"]
mod tests;
