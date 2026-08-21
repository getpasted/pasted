use parking_lot::Mutex;
use regex::RegexBuilder;
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, Result, Row, ToSql};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::external_import::ExternalTextClip;

mod activity;
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
mod extractors;
mod full_backups;
mod intelligence_connections;
mod lifecycle;
mod operations;
mod search_indexes;
pub use clip_names::clip_name_input_limit;
mod clip_protection;
mod clip_queries;
mod clip_records;
mod clip_revisions;
mod retention;
mod schema;
mod settings;
mod stored_analysis;
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
pub use intelligence_connections::{IntelligenceConnection, IntelligenceConnectionUpdate};
pub use lifecycle::open_pasted_database;
use lifecycle::open_pasted_database_read_only;
pub use operations::{Operation, ResolvedCustomOperation};
use schema::{
    add_column_if_missing, column_exists, insert_default_bins,
    retire_structural_content_type_entries, table_exists,
};
#[cfg(test)]
use schema::{
    migrate_legacy_semantic_clip_types, migrate_pipelines_to_saved_transforms,
    run_named_migrations, NamedMigration,
};
pub use search_indexes::{SearchIndexEntry, SearchIndexStatus};
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

fn analysis_toggle_activity(
    kind: &str,
    name: &str,
    enabled: bool,
) -> Option<(&'static str, String)> {
    let event_type = match (kind, enabled) {
        ("extractor", true) => "content_extractor_enabled",
        ("extractor", false) => "content_extractor_disabled",
        ("classifier", true) => "content_classifier_enabled",
        ("classifier", false) => "content_classifier_disabled",
        _ => return None,
    };
    let label = if kind == "extractor" {
        "Extractor"
    } else {
        "Classifier"
    };
    Some((
        event_type,
        format!(
            "{} {label} \"{name}\"",
            if enabled { "Enabled" } else { "Disabled" }
        ),
    ))
}

struct ClipSaveInput<'a> {
    content_type: &'a str,
    text_content: Option<&'a str>,
    html_content: Option<&'a str>,
    image_base64: Option<&'a str>,
    content_hash: &'a str,
    source: &'a str,
}

#[derive(Clone, Copy)]
pub struct OcrExtractorProvenance<'a> {
    pub engine_version: &'a str,
    pub stable_ref: Option<&'a str>,
    pub name: Option<&'a str>,
}

impl<'a> OcrExtractorProvenance<'a> {
    pub fn identified(engine_version: &'a str, stable_ref: &'a str, name: &'a str) -> Self {
        Self {
            engine_version,
            stable_ref: Some(stable_ref),
            name: Some(name),
        }
    }

    fn engine_only(engine_version: &'a str) -> Self {
        Self {
            engine_version,
            stable_ref: None,
            name: None,
        }
    }
}

