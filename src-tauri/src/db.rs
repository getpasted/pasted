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
pub mod clip_visual_labels;
mod content_type_registry;
mod contracts;
mod extractors;
mod full_backups;
mod helpers;
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
mod clip_revision_retention;
mod clip_revision_state;
mod clip_revisions;
mod clip_version_delete;
mod clip_version_queries;
mod clip_version_restore;
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
    SmartBinFeaturePolicy, MAX_CLIP_SEARCH_FILTERS, MAX_CLIP_SEARCH_IDS, MAX_CLIP_SEARCH_OFFSET,
    MAX_CLIP_SEARCH_QUERY_BYTES, MAX_CLIP_SEARCH_TERMS,
};
pub use clip_records::{
    ClipItem, ClipSearchRequest, ClipSearchResult, DEFAULT_CLIP_SEARCH_PAGE_SIZE,
    MAX_CLIP_SEARCH_PAGE_SIZE,
};
#[cfg(test)]
use clip_search::parse_clip_search;
pub use contracts::{
    AnalyticsSummary, BackupPayload, Bin, BinTransformBinding, ClipImportReport,
    ClipMutationSummary, ClipTypeStat, ClipVersion, ContentClassificationRescanReport, DailyStat,
    DbState, FactoryResetReport, FileFormatRescanReport, FileFormatStat, FullBackupInspection,
    FullBackupReport, FullRestoreReport, LibraryArchiveInspection, OcrBackfillStatus,
    OcrBackupMetadata, OcrCandidate, OcrExtractorProvenance, SourceStat, TypeStat,
};
use contracts::{
    ClipRevisionContext, ClipRevisionOrganization, ClipSaveInput, FullBackupManifest,
    BACKUP_SCHEMA_VERSION,
};
use helpers::*;
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
pub use stored_analysis::{
    AnalysisClassification, AnalysisFailureClass, ClipSearchableText, ExtractionAttemptContext,
    StoredExtractionAttempt, StoredExtractionObservation,
};
use timestamps::{
    canonical_utc_timestamp, migrate_analysis_transform_timestamps, migrate_canonical_timestamps,
    normalize_library_archive_timestamps,
};
pub use transforms::{
    ClipTransformationProvenance, Pipeline, PipelineStep, PipelineStepInput, SavedTransform,
    TransformAuthoringKind, TransformClipApplication, TransformDefinition, TransformationExecution,
    TransformationExecutionStart,
};
const FULL_BACKUP_FORMAT_VERSION: i64 = 1;
const PENDING_CLIENT_STATE_SETTING: &str = "pendingFullBackupClientState";
#[cfg(test)]
use analytics::MAX_ANALYTICS_FILE_FORMATS;

#[cfg(test)]
#[path = "db/tests/mod.rs"]
mod tests;