/// Stable result contract shared by GUI commands and the CLI for clip mutations.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClipMutationSummary {
    pub action: String,
    pub requested_count: usize,
    pub changed_count: usize,
    pub skipped_count: usize,
    pub clip_ids: Vec<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClipImportReport {
    pub scanned_count: usize,
    pub imported_count: usize,
    pub duplicate_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContentClassificationRescanReport {
    pub scanned_count: usize,
    pub changed_count: usize,
    pub unchanged_count: usize,
    pub failed_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileFormatRescanReport {
    pub scanned_count: usize,
    pub changed_count: usize,
    pub unchanged_count: usize,
    pub missing_count: usize,
    pub failed_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisClassification {
    pub id: i64,
    pub clip_id: i64,
    pub content_type: String,
    pub classifier_ref: String,
    pub classifier_name: String,
    pub priority: i64,
    pub source_representation: String,
    pub input_hash: String,
    pub start_offset: Option<usize>,
    pub end_offset: Option<usize>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClipSearchableText {
    pub clip_id: i64,
    pub extractor_ref: String,
    pub extractor_name: String,
    pub engine: String,
    pub input_hash: String,
    pub searchable_text: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoredExtractionObservation {
    #[serde(flatten)]
    pub observation: crate::content_analysis::ExtractionObservation,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoredExtractionAttempt {
    #[serde(flatten)]
    pub observation: crate::content_analysis::ExtractionObservation,
    pub run_id: String,
    pub run_at: String,
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

impl ClipMutationSummary {
    fn new(action: &str, requested_count: usize, clip_ids: Vec<i64>) -> Self {
        let changed_count = clip_ids.len();
        Self {
            action: action.to_string(),
            requested_count,
            changed_count,
            skipped_count: requested_count.saturating_sub(changed_count),
            clip_ids,
        }
    }
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

fn canonical_utc_timestamp(value: &str, label: &str) -> Result<String> {
    if let Ok(timestamp) = chrono::DateTime::parse_from_rfc3339(value) {
        return Ok(timestamp
            .with_timezone(&chrono::Utc)
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    }
    for format in ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S%.f"] {
        if let Ok(timestamp) = chrono::NaiveDateTime::parse_from_str(value, format) {
            return Ok(timestamp
                .and_utc()
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
        }
    }
    Err(rusqlite::Error::InvalidParameterName(format!(
        "{label} contains an invalid timestamp"
    )))
}

fn canonicalize_optional_timestamp(value: &mut Option<String>, label: &str) -> Result<()> {
    if let Some(timestamp) = value.as_deref() {
        *value = Some(canonical_utc_timestamp(timestamp, label)?);
    }
    Ok(())
}

fn migrate_canonical_timestamps(conn: &Connection) -> Result<()> {
    const MIGRATION_KEY: &str = "canonicalUtcTimestampsV1";
    let applied: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE key = ?1)",
        [MIGRATION_KEY],
        |row| row.get(0),
    )?;
    if applied {
        return Ok(());
    }

    let transaction = conn.unchecked_transaction()?;
    for (table, columns) in [
        (
            "clips",
            &["created_at", "trashed_at", "ocr_attempted_at"][..],
        ),
        ("activity_logs", &["created_at", "observed_at"][..]),
    ] {
        if !table_exists(&transaction, table)? {
            continue;
        }
        for column in columns {
            if !column_exists(&transaction, table, column)? {
                continue;
            }
            transaction.execute(
                &format!(
                    "UPDATE {table}
                     SET {column} = strftime('%Y-%m-%dT%H:%M:%SZ', {column})
                     WHERE {column} IS NOT NULL AND datetime({column}) IS NOT NULL"
                ),
                [],
            )?;
        }
    }
    transaction.execute(
        "INSERT INTO schema_migrations (key, applied_at)
         VALUES (?1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
        [MIGRATION_KEY],
    )?;
    transaction.commit()
}

fn migrate_analysis_classification_timestamps(conn: &Connection) -> Result<()> {
    if !table_exists(conn, "clip_analysis_classifications")? {
        return Ok(());
    }
    let transaction = conn.unchecked_transaction()?;
    transaction.execute(
        "UPDATE clip_analysis_classifications
         SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', updated_at)
         WHERE updated_at NOT GLOB '????-??-??T??:??:??Z'
           AND datetime(updated_at) IS NOT NULL",
        [],
    )?;
    let invalid: bool = transaction.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM clip_analysis_classifications
            WHERE updated_at NOT GLOB '????-??-??T??:??:??Z'
        )",
        [],
        |row| row.get(0),
    )?;
    if invalid {
        return Err(rusqlite::Error::InvalidParameterName(
            "Analysis classification contains an invalid timestamp".into(),
        ));
    }
    transaction.commit()
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OcrBackfillStatus {
    pub total_images: i64,
    pub eligible_count: i64,
    pub queued_count: i64,
    pub running_count: i64,
    pub completed_count: i64,
    pub no_text_count: i64,
    pub failed_count: i64,
}

#[derive(Debug, Clone)]
pub struct OcrCandidate {
    pub clip_id: i64,
    pub content_hash: String,
    pub image_base64: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Bin {
    pub id: i64,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub smart_rule: Option<String>, // JSON string for auto-smart rules
    pub bin_type: String,           // "category" or "tag"
    #[serde(rename = "hotkey", alias = "shortcut")]
    pub shortcut: Option<String>,
    #[serde(default)]
    pub protect_clips: bool,
    #[serde(default)]
    pub conceal_clips: bool,
    pub clip_count: Option<i64>,
    #[serde(default)]
    pub clip_order: Vec<i64>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClipVersion {
    pub id: i64,
    pub clip_id: i64,
    pub text_content: String,
    pub action_kind: Option<String>,
    pub action_label: Option<String>,
    pub restores_organization: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ClipRevisionContext {
    schema_version: i64,
    action_kind: String,
    action_label: String,
    organization: Option<ClipRevisionOrganization>,
    #[serde(default)]
    current_transformation_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ClipRevisionOrganization {
    category_bin_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceStat {
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeStat {
    pub content_type: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipTypeStat {
    pub clip_type: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileFormatStat {
    pub file_format: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStat {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSummary {
    pub total_clips: i64,
    pub total_chars: i64,
    pub top_sources: Vec<SourceStat>,
    pub clip_types: Vec<ClipTypeStat>,
    pub file_formats: Vec<FileFormatStat>,
    pub content_types: Vec<TypeStat>,
    pub daily_activity: Vec<DailyStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullBackupReport {
    pub path: String,
    pub created_at: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullBackupInspection {
    pub format_version: i64,
    pub created_at: String,
    pub size_bytes: u64,
}

struct FullBackupManifest {
    format_version: i64,
    created_at: String,
    client_state_json: Option<String>,
    window_state_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullRestoreReport {
    pub recovery_path: String,
    pub backup_created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupPayload {
    pub version: u32,
    pub timestamp: String,
    pub clips: Vec<ClipItem>,
    pub bins: Vec<Bin>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pipelines: Vec<Pipeline>,
    pub operations: Vec<Operation>,
    #[serde(default)]
    pub saved_transforms: Vec<SavedTransform>,
    #[serde(default)]
    pub bin_transforms: Vec<BinTransformBinding>,
    #[serde(default)]
    pub ocr_metadata: Vec<OcrBackupMetadata>,
    #[serde(default, alias = "content_detectors", alias = "contentDetectors")]
    pub content_classifiers: Vec<crate::content_classification::Classifier>,
    #[serde(default)]
    pub content_types: Vec<crate::content_types::ContentTypeDefinition>,
    #[serde(default)]
    pub content_type_groups: Vec<crate::content_types::ContentTypeGroupDefinition>,
}

fn normalize_library_archive_timestamps(payload: &mut BackupPayload) -> Result<()> {
    payload.timestamp = canonical_utc_timestamp(&payload.timestamp, "Transfer file")?;
    for clip in &mut payload.clips {
        clip.created_at = canonical_utc_timestamp(&clip.created_at, "Transfer clip")?;
        canonicalize_optional_timestamp(&mut clip.trashed_at, "Transfer clip")?;
    }
    for bin in &mut payload.bins {
        bin.created_at = canonical_utc_timestamp(&bin.created_at, "Transfer Bin")?;
    }
    for operation in &mut payload.operations {
        if operation.id >= 0 {
            operation.created_at =
                canonical_utc_timestamp(&operation.created_at, "Transfer Operation")?;
        }
    }
    for pipeline in &mut payload.pipelines {
        pipeline.created_at = canonical_utc_timestamp(&pipeline.created_at, "Transfer Transform")?;
        pipeline.updated_at = canonical_utc_timestamp(&pipeline.updated_at, "Transfer Transform")?;
    }
    for transform in &mut payload.saved_transforms {
        transform.created_at =
            canonical_utc_timestamp(&transform.created_at, "Transfer Transform")?;
        transform.updated_at =
            canonical_utc_timestamp(&transform.updated_at, "Transfer Transform")?;
    }
    for metadata in &mut payload.ocr_metadata {
        canonicalize_optional_timestamp(&mut metadata.attempted_at, "Transfer OCR metadata")?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryArchiveInspection {
    pub schema_version: u32,
    pub clip_count: usize,
    pub bin_count: usize,
    pub operation_count: usize,
    pub transform_count: usize,
    pub classifier_count: usize,
    pub content_type_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OcrBackupMetadata {
    pub content_hash: String,
    pub status: String,
    pub input_hash: Option<String>,
    pub engine_version: Option<String>,
    #[serde(default)]
    pub extractor_ref: Option<String>,
    #[serde(default)]
    pub extractor_name: Option<String>,
    pub attempted_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BinTransformBinding {
    pub bin_id: i64,
    pub transform_ref: String,
}

pub struct DbState {
    pub conn: Mutex<Connection>,
    path: Mutex<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FactoryResetReport {
    pub clips_deleted: usize,
    pub bins_deleted: usize,
    pub transforms_deleted: usize,
    pub connections_deleted: usize,
    pub activity_entries_deleted: usize,
}

impl DbState {
    #[allow(clippy::type_complexity)]
    fn log_analysis_participant_toggle(
        &self,
        kind: &str,
        stable_ref: &str,
        name: &str,
        enabled: bool,
    ) {
        let Some((event_type, description)) = analysis_toggle_activity(kind, name, enabled) else {
            return;
        };
        let _ = self.log_activity_with_attributes(
            event_type,
            &description,
            &serde_json::json!({
                "analysis.participant.kind": kind,
                "analysis.participant.ref": stable_ref,
                "analysis.participant.enabled": enabled,
            }),
        );
    }

    fn log_analysis_participant_update(
        &self,
        kind: &str,
        stable_ref: &str,
        name: &str,
        previous_enabled: bool,
        enabled: bool,
    ) {
        if previous_enabled != enabled {
            self.log_analysis_participant_toggle(kind, stable_ref, name, enabled);
            return;
        }
        let (event_type, label) = if kind == "extractor" {
            ("content_extractor_updated", "Extractor")
        } else {
            ("content_classifier_updated", "Classifier")
        };
        let _ = self.log_activity(event_type, &format!("Updated {label} \"{name}\""));
    }

    pub fn clear_history(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM clips WHERE is_pinned = 0 AND clips.id NOT IN (SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1)",
            [],
        )?;
        Ok(())
    }

    pub fn rescan_file_formats(&self) -> Result<FileFormatRescanReport> {
        let clips = {
            let conn = self.conn.lock();
            let mut statement = conn.prepare(
                "SELECT id, content_hash, text_content
                 FROM clips
                 WHERE content_type = 'file' AND COALESCE(is_trashed, 0) = 0
                 ORDER BY id ASC",
            )?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                })?
                .collect::<Result<Vec<_>>>()?;
            rows
        };
        let mut changed_count = 0usize;
        let mut missing_count = 0usize;
        let mut failed_count = 0usize;
        for (clip_id, content_hash, payload) in &clips {
            let paths = payload
                .as_deref()
                .map(crate::content_inspection::parse_file_paths)
                .unwrap_or_default();
            if paths.is_empty() || !crate::resource_limits::file_list_within_limit(&paths) {
                failed_count += 1;
                continue;
            }
            let inspection = crate::content_inspection::inspect_file_formats(&paths);
            if inspection.unavailable_count == paths.len() {
                missing_count += 1;
                continue;
            }
            let existing = self.get_file_format_inspection(*clip_id, content_hash)?;
            if existing.as_ref() != Some(&inspection)
                && self.record_file_format_inspection(*clip_id, content_hash, &inspection)?
            {
                changed_count += 1;
            }
        }
        let report = FileFormatRescanReport {
            scanned_count: clips.len(),
            changed_count,
            unchanged_count: clips
                .len()
                .saturating_sub(changed_count)
                .saturating_sub(missing_count)
                .saturating_sub(failed_count),
            missing_count,
            failed_count,
        };
        let _ = self.log_activity(
            "file_format_history_rescanned",
            &format!(
                "Rescanned {} file clips; updated {}; missing {}; failed {}",
                report.scanned_count,
                report.changed_count,
                report.missing_count,
                report.failed_count
            ),
        );
        Ok(report)
    }

    pub fn get_distinct_sources(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT source FROM clips WHERE source IS NOT NULL AND source != '' ORDER BY source ASC"
        )?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut sources = Vec::new();
        for r in rows {
            sources.push(r?);
        }
        Ok(sources)
    }
}

#[cfg(test)]
#[path = "db/tests/mod.rs"]
mod tests;
