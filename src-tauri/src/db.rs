use parking_lot::Mutex;
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, Result, ToSql};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::external_import::ExternalTextClip;

const BACKUP_SCHEMA_VERSION: u32 = 11;
const FULL_BACKUP_FORMAT_VERSION: i64 = 1;
const PENDING_CLIENT_STATE_SETTING: &str = "pendingFullBackupClientState";
const MAX_BACKUP_INTERFACE_STATE_BYTES: usize = 1024 * 1024;

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

fn push_smart_condition(
    kind: &str,
    value: &str,
    conditions: &mut Vec<String>,
    parameters: &mut Vec<Box<dyn ToSql>>,
) {
    let value = value.trim();
    if value.is_empty() {
        return;
    }
    let condition = match kind {
        "content_type" => {
            parameters.push(Box::new(value.to_string()));
            "content_type = ?".to_string()
        }
        "origin_kind" => {
            parameters.push(Box::new(value.to_lowercase()));
            "CASE WHEN content_type IN ('image', 'file') AND (LOWER(source) LIKE '%screenshot%' OR LOWER(source) LIKE '%screencapture%' OR LOWER(source) LIKE '%cleanshot%') THEN 'screenshot' WHEN content_type = 'file' THEN 'file_reference' WHEN LOWER(source) IN ('cli terminal', 'pasted cli') THEN 'command_line' ELSE 'clipboard_content' END = ?".to_string()
        }
        "source" => {
            parameters.push(Box::new(format!("%{}%", value)));
            "source LIKE ?".to_string()
        }
        "contains" => {
            parameters.push(Box::new(format!("%{}%", value)));
            "text_content LIKE ?".to_string()
        }
        "file_extension" => {
            let extension =
                escape_like_literal(value.trim_start_matches('.').to_lowercase().as_str());
            if extension.is_empty() {
                return;
            }
            parameters.push(Box::new(format!("%.{extension}")));
            "content_type = 'file' AND EXISTS (SELECT 1 FROM json_each(CASE WHEN json_valid(text_content) THEN text_content ELSE '[]' END) AS pasted_file WHERE LOWER(CAST(pasted_file.value AS TEXT)) LIKE ? ESCAPE '\\')".to_string()
        }
        "file_path" => {
            parameters.push(Box::new(format!(
                "%{}%",
                escape_like_literal(&value.to_lowercase())
            )));
            "content_type = 'file' AND EXISTS (SELECT 1 FROM json_each(CASE WHEN json_valid(text_content) THEN text_content ELSE '[]' END) AS pasted_file WHERE LOWER(CAST(pasted_file.value AS TEXT)) LIKE ? ESCAPE '\\')".to_string()
        }
        _ => return,
    };
    conditions.push(condition);
}

fn append_smart_bin_memberships(conn: &Connection, clips: &mut [ClipItem]) -> Result<()> {
    if clips.is_empty() {
        return Ok(());
    }
    let requested_ids = clips.iter().map(|clip| clip.id).collect::<HashSet<_>>();
    let mut memberships = HashMap::<i64, Vec<i64>>::new();
    let mut bins_statement = conn
        .prepare("SELECT id, smart_rule FROM bins WHERE smart_rule IS NOT NULL ORDER BY id ASC")?;
    let smart_bins = bins_statement
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>>>()?;

    for (bin_id, smart_rule) in smart_bins {
        let mut conditions = Vec::new();
        let mut parameters: Vec<Box<dyn ToSql>> = Vec::new();
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&smart_rule) {
            if let Some(items) = parsed["conditions"].as_array() {
                for condition in items {
                    push_smart_condition(
                        condition["type"].as_str().unwrap_or(""),
                        condition["value"].as_str().unwrap_or(""),
                        &mut conditions,
                        &mut parameters,
                    );
                }
            } else {
                push_smart_condition(
                    parsed["type"].as_str().unwrap_or(""),
                    parsed["value"].as_str().unwrap_or(""),
                    &mut conditions,
                    &mut parameters,
                );
            }
        }
        let join = if serde_json::from_str::<serde_json::Value>(&smart_rule)
            .ok()
            .and_then(|rule| rule["match"].as_str().map(str::to_owned))
            .as_deref()
            == Some("all")
        {
            " AND "
        } else {
            " OR "
        };
        let rule_clause = if conditions.is_empty() {
            "0".to_string()
        } else {
            format!("({})", conditions.join(join))
        };
        let sql = format!(
            "SELECT id FROM clips
             WHERE (is_trashed IS NULL OR is_trashed = 0)
               AND ({rule_clause} OR bin_id = ? OR id IN (
                    SELECT clip_id FROM clip_bins WHERE bin_id = ?
               ))"
        );
        parameters.push(Box::new(bin_id));
        parameters.push(Box::new(bin_id));
        let parameter_refs = parameters
            .iter()
            .map(|parameter| parameter.as_ref())
            .collect::<Vec<&dyn ToSql>>();
        let mut match_statement = conn.prepare(&sql)?;
        let matching_ids = match_statement
            .query_map(parameter_refs.as_slice(), |row| row.get::<_, i64>(0))?
            .collect::<Result<Vec<_>>>()?;
        for clip_id in matching_ids {
            if requested_ids.contains(&clip_id) {
                memberships.entry(clip_id).or_default().push(bin_id);
            }
        }
    }

    for clip in clips {
        let bin_ids = clip.bin_ids.get_or_insert_with(Vec::new);
        for bin_id in memberships.remove(&clip.id).unwrap_or_default() {
            if !bin_ids.contains(&bin_id) {
                bin_ids.push(bin_id);
            }
        }
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClipItem {
    pub id: i64,
    pub content_type: String, // "text", "image", "color", "link", "code"
    pub text_content: Option<String>,
    pub html_content: Option<String>,
    pub image_base64: Option<String>,
    pub image_path: Option<String>,
    pub content_hash: String,
    #[serde(alias = "source_app")]
    pub source: String,
    pub is_pinned: bool,
    pub is_protected: bool,
    pub is_transformed: bool,
    pub pin_order: i32,
    pub bin_id: Option<i64>,
    pub bin_ids: Option<Vec<i64>>,
    pub note: Option<String>,
    pub is_trashed: bool,
    pub trashed_at: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub ocr_extractor_ref: Option<String>,
    #[serde(default)]
    pub ocr_extractor_name: Option<String>,
    #[serde(default)]
    pub ocr_engine_version: Option<String>,
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
pub struct ContentDetectionRescanReport {
    pub scanned_count: usize,
    pub changed_count: usize,
    pub unchanged_count: usize,
    pub failed_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisClassification {
    pub clip_id: i64,
    pub content_type: String,
    pub detector_ref: String,
    pub source_representation: String,
    pub input_hash: String,
    pub updated_at: String,
}

fn content_detector_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<crate::content_detection::Detector> {
    let patterns_json: String = row.get(5)?;
    let patterns = serde_json::from_str(&patterns_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let stable_ref: String = row.get(1)?;
    let is_builtin: bool = row.get(9)?;
    Ok(crate::content_detection::Detector {
        id: row.get(0)?,
        defaults: is_builtin
            .then(|| crate::content_detection::detector_defaults(&stable_ref))
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActivityLog {
    pub id: i64,
    pub event_type: String,
    pub description: String,
    pub created_at: String,
    pub observed_at: String,
    pub severity_text: String,
    pub category: String,
    pub outcome: String,
    pub attributes: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActivityArchiveEntry {
    pub timestamp: String,
    pub observed_timestamp: String,
    pub event_name: String,
    pub severity_text: String,
    pub body: String,
    pub attributes: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityArchive {
    pub schema_version: u32,
    pub exported_at: String,
    pub resource: serde_json::Map<String, serde_json::Value>,
    pub entries: Vec<ActivityArchiveEntry>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActivityImportReport {
    pub scanned_count: usize,
    pub imported_count: usize,
    pub duplicate_count: usize,
    pub retained_count: usize,
}

fn activity_classification(event_name: &str) -> (&'static str, &'static str, &'static str) {
    let severity = if event_name.contains("failed") || event_name.contains("error") {
        "error"
    } else if event_name.contains("ignored")
        || event_name.contains("skipped")
        || event_name.contains("cancelled")
        || event_name.contains("auto_paused")
    {
        "warn"
    } else {
        "info"
    };
    let category = if event_name.starts_with("clip_")
        || event_name.starts_with("clips_")
        || event_name.starts_with("trash_")
        || event_name.starts_with("note_")
    {
        "clip"
    } else if event_name.starts_with("recording_") || event_name.starts_with("clipboard_") {
        "capture"
    } else if event_name.starts_with("bin_")
        || event_name.starts_with("type_")
        || event_name.starts_with("detector_")
        || event_name.starts_with("content_")
    {
        "organization"
    } else if event_name.starts_with("transform")
        || event_name.starts_with("operation_")
        || event_name.starts_with("intelligence_")
    {
        "transformation"
    } else if event_name.starts_with("setting_") || event_name == "settings_changed" {
        "settings"
    } else if event_name.starts_with("queue_") || event_name.starts_with("hud_") {
        "workflow"
    } else if event_name.starts_with("app_")
        || event_name.starts_with("library_")
        || event_name.starts_with("backup_")
        || event_name.starts_with("data_export_")
        || event_name.starts_with("external_")
    {
        "system"
    } else {
        "general"
    };
    let outcome = if event_name.contains("failed") || event_name.contains("error") {
        "failure"
    } else if event_name.contains("succeeded") || event_name.ends_with("_completed") {
        "success"
    } else {
        "unknown"
    };
    (severity, category, outcome)
}

fn canonical_activity_timestamp(value: &str) -> Result<String> {
    if let Ok(timestamp) = chrono::DateTime::parse_from_rfc3339(value) {
        return Ok(timestamp
            .with_timezone(&chrono::Utc)
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    }
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
        .map(|timestamp| {
            timestamp
                .and_utc()
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        })
        .map_err(|_| {
            rusqlite::Error::InvalidParameterName(
                "Activity contains an invalid stored timestamp".to_string(),
            )
        })
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
    pub shortcut: Option<String>,
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
pub struct DailyStat {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSummary {
    pub total_clips: i64,
    pub total_chars: i64,
    pub top_sources: Vec<SourceStat>,
    pub content_types: Vec<TypeStat>,
    pub daily_activity: Vec<DailyStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipCollectionSummary {
    pub active_count: i64,
    pub trash_count: i64,
    pub pinned_count: i64,
    pub protected_count: i64,
    pub noted_count: i64,
    pub type_counts: Vec<TypeStat>,
    pub source_counts: Vec<SourceStat>,
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
    #[serde(default)]
    pub content_detectors: Vec<crate::content_detection::Detector>,
    #[serde(default)]
    pub content_types: Vec<crate::content_types::ContentTypeDefinition>,
    #[serde(default)]
    pub content_type_groups: Vec<crate::content_types::ContentTypeGroupDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryArchiveInspection {
    pub schema_version: u32,
    pub clip_count: usize,
    pub bin_count: usize,
    pub operation_count: usize,
    pub transform_count: usize,
    pub detector_count: usize,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Pipeline {
    pub id: i64,
    pub stable_ref: String,
    pub name: String,
    pub shortcut: Option<String>,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
    pub steps: Vec<PipelineStep>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SavedTransform {
    pub id: i64,
    pub stable_ref: String,
    pub name: String,
    pub plan: crate::transformation_intent::TransformationPlan,
    pub connection_id: Option<String>,
    #[serde(default)]
    pub shortcut: Option<String>,
    #[serde(default = "default_transform_authoring_kind")]
    pub authoring_kind: String,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
}

fn default_transform_authoring_kind() -> String {
    "intent".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransformAuthoringKind {
    Intent,
    Manual,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransformDefinition {
    pub id: i64,
    pub stable_ref: String,
    pub name: String,
    pub authoring_kind: TransformAuthoringKind,
    pub execution_character: String,
    pub connection_id: Option<String>,
    pub shortcut: Option<String>,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
    pub plan: Option<crate::transformation_intent::TransformationPlan>,
    pub steps: Vec<PipelineStep>,
}

impl From<SavedTransform> for TransformDefinition {
    fn from(transform: SavedTransform) -> Self {
        let execution_character = match transform.plan.execution_character() {
            crate::transformation_intent::ExecutionCharacter::Replayable => "replayable",
            crate::transformation_intent::ExecutionCharacter::Interpretive => "interpretive",
            crate::transformation_intent::ExecutionCharacter::Mixed => "mixed",
        }
        .to_string();
        let is_manual = transform.authoring_kind == "manual";
        let manual_steps = if is_manual {
            transform
                .plan
                .steps
                .iter()
                .enumerate()
                .filter_map(|(position, step)| match &step.executor {
                    crate::transformation_intent::PlannedExecutor::Deterministic {
                        operation_ref,
                        config_json,
                    } => Some(PipelineStep {
                        position: position as i64,
                        operation_ref: operation_ref.clone(),
                        config_json: config_json.clone(),
                        failure_policy: match step.failure_policy {
                            crate::transformation_intent::StepFailurePolicy::Stop => "stop",
                            crate::transformation_intent::StepFailurePolicy::Skip => "skip",
                        }
                        .to_string(),
                    }),
                    crate::transformation_intent::PlannedExecutor::Semantic { .. } => None,
                })
                .collect()
        } else {
            Vec::new()
        };
        Self {
            id: transform.id,
            stable_ref: transform.stable_ref,
            name: transform.name,
            authoring_kind: if is_manual {
                TransformAuthoringKind::Manual
            } else {
                TransformAuthoringKind::Intent
            },
            execution_character,
            connection_id: transform.connection_id,
            shortcut: transform.shortcut,
            revision: transform.revision,
            created_at: transform.created_at,
            updated_at: transform.updated_at,
            plan: (!is_manual).then_some(transform.plan),
            steps: manual_steps,
        }
    }
}

impl From<Pipeline> for TransformDefinition {
    fn from(pipeline: Pipeline) -> Self {
        Self {
            id: pipeline.id,
            stable_ref: pipeline.stable_ref,
            name: pipeline.name,
            authoring_kind: TransformAuthoringKind::Manual,
            execution_character: "replayable".to_string(),
            connection_id: None,
            shortcut: pipeline.shortcut,
            revision: pipeline.revision,
            created_at: pipeline.created_at,
            updated_at: pipeline.updated_at,
            plan: None,
            steps: pipeline.steps,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClipTransformationProvenance {
    pub transform_ref: String,
    pub transform_name: String,
    pub transform_revision: i64,
    pub connection_id: Option<String>,
    pub duration_ms: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransformationExecution {
    pub id: String,
    pub target_kind: String,
    pub target_ref: String,
    pub target_revision: Option<i64>,
    pub source_clip_id: Option<i64>,
    pub trigger_kind: String,
    pub destination_kind: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub status: String,
    pub error_summary: Option<String>,
}

pub struct TransformationExecutionStart<'a> {
    pub target_kind: &'a str,
    pub target_ref: &'a str,
    pub target_revision: Option<i64>,
    pub source_clip_id: Option<i64>,
    pub trigger_kind: &'a str,
    pub destination_kind: &'a str,
    pub input_hash: &'a str,
}

pub struct TransformClipApplication<'a> {
    pub clip_id: i64,
    pub transform_ref: &'a str,
    pub expected_input: &'a str,
    pub output: &'a str,
    pub connection_id: Option<&'a str>,
    pub duration_ms: i64,
    pub bin_move: Option<(Option<i64>, i64)>,
}

pub struct IntelligenceConnectionUpdate<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub provider_kind: &'a str,
    pub endpoint: Option<&'a str>,
    pub model: Option<&'a str>,
    pub credential_ref: Option<&'a str>,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStepInput {
    pub operation_ref: String,
    pub config_json: Option<String>,
    #[serde(default = "default_pipeline_failure_policy")]
    pub failure_policy: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStep {
    pub position: i64,
    pub operation_ref: String,
    pub config_json: Option<String>,
    pub failure_policy: String,
}

fn default_pipeline_failure_policy() -> String {
    "stop".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Operation {
    pub id: i64,
    #[serde(default)]
    pub stable_id: String,
    pub name: String,
    pub op_type: String,
    pub config: Option<String>,
    pub category: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedCustomOperation {
    pub executor_kind: String,
    pub config_json: String,
    pub enabled: bool,
    pub trusted: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IntelligenceConnection {
    pub id: String,
    pub name: String,
    pub provider_kind: String,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub credential_ref: Option<String>,
    pub enabled: bool,
    pub priority: i64,
    pub created_at: String,
    pub updated_at: String,
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

fn insert_default_bins(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT INTO bins (name, icon, color, smart_rule) VALUES ('Screenshots', '📸', '#ec4899', '{\"type\":\"origin_kind\",\"value\":\"screenshot\"}')",
        [],
    )?;
    conn.execute(
        "INSERT INTO bins (name, icon, color, smart_rule) VALUES ('Links and web', 'Link', '#3b82f6', '{\"type\":\"content_type\",\"value\":\"link\"}')",
        [],
    )?;
    conn.execute(
        "INSERT INTO bins (name, icon, color, smart_rule) VALUES ('Code Snippets', 'Code', '#10b981', '{\"type\":\"content_type\",\"value\":\"code\"}')",
        [],
    )?;
    Ok(())
}

fn table_exists(conn: &Connection, name: &str) -> Result<bool> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        params![name],
        |row| row.get(0),
    )
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let names = statement.query_map([], |row| row.get::<_, String>(1))?;
    for name in names {
        if name? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn migrate_legacy_container_schema(conn: &Connection) -> Result<()> {
    // Pre-release databases used "board" for the same concept now consistently named "bin".
    if table_exists(conn, "boards")? && !table_exists(conn, "bins")? {
        conn.execute("ALTER TABLE boards RENAME TO bins", [])?;
    }
    if table_exists(conn, "clips")?
        && column_exists(conn, "clips", "board_id")?
        && !column_exists(conn, "clips", "bin_id")?
    {
        conn.execute("ALTER TABLE clips RENAME COLUMN board_id TO bin_id", [])?;
    }
    if table_exists(conn, "bins")?
        && column_exists(conn, "bins", "board_type")?
        && !column_exists(conn, "bins", "bin_type")?
    {
        conn.execute("ALTER TABLE bins RENAME COLUMN board_type TO bin_type", [])?;
    }
    if table_exists(conn, "clip_boards")? && !table_exists(conn, "clip_bins")? {
        conn.execute("ALTER TABLE clip_boards RENAME TO clip_bins", [])?;
    }
    if table_exists(conn, "clip_bins")?
        && column_exists(conn, "clip_bins", "board_id")?
        && !column_exists(conn, "clip_bins", "bin_id")?
    {
        conn.execute("ALTER TABLE clip_bins RENAME COLUMN board_id TO bin_id", [])?;
    }

    // SQLite cannot rename indexes; replace any pre-release names after their tables move.
    conn.execute("DROP INDEX IF EXISTS idx_clips_board_created", [])?;
    conn.execute("DROP INDEX IF EXISTS idx_clip_boards_board_id", [])?;
    conn.execute("DROP INDEX IF EXISTS idx_clip_boards_clip_id", [])?;
    Ok(())
}

fn migrate_clip_source_schema(conn: &Connection) -> Result<()> {
    if !table_exists(conn, "clips")? {
        return Ok(());
    }
    let has_legacy_column = column_exists(conn, "clips", "source_app")?;
    let has_source_column = column_exists(conn, "clips", "source")?;
    if has_legacy_column && has_source_column {
        return Err(rusqlite::Error::InvalidQuery);
    }
    if !has_legacy_column {
        return Ok(());
    }

    // The FTS table is a derived cache whose schema cannot be altered in place.
    // Remove it and its writers inside the same transaction as the canonical
    // column rename; the normal startup path recreates and rebuilds it below.
    let transaction = conn.unchecked_transaction()?;
    transaction.execute_batch(
        "DROP TRIGGER IF EXISTS clips_ai;
         DROP TRIGGER IF EXISTS clips_ad;
         DROP TRIGGER IF EXISTS clips_au;
         DROP TABLE IF EXISTS clips_fts;
         ALTER TABLE clips RENAME COLUMN source_app TO source;",
    )?;
    if table_exists(&transaction, "bins")? && column_exists(&transaction, "bins", "smart_rule")? {
        transaction.execute(
            "UPDATE bins
             SET smart_rule = replace(smart_rule, '\"source_app\"', '\"source\"')
             WHERE smart_rule LIKE '%\"source_app\"%'",
            [],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

fn migrate_pipelines_to_saved_transforms(conn: &Connection) -> Result<()> {
    if !table_exists(conn, "pipelines")? {
        return Ok(());
    }
    let transaction = conn.unchecked_transaction()?;
    transaction.execute_batch(
        "CREATE TEMP TABLE pipeline_transform_map (
            pipeline_id TEXT PRIMARY KEY,
            transform_id TEXT NOT NULL UNIQUE
        );",
    )?;
    if !column_exists(&transaction, "pipelines", "shortcut")? {
        transaction.execute("ALTER TABLE pipelines ADD COLUMN shortcut TEXT", [])?;
    }
    if !column_exists(&transaction, "pipelines", "revision")? {
        transaction.execute(
            "ALTER TABLE pipelines ADD COLUMN revision INTEGER NOT NULL DEFAULT 1",
            [],
        )?;
    }
    if !column_exists(&transaction, "pipelines", "created_at")? {
        transaction.execute("ALTER TABLE pipelines ADD COLUMN created_at DATETIME", [])?;
        transaction.execute(
            "UPDATE pipelines SET created_at = CURRENT_TIMESTAMP WHERE created_at IS NULL",
            [],
        )?;
    }
    if !column_exists(&transaction, "pipelines", "updated_at")? {
        transaction.execute("ALTER TABLE pipelines ADD COLUMN updated_at DATETIME", [])?;
        transaction.execute(
            "UPDATE pipelines SET updated_at = COALESCE(created_at, CURRENT_TIMESTAMP)
             WHERE updated_at IS NULL",
            [],
        )?;
    }
    if !column_exists(&transaction, "pipeline_steps", "config_json")? {
        transaction.execute("ALTER TABLE pipeline_steps ADD COLUMN config_json TEXT", [])?;
    }
    if !column_exists(&transaction, "pipeline_steps", "failure_policy")? {
        transaction.execute(
            "ALTER TABLE pipeline_steps ADD COLUMN failure_policy TEXT NOT NULL DEFAULT 'stop'",
            [],
        )?;
    }
    let orphaned_step: Option<String> = transaction
        .query_row(
            "SELECT pipeline_id FROM pipeline_steps WHERE NOT EXISTS (
                SELECT 1 FROM pipelines WHERE pipelines.id = pipeline_steps.pipeline_id
             ) LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(reference) = orphaned_step {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "Cannot migrate Pipeline steps: {reference} does not identify a legacy Pipeline"
        )));
    }
    let pipeline_rows = {
        let mut statement = transaction.prepare(
            "SELECT id, name, shortcut, COALESCE(revision, 1),
                    COALESCE(created_at, CURRENT_TIMESTAMP),
                    COALESCE(updated_at, created_at, CURRENT_TIMESTAMP)
             FROM pipelines ORDER BY row_id ASC",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })?
            .collect::<Result<Vec<_>>>()?;
        rows
    };
    for (pipeline_id, name, shortcut, revision, created_at, updated_at) in pipeline_rows {
        let steps = {
            let mut statement = transaction.prepare(
                "SELECT operation_ref, config_json, failure_policy
                 FROM pipeline_steps WHERE pipeline_id = ?1 ORDER BY position ASC",
            )?;
            let rows = statement
                .query_map(params![pipeline_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })?
                .collect::<Result<Vec<_>>>()?;
            rows
        };
        let plan = crate::transformation_intent::TransformationPlan {
            schema_version: crate::transformation_intent::TRANSFORMATION_PLAN_SCHEMA_VERSION,
            intent: format!("Run {name}"),
            summary: name.clone(),
            planning_mode: crate::transformation_intent::IntentPlanningMode::Pinned,
            steps: steps
                .into_iter()
                .map(|(operation_ref, config_json, failure_policy)| {
                    let failure_policy = match failure_policy.as_str() {
                        "stop" => crate::transformation_intent::StepFailurePolicy::Stop,
                        "skip" => crate::transformation_intent::StepFailurePolicy::Skip,
                        value => {
                            return Err(rusqlite::Error::InvalidParameterName(format!(
                                "invalid legacy Pipeline failure policy: {value}"
                            )))
                        }
                    };
                    Ok(crate::transformation_intent::PlannedTransformationStep {
                        name: operation_ref
                            .strip_prefix("builtin:")
                            .or_else(|| operation_ref.strip_prefix("custom:"))
                            .unwrap_or(&operation_ref)
                            .replace('_', " "),
                        rationale: "Manually configured Operation".to_string(),
                        scope: crate::transformation_intent::StepExecutionScope::WholeInput,
                        failure_policy,
                        executor: crate::transformation_intent::PlannedExecutor::Deterministic {
                            operation_ref,
                            config_json,
                        },
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        };
        plan.validate()
            .map_err(rusqlite::Error::InvalidParameterName)?;
        let plan_json = serde_json::to_string(&plan)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let collision: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM saved_transforms WHERE id = ?1)",
            params![pipeline_id],
            |row| row.get(0),
        )?;
        let transform_id = if collision {
            transaction.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))?
        } else {
            pipeline_id.clone()
        };
        transaction.execute(
            "INSERT INTO saved_transforms
                (id, name, plan_json, connection_id, shortcut, authoring_kind, revision, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, ?4, 'manual', ?5, ?6, ?7)",
            params![
                transform_id,
                name,
                plan_json,
                shortcut,
                revision,
                created_at,
                updated_at
            ],
        )?;
        transaction.execute(
            "INSERT INTO pipeline_transform_map (pipeline_id, transform_id) VALUES (?1, ?2)",
            params![pipeline_id, transform_id],
        )?;
    }

    let unmapped_reference = |table: &str, reference: &str| {
        rusqlite::Error::InvalidParameterName(format!(
            "Cannot migrate {table}: {reference} does not identify a legacy Pipeline"
        ))
    };

    if column_exists(&transaction, "bins", "default_pipeline_id")? {
        let invalid: Option<String> = transaction
            .query_row(
                "SELECT default_pipeline_id FROM bins
                 WHERE default_pipeline_id IS NOT NULL AND NOT EXISTS (
                    SELECT 1 FROM pipeline_transform_map
                    WHERE pipeline_id = replace(bins.default_pipeline_id, 'pipeline:', '')
                 ) LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(reference) = invalid {
            return Err(unmapped_reference("Bins", &reference));
        }
        transaction.execute(
            "UPDATE bins SET default_transform_id = (
                SELECT transform_id FROM pipeline_transform_map
                WHERE pipeline_id = replace(bins.default_pipeline_id, 'pipeline:', '')
             ) WHERE default_pipeline_id IS NOT NULL
               AND EXISTS (
                SELECT 1 FROM pipeline_transform_map
                WHERE pipeline_id = replace(bins.default_pipeline_id, 'pipeline:', '')
             )",
            [],
        )?;
    }
    let invalid_provenance: Option<String> = transaction
        .query_row(
            "SELECT transform_ref FROM clip_transformations
             WHERE transform_ref LIKE 'pipeline:%' AND NOT EXISTS (
                SELECT 1 FROM pipeline_transform_map
                WHERE pipeline_id = replace(clip_transformations.transform_ref, 'pipeline:', '')
             ) LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(reference) = invalid_provenance {
        return Err(unmapped_reference("clip provenance", &reference));
    }
    transaction.execute(
        "UPDATE clip_transformations SET
            transform_id = (
                SELECT transform_id FROM pipeline_transform_map
                WHERE pipeline_id = replace(clip_transformations.transform_ref, 'pipeline:', '')
            ),
            transform_ref = 'transform:' || (
                SELECT transform_id FROM pipeline_transform_map
                WHERE pipeline_id = replace(clip_transformations.transform_ref, 'pipeline:', '')
            )
         WHERE transform_ref LIKE 'pipeline:%'",
        [],
    )?;
    let invalid_execution: Option<String> = transaction
        .query_row(
            "SELECT target_ref FROM transformation_executions
             WHERE target_kind = 'pipeline' AND NOT EXISTS (
                SELECT 1 FROM pipeline_transform_map
                WHERE pipeline_id = replace(transformation_executions.target_ref, 'pipeline:', '')
             ) LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(reference) = invalid_execution {
        return Err(unmapped_reference("execution history", &reference));
    }
    transaction.execute(
        "UPDATE transformation_executions
         SET target_kind = 'transform', target_ref = 'transform:' || (
            SELECT transform_id FROM pipeline_transform_map
            WHERE pipeline_id = replace(transformation_executions.target_ref, 'pipeline:', '')
         ) WHERE target_kind = 'pipeline' AND EXISTS (
            SELECT 1 FROM pipeline_transform_map
            WHERE pipeline_id = replace(transformation_executions.target_ref, 'pipeline:', '')
         )",
        [],
    )?;
    let invalid_last_used: Option<String> = transaction
        .query_row(
            "SELECT value FROM settings
             WHERE key = 'lastExecutedPipelineRef' AND NOT EXISTS (
                SELECT 1 FROM pipeline_transform_map
                WHERE pipeline_id = replace(settings.value, 'pipeline:', '')
             ) LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(reference) = invalid_last_used {
        return Err(unmapped_reference("last-used setting", &reference));
    }
    transaction.execute(
        "UPDATE settings SET value = 'transform:' || (
            SELECT transform_id FROM pipeline_transform_map
            WHERE pipeline_id = replace(settings.value, 'pipeline:', '')
         ) WHERE key = 'lastExecutedPipelineRef' AND EXISTS (
            SELECT 1 FROM pipeline_transform_map
            WHERE pipeline_id = replace(settings.value, 'pipeline:', '')
         )",
        [],
    )?;
    transaction.execute(
        "INSERT INTO settings (key, value)
         SELECT 'lastExecutedTransformRef', value FROM settings
         WHERE key = 'lastExecutedPipelineRef'
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [],
    )?;
    transaction.execute(
        "DELETE FROM settings WHERE key = 'lastExecutedPipelineRef'",
        [],
    )?;

    if table_exists(&transaction, "automations")?
        && column_exists(&transaction, "automations", "pipeline_id")?
    {
        let invalid_automation: Option<String> = transaction
            .query_row(
                "SELECT pipeline_id FROM automations WHERE NOT EXISTS (
                    SELECT 1 FROM pipeline_transform_map
                    WHERE pipeline_id = automations.pipeline_id
                 ) LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(reference) = invalid_automation {
            return Err(unmapped_reference("Automations", &reference));
        }
        let orphaned_condition: Option<String> = transaction
            .query_row(
                "SELECT automation_id FROM automation_conditions WHERE NOT EXISTS (
                    SELECT 1 FROM automations
                    WHERE automations.id = automation_conditions.automation_id
                 ) LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(reference) = orphaned_condition {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "Cannot migrate Automation conditions: {reference} does not identify an Automation"
            )));
        }
        transaction.execute_batch(
            "ALTER TABLE automation_conditions RENAME TO automation_conditions_pipeline_legacy;
             ALTER TABLE automations RENAME TO automations_pipeline_legacy;
             CREATE TABLE automations (
                row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                id TEXT NOT NULL UNIQUE DEFAULT (lower(hex(randomblob(16)))),
                name TEXT NOT NULL,
                trigger_kind TEXT NOT NULL CHECK (trigger_kind IN ('capture', 'copy', 'paste')),
                transform_id TEXT NOT NULL REFERENCES saved_transforms(id) ON DELETE RESTRICT,
                enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
                trusted INTEGER NOT NULL DEFAULT 0 CHECK (trusted IN (0, 1)),
                priority INTEGER NOT NULL DEFAULT 0,
                action_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(action_json)),
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
             );
             INSERT INTO automations
                (row_id, id, name, trigger_kind, transform_id, enabled, trusted,
                 priority, action_json, created_at, updated_at)
             SELECT legacy.row_id, legacy.id, legacy.name, legacy.trigger_kind,
                    mapping.transform_id, legacy.enabled, legacy.trusted,
                    legacy.priority, legacy.action_json, legacy.created_at, legacy.updated_at
             FROM automations_pipeline_legacy AS legacy
             JOIN pipeline_transform_map AS mapping ON mapping.pipeline_id = legacy.pipeline_id;
             CREATE TABLE automation_conditions (
                id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                automation_id TEXT NOT NULL REFERENCES automations(id) ON DELETE CASCADE,
                position INTEGER NOT NULL CHECK (position >= 0),
                condition_kind TEXT NOT NULL,
                config_json TEXT NOT NULL CHECK (json_valid(config_json)),
                UNIQUE (automation_id, position)
             );
             INSERT INTO automation_conditions
                (id, automation_id, position, condition_kind, config_json)
             SELECT conditions.id, conditions.automation_id, conditions.position,
                    conditions.condition_kind, conditions.config_json
             FROM automation_conditions_pipeline_legacy AS conditions
             JOIN automations ON automations.id = conditions.automation_id;
             DROP TABLE automation_conditions_pipeline_legacy;
             DROP TABLE automations_pipeline_legacy;",
        )?;
    }
    transaction.execute_batch(
        "DROP TRIGGER IF EXISTS library_items_pipeline_insert;
         DROP TRIGGER IF EXISTS library_items_pipeline_update;
         DROP TRIGGER IF EXISTS library_items_pipeline_delete;
         DROP TRIGGER IF EXISTS custom_operation_delete_guard;
         DROP TABLE pipeline_steps;
         DROP TABLE pipelines;
         DROP TABLE pipeline_transform_map;",
    )?;
    transaction.commit()?;
    Ok(())
}

fn configure_connection(conn: &Connection) -> Result<()> {
    conn.set_db_config(rusqlite::config::DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    let _ = conn.pragma_update(None, "synchronous", "NORMAL");
    let _ = conn.pragma_update(None, "temp_store", "MEMORY");
    let _ = conn.pragma_update(None, "wal_autocheckpoint", "500");
    Ok(())
}

impl DbState {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let conn = Connection::open(&db_path)?;
        configure_connection(&conn)?;
        let state = DbState {
            conn: Mutex::new(conn),
            path: Mutex::new(db_path),
        };
        state.init_tables()?;
        Ok(state)
    }

    pub fn database_path(&self) -> PathBuf {
        self.path.lock().clone()
    }

    pub fn create_full_backup(
        &self,
        destination_path: &Path,
        client_state_json: Option<&str>,
        window_state_json: Option<&str>,
    ) -> Result<FullBackupReport> {
        if destination_path == self.database_path() {
            return Err(rusqlite::Error::InvalidPath(destination_path.to_path_buf()));
        }
        validate_backup_json(client_state_json, "Backup UI state")?;
        validate_backup_json(window_state_json, "Backup window state")?;
        let parent = destination_path
            .parent()
            .ok_or_else(|| rusqlite::Error::InvalidPath(destination_path.to_path_buf()))?;
        fs::create_dir_all(parent)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let temporary = parent.join(format!(
            ".pasted-full-backup-{}-{}.tmp",
            std::process::id(),
            chrono::Utc::now().timestamp_millis()
        ));
        if temporary.exists() {
            fs::remove_file(&temporary)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        }

        let created_at = chrono::Utc::now().to_rfc3339();
        let source = self.conn.lock();
        let _ = source.pragma_update(None, "wal_checkpoint", "PASSIVE");
        let mut destination = Connection::open(&temporary)?;
        configure_connection(&destination)?;
        {
            let backup = rusqlite::backup::Backup::new(&source, &mut destination)?;
            backup.run_to_completion(128, std::time::Duration::from_millis(5), None)?;
        }
        let effective_client_state = client_state_json.map(str::to_owned).or_else(|| {
            destination
                .query_row(
                    "SELECT value FROM settings WHERE key = 'backedUpClientState'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .ok()
                .flatten()
        });
        destination.execute_batch(
            "DROP TABLE IF EXISTS pasted_backup_manifest;
             CREATE TABLE pasted_backup_manifest (
                format_version INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                app_version TEXT NOT NULL,
                platform TEXT NOT NULL,
                client_state_json TEXT,
                window_state_json TEXT,
                external_state_notice TEXT NOT NULL
             );",
        )?;
        destination.execute(
            "INSERT INTO pasted_backup_manifest
                (format_version, created_at, app_version, platform, client_state_json,
                 window_state_json, external_state_notice)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                FULL_BACKUP_FORMAT_VERSION,
                created_at,
                env!("CARGO_PKG_VERSION"),
                std::env::consts::OS,
                effective_client_state,
                window_state_json,
                "Copied file clips contain paths to original files rather than copies of those files. Paths are preserved. API keys and passwords remain in their credential stores."
            ],
        )?;
        let _ = destination.pragma_update(None, "wal_checkpoint", "TRUNCATE");
        let integrity: String =
            destination.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if integrity != "ok" {
            drop(destination);
            let _ = fs::remove_file(&temporary);
            return Err(rusqlite::Error::InvalidQuery);
        }
        drop(destination);
        drop(source);

        if destination_path.exists() {
            fs::remove_file(destination_path)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        }
        fs::rename(&temporary, destination_path)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(destination_path, fs::Permissions::from_mode(0o600))
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        }
        let size_bytes = fs::metadata(destination_path)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?
            .len();
        Ok(FullBackupReport {
            path: destination_path.to_string_lossy().into_owned(),
            created_at,
            size_bytes,
        })
    }

    pub fn restore_full_backup(
        &self,
        backup_path: &Path,
        current_client_state_json: Option<&str>,
        current_window_state_json: Option<&str>,
    ) -> Result<(FullRestoreReport, Option<String>, Option<String>)> {
        let (source, manifest) = self.open_validated_full_backup(backup_path)?;

        let current_path = self.database_path();
        let parent = current_path
            .parent()
            .ok_or_else(|| rusqlite::Error::InvalidPath(current_path.clone()))?;
        let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S-%3f");
        let recovery_path = parent.join(format!("Pasted_Pre_Restore_{stamp}.pastedbackup"));
        self.create_full_backup(
            &recovery_path,
            current_client_state_json,
            current_window_state_json,
        )?;

        let temporary = parent.join(format!(
            ".pasted-full-restore-{}-{}.tmp",
            std::process::id(),
            chrono::Utc::now().timestamp_millis()
        ));
        let mut restored = Connection::open(&temporary)?;
        configure_connection(&restored)?;
        {
            let backup = rusqlite::backup::Backup::new(&source, &mut restored)?;
            backup.run_to_completion(128, std::time::Duration::from_millis(5), None)?;
        }
        drop(restored);
        drop(source);

        // Opening through DbState applies any forward migrations before the live
        // library is replaced. A failed migration leaves the current library intact.
        let migrated = DbState::new(temporary.clone())?;
        if let Some(client_state) = manifest.client_state_json.as_deref() {
            migrated.save_setting(PENDING_CLIENT_STATE_SETTING, client_state)?;
        }
        let migrated_integrity: String =
            migrated
                .conn
                .lock()
                .query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if migrated_integrity != "ok" {
            drop(migrated);
            let _ = fs::remove_file(&temporary);
            return Err(rusqlite::Error::InvalidQuery);
        }
        let _ = migrated
            .conn
            .lock()
            .pragma_update(None, "wal_checkpoint", "TRUNCATE");
        drop(migrated);

        let mut current = self.conn.lock();
        let _ = current.pragma_update(None, "wal_checkpoint", "TRUNCATE");
        let placeholder = Connection::open_in_memory()?;
        let previous = std::mem::replace(&mut *current, placeholder);
        drop(previous);
        crate::library_storage::remove_database_files(&current_path);
        let activate_result = fs::rename(&temporary, &current_path);
        if let Err(error) = activate_result {
            let _ = fs::copy(&recovery_path, &current_path);
            let replacement = Connection::open(&current_path)?;
            configure_connection(&replacement)?;
            *current = replacement;
            return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(error)));
        }
        let replacement = match Connection::open(&current_path).and_then(|connection| {
            configure_connection(&connection)?;
            Ok(connection)
        }) {
            Ok(connection) => connection,
            Err(error) => {
                let _ = fs::copy(&recovery_path, &current_path);
                let fallback = Connection::open(&current_path)?;
                configure_connection(&fallback)?;
                *current = fallback;
                return Err(error);
            }
        };
        *current = replacement;

        Ok((
            FullRestoreReport {
                recovery_path: recovery_path.to_string_lossy().into_owned(),
                backup_created_at: manifest.created_at,
            },
            manifest.client_state_json,
            manifest.window_state_json,
        ))
    }

    pub fn inspect_full_backup(&self, backup_path: &Path) -> Result<FullBackupInspection> {
        let (_source, manifest) = self.open_validated_full_backup(backup_path)?;
        let size_bytes = fs::metadata(backup_path)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?
            .len();
        Ok(FullBackupInspection {
            format_version: manifest.format_version,
            created_at: manifest.created_at,
            size_bytes,
        })
    }

    fn open_validated_full_backup(
        &self,
        backup_path: &Path,
    ) -> Result<(Connection, FullBackupManifest)> {
        if !backup_path.is_file() || backup_path == self.database_path() {
            return Err(rusqlite::Error::InvalidPath(backup_path.to_path_buf()));
        }
        let source = Connection::open_with_flags(backup_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let integrity: String = source.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if integrity != "ok" {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let manifest = source
            .query_row(
                "SELECT format_version, created_at, client_state_json, window_state_json
                 FROM pasted_backup_manifest LIMIT 1",
                [],
                |row| {
                    Ok(FullBackupManifest {
                        format_version: row.get(0)?,
                        created_at: row.get(1)?,
                        client_state_json: row.get(2)?,
                        window_state_json: row.get(3)?,
                    })
                },
            )
            .map_err(|_| {
                rusqlite::Error::InvalidParameterName(
                    "The selected file is not a complete Pasted backup".to_string(),
                )
            })?;
        if manifest.format_version != FULL_BACKUP_FORMAT_VERSION {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "Unsupported full-backup format version {}",
                manifest.format_version
            )));
        }
        validate_backup_json(manifest.client_state_json.as_deref(), "Backup UI state")?;
        validate_backup_json(manifest.window_state_json.as_deref(), "Backup window state")?;
        Ok((source, manifest))
    }

    pub fn consume_pending_full_restore_client_state(&self) -> Result<Option<String>> {
        let conn = self.conn.lock();
        let state = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![PENDING_CLIENT_STATE_SETTING],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if state.is_some() {
            conn.execute(
                "DELETE FROM settings WHERE key = ?1",
                params![PENDING_CLIENT_STATE_SETTING],
            )?;
        }
        Ok(state)
    }

    pub fn relocate_database(&self, target_path: PathBuf) -> Result<PathBuf> {
        let previous_path = self.database_path();
        if previous_path == target_path {
            return Ok(previous_path);
        }
        if target_path.exists() {
            return Err(rusqlite::Error::InvalidPath(target_path));
        }
        let parent = target_path
            .parent()
            .ok_or_else(|| rusqlite::Error::InvalidPath(target_path.clone()))?;
        fs::create_dir_all(parent).map_err(|_| rusqlite::Error::InvalidPath(parent.into()))?;
        let temporary = parent.join(format!(".pasted-library-{}.tmp", std::process::id()));
        if temporary.exists() {
            fs::remove_file(&temporary)
                .map_err(|_| rusqlite::Error::InvalidPath(temporary.clone()))?;
        }

        let mut source = self.conn.lock();
        let _ = source.pragma_update(None, "wal_checkpoint", "TRUNCATE");
        let mut destination = Connection::open(&temporary)?;
        configure_connection(&destination)?;
        {
            let backup = rusqlite::backup::Backup::new(&source, &mut destination)?;
            backup.run_to_completion(128, std::time::Duration::from_millis(5), None)?;
        }
        let integrity: String =
            destination.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if integrity != "ok" {
            let _ = fs::remove_file(&temporary);
            return Err(rusqlite::Error::InvalidQuery);
        }
        drop(destination);
        fs::rename(&temporary, &target_path)
            .map_err(|_| rusqlite::Error::InvalidPath(target_path.clone()))?;
        let replacement = Connection::open(&target_path)?;
        configure_connection(&replacement)?;
        *source = replacement;
        *self.path.lock() = target_path;
        Ok(previous_path)
    }

    pub fn switch_to_database(&self, database_path: PathBuf) -> Result<()> {
        let replacement = Connection::open(&database_path)?;
        configure_connection(&replacement)?;
        let integrity: String =
            replacement.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if integrity != "ok" {
            return Err(rusqlite::Error::InvalidQuery);
        }
        *self.conn.lock() = replacement;
        *self.path.lock() = database_path;
        Ok(())
    }

    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock();

        // High-performance SQLite configuration
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        let _ = conn.pragma_update(None, "synchronous", "NORMAL");
        let _ = conn.pragma_update(None, "temp_store", "MEMORY");
        let _ = conn.pragma_update(None, "wal_autocheckpoint", "500");
        let _ = conn.pragma_update(None, "auto_vacuum", "INCREMENTAL");
        let _ = conn.pragma_update(None, "optimize", "");

        migrate_legacy_container_schema(&conn)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS clips (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content_type TEXT NOT NULL,
                text_content TEXT,
                html_content TEXT,
                image_base64 TEXT,
                content_hash TEXT UNIQUE NOT NULL,
                source TEXT DEFAULT 'Unknown',
                is_pinned INTEGER DEFAULT 0,
                bin_id INTEGER,
                note TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // High-speed composite indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_pinned_created ON clips (is_pinned, created_at DESC)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_bin_created ON clips (bin_id, created_at DESC)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_hash ON clips (content_hash)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS bins (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                icon TEXT DEFAULT 'Folder',
                color TEXT DEFAULT 'default',
                smart_rule TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // Migrations if existing tables don't have new columns
        let _ = conn.execute("ALTER TABLE clips ADD COLUMN note TEXT", []);
        let _ = conn.execute(
            "ALTER TABLE clips ADD COLUMN is_trashed INTEGER DEFAULT 0",
            [],
        );
        let _ = conn.execute("ALTER TABLE clips ADD COLUMN trashed_at DATETIME", []);
        let _ = conn.execute(
            "ALTER TABLE clips ADD COLUMN is_protected INTEGER DEFAULT 0",
            [],
        );
        let _ = conn.execute("ALTER TABLE clips ADD COLUMN image_path TEXT", []);
        let _ = conn.execute(
            "ALTER TABLE clips ADD COLUMN pin_order INTEGER DEFAULT 0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE clips ADD COLUMN current_transformation_id TEXT",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE clips ADD COLUMN ocr_status TEXT NOT NULL DEFAULT 'not_applicable'",
            [],
        );
        let _ = conn.execute("ALTER TABLE clips ADD COLUMN ocr_input_hash TEXT", []);
        let _ = conn.execute("ALTER TABLE clips ADD COLUMN ocr_engine_version TEXT", []);
        let _ = conn.execute("ALTER TABLE clips ADD COLUMN ocr_extractor_ref TEXT", []);
        let _ = conn.execute("ALTER TABLE clips ADD COLUMN ocr_extractor_name TEXT", []);
        let _ = conn.execute("ALTER TABLE clips ADD COLUMN ocr_attempted_at DATETIME", []);
        let _ = conn.execute("ALTER TABLE clips ADD COLUMN ocr_error TEXT", []);
        conn.execute(
            "UPDATE clips
             SET ocr_status = CASE
                    WHEN content_type = 'image' AND COALESCE(text_content, '') != '' THEN 'complete'
                    WHEN content_type = 'image' THEN 'never'
                    ELSE 'not_applicable'
                 END,
                 ocr_input_hash = CASE WHEN content_type = 'image' THEN content_hash ELSE NULL END,
                 ocr_engine_version = CASE
                    WHEN content_type = 'image' AND COALESCE(text_content, '') != '' THEN COALESCE(ocr_engine_version, 'legacy')
                    ELSE ocr_engine_version
                 END
             WHERE content_type = 'image' AND ocr_input_hash IS NULL",
            [],
        )?;
        conn.execute(
            "UPDATE clips
             SET ocr_extractor_ref = CASE ocr_engine_version
                    WHEN 'macos-vision-v1' THEN 'extractor:apple-vision-ocr'
                    ELSE ocr_extractor_ref
                 END,
                 ocr_extractor_name = CASE ocr_engine_version
                    WHEN 'macos-vision-v1' THEN 'Apple Vision OCR'
                    WHEN 'legacy' THEN 'Legacy OCR'
                    ELSE ocr_extractor_name
                 END
             WHERE ocr_status = 'complete' AND ocr_extractor_name IS NULL",
            [],
        )?;
        conn.execute(
            "UPDATE clips SET ocr_status = 'never', ocr_error = NULL
             WHERE content_type = 'image' AND ocr_status IN ('queued', 'running')",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_ocr_backfill
             ON clips (content_type, ocr_status, is_trashed, id)",
            [],
        )?;
        let _ = conn.execute("ALTER TABLE bins ADD COLUMN smart_rule TEXT", []);
        let _ = conn.execute(
            "ALTER TABLE bins ADD COLUMN bin_type TEXT DEFAULT 'category'",
            [],
        );
        let _ = conn.execute("ALTER TABLE bins ADD COLUMN shortcut TEXT", []);

        migrate_clip_source_schema(&conn)?;

        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS clip_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                text_content TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clip_versions_clip_id ON clip_versions(clip_id, created_at DESC)",
            [],
        );
        let _ = conn.execute("ALTER TABLE clip_versions ADD COLUMN context_json TEXT", []);

        conn.execute(
            "CREATE TABLE IF NOT EXISTS clip_analysis_classifications (
                clip_id INTEGER PRIMARY KEY REFERENCES clips(id) ON DELETE CASCADE,
                content_type TEXT NOT NULL,
                detector_ref TEXT NOT NULL,
                source_representation TEXT NOT NULL
                    CHECK (source_representation IN ('original_text', 'searchable_text')),
                input_hash TEXT NOT NULL,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clip_analysis_classification_type
             ON clip_analysis_classifications(content_type, clip_id)",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS clip_analysis_results (
                clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                participant_ref TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                input_hash TEXT NOT NULL,
                format_version INTEGER NOT NULL CHECK(format_version > 0),
                result_json TEXT NOT NULL,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (clip_id, participant_ref)
            )",
            [],
        )?;

        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_trashed ON clips (is_trashed, created_at DESC)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_protected ON clips (is_protected, created_at DESC)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_active_timeline ON clips (is_trashed, is_pinned DESC, created_at DESC)",
            [],
        );

        // FTS5 Full-Text Search Virtual Table Setup
        let fts_res = conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS clips_fts USING fts5(
                text_content,
                note,
                source,
                content='clips',
                content_rowid='id'
            )",
            [],
        );

        if fts_res.is_ok() {
            let _ = conn.execute(
                "CREATE TRIGGER IF NOT EXISTS clips_ai AFTER INSERT ON clips BEGIN
                    INSERT INTO clips_fts(rowid, text_content, note, source)
                    VALUES (new.id, new.text_content, new.note, new.source);
                END;",
                [],
            );
            let _ = conn.execute(
                "CREATE TRIGGER IF NOT EXISTS clips_ad AFTER DELETE ON clips BEGIN
                    INSERT INTO clips_fts(clips_fts, rowid, text_content, note, source)
                    VALUES ('delete', old.id, old.text_content, old.note, old.source);
                END;",
                [],
            );
            let _ = conn.execute(
                "CREATE TRIGGER IF NOT EXISTS clips_au AFTER UPDATE ON clips BEGIN
                    INSERT INTO clips_fts(clips_fts, rowid, text_content, note, source)
                    VALUES ('delete', old.id, old.text_content, old.note, old.source);
                    INSERT INTO clips_fts(rowid, text_content, note, source)
                    VALUES (new.id, new.text_content, new.note, new.source);
                END;",
                [],
            );

            // FTS5 is a derived cache. Rebuild it at startup so an interrupted write or an
            // older trigger implementation cannot leave clip updates failing with a
            // misleading "database disk image is malformed" error.
            let _ = conn.execute("INSERT INTO clips_fts(clips_fts) VALUES('rebuild')", []);
        }

        conn.execute(
            "CREATE TABLE IF NOT EXISTS clip_bins (
                clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                bin_id INTEGER NOT NULL REFERENCES bins(id) ON DELETE CASCADE,
                PRIMARY KEY (clip_id, bin_id)
            )",
            [],
        )?;
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clip_bins_bin_id ON clip_bins (bin_id)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clip_bins_clip_id ON clip_bins (clip_id)",
            [],
        );

        conn.execute(
            "CREATE TABLE IF NOT EXISTS bin_clip_order (
                bin_id INTEGER NOT NULL REFERENCES bins(id) ON DELETE CASCADE,
                clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                position INTEGER NOT NULL CHECK(position >= 0),
                PRIMARY KEY (bin_id, clip_id),
                UNIQUE (bin_id, position)
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_bin_clip_order_position
             ON bin_clip_order (bin_id, position)",
            [],
        )?;

        let _ = conn.execute(
            "INSERT OR IGNORE INTO clip_bins (clip_id, bin_id)
             SELECT id, bin_id FROM clips WHERE bin_id IS NOT NULL",
            [],
        );

        // Trash is deliberately outside the organizational hierarchy. Clean up
        // legacy rows so restored clips never silently reappear in an old Bin.
        let _ = conn.execute(
            "DELETE FROM clip_bins
             WHERE clip_id IN (SELECT id FROM clips WHERE is_trashed = 1)
               AND bin_id IN (
                   SELECT id FROM bins WHERE COALESCE(bin_type, 'category') != 'tag'
               )",
            [],
        );
        let _ = conn.execute("UPDATE clips SET bin_id = NULL WHERE is_trashed = 1", []);

        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS activity_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                description TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                observed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                severity_text TEXT NOT NULL DEFAULT 'info',
                category TEXT NOT NULL DEFAULT 'general',
                outcome TEXT NOT NULL DEFAULT 'unknown',
                attributes_json TEXT NOT NULL DEFAULT '{}'
            )",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE activity_logs ADD COLUMN observed_at DATETIME",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE activity_logs ADD COLUMN severity_text TEXT NOT NULL DEFAULT 'info'",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE activity_logs ADD COLUMN category TEXT NOT NULL DEFAULT 'general'",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE activity_logs ADD COLUMN outcome TEXT NOT NULL DEFAULT 'unknown'",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE activity_logs ADD COLUMN attributes_json TEXT NOT NULL DEFAULT '{}'",
            [],
        );
        let _ = conn.execute(
            "UPDATE activity_logs SET observed_at = created_at WHERE observed_at IS NULL",
            [],
        );
        let _ = conn.execute(
            "UPDATE activity_logs
             SET severity_text = CASE
                    WHEN event_type LIKE '%failed%' OR event_type LIKE '%error%' THEN 'error'
                    WHEN event_type LIKE '%ignored%' OR event_type LIKE '%skipped%'
                      OR event_type LIKE '%cancelled%' OR event_type LIKE '%auto_paused%' THEN 'warn'
                    ELSE severity_text
                 END,
                 category = CASE
                    WHEN event_type LIKE 'clip_%' OR event_type LIKE 'clips_%'
                      OR event_type LIKE 'trash_%' OR event_type LIKE 'note_%' THEN 'clip'
                    WHEN event_type LIKE 'recording_%' OR event_type LIKE 'clipboard_%' THEN 'capture'
                    WHEN event_type LIKE 'bin_%' OR event_type LIKE 'type_%'
                      OR event_type LIKE 'detector_%' OR event_type LIKE 'content_%' THEN 'organization'
                    WHEN event_type LIKE 'transform%' OR event_type LIKE 'operation_%'
                      OR event_type LIKE 'intelligence_%' THEN 'transformation'
                    WHEN event_type LIKE 'setting_%' OR event_type = 'settings_changed' THEN 'settings'
                    WHEN event_type LIKE 'queue_%' OR event_type LIKE 'hud_%' THEN 'workflow'
                    WHEN event_type LIKE 'app_%' OR event_type LIKE 'library_%'
                      OR event_type LIKE 'backup_%' OR event_type LIKE 'external_%' THEN 'system'
                    ELSE category
                 END,
                 outcome = CASE
                    WHEN event_type LIKE '%failed%' OR event_type LIKE '%error%' THEN 'failure'
                    WHEN event_type LIKE '%succeeded%' OR event_type LIKE '%_completed' THEN 'success'
                    ELSE outcome
                 END",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_logs_created ON activity_logs (created_at DESC)",
            [],
        );

        self.init_transformation_tables(&conn)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;
        migrate_pipelines_to_saved_transforms(&conn)?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS content_type_groups (
                id TEXT PRIMARY KEY,
                label TEXT NOT NULL,
                sort_order INTEGER NOT NULL DEFAULT 100,
                is_builtin INTEGER NOT NULL DEFAULT 0,
                is_archived INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS content_types (
                id TEXT PRIMARY KEY,
                label TEXT NOT NULL,
                icon TEXT NOT NULL,
                group_name TEXT NOT NULL,
                is_builtin INTEGER NOT NULL DEFAULT 0,
                is_archived INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_content_types_order
                ON content_types (is_archived, is_builtin DESC, group_name, label);
            CREATE TABLE IF NOT EXISTS content_detectors (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                stable_ref TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                content_type TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                patterns_json TEXT NOT NULL,
                validator TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                priority INTEGER NOT NULL DEFAULT 100,
                is_builtin INTEGER NOT NULL DEFAULT 0,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_content_detectors_order
                ON content_detectors (is_deleted, enabled, priority, id);
            CREATE TABLE IF NOT EXISTS content_extractors (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                stable_ref TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                engine TEXT NOT NULL,
                input_contract TEXT NOT NULL,
                output_contract TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                priority INTEGER NOT NULL DEFAULT 100,
                is_builtin INTEGER NOT NULL DEFAULT 0,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_content_extractors_order
                ON content_extractors (is_deleted, enabled, priority, id);",
        )?;
        for preset in crate::content_types::CONTENT_TYPE_GROUP_PRESETS {
            conn.execute(
                "INSERT OR IGNORE INTO content_type_groups
                    (id, label, sort_order, is_builtin, is_archived)
                 VALUES (?1, ?2, ?3, 1, 0)",
                params![preset.id, preset.label, preset.sort_order],
            )?;
        }
        for preset in crate::content_types::CONTENT_TYPE_PRESETS {
            conn.execute(
                "INSERT OR IGNORE INTO content_types
                    (id, label, icon, group_name, is_builtin, is_archived)
                 VALUES (?1, ?2, ?3, ?4, 1, 0)",
                params![preset.id, preset.label, preset.icon, preset.group],
            )?;
        }
        for preset in crate::content_detection::DETECTOR_PRESETS {
            let patterns_json = serde_json::to_string(&preset.patterns)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            conn.execute(
                "INSERT OR IGNORE INTO content_detectors
                    (stable_ref, name, content_type, description, patterns_json, validator, enabled, priority, is_builtin)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, 1)",
                params![preset.stable_ref, preset.name, preset.content_type, preset.description, patterns_json, preset.validator, preset.priority],
            )?;
        }
        for preset in crate::content_extraction::EXTRACTOR_PRESETS {
            conn.execute(
                "INSERT OR IGNORE INTO content_extractors
                    (stable_ref, name, description, engine, input_contract, output_contract,
                     enabled, priority, is_builtin)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, 1)",
                params![
                    preset.stable_ref,
                    preset.name,
                    preset.description,
                    preset.engine,
                    preset.input_contract,
                    preset.output_contract,
                    preset.priority
                ],
            )?;
        }
        let legacy_type_ids = {
            let mut statement = conn.prepare(
                "SELECT content_type FROM content_detectors
                 UNION SELECT content_type FROM clips
                 ORDER BY content_type",
            )?;
            let ids = statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>>>()?;
            ids
        };
        for id in legacy_type_ids {
            conn.execute(
                "INSERT OR IGNORE INTO content_types
                    (id, label, icon, group_name, is_builtin, is_archived)
                 VALUES (?1, ?2, 'FileText', 'custom', 0, 0)",
                params![id, crate::content_types::fallback_label(&id)],
            )?;
        }
        let detector_migration_applied: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE key = 'contentDetectorRegistryV1')",
            [],
            |row| row.get(0),
        )?;
        if !detector_migration_applied {
            for (setting_key, stable_ref) in [
                ("detectColors", "color"),
                ("detectLinks", "url"),
                ("detectCode", "code"),
            ] {
                let disabled: bool = conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM settings WHERE key = ?1 AND value = 'false')",
                    params![setting_key],
                    |row| row.get(0),
                )?;
                if disabled {
                    conn.execute(
                        "UPDATE content_detectors SET enabled = 0 WHERE stable_ref = ?1",
                        params![stable_ref],
                    )?;
                }
            }
            conn.execute(
                "INSERT INTO schema_migrations (key) VALUES ('contentDetectorRegistryV1')",
                [],
            )?;
        }
        Self::init_library_items(&conn)?;

        // Insert default bins if empty
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM bins", [], |r| r.get(0))
            .unwrap_or(0);
        if count == 0 {
            insert_default_bins(&conn)?;
        }

        Ok(())
    }

    fn init_library_items(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "DROP TRIGGER IF EXISTS library_items_extractor_insert;
            DROP TRIGGER IF EXISTS library_items_extractor_update;
            DROP TRIGGER IF EXISTS library_items_extractor_delete;
            DROP TRIGGER IF EXISTS library_items_detector_insert;
            DROP TRIGGER IF EXISTS library_items_detector_update;
            DROP TRIGGER IF EXISTS library_items_detector_delete;
            DROP TRIGGER IF EXISTS library_items_content_type_update;
            DROP TRIGGER IF EXISTS library_items_content_group_update;
            DROP TRIGGER IF EXISTS library_items_operation_insert;
            DROP TRIGGER IF EXISTS library_items_operation_update;
            DROP TRIGGER IF EXISTS library_items_operation_delete;
            DROP TRIGGER IF EXISTS library_items_pipeline_insert;
            DROP TRIGGER IF EXISTS library_items_pipeline_update;
            DROP TRIGGER IF EXISTS library_items_pipeline_delete;
            DROP TRIGGER IF EXISTS library_items_transform_insert;
            DROP TRIGGER IF EXISTS library_items_transform_update;
            DROP TRIGGER IF EXISTS library_items_transform_delete;
            DROP TABLE IF EXISTS library_items;
            CREATE TABLE library_items (
                stable_ref TEXT PRIMARY KEY,
                kind TEXT NOT NULL CHECK (kind IN ('inspector', 'extractor', 'detector', 'enricher', 'operation', 'transform')),
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                group_label TEXT,
                icon TEXT NOT NULL DEFAULT 'FileText',
                enabled INTEGER CHECK (enabled IS NULL OR enabled IN (0, 1)),
                is_builtin INTEGER NOT NULL DEFAULT 0 CHECK (is_builtin IN (0, 1)),
                is_archived INTEGER NOT NULL DEFAULT 0 CHECK (is_archived IN (0, 1)),
                sort_order INTEGER,
                revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
                input_contract TEXT NOT NULL DEFAULT 'text',
                output_contract TEXT NOT NULL DEFAULT 'preserve_type',
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_library_items_kind_order
                ON library_items(kind, is_archived, sort_order, name);

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            VALUES ('inspector:structure-v1', 'inspector', 'Structure',
                    'Measures stable clip structure without retaining clipboard contents.',
                    'Content Analysis', 'ScanSearch', NULL, 1, 0, 0, 1,
                    'clip', 'structural_metadata', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            VALUES ('enricher:smart-actions-v1', 'enricher', 'Smart Actions',
                    'Recommends saved Transforms from content-free analysis signals.',
                    'Content Analysis', 'Lightbulb', NULL, 1, 0, 0, 1,
                    'analyzable_text+classification+structural_metadata', 'recommendations',
                    CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            SELECT stable_ref, 'extractor', name, description, 'Content Analysis',
                   'ScanText', enabled, is_builtin, is_deleted, priority, 1,
                   input_contract, output_contract, created_at, updated_at
            FROM content_extractors
            WHERE 1 = 1
            ON CONFLICT(stable_ref) DO UPDATE SET
                name=excluded.name, description=excluded.description,
                enabled=excluded.enabled, is_builtin=excluded.is_builtin,
                is_archived=excluded.is_archived, sort_order=excluded.sort_order,
                input_contract=excluded.input_contract,
                output_contract=excluded.output_contract, updated_at=excluded.updated_at;

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            SELECT detectors.stable_ref, 'detector', detectors.name, detectors.description,
                   groups.label, types.icon, detectors.enabled, detectors.is_builtin,
                   detectors.is_deleted, detectors.priority, 1, 'text',
                   'set_type:' || detectors.content_type, detectors.created_at, detectors.updated_at
            FROM content_detectors AS detectors
            LEFT JOIN content_types AS types ON types.id = detectors.content_type
            LEFT JOIN content_type_groups AS groups ON groups.id = types.group_name
            WHERE 1 = 1
            ON CONFLICT(stable_ref) DO UPDATE SET
                name=excluded.name, description=excluded.description,
                group_label=excluded.group_label, icon=excluded.icon,
                enabled=excluded.enabled, is_builtin=excluded.is_builtin,
                is_archived=excluded.is_archived, sort_order=excluded.sort_order,
                output_contract=excluded.output_contract, updated_at=excluded.updated_at;

            INSERT INTO library_items
                (stable_ref, kind, name, group_label, icon, enabled, is_builtin,
                 is_archived, sort_order, revision, input_contract, output_contract,
                 created_at, updated_at)
            SELECT 'custom:' || id, 'operation', name, category, 'Wrench', enabled, 0,
                   0, row_id, 1, 'text', 'preserve_type', created_at, updated_at
            FROM custom_operations
            WHERE 1 = 1
            ON CONFLICT(stable_ref) DO UPDATE SET
                name=excluded.name, group_label=excluded.group_label,
                enabled=excluded.enabled, sort_order=excluded.sort_order,
                updated_at=excluded.updated_at;

            INSERT INTO library_items
                (stable_ref, kind, name, group_label, icon, enabled, is_builtin,
                 is_archived, sort_order, revision, input_contract, output_contract,
                 created_at, updated_at)
            SELECT 'transform:' || id, 'transform', name,
                   CASE authoring_kind WHEN 'manual' THEN 'Local Transforms' ELSE 'Transforms' END,
                   'Workflow', NULL, 0,
                   0, row_id, revision, 'text', 'preserve_type', created_at, updated_at
            FROM saved_transforms
            WHERE 1 = 1
            ON CONFLICT(stable_ref) DO UPDATE SET
                name=excluded.name, sort_order=excluded.sort_order,
                revision=excluded.revision, updated_at=excluded.updated_at;

            DROP TRIGGER IF EXISTS library_items_extractor_insert;
            DROP TRIGGER IF EXISTS library_items_extractor_update;
            DROP TRIGGER IF EXISTS library_items_extractor_delete;
            DROP TRIGGER IF EXISTS library_items_detector_insert;
            DROP TRIGGER IF EXISTS library_items_detector_update;
            DROP TRIGGER IF EXISTS library_items_detector_delete;
            DROP TRIGGER IF EXISTS library_items_content_type_update;
            DROP TRIGGER IF EXISTS library_items_content_group_update;
            DROP TRIGGER IF EXISTS library_items_operation_insert;
            DROP TRIGGER IF EXISTS library_items_operation_update;
            DROP TRIGGER IF EXISTS library_items_operation_delete;
            DROP TRIGGER IF EXISTS library_items_pipeline_insert;
            DROP TRIGGER IF EXISTS library_items_pipeline_update;
            DROP TRIGGER IF EXISTS library_items_pipeline_delete;

            CREATE TRIGGER library_items_extractor_insert AFTER INSERT ON content_extractors BEGIN
              DELETE FROM library_items WHERE stable_ref=NEW.stable_ref;
              INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled, is_builtin,
                 is_archived, sort_order, revision, input_contract, output_contract, created_at, updated_at)
              VALUES (NEW.stable_ref, 'extractor', NEW.name, NEW.description, 'Content Analysis',
                      'ScanText', NEW.enabled, NEW.is_builtin, NEW.is_deleted, NEW.priority,
                      1, NEW.input_contract, NEW.output_contract, NEW.created_at, NEW.updated_at);
            END;
            CREATE TRIGGER library_items_extractor_update AFTER UPDATE ON content_extractors BEGIN
              DELETE FROM library_items WHERE stable_ref=OLD.stable_ref OR stable_ref=NEW.stable_ref;
              INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled, is_builtin,
                 is_archived, sort_order, revision, input_contract, output_contract, created_at, updated_at)
              VALUES (NEW.stable_ref, 'extractor', NEW.name, NEW.description, 'Content Analysis',
                      'ScanText', NEW.enabled, NEW.is_builtin, NEW.is_deleted, NEW.priority,
                      1, NEW.input_contract, NEW.output_contract, NEW.created_at, NEW.updated_at);
            END;
            CREATE TRIGGER library_items_extractor_delete AFTER DELETE ON content_extractors BEGIN
              DELETE FROM library_items WHERE stable_ref=OLD.stable_ref;
            END;
            CREATE TRIGGER library_items_detector_insert AFTER INSERT ON content_detectors BEGIN
              DELETE FROM library_items WHERE stable_ref=NEW.stable_ref;
              INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled, is_builtin,
                 is_archived, sort_order, revision, input_contract, output_contract, created_at, updated_at)
              SELECT NEW.stable_ref, 'detector', NEW.name, NEW.description, groups.label,
                     types.icon, NEW.enabled, NEW.is_builtin, NEW.is_deleted, NEW.priority,
                     1, 'text', 'set_type:' || NEW.content_type, NEW.created_at, NEW.updated_at
              FROM content_types AS types LEFT JOIN content_type_groups AS groups ON groups.id=types.group_name
              WHERE types.id=NEW.content_type;
            END;
            CREATE TRIGGER library_items_detector_update AFTER UPDATE ON content_detectors BEGIN
              DELETE FROM library_items WHERE stable_ref=OLD.stable_ref OR stable_ref=NEW.stable_ref;
              INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled, is_builtin,
                 is_archived, sort_order, revision, input_contract, output_contract, created_at, updated_at)
              SELECT NEW.stable_ref, 'detector', NEW.name, NEW.description, groups.label,
                     types.icon, NEW.enabled, NEW.is_builtin, NEW.is_deleted, NEW.priority,
                     1, 'text', 'set_type:' || NEW.content_type, NEW.created_at, NEW.updated_at
              FROM content_types AS types LEFT JOIN content_type_groups AS groups ON groups.id=types.group_name
              WHERE types.id=NEW.content_type;
            END;
            CREATE TRIGGER library_items_detector_delete AFTER DELETE ON content_detectors BEGIN
              DELETE FROM library_items WHERE stable_ref=OLD.stable_ref;
            END;
            CREATE TRIGGER library_items_content_type_update AFTER UPDATE ON content_types BEGIN
              UPDATE library_items SET
                icon=NEW.icon,
                group_label=(SELECT label FROM content_type_groups WHERE id=NEW.group_name),
                output_contract='set_type:'||NEW.id,
                updated_at=CURRENT_TIMESTAMP
              WHERE kind='detector' AND stable_ref IN (
                SELECT stable_ref FROM content_detectors WHERE content_type=NEW.id
              );
            END;
            CREATE TRIGGER library_items_content_group_update AFTER UPDATE ON content_type_groups BEGIN
              UPDATE library_items SET group_label=NEW.label,updated_at=CURRENT_TIMESTAMP
              WHERE kind='detector' AND stable_ref IN (
                SELECT detectors.stable_ref FROM content_detectors AS detectors
                JOIN content_types AS types ON types.id=detectors.content_type
                WHERE types.group_name=NEW.id
              );
            END;
            CREATE TRIGGER library_items_operation_insert AFTER INSERT ON custom_operations BEGIN
              INSERT OR REPLACE INTO library_items (stable_ref,kind,name,group_label,icon,enabled,is_builtin,is_archived,sort_order,revision,input_contract,output_contract,created_at,updated_at)
              VALUES ('custom:'||NEW.id,'operation',NEW.name,NEW.category,'Wrench',NEW.enabled,0,0,NEW.row_id,1,'text','preserve_type',NEW.created_at,NEW.updated_at);
            END;
            CREATE TRIGGER library_items_operation_update AFTER UPDATE ON custom_operations BEGIN
              UPDATE library_items SET name=NEW.name,group_label=NEW.category,enabled=NEW.enabled,updated_at=NEW.updated_at WHERE stable_ref='custom:'||NEW.id;
            END;
            CREATE TRIGGER library_items_operation_delete AFTER DELETE ON custom_operations BEGIN
              DELETE FROM library_items WHERE stable_ref='custom:'||OLD.id;
            END;
            CREATE TRIGGER library_items_transform_insert AFTER INSERT ON saved_transforms BEGIN
              INSERT OR REPLACE INTO library_items (stable_ref,kind,name,group_label,icon,enabled,is_builtin,is_archived,sort_order,revision,input_contract,output_contract,created_at,updated_at)
              VALUES ('transform:'||NEW.id,'transform',NEW.name,CASE NEW.authoring_kind WHEN 'manual' THEN 'Local Transforms' ELSE 'Transforms' END,'Workflow',NULL,0,0,NEW.row_id,NEW.revision,'text','preserve_type',NEW.created_at,NEW.updated_at);
            END;
            CREATE TRIGGER library_items_transform_update AFTER UPDATE ON saved_transforms BEGIN
              UPDATE library_items SET name=NEW.name,group_label=CASE NEW.authoring_kind WHEN 'manual' THEN 'Local Transforms' ELSE 'Transforms' END,revision=NEW.revision,updated_at=NEW.updated_at WHERE stable_ref='transform:'||NEW.id;
            END;
            CREATE TRIGGER library_items_transform_delete AFTER DELETE ON saved_transforms BEGIN
              DELETE FROM library_items WHERE stable_ref='transform:'||OLD.id;
            END;",
        )?;
        for (index, definition) in crate::operation_registry::BUILTIN_OPERATIONS
            .iter()
            .enumerate()
        {
            conn.execute(
                "INSERT INTO library_items
                    (stable_ref, kind, name, group_label, icon, enabled, is_builtin,
                     is_archived, sort_order, revision, input_contract, output_contract)
                 VALUES (?1, 'operation', ?2, ?3, 'Wrench', 1, 1, 0, ?4, 1, 'text', 'preserve_type')
                 ON CONFLICT(stable_ref) DO UPDATE SET name=excluded.name,
                    group_label=excluded.group_label, sort_order=excluded.sort_order",
                params![
                    format!("builtin:{}", definition.key),
                    definition.name,
                    definition.category_label,
                    index as i64
                ],
            )?;
        }
        Ok(())
    }

    /// Removes all user-owned application state while preserving the initialized schema.
    /// The transaction recreates the starter Smart Bins so every caller observes a valid,
    /// first-launch database immediately after it commits.
    pub fn factory_reset(&self) -> Result<FactoryResetReport> {
        let mut conn = self.conn.lock();
        let transaction = conn.transaction()?;

        let report = FactoryResetReport {
            clips_deleted: transaction.query_row("SELECT COUNT(*) FROM clips", [], |row| row.get(0))?,
            bins_deleted: transaction.query_row("SELECT COUNT(*) FROM bins", [], |row| row.get(0))?,
            transforms_deleted: transaction.query_row(
                "SELECT (SELECT COUNT(*) FROM saved_transforms) + (SELECT COUNT(*) FROM custom_operations)",
                [],
                |row| row.get(0),
            )?,
            connections_deleted: transaction.query_row(
                "SELECT COUNT(*) FROM intelligence_connections",
                [],
                |row| row.get(0),
            )?,
            activity_entries_deleted: transaction.query_row(
                "SELECT COUNT(*) FROM activity_logs",
                [],
                |row| row.get(0),
            )?,
        };

        transaction.execute_batch(
            "DELETE FROM automation_conditions;
             DELETE FROM automations;
             DELETE FROM clip_transformations;
             DELETE FROM transformation_executions;
             DELETE FROM saved_transforms;
             DELETE FROM custom_operations;
             DELETE FROM intelligence_connections;
             DELETE FROM clip_versions;
             DELETE FROM clip_bins;
             DELETE FROM clips;
             DELETE FROM bins;
             DELETE FROM activity_logs;
             DELETE FROM content_extractors;
             DELETE FROM content_detectors;
             DELETE FROM content_types;
             DELETE FROM content_type_groups;
             DELETE FROM settings;",
        )?;
        transaction.execute(
            "DELETE FROM sqlite_sequence WHERE name IN (
                'clips', 'bins', 'clip_versions', 'activity_logs', 'custom_operations',
                'saved_transforms', 'automations', 'intelligence_connections'
            )",
            [],
        )?;
        insert_default_bins(&transaction)?;
        for preset in crate::content_types::CONTENT_TYPE_GROUP_PRESETS {
            transaction.execute(
                "INSERT INTO content_type_groups
                    (id, label, sort_order, is_builtin, is_archived)
                 VALUES (?1, ?2, ?3, 1, 0)",
                params![preset.id, preset.label, preset.sort_order],
            )?;
        }
        for preset in crate::content_types::CONTENT_TYPE_PRESETS {
            transaction.execute(
                "INSERT INTO content_types
                    (id, label, icon, group_name, is_builtin, is_archived)
                 VALUES (?1, ?2, ?3, ?4, 1, 0)",
                params![preset.id, preset.label, preset.icon, preset.group],
            )?;
        }
        for preset in crate::content_detection::DETECTOR_PRESETS {
            let patterns_json = serde_json::to_string(&preset.patterns)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            transaction.execute(
                "INSERT INTO content_detectors
                    (stable_ref, name, content_type, description, patterns_json, validator, enabled, priority, is_builtin)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, 1)",
                params![preset.stable_ref, preset.name, preset.content_type, preset.description, patterns_json, preset.validator, preset.priority],
            )?;
        }
        for preset in crate::content_extraction::EXTRACTOR_PRESETS {
            transaction.execute(
                "INSERT INTO content_extractors
                    (stable_ref, name, description, engine, input_contract, output_contract,
                     enabled, priority, is_builtin)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, 1)",
                params![
                    preset.stable_ref,
                    preset.name,
                    preset.description,
                    preset.engine,
                    preset.input_contract,
                    preset.output_contract,
                    preset.priority
                ],
            )?;
        }
        let _ = transaction.execute("INSERT INTO clips_fts(clips_fts) VALUES('rebuild')", []);
        transaction.commit()?;
        let _ = conn.pragma_update(None, "optimize", "");
        Ok(report)
    }

    fn init_transformation_tables(&self, conn: &Connection) -> Result<()> {
        // The app has not shipped, so keep the domain and storage vocabulary
        // aligned. These renames preserve development data without carrying a
        // second set of compatibility APIs through the codebase.
        let has_legacy_transforms = table_exists(conn, "transformation_recipes")?;
        let has_saved_transforms = table_exists(conn, "saved_transforms")?;
        if has_legacy_transforms && !has_saved_transforms {
            conn.execute(
                "ALTER TABLE transformation_recipes RENAME TO saved_transforms",
                [],
            )?;
        } else if has_legacy_transforms && has_saved_transforms {
            // A hot-reloaded frontend can call the new API before the Rust
            // process restarts, leaving both pre-release tables behind. Merge
            // them instead of treating the new-but-empty table as authoritative.
            conn.execute(
                "INSERT OR IGNORE INTO saved_transforms
                    (row_id, id, name, plan_json, connection_id, revision, created_at, updated_at)
                 SELECT row_id, id, name, plan_json, connection_id, revision, created_at, updated_at
                 FROM transformation_recipes",
                [],
            )?;
        }
        if column_exists(conn, "clip_transformations", "recipe_id")?
            && !column_exists(conn, "clip_transformations", "transform_id")?
        {
            conn.execute(
                "ALTER TABLE clip_transformations RENAME COLUMN recipe_id TO transform_id",
                [],
            )?;
            conn.execute(
                "ALTER TABLE clip_transformations RENAME COLUMN recipe_name TO transform_name",
                [],
            )?;
            conn.execute(
                "ALTER TABLE clip_transformations RENAME COLUMN recipe_revision TO transform_revision",
                [],
            )?;
        }
        let has_legacy_bin_transform = column_exists(conn, "bins", "default_recipe_id")?;
        let has_current_bin_transform = column_exists(conn, "bins", "default_transform_id")?;
        if has_legacy_bin_transform && !has_current_bin_transform {
            conn.execute(
                "ALTER TABLE bins RENAME COLUMN default_recipe_id TO default_transform_id",
                [],
            )?;
        } else if has_legacy_bin_transform && has_current_bin_transform {
            conn.execute(
                "UPDATE bins SET default_transform_id = default_recipe_id
                 WHERE default_transform_id IS NULL AND default_recipe_id IS NOT NULL",
                [],
            )?;
            conn.execute("ALTER TABLE bins DROP COLUMN default_recipe_id", [])?;
        }

        if has_legacy_transforms && has_saved_transforms {
            let provenance_sql: String = conn.query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'clip_transformations'",
                [],
                |row| row.get(0),
            )?;
            if provenance_sql.contains("transformation_recipes") {
                conn.execute_batch(
                    "CREATE TABLE clip_transformations_migrated (
                        id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                        clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                        transform_id TEXT REFERENCES saved_transforms(id) ON DELETE SET NULL,
                        transform_ref TEXT,
                        transform_name TEXT NOT NULL,
                        transform_revision INTEGER NOT NULL,
                        connection_id TEXT REFERENCES intelligence_connections(id) ON DELETE SET NULL,
                        duration_ms INTEGER NOT NULL DEFAULT 0 CHECK (duration_ms >= 0),
                        created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
                    );
                    INSERT INTO clip_transformations_migrated
                        (id, clip_id, transform_id, transform_ref, transform_name, transform_revision,
                         connection_id, duration_ms, created_at)
                    SELECT id, clip_id, transform_id,
                           CASE WHEN transform_id IS NOT NULL THEN 'transform:' || transform_id END,
                           transform_name, transform_revision,
                           connection_id, duration_ms, created_at
                    FROM clip_transformations;
                    DROP TABLE clip_transformations;
                    ALTER TABLE clip_transformations_migrated RENAME TO clip_transformations;",
                )?;
            }
            conn.execute("DROP TABLE transformation_recipes", [])?;
        }

        let execution_ledger_exists = table_exists(conn, "transformation_executions")?;
        let legacy_execution_has_destination = execution_ledger_exists
            && column_exists(conn, "transformation_executions", "destination_kind")?;
        let legacy_execution_has_completed = execution_ledger_exists
            && column_exists(conn, "transformation_executions", "completed_at")?;
        let rebuild_execution_ledger = if execution_ledger_exists {
            let table_sql: String = conn.query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'transformation_executions'",
                [],
                |row| row.get(0),
            )?;
            !table_sql.contains("'transform'")
                || !table_sql.contains("'queued'")
                || !table_sql.contains("'cancelled'")
        } else {
            false
        };
        if rebuild_execution_ledger {
            conn.execute(
                "ALTER TABLE transformation_executions RENAME TO transformation_executions_legacy",
                [],
            )?;
        }

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS custom_operations (
                row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                id TEXT NOT NULL UNIQUE DEFAULT (lower(hex(randomblob(16)))),
                name TEXT NOT NULL,
                executor_kind TEXT NOT NULL CHECK (
                    executor_kind IN ('builtin', 'regex', 'cli', 'shell', 'http', 'ai')
                ),
                config_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(config_json)),
                category TEXT NOT NULL DEFAULT 'Custom Operations',
                enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
                trusted INTEGER NOT NULL DEFAULT 0 CHECK (trusted IN (0, 1)),
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS saved_transforms (
                row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                id TEXT NOT NULL UNIQUE DEFAULT (lower(hex(randomblob(16)))),
                name TEXT NOT NULL,
                plan_json TEXT NOT NULL CHECK (json_valid(plan_json)),
                connection_id TEXT REFERENCES intelligence_connections(id) ON DELETE SET NULL,
                shortcut TEXT,
                authoring_kind TEXT NOT NULL DEFAULT 'intent' CHECK (authoring_kind IN ('intent', 'manual')),
                revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS clip_transformations (
                id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                transform_id TEXT REFERENCES saved_transforms(id) ON DELETE SET NULL,
                transform_ref TEXT,
                transform_name TEXT NOT NULL,
                transform_revision INTEGER NOT NULL,
                connection_id TEXT REFERENCES intelligence_connections(id) ON DELETE SET NULL,
                duration_ms INTEGER NOT NULL DEFAULT 0 CHECK (duration_ms >= 0),
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_clip_transformations_clip
                ON clip_transformations(clip_id, created_at DESC);

            CREATE TABLE IF NOT EXISTS automations (
                row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                id TEXT NOT NULL UNIQUE DEFAULT (lower(hex(randomblob(16)))),
                name TEXT NOT NULL,
                trigger_kind TEXT NOT NULL CHECK (trigger_kind IN ('capture', 'copy', 'paste')),
                transform_id TEXT NOT NULL REFERENCES saved_transforms(id) ON DELETE RESTRICT,
                enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
                trusted INTEGER NOT NULL DEFAULT 0 CHECK (trusted IN (0, 1)),
                priority INTEGER NOT NULL DEFAULT 0,
                action_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(action_json)),
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS automation_conditions (
                id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                automation_id TEXT NOT NULL REFERENCES automations(id) ON DELETE CASCADE,
                position INTEGER NOT NULL CHECK (position >= 0),
                condition_kind TEXT NOT NULL,
                config_json TEXT NOT NULL CHECK (json_valid(config_json)),
                UNIQUE (automation_id, position)
            );

            CREATE TABLE IF NOT EXISTS transformation_executions (
                id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                target_kind TEXT NOT NULL CHECK (target_kind IN ('operation', 'pipeline', 'transform')),
                target_ref TEXT NOT NULL,
                target_revision INTEGER,
                source_clip_id INTEGER REFERENCES clips(id) ON DELETE SET NULL,
                trigger_kind TEXT NOT NULL CHECK (
                    trigger_kind IN ('manual', 'shortcut', 'bin', 'automation', 'cli')
                ),
                destination_kind TEXT NOT NULL DEFAULT 'preview' CHECK (
                    destination_kind IN ('preview', 'replace', 'copy', 'paste', 'route')
                ),
                started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                completed_at DATETIME,
                duration_ms INTEGER CHECK (duration_ms IS NULL OR duration_ms >= 0),
                status TEXT NOT NULL DEFAULT 'queued' CHECK (
                    status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')
                ),
                error_summary TEXT,
                input_hash TEXT NOT NULL,
                output_hash TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_transformation_executions_started
                ON transformation_executions(started_at DESC);
            CREATE INDEX IF NOT EXISTS idx_transformation_executions_target
                ON transformation_executions(target_kind, target_ref, started_at DESC);

            CREATE TABLE IF NOT EXISTS intelligence_connections (
                row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                id TEXT NOT NULL UNIQUE DEFAULT (lower(hex(randomblob(16)))),
                name TEXT NOT NULL,
                provider_kind TEXT NOT NULL CHECK (
                    provider_kind IN ('openai_compatible', 'anthropic', 'gemini', 'ollama', 'lm_studio', 'cli')
                ),
                endpoint TEXT,
                model TEXT,
                credential_ref TEXT,
                enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
                priority INTEGER NOT NULL DEFAULT 0 CHECK (priority >= 0),
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_intelligence_connections_enabled
                ON intelligence_connections(enabled, provider_kind);

            ",
        )?;

        if !column_exists(conn, "saved_transforms", "shortcut")? {
            conn.execute("ALTER TABLE saved_transforms ADD COLUMN shortcut TEXT", [])?;
        }
        if !column_exists(conn, "saved_transforms", "authoring_kind")? {
            conn.execute(
                "ALTER TABLE saved_transforms ADD COLUMN authoring_kind TEXT NOT NULL DEFAULT 'intent'",
                [],
            )?;
        }

        if !column_exists(conn, "clip_transformations", "transform_ref")? {
            conn.execute(
                "ALTER TABLE clip_transformations ADD COLUMN transform_ref TEXT",
                [],
            )?;
        }
        conn.execute(
            "UPDATE clip_transformations
             SET transform_ref = 'transform:' || transform_id
             WHERE transform_ref IS NULL AND transform_id IS NOT NULL",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clip_transformations_ref
             ON clip_transformations(transform_ref, created_at DESC)",
            [],
        )?;

        if rebuild_execution_ledger {
            let destination_expression = if legacy_execution_has_destination {
                "destination_kind"
            } else {
                "'preview'"
            };
            let completed_expression = if legacy_execution_has_completed {
                "completed_at"
            } else {
                "CASE WHEN status = 'running' THEN NULL ELSE started_at END"
            };
            conn.execute(
                &format!(
                    "INSERT INTO transformation_executions
                    (id, target_kind, target_ref, target_revision, source_clip_id,
                     trigger_kind, destination_kind, started_at, completed_at,
                     duration_ms, status, error_summary, input_hash, output_hash)
                 SELECT id, target_kind, target_ref, target_revision, source_clip_id,
                        trigger_kind, {destination_expression}, started_at,
                        {completed_expression},
                        duration_ms, status, error_summary, input_hash, output_hash
                 FROM transformation_executions_legacy"
                ),
                [],
            )?;
            conn.execute("DROP TABLE transformation_executions_legacy", [])?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_transformation_executions_started
                 ON transformation_executions(started_at DESC)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_transformation_executions_target
                 ON transformation_executions(target_kind, target_ref, started_at DESC)",
                [],
            )?;
        }

        if !column_exists(conn, "bins", "default_transform_id")? {
            conn.execute("ALTER TABLE bins ADD COLUMN default_transform_id TEXT", [])?;
        }
        if !column_exists(conn, "intelligence_connections", "priority")? {
            conn.execute(
                "ALTER TABLE intelligence_connections ADD COLUMN priority INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }
        if !column_exists(conn, "transformation_executions", "destination_kind")? {
            conn.execute(
                "ALTER TABLE transformation_executions ADD COLUMN destination_kind TEXT NOT NULL DEFAULT 'preview'",
                [],
            )?;
        }
        if !column_exists(conn, "transformation_executions", "completed_at")? {
            conn.execute(
                "ALTER TABLE transformation_executions ADD COLUMN completed_at DATETIME",
                [],
            )?;
        }
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                key TEXT PRIMARY KEY,
                applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        let transform_terms_migrated: i64 = conn.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE key = 'transformTerminologyV1'",
            [],
            |row| row.get(0),
        )?;
        if transform_terms_migrated == 0 {
            conn.execute(
                "UPDATE activity_logs
                 SET event_type = replace(event_type, 'recipe_', 'transform_'),
                     description = replace(replace(description, 'Recipes', 'Transforms'), 'Recipe', 'Transform')
                 WHERE event_type LIKE '%recipe%' OR description LIKE '%Recipe%'",
                [],
            )?;
            conn.execute(
                "INSERT INTO schema_migrations (key) VALUES ('transformTerminologyV1')",
                [],
            )?;
        }
        let provenance_backfilled: i64 = conn.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE key = 'currentTransformationBackfillV1'",
            [],
            |row| row.get(0),
        )?;
        if provenance_backfilled == 0 {
            conn.execute(
                "UPDATE clips SET current_transformation_id = (
                    SELECT id FROM clip_transformations
                    WHERE clip_id = clips.id
                    ORDER BY created_at DESC, rowid DESC LIMIT 1
                 )
                 WHERE current_transformation_id IS NULL
                   AND EXISTS (SELECT 1 FROM clip_transformations WHERE clip_id = clips.id)",
                [],
            )?;
            conn.execute(
                "INSERT INTO schema_migrations (key) VALUES ('currentTransformationBackfillV1')",
                [],
            )?;
        }

        Ok(())
    }

    pub fn get_total_clip_count(&self) -> Result<i64> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT COUNT(*) FROM clips WHERE is_trashed IS NULL OR is_trashed = 0",
            [],
            |r| r.get(0),
        )
    }

    pub fn get_ocr_backfill_status(&self) -> Result<OcrBackfillStatus> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT
                COUNT(*),
                SUM(CASE WHEN ocr_status = 'never' THEN 1 ELSE 0 END),
                SUM(CASE WHEN ocr_status = 'queued' THEN 1 ELSE 0 END),
                SUM(CASE WHEN ocr_status = 'running' THEN 1 ELSE 0 END),
                SUM(CASE WHEN ocr_status = 'complete' THEN 1 ELSE 0 END),
                SUM(CASE WHEN ocr_status = 'no_text' THEN 1 ELSE 0 END),
                SUM(CASE WHEN ocr_status = 'failed' THEN 1 ELSE 0 END)
             FROM clips
             WHERE content_type = 'image' AND COALESCE(is_trashed, 0) = 0",
            [],
            |row| {
                Ok(OcrBackfillStatus {
                    total_images: row.get(0)?,
                    eligible_count: row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                    queued_count: row.get::<_, Option<i64>>(2)?.unwrap_or(0),
                    running_count: row.get::<_, Option<i64>>(3)?.unwrap_or(0),
                    completed_count: row.get::<_, Option<i64>>(4)?.unwrap_or(0),
                    no_text_count: row.get::<_, Option<i64>>(5)?.unwrap_or(0),
                    failed_count: row.get::<_, Option<i64>>(6)?.unwrap_or(0),
                })
            },
        )
    }

    pub fn claim_next_ocr_candidate(&self) -> Result<Option<OcrCandidate>> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let candidate = tx
            .query_row(
                "SELECT id, content_hash, image_base64
                 FROM clips
                 WHERE content_type = 'image'
                   AND ocr_status = 'never'
                   AND COALESCE(is_trashed, 0) = 0
                   AND image_base64 IS NOT NULL
                 ORDER BY id ASC LIMIT 1",
                [],
                |row| {
                    Ok(OcrCandidate {
                        clip_id: row.get(0)?,
                        content_hash: row.get(1)?,
                        image_base64: row.get(2)?,
                    })
                },
            )
            .optional()?;
        if let Some(candidate) = candidate.as_ref() {
            let changed = tx.execute(
                "UPDATE clips SET ocr_status = 'running', ocr_error = NULL
                 WHERE id = ?1 AND content_hash = ?2 AND ocr_status = 'never'
                   AND COALESCE(is_trashed, 0) = 0",
                params![candidate.clip_id, candidate.content_hash],
            )?;
            if changed == 0 {
                tx.commit()?;
                return Ok(None);
            }
        }
        tx.commit()?;
        Ok(candidate)
    }

    pub fn mark_ocr_running(&self, clip_id: i64, content_hash: &str) -> Result<bool> {
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE clips SET ocr_status = 'running', ocr_error = NULL
             WHERE id = ?1 AND content_hash = ?2 AND content_type = 'image'
               AND ocr_status IN ('never', 'queued', 'running', 'failed')
               AND COALESCE(is_trashed, 0) = 0",
            params![clip_id, content_hash],
        )?;
        Ok(changed > 0)
    }

    pub fn force_ocr_running(&self, clip_id: i64, content_hash: &str) -> Result<bool> {
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE clips SET ocr_status = 'running', ocr_error = NULL
             WHERE id = ?1 AND content_hash = ?2 AND content_type = 'image'
               AND COALESCE(is_trashed, 0) = 0",
            params![clip_id, content_hash],
        )?;
        Ok(changed > 0)
    }

    pub fn reset_ocr_work(&self, clip_id: Option<i64>, content_hash: Option<&str>) -> Result<()> {
        let conn = self.conn.lock();
        match (clip_id, content_hash) {
            (Some(id), Some(hash)) => {
                conn.execute(
                    "UPDATE clips SET ocr_status = 'never', ocr_error = NULL
                     WHERE id = ?1 AND content_hash = ?2 AND content_type = 'image'
                       AND ocr_status IN ('queued', 'running')",
                    params![id, hash],
                )?;
            }
            _ => {
                conn.execute(
                    "UPDATE clips SET ocr_status = 'never', ocr_error = NULL
                     WHERE content_type = 'image' AND ocr_status IN ('queued', 'running')",
                    [],
                )?;
            }
        }
        Ok(())
    }

    pub fn reset_failed_ocr(&self) -> Result<usize> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE clips SET ocr_status = 'never', ocr_error = NULL
             WHERE content_type = 'image' AND ocr_status = 'failed'
               AND COALESCE(is_trashed, 0) = 0",
            [],
        )
    }

    pub fn complete_ocr_attempt(
        &self,
        clip_id: i64,
        content_hash: &str,
        recognized_text: Option<&str>,
        engine_version: &str,
        error: Option<&str>,
    ) -> Result<bool> {
        self.complete_ocr_attempt_with_extractor(
            clip_id,
            content_hash,
            recognized_text,
            OcrExtractorProvenance::engine_only(engine_version),
            error,
        )
    }

    pub fn complete_ocr_attempt_with_extractor(
        &self,
        clip_id: i64,
        content_hash: &str,
        recognized_text: Option<&str>,
        provenance: OcrExtractorProvenance<'_>,
        error: Option<&str>,
    ) -> Result<bool> {
        if let Some(text) = recognized_text {
            ensure_resource_size(text, crate::resource_limits::MAX_OCR_TEXT_BYTES, "OCR text")?;
        }
        if provenance.engine_version.is_empty()
            || provenance.engine_version.len() > 80
            || provenance
                .stable_ref
                .is_some_and(|value| value.is_empty() || value.len() > 160)
            || provenance
                .name
                .is_some_and(|value| value.is_empty() || value.len() > 80)
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "OCR extractor provenance exceeds supported limits".into(),
            ));
        }
        if error.is_some_and(|code| {
            code.is_empty()
                || code.len() > 160
                || !code
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        }) {
            return Err(rusqlite::Error::InvalidParameterName(
                "OCR error codes require 1–160 lowercase ASCII letters, numbers, or underscores"
                    .into(),
            ));
        }
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let current = tx
            .query_row(
                "SELECT text_content FROM clips
                 WHERE id = ?1 AND content_hash = ?2 AND content_type = 'image'
                   AND COALESCE(is_trashed, 0) = 0",
                params![clip_id, content_hash],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?;
        let Some(previous_text) = current else {
            tx.execute(
                "UPDATE clips SET ocr_status = 'never', ocr_error = NULL
                 WHERE id = ?1 AND content_hash = ?2 AND content_type = 'image'
                   AND ocr_status IN ('queued', 'running')",
                params![clip_id, content_hash],
            )?;
            tx.commit()?;
            return Ok(false);
        };

        let status = if error.is_some() {
            "failed"
        } else if recognized_text.is_some_and(|text| !text.trim().is_empty()) {
            "complete"
        } else {
            "no_text"
        };
        if status == "complete" {
            let recognized_text = recognized_text.unwrap_or_default();
            if previous_text.as_deref() != Some(recognized_text)
                && Self::revision_history_enabled_internal(&tx)
            {
                if let Some(previous_text) = previous_text.as_ref() {
                    let context_json = serde_json::to_string(&ClipRevisionContext {
                        schema_version: 1,
                        action_kind: "ocr".to_string(),
                        action_label: "Updated OCR text".to_string(),
                        organization: None,
                        current_transformation_id: None,
                    })
                    .map_err(|reason| rusqlite::Error::InvalidParameterName(reason.to_string()))?;
                    tx.execute(
                        "INSERT INTO clip_versions (clip_id, text_content, context_json)
                         VALUES (?1, ?2, ?3)",
                        params![clip_id, previous_text, context_json],
                    )?;
                    Self::prune_clip_versions_internal(&tx, clip_id)?;
                }
            }
            tx.execute(
                "UPDATE clips
                 SET text_content = ?1, current_transformation_id = NULL,
                     ocr_status = 'complete', ocr_input_hash = ?2,
                     ocr_engine_version = ?3,
                     ocr_extractor_ref = COALESCE(?4, ocr_extractor_ref),
                     ocr_extractor_name = COALESCE(?5, ocr_extractor_name),
                     ocr_attempted_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                     ocr_error = NULL
                 WHERE id = ?6 AND content_hash = ?2 AND content_type = 'image'
                   AND COALESCE(is_trashed, 0) = 0",
                params![
                    recognized_text,
                    content_hash,
                    provenance.engine_version,
                    provenance.stable_ref,
                    provenance.name,
                    clip_id
                ],
            )?;
        } else {
            tx.execute(
                "UPDATE clips
                 SET ocr_status = ?1, ocr_input_hash = ?2,
                     ocr_engine_version = CASE
                        WHEN COALESCE(text_content, '') = '' THEN ?3
                        ELSE ocr_engine_version
                     END,
                     ocr_attempted_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                     ocr_error = ?4
                 WHERE id = ?5 AND content_hash = ?2 AND content_type = 'image'
                   AND COALESCE(is_trashed, 0) = 0",
                params![
                    status,
                    content_hash,
                    provenance.engine_version,
                    error,
                    clip_id
                ],
            )?;
        }
        tx.commit()?;
        Ok(true)
    }

    pub fn complete_or_reset_ocr_attempt(
        &self,
        clip_id: i64,
        content_hash: &str,
        recognized_text: Option<&str>,
        engine_version: &str,
        error: Option<&str>,
    ) -> Result<bool> {
        let result = self.complete_ocr_attempt(
            clip_id,
            content_hash,
            recognized_text,
            engine_version,
            error,
        );
        if result.is_err() {
            let _ = self.reset_ocr_work(Some(clip_id), Some(content_hash));
        }
        result
    }

    pub fn complete_or_reset_ocr_attempt_with_extractor(
        &self,
        clip_id: i64,
        content_hash: &str,
        recognized_text: Option<&str>,
        provenance: OcrExtractorProvenance<'_>,
        error: Option<&str>,
    ) -> Result<bool> {
        let result = self.complete_ocr_attempt_with_extractor(
            clip_id,
            content_hash,
            recognized_text,
            provenance,
            error,
        );
        if result.is_err() {
            let _ = self.reset_ocr_work(Some(clip_id), Some(content_hash));
        }
        result
    }

    pub fn record_analysis_classification(
        &self,
        clip_id: i64,
        input_hash: &str,
        content_type: Option<&str>,
        detector_ref: Option<&str>,
        source_representation: &str,
    ) -> Result<bool> {
        if !matches!(source_representation, "original_text" | "searchable_text") {
            return Err(rusqlite::Error::InvalidParameterName(
                "Unknown analysis source representation".into(),
            ));
        }
        if content_type.is_some_and(|value| value.len() > 80)
            || detector_ref.is_some_and(|value| value.len() > 160)
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "Analysis classification metadata exceeds its safety limit".into(),
            ));
        }
        let conn = self.conn.lock();
        let clip_matches: bool = conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM clips
                WHERE id = ?1 AND content_hash = ?2 AND COALESCE(is_trashed, 0) = 0
            )",
            params![clip_id, input_hash],
            |row| row.get(0),
        )?;
        if !clip_matches {
            return Ok(false);
        }
        let (Some(content_type), Some(detector_ref)) = (content_type, detector_ref) else {
            conn.execute(
                "DELETE FROM clip_analysis_classifications
                 WHERE clip_id = ?1 AND input_hash = ?2",
                params![clip_id, input_hash],
            )?;
            return Ok(true);
        };
        conn.execute(
            "INSERT INTO clip_analysis_classifications
                (clip_id, content_type, detector_ref, source_representation, input_hash)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(clip_id) DO UPDATE SET
                content_type = excluded.content_type,
                detector_ref = excluded.detector_ref,
                source_representation = excluded.source_representation,
                input_hash = excluded.input_hash,
                updated_at = CURRENT_TIMESTAMP",
            params![
                clip_id,
                content_type,
                detector_ref,
                source_representation,
                input_hash
            ],
        )?;
        Ok(true)
    }

    pub fn get_analysis_classification(
        &self,
        clip_id: i64,
    ) -> Result<Option<AnalysisClassification>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT clip_id, content_type, detector_ref, source_representation,
                    input_hash, updated_at
             FROM clip_analysis_classifications WHERE clip_id = ?1",
            params![clip_id],
            |row| {
                Ok(AnalysisClassification {
                    clip_id: row.get(0)?,
                    content_type: row.get(1)?,
                    detector_ref: row.get(2)?,
                    source_representation: row.get(3)?,
                    input_hash: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )
        .optional()
    }

    pub fn record_structural_inspection(
        &self,
        clip_id: i64,
        content_hash: &str,
        input_hash: &str,
        metadata: &crate::content_inspection::StructuralMetadata,
    ) -> Result<bool> {
        let result_json = serde_json::to_string(metadata)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        if result_json.len() > 64 * 1024 || input_hash.len() > 128 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Structural inspection metadata exceeds its safety limit".into(),
            ));
        }
        let conn = self.conn.lock();
        let changed = conn.execute(
            "INSERT INTO clip_analysis_results
                (clip_id, participant_ref, content_hash, input_hash, format_version, result_json)
             SELECT id, ?1, content_hash, ?2, ?3, ?4 FROM clips
             WHERE id = ?5 AND content_hash = ?6 AND COALESCE(is_trashed, 0) = 0
             ON CONFLICT(clip_id, participant_ref) DO UPDATE SET
                content_hash = excluded.content_hash,
                input_hash = excluded.input_hash,
                format_version = excluded.format_version,
                result_json = excluded.result_json,
                updated_at = CURRENT_TIMESTAMP",
            params![
                crate::content_inspection::STRUCTURE_INSPECTOR_REF,
                input_hash,
                crate::analysis_contract::ANALYSIS_CONTRACT_VERSION,
                result_json,
                clip_id,
                content_hash,
            ],
        )?;
        Ok(changed == 1)
    }

    pub fn get_structural_inspection(
        &self,
        clip_id: i64,
        input_hash: &str,
    ) -> Result<Option<crate::content_inspection::StructuralMetadata>> {
        let conn = self.conn.lock();
        let result_json = conn
            .query_row(
                "SELECT results.result_json FROM clip_analysis_results AS results
                 JOIN clips ON clips.id = results.clip_id
                 WHERE results.clip_id = ?1 AND results.participant_ref = ?2
                   AND results.input_hash = ?3
                   AND results.content_hash = clips.content_hash
                   AND results.format_version = ?4
                   AND COALESCE(clips.is_trashed, 0) = 0",
                params![
                    clip_id,
                    crate::content_inspection::STRUCTURE_INSPECTOR_REF,
                    input_hash,
                    crate::analysis_contract::ANALYSIS_CONTRACT_VERSION,
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(result_json.and_then(|json| serde_json::from_str(&json).ok()))
    }

    pub fn save_clip(
        &self,
        content_type: &str,
        text_content: Option<&str>,
        html_content: Option<&str>,
        image_base64: Option<&str>,
        content_hash: &str,
        source: &str,
    ) -> Result<ClipItem> {
        self.save_clip_with_structure(
            ClipSaveInput {
                content_type,
                text_content,
                html_content,
                image_base64,
                content_hash,
                source,
            },
            None,
        )
    }

    fn save_clip_with_structure(
        &self,
        input: ClipSaveInput<'_>,
        structure: Option<&crate::content_inspection::StructuralMetadata>,
    ) -> Result<ClipItem> {
        let ClipSaveInput {
            content_type,
            text_content,
            html_content,
            image_base64,
            content_hash,
            source,
        } = input;
        if let Some(text) = text_content {
            ensure_resource_size(
                text,
                crate::resource_limits::MAX_CLIP_TEXT_BYTES,
                "Clip text",
            )?;
        }
        if let Some(html) = html_content {
            ensure_resource_size(
                html,
                crate::resource_limits::MAX_CLIP_TEXT_BYTES,
                "Clip HTML",
            )?;
        }
        if let Some(image) = image_base64 {
            ensure_safe_raster_data_url(image, "Clip image")?;
        }
        let conn = self.conn.lock();

        let existing: Result<i64> = conn.query_row(
            "SELECT id FROM clips WHERE content_hash = ?1",
            params![content_hash],
            |r| r.get(0),
        );

        if let Ok(id) = existing {
            conn.execute(
                "UPDATE clips SET created_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), is_trashed = 0, trashed_at = NULL WHERE id = ?1",
                params![id],
            )?;
            let clip = self.get_clip_by_id_internal(&conn, id)?;
            drop(conn);
            self.persist_capture_structure(&clip, structure);
            return Ok(clip);
        }

        let ocr_status = if content_type == "image" {
            "never"
        } else {
            "not_applicable"
        };
        let ocr_input_hash = (content_type == "image").then_some(content_hash);
        conn.execute(
            "INSERT INTO clips
                (content_type, text_content, html_content, image_base64, content_hash, source,
                 ocr_status, ocr_input_hash, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
            params![
                content_type,
                text_content,
                html_content,
                image_base64,
                content_hash,
                source,
                ocr_status,
                ocr_input_hash
            ],
        )?;

        let id = conn.last_insert_rowid();
        let _ = self.enforce_history_limit_internal(&conn);
        let _ = self.enforce_trash_limit_internal(&conn);
        let clip = self.get_clip_by_id_internal(&conn, id)?;
        drop(conn);
        self.persist_capture_structure(&clip, structure);
        Ok(clip)
    }

    fn persist_capture_structure(
        &self,
        clip: &ClipItem,
        structure: Option<&crate::content_inspection::StructuralMetadata>,
    ) {
        let persisted = structure.is_some_and(|metadata| {
            let stored_origin =
                crate::content_inspection::origin_kind(&clip.content_type, Some(&clip.source));
            if metadata.origin != stored_origin {
                return false;
            }
            let input_hash = crate::inspection_execution::inspection_input_hash(clip);
            self.record_structural_inspection(clip.id, &clip.content_hash, &input_hash, metadata)
                .unwrap_or(false)
        });
        if !persisted {
            let _ = crate::inspection_execution::inspect_clip_with_policy(
                self,
                clip.id,
                true,
                crate::analysis_contract::AnalysisPolicy::Capture,
            );
        }
    }

    pub fn save_text_clip(&self, text: &str, source: &str) -> Result<ClipItem> {
        let include_detectors =
            crate::features::is_enabled(self, crate::features::Feature::ContentDetection);
        let analysis = crate::analysis_execution::analyze_text(
            self,
            text,
            Some(source),
            crate::analysis_execution::AnalyzerOptions {
                policy: crate::analysis_contract::AnalysisPolicy::Capture,
                include_extractor: false,
                include_detectors,
                include_enricher: false,
            },
        )
        .ok();
        let content_type = analysis
            .as_ref()
            .and_then(|result| result.analysis.result.detected_type.as_deref())
            .unwrap_or("text");
        let structure = analysis
            .as_ref()
            .and_then(|result| result.analysis.result.structure.as_ref());
        let content_hash = crate::clipboard_fingerprint::text(text);
        self.save_clip_with_structure(
            ClipSaveInput {
                content_type,
                text_content: Some(text),
                html_content: None,
                image_base64: None,
                content_hash: &content_hash,
                source,
            },
            structure,
        )
    }

    pub(crate) fn merge_external_text_clips(
        &self,
        source_label: &str,
        clips: &[ExternalTextClip],
    ) -> Result<(usize, usize, Option<usize>)> {
        let mut conn = self.conn.lock();
        let transaction = conn.transaction()?;
        let active_count_before: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM clips WHERE COALESCE(is_trashed, 0) = 0",
            [],
            |row| row.get(0),
        )?;
        let current_capacity = transaction
            .query_row(
                "SELECT value FROM settings WHERE key = 'keepClipCount'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(1000);
        let mut imported_count = 0usize;
        let mut duplicate_count = 0usize;

        for clip in clips {
            let changed = transaction.execute(
                "INSERT OR IGNORE INTO clips
                    (content_type, text_content, content_hash, source, ocr_status, created_at)
                 VALUES ('text', ?1, ?2, ?3, 'not_applicable', COALESCE(?4, strftime('%Y-%m-%dT%H:%M:%SZ', 'now')))",
                params![clip.text, clip.content_hash, clip.source, clip.created_at],
            )?;
            if changed == 1 {
                imported_count += 1;
            } else {
                duplicate_count += 1;
            }
        }

        let required_capacity =
            (active_count_before.max(0) as usize).saturating_add(imported_count);
        let history_capacity_adjusted_to =
            if current_capacity > 0 && required_capacity > current_capacity {
                transaction.execute(
                    "INSERT INTO settings (key, value) VALUES ('keepClipCount', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    [required_capacity.to_string()],
                )?;
                Some(required_capacity)
            } else {
                None
            };

        transaction.execute(
            "INSERT INTO activity_logs (event_type, description) VALUES ('external_history_imported', ?1)",
            [format!(
                "Imported {imported_count} clips from {source_label}; skipped {duplicate_count} duplicates"
            )],
        )?;
        transaction.commit()?;
        Ok((
            imported_count,
            duplicate_count,
            history_capacity_adjusted_to,
        ))
    }

    pub fn reattribute_image_capture(
        &self,
        clip_id: i64,
        content_hash: &str,
        source: &str,
    ) -> Result<Option<ClipItem>> {
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE clips SET source = ?1
             WHERE id = ?2 AND content_hash = ?3 AND content_type = 'image'
               AND COALESCE(is_trashed, 0) = 0",
            params![source, clip_id, content_hash],
        )?;
        if changed == 0 {
            return Ok(None);
        }
        self.get_clip_by_id_internal(&conn, clip_id).map(Some)
    }

    pub fn enforce_history_limit_internal(&self, conn: &Connection) -> Result<()> {
        let keep_count: i64 = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'keepClipCount'",
                [],
                |r| r.get(0),
            )
            .ok()
            .and_then(|v: String| v.parse().ok())
            .unwrap_or(1000);
        let keep_age_days: i64 = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'keepClipAgeDays'",
                [],
                |r| r.get(0),
            )
            .ok()
            .and_then(|v: String| v.parse().ok())
            .unwrap_or(0);

        self.enforce_clip_retention_internal(conn, keep_count, keep_age_days)
    }

    fn enforce_clip_retention_internal(
        &self,
        conn: &Connection,
        keep_count: i64,
        keep_age_days: i64,
    ) -> Result<()> {
        let keep_count = keep_count.max(0);
        let keep_age_days = keep_age_days.max(0);

        let enable_trash: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'enableTrash'",
                [],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "true".to_string());

        let mut ids = Vec::new();
        if keep_age_days > 0 {
            let age_modifier = format!("-{keep_age_days} days");
            let mut stmt = conn.prepare(
                "SELECT id FROM clips
                 WHERE is_pinned = 0
                   AND (is_protected IS NULL OR is_protected = 0)
                   AND (is_trashed IS NULL OR is_trashed = 0)
                   AND datetime(created_at) < datetime('now', ?1)
                 ORDER BY created_at ASC, id ASC",
            )?;
            ids.extend(
                stmt.query_map([age_modifier], |r| r.get::<_, i64>(0))?
                    .filter_map(|r| r.ok()),
            );
        }

        if keep_count > 0 {
            let active_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM clips
                     WHERE is_pinned = 0
                       AND (is_protected IS NULL OR is_protected = 0)
                       AND (is_trashed IS NULL OR is_trashed = 0)",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            let excess = active_count.saturating_sub(keep_count);
            if excess > 0 {
                let mut stmt = conn.prepare(
                    "SELECT id FROM clips
                     WHERE is_pinned = 0
                       AND (is_protected IS NULL OR is_protected = 0)
                       AND (is_trashed IS NULL OR is_trashed = 0)
                     ORDER BY created_at ASC, id ASC LIMIT ?1",
                )?;
                ids.extend(
                    stmt.query_map(params![excess], |r| r.get::<_, i64>(0))?
                        .filter_map(|r| r.ok()),
                );
            }
        }

        ids.sort_unstable();
        ids.dedup();
        for id in ids {
            if enable_trash == "true" {
                let changed = conn.execute(
                        "UPDATE clips SET is_trashed = 1, trashed_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?1",
                        params![id],
                    ).unwrap_or(0);
                if changed > 0 {
                    let _ = self.clear_category_bin_assignments_internal(conn, id);
                }
                let _ = self.log_activity_internal(
                    conn,
                    "clip_auto_trashed",
                    &format!(
                        "Auto-trashed clip #{} (history retention policy exceeded)",
                        id
                    ),
                );
            } else {
                let _ = conn.execute("DELETE FROM clips WHERE id = ?1", params![id]);
                let _ = self.log_activity_internal(
                    conn,
                    "clip_deleted",
                    &format!(
                        "Auto-purged clip #{} (history retention policy exceeded)",
                        id
                    ),
                );
            }
        }
        Ok(())
    }

    pub fn enforce_trash_limit_internal(&self, conn: &Connection) -> Result<()> {
        let capacity: i64 = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'trashCapacityCount'",
                [],
                |r| r.get(0),
            )
            .ok()
            .and_then(|v: String| v.parse().ok())
            .unwrap_or(500);
        let keep_age_days: i64 = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'trashAgeDays'",
                [],
                |r| r.get(0),
            )
            .ok()
            .and_then(|v: String| v.parse().ok())
            .unwrap_or(0);

        self.enforce_trash_retention_internal(conn, capacity, keep_age_days)
    }

    fn enforce_trash_retention_internal(
        &self,
        conn: &Connection,
        keep_count: i64,
        keep_age_days: i64,
    ) -> Result<()> {
        let keep_count = keep_count.max(0);
        let keep_age_days = keep_age_days.max(0);

        if keep_age_days > 0 {
            let age_modifier = format!("-{keep_age_days} days");
            conn.execute(
                "DELETE FROM clips
                 WHERE is_trashed = 1
                   AND (is_protected IS NULL OR is_protected = 0)
                   AND datetime(COALESCE(trashed_at, created_at)) < datetime('now', ?1)",
                [age_modifier],
            )?;
        }

        if keep_count > 0 {
            conn.execute(
                "DELETE FROM clips
                 WHERE is_trashed = 1
                   AND (is_protected IS NULL OR is_protected = 0)
                   AND id NOT IN (
                       SELECT id FROM clips
                       WHERE is_trashed = 1 AND (is_protected IS NULL OR is_protected = 0)
                       ORDER BY COALESCE(trashed_at, created_at) DESC, id DESC LIMIT ?1
                   )",
                params![keep_count],
            )?;
        }
        Ok(())
    }

    pub fn get_clip_image(&self, id: i64) -> Result<Option<String>> {
        let conn = self.conn.lock();
        let image: Option<String> = conn.query_row(
            "SELECT image_base64 FROM clips WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        Ok(image.filter(|value| crate::resource_limits::validate_raster_data_url(value).is_ok()))
    }

    pub fn get_active_clip_text(&self, id: i64) -> Result<Option<String>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT text_content FROM clips
             WHERE id = ?1 AND (is_trashed IS NULL OR is_trashed = 0)",
            params![id],
            |row| row.get(0),
        )
    }

    pub fn get_clip_by_id(&self, id: i64) -> Result<ClipItem> {
        let conn = self.conn.lock();
        self.get_clip_by_id_internal(&conn, id)
    }

    fn get_clip_by_id_internal(&self, conn: &Connection, id: i64) -> Result<ClipItem> {
        let mut clip = conn.query_row(
            "SELECT id, content_type, text_content, html_content, image_base64, image_path, content_hash, source, is_pinned, is_protected, COALESCE(pin_order, 0), bin_id, note, is_trashed, trashed_at, created_at,
                    (SELECT GROUP_CONCAT(bin_id) FROM clip_bins WHERE clip_id = clips.id),
                    current_transformation_id IS NOT NULL,
                    ocr_extractor_ref, ocr_extractor_name, ocr_engine_version
             FROM clips WHERE id = ?1",
            params![id],
            |row| {
                let bid: Option<i64> = row.get(11)?;
                let bin_ids_str: Option<String> = row.get(16)?;
                let mut bin_ids = bid.into_iter().collect::<Vec<_>>();
                if let Some(value) = bin_ids_str {
                    for value in value.split(',').filter_map(|part| part.parse::<i64>().ok()) {
                        if !bin_ids.contains(&value) {
                            bin_ids.push(value);
                        }
                    }
                }
                Ok(ClipItem {
                    id: row.get(0)?,
                    content_type: row.get(1)?,
                    text_content: row.get(2)?,
                    html_content: row.get(3)?,
                    image_base64: row.get(4)?,
                    image_path: row.get(5)?,
                    content_hash: row.get(6)?,
                    source: row.get(7)?,
                    is_pinned: row.get::<_, i32>(8)? != 0,
                    is_protected: row.get::<_, i32>(9)? != 0,
                    is_transformed: row.get::<_, i32>(17)? != 0,
                    pin_order: row.get(10)?,
                    bin_id: bid,
                    bin_ids: Some(bin_ids),
                    note: row.get(12)?,
                    is_trashed: row.get::<_, i32>(13)? != 0,
                    trashed_at: row.get(14)?,
                    created_at: row.get(15)?,
                    ocr_extractor_ref: row.get(18)?,
                    ocr_extractor_name: row.get(19)?,
                    ocr_engine_version: row.get(20)?,
                })
            },
        )?;
        append_smart_bin_memberships(conn, std::slice::from_mut(&mut clip))?;
        Ok(clip)
    }

    pub fn get_clips(
        &self,
        search_query: Option<&str>,
        bin_id: Option<i64>,
        only_pinned: bool,
    ) -> Result<Vec<ClipItem>> {
        self.get_clips_page(search_query, bin_id, only_pinned, None, None)
    }

    pub fn get_clips_page(
        &self,
        search_query: Option<&str>,
        bin_id: Option<i64>,
        only_pinned: bool,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<ClipItem>> {
        let conn = self.conn.lock();

        // Check if target bin has smart_rule
        let mut smart_rule_str: Option<String> = None;
        if let Some(bid) = bin_id {
            let res: Result<Option<String>> = conn.query_row(
                "SELECT smart_rule FROM bins WHERE id = ?1",
                params![bid],
                |r| r.get(0),
            );
            if let Ok(sr) = res {
                smart_rule_str = sr;
            }
        }

        let mut sql = String::from(
            "SELECT id, content_type, text_content, NULL as html_content, NULL as image_base64, image_path, content_hash, source, is_pinned, is_protected, COALESCE(pin_order, 0), bin_id, note, is_trashed, trashed_at, created_at,
             (SELECT GROUP_CONCAT(bin_id) FROM clip_bins WHERE clip_id = clips.id) as bin_ids_str,
             current_transformation_id IS NOT NULL,
             ocr_extractor_ref, ocr_extractor_name, ocr_engine_version
             FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0)"
        );

        let mut query_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if only_pinned {
            sql.push_str(" AND is_pinned = 1");
        }

        if let Some(ref sr_json) = smart_rule_str {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(sr_json) {
                let match_mode = parsed["match"].as_str().unwrap_or("any");
                let is_and = match_mode == "all";
                let join_op = if is_and { " AND " } else { " OR " };

                let mut cond_sqls: Vec<String> = Vec::new();

                if let Some(conds) = parsed["conditions"].as_array() {
                    for cond in conds {
                        let c_type = cond["type"].as_str().unwrap_or("");
                        let c_val = cond["value"].as_str().unwrap_or("");
                        push_smart_condition(c_type, c_val, &mut cond_sqls, &mut query_params);
                    }
                } else {
                    let rule_type = parsed["type"].as_str().unwrap_or("");
                    let rule_val = parsed["value"].as_str().unwrap_or("");
                    push_smart_condition(rule_type, rule_val, &mut cond_sqls, &mut query_params);
                }

                if !cond_sqls.is_empty() {
                    let combined = cond_sqls.join(join_op);
                    if let Some(bid) = bin_id {
                        sql.push_str(&format!(" AND (({}) OR bin_id = ? OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?))", combined));
                        query_params.push(Box::new(bid));
                        query_params.push(Box::new(bid));
                    } else {
                        sql.push_str(&format!(" AND ({})", combined));
                    }
                } else if let Some(bid) = bin_id {
                    sql.push_str(" AND (bin_id = ? OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?))");
                    query_params.push(Box::new(bid));
                    query_params.push(Box::new(bid));
                }
            } else if let Some(bid) = bin_id {
                sql.push_str(
                    " AND (bin_id = ? OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?))",
                );
                query_params.push(Box::new(bid));
                query_params.push(Box::new(bid));
            }
        } else if let Some(bid) = bin_id {
            sql.push_str(
                " AND (bin_id = ? OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?))",
            );
            query_params.push(Box::new(bid));
            query_params.push(Box::new(bid));
        }

        if let Some(q) = search_query {
            let cleaned = q.trim();
            if !cleaned.is_empty() {
                let fts_query = cleaned.replace('"', "\"\"").replace('*', "");
                if !fts_query.trim().is_empty() {
                    sql.push_str(" AND (id IN (SELECT rowid FROM clips_fts WHERE clips_fts MATCH ?) OR content_type LIKE ?)");
                    query_params.push(Box::new(format!("\"{}\"*", fts_query)));
                    query_params.push(Box::new(format!("%{}%", cleaned)));
                } else {
                    sql.push_str(" AND (text_content LIKE ? OR source LIKE ? OR content_type LIKE ? OR note LIKE ?)");
                    let pattern = format!("%{}%", cleaned);
                    query_params.push(Box::new(pattern.clone()));
                    query_params.push(Box::new(pattern.clone()));
                    query_params.push(Box::new(pattern.clone()));
                    query_params.push(Box::new(pattern));
                }
            }
        }

        if let Some(bid) = bin_id {
            sql.push_str(
                " ORDER BY
                    CASE WHEN EXISTS(
                        SELECT 1 FROM bin_clip_order ordered
                        WHERE ordered.bin_id = ? AND ordered.clip_id = clips.id
                    ) THEN 0 ELSE 1 END,
                    (SELECT position FROM bin_clip_order ordered
                     WHERE ordered.bin_id = ? AND ordered.clip_id = clips.id),
                    created_at DESC,
                    id DESC",
            );
            query_params.push(Box::new(bid));
            query_params.push(Box::new(bid));
        } else {
            sql.push_str(" ORDER BY is_pinned DESC, pin_order ASC, created_at DESC, id DESC");
        }

        if let Some(limit) = limit {
            sql.push_str(" LIMIT ? OFFSET ?");
            query_params.push(Box::new(limit.clamp(1, 10_000)));
            query_params.push(Box::new(offset.unwrap_or(0).max(0)));
        }

        let param_refs: Vec<&dyn rusqlite::ToSql> =
            query_params.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let clip_iter = stmt.query_map(param_refs.as_slice(), |row| {
            let primary_bid: Option<i64> = row.get(11)?;
            let bin_ids_str: Option<String> = row.get(16)?;
            let mut b_ids = Vec::new();
            if let Some(b) = primary_bid {
                b_ids.push(b);
            }
            if let Some(ref s) = bin_ids_str {
                for part in s.split(',') {
                    if let Ok(parsed_id) = part.parse::<i64>() {
                        if !b_ids.contains(&parsed_id) {
                            b_ids.push(parsed_id);
                        }
                    }
                }
            }

            Ok(ClipItem {
                id: row.get(0)?,
                content_type: row.get(1)?,
                text_content: row.get(2)?,
                html_content: row.get(3)?,
                image_base64: row.get(4)?,
                image_path: row.get(5)?,
                content_hash: row.get(6)?,
                source: row.get(7)?,
                is_pinned: row.get::<_, i32>(8)? != 0,
                is_protected: row.get::<_, i32>(9)? != 0,
                is_transformed: row.get::<_, i32>(17)? != 0,
                pin_order: row.get(10)?,
                bin_id: primary_bid,
                bin_ids: Some(b_ids),
                note: row.get(12)?,
                is_trashed: row.get::<_, i32>(13)? != 0,
                trashed_at: row.get(14)?,
                created_at: row.get(15)?,
                ocr_extractor_ref: row.get(18)?,
                ocr_extractor_name: row.get(19)?,
                ocr_engine_version: row.get(20)?,
            })
        })?;

        let mut clips = Vec::new();
        for clip in clip_iter {
            clips.push(clip?);
        }
        append_smart_bin_memberships(&conn, &mut clips)?;
        Ok(clips)
    }

    pub fn get_trashed_clips(&self) -> Result<Vec<ClipItem>> {
        self.get_trashed_clips_page(None, None)
    }

    pub fn get_trashed_clip_count(&self) -> Result<i64> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT COUNT(*) FROM clips WHERE is_trashed = 1",
            [],
            |row| row.get(0),
        )
    }

    pub fn get_trashed_clips_page(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<ClipItem>> {
        let conn = self.conn.lock();
        let mut sql = String::from(
            "SELECT id, content_type, text_content, NULL as html_content, NULL as image_base64, image_path, content_hash, source, is_pinned, is_protected, COALESCE(pin_order, 0), bin_id, note, is_trashed, trashed_at, created_at,
                    current_transformation_id IS NOT NULL,
                    ocr_extractor_ref, ocr_extractor_name, ocr_engine_version
             FROM clips WHERE is_trashed = 1 ORDER BY COALESCE(trashed_at, created_at) DESC, id DESC"
        );
        let mut query_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(limit) = limit {
            sql.push_str(" LIMIT ? OFFSET ?");
            query_params.push(Box::new(limit.clamp(1, 10_000)));
            query_params.push(Box::new(offset.unwrap_or(0).max(0)));
        }
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            query_params.iter().map(|value| value.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let clip_iter = stmt.query_map(param_refs.as_slice(), |row| {
            let bid: Option<i64> = row.get(11)?;
            Ok(ClipItem {
                id: row.get(0)?,
                content_type: row.get(1)?,
                text_content: row.get(2)?,
                html_content: row.get(3)?,
                image_base64: row.get(4)?,
                image_path: row.get(5)?,
                content_hash: row.get(6)?,
                source: row.get(7)?,
                is_pinned: row.get::<_, i32>(8)? != 0,
                is_protected: row.get::<_, i32>(9)? != 0,
                is_transformed: row.get::<_, i32>(16)? != 0,
                pin_order: row.get(10)?,
                bin_id: bid,
                bin_ids: bid.map(|b| vec![b]),
                note: row.get(12)?,
                is_trashed: row.get::<_, i32>(13)? != 0,
                trashed_at: row.get(14)?,
                created_at: row.get(15)?,
                ocr_extractor_ref: row.get(17)?,
                ocr_extractor_name: row.get(18)?,
                ocr_engine_version: row.get(19)?,
            })
        })?;
        let mut clips = Vec::new();
        for clip in clip_iter {
            clips.push(clip?);
        }
        append_smart_bin_memberships(&conn, &mut clips)?;
        Ok(clips)
    }

    pub fn get_protected_clips(&self) -> Result<Vec<ClipItem>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare_cached(
            "SELECT id, content_type, text_content, NULL as html_content, NULL as image_base64, image_path, content_hash, source, is_pinned, is_protected, COALESCE(pin_order, 0), bin_id, note, is_trashed, trashed_at, created_at,
                    current_transformation_id IS NOT NULL,
                    ocr_extractor_ref, ocr_extractor_name, ocr_engine_version
             FROM clips WHERE is_protected = 1 AND (is_trashed IS NULL OR is_trashed = 0) ORDER BY created_at DESC"
        )?;
        let clip_iter = stmt.query_map([], |row| {
            let bid: Option<i64> = row.get(11)?;
            Ok(ClipItem {
                id: row.get(0)?,
                content_type: row.get(1)?,
                text_content: row.get(2)?,
                html_content: row.get(3)?,
                image_base64: row.get(4)?,
                image_path: row.get(5)?,
                content_hash: row.get(6)?,
                source: row.get(7)?,
                is_pinned: row.get::<_, i32>(8)? != 0,
                is_protected: row.get::<_, i32>(9)? != 0,
                is_transformed: row.get::<_, i32>(16)? != 0,
                pin_order: row.get(10)?,
                bin_id: bid,
                bin_ids: bid.map(|b| vec![b]),
                note: row.get(12)?,
                is_trashed: row.get::<_, i32>(13)? != 0,
                trashed_at: row.get(14)?,
                created_at: row.get(15)?,
                ocr_extractor_ref: row.get(17)?,
                ocr_extractor_name: row.get(18)?,
                ocr_engine_version: row.get(19)?,
            })
        })?;
        let mut clips = Vec::new();
        for clip in clip_iter {
            clips.push(clip?);
        }
        Ok(clips)
    }

    pub fn update_clip_note(&self, clip_id: i64, note: Option<&str>) -> Result<()> {
        if let Some(note) = note {
            ensure_resource_size(
                note,
                crate::resource_limits::MAX_CLIP_NOTE_BYTES,
                "Clip note",
            )?;
        }
        let conn = self.conn.lock();
        let mut stmt = conn.prepare_cached(
            "UPDATE clips SET note = ?1
             WHERE id = ?2 AND (is_trashed IS NULL OR is_trashed = 0)",
        )?;
        let changed = stmt.execute(params![note, clip_id])?;
        if changed == 0 {
            let exists = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM clips WHERE id = ?1)",
                [clip_id],
                |row| row.get::<_, bool>(0),
            )?;
            return if exists {
                Ok(())
            } else {
                Err(rusqlite::Error::QueryReturnedNoRows)
            };
        }
        let _ = self.log_activity_internal(
            &conn,
            "note_updated",
            &format!("Updated note for clip #{}", clip_id),
        );
        Ok(())
    }

    fn revision_history_limit_internal(conn: &Connection) -> i64 {
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'revisionHistoryLimit'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(50)
        .max(0)
    }

    fn revision_history_enabled_internal(conn: &Connection) -> bool {
        let value = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'enableRevisions'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok();
        crate::features::setting_value_is_enabled(value.as_deref())
    }

    fn prune_clip_versions_internal(conn: &Connection, clip_id: i64) -> Result<()> {
        let limit = Self::revision_history_limit_internal(conn);
        if limit == 0 {
            return Ok(());
        }
        conn.execute(
            "DELETE FROM clip_versions
             WHERE clip_id = ?1 AND id NOT IN (
                SELECT id FROM clip_versions
                WHERE clip_id = ?1 ORDER BY id DESC LIMIT ?2
             )",
            params![clip_id, limit],
        )?;
        Ok(())
    }

    pub fn update_clip_text(&self, clip_id: i64, text: &str) -> Result<()> {
        ensure_resource_size(
            text,
            crate::resource_limits::MAX_CLIP_TEXT_BYTES,
            "Clip text",
        )?;
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let (previous_text, is_trashed, current_transformation_id): (
            Option<String>,
            i32,
            Option<String>,
        ) = tx.query_row(
            "SELECT text_content, COALESCE(is_trashed, 0), current_transformation_id FROM clips WHERE id = ?1",
            params![clip_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;

        if is_trashed != 0 {
            return tx.commit();
        }

        if previous_text.as_deref() == Some(text) {
            return tx.commit();
        }

        if Self::revision_history_enabled_internal(&tx) {
            if let Some(previous_text) = previous_text {
                let context_json = serde_json::to_string(&ClipRevisionContext {
                    schema_version: 1,
                    action_kind: "edit".to_string(),
                    action_label: "Edited clip content".to_string(),
                    organization: None,
                    current_transformation_id,
                })
                .map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))?;
                tx.execute(
                "INSERT INTO clip_versions (clip_id, text_content, context_json) VALUES (?1, ?2, ?3)",
                params![clip_id, previous_text, context_json],
            )?;
                Self::prune_clip_versions_internal(&tx, clip_id)?;
            }
        }
        tx.execute(
            "UPDATE clips SET text_content = ?1, current_transformation_id = NULL WHERE id = ?2",
            params![text, clip_id],
        )?;
        tx.commit()
    }

    fn clear_category_bin_assignments_internal(
        &self,
        conn: &Connection,
        clip_id: i64,
    ) -> Result<()> {
        conn.execute(
            "DELETE FROM clip_bins
             WHERE clip_id = ?1
               AND bin_id IN (
                   SELECT id FROM bins WHERE COALESCE(bin_type, 'category') != 'tag'
               )",
            params![clip_id],
        )?;
        conn.execute(
            "UPDATE clips SET bin_id = NULL WHERE id = ?1",
            params![clip_id],
        )?;
        Ok(())
    }

    pub fn delete_clip(&self, id: i64) -> Result<ClipMutationSummary> {
        self.batch_trash_clips(vec![id])
    }

    pub fn batch_trash_clips(&self, ids: Vec<i64>) -> Result<ClipMutationSummary> {
        let requested_count = ids.len();
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let mut changed_ids = Vec::new();
        for id in ids {
            let changed = tx.execute(
                "UPDATE clips SET is_trashed = 1, trashed_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 WHERE id = ?1
                   AND (is_protected IS NULL OR is_protected = 0)
                   AND (is_trashed IS NULL OR is_trashed = 0)",
                params![id],
            )?;
            if changed > 0 {
                self.clear_category_bin_assignments_internal(&tx, id)?;
                changed_ids.push(id);
            }
        }
        if !changed_ids.is_empty() {
            self.enforce_trash_limit_internal(&tx)?;
        }
        tx.commit()?;
        if !changed_ids.is_empty() {
            let event_type = if changed_ids.len() == 1 {
                "clip_trashed"
            } else {
                "clips_trashed"
            };
            let _ = self.log_activity_internal(
                &conn,
                event_type,
                &format!("Moved {} to Trash", describe_clip_ids(&changed_ids)),
            );
        }
        Ok(ClipMutationSummary::new(
            "trash",
            requested_count,
            changed_ids,
        ))
    }

    pub fn restore_clip(&self, id: i64) -> Result<ClipMutationSummary> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare_cached(
            "UPDATE clips SET is_trashed = 0, trashed_at = NULL WHERE id = ?1 AND is_trashed = 1",
        )?;
        let changed = stmt.execute(params![id])?;
        if changed > 0 {
            let _ = self.log_activity_internal(
                &conn,
                "clip_restored",
                &format!("Restored clip #{} from Trash", id),
            );
        }
        Ok(ClipMutationSummary::new(
            "restore",
            1,
            if changed > 0 { vec![id] } else { Vec::new() },
        ))
    }

    pub fn restore_all_trashed_clips(&self) -> Result<ClipMutationSummary> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let clip_ids = {
            let mut stmt =
                tx.prepare_cached("SELECT id FROM clips WHERE is_trashed = 1 ORDER BY id ASC")?;
            let rows = stmt
                .query_map([], |row| row.get::<_, i64>(0))?
                .collect::<Result<Vec<_>>>()?;
            rows
        };
        let requested_count = clip_ids.len();
        if !clip_ids.is_empty() {
            tx.execute(
                "UPDATE clips SET is_trashed = 0, trashed_at = NULL WHERE is_trashed = 1",
                [],
            )?;
        }
        tx.commit()?;
        if !clip_ids.is_empty() {
            let _ = self.log_activity_internal(
                &conn,
                "clips_restored_all",
                &format!("Restored all clips from Trash ({} items)", clip_ids.len()),
            );
        }
        Ok(ClipMutationSummary::new(
            "restore_all",
            requested_count,
            clip_ids,
        ))
    }

    pub fn purge_clip_permanently(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        let is_protected: i32 = conn
            .query_row(
                "SELECT is_protected FROM clips WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if is_protected != 0 {
            return Ok(());
        }
        let mut stmt = conn.prepare_cached(
            "DELETE FROM clips WHERE id = ?1 AND (is_protected IS NULL OR is_protected = 0)",
        )?;
        stmt.execute(params![id])?;
        let _ = self.log_activity_internal(
            &conn,
            "clip_deleted",
            &format!("Permanently deleted clip #{}", id),
        );
        Ok(())
    }

    pub fn empty_trash(&self) -> Result<()> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM clips WHERE is_trashed = 1 AND (is_protected IS NULL OR is_protected = 0)",
            [],
            |r| r.get(0),
        ).unwrap_or(0);
        let mut stmt = conn.prepare_cached(
            "DELETE FROM clips WHERE is_trashed = 1 AND (is_protected IS NULL OR is_protected = 0)",
        )?;
        stmt.execute([])?;
        let _ = self.log_activity_internal(
            &conn,
            "trash_emptied",
            &format!("Emptied Trash (permanently deleted {} items)", count),
        );
        Ok(())
    }

    pub fn log_activity(&self, event_type: &str, description: &str) -> Result<()> {
        let conn = self.conn.lock();
        self.log_activity_internal(&conn, event_type, description)
    }

    fn log_activity_internal(
        &self,
        conn: &Connection,
        event_type: &str,
        description: &str,
    ) -> Result<()> {
        let is_enabled: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'enableActivityLog'",
                [],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "true".to_string());
        if is_enabled == "false" {
            return Ok(());
        }

        let keep_count: i64 = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'activityLogCapacity'",
                [],
                |r| r.get(0),
            )
            .ok()
            .and_then(|v: String| v.parse().ok())
            .unwrap_or(1000);
        let keep_age_days: i64 = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'activityLogAgeDays'",
                [],
                |r| r.get(0),
            )
            .ok()
            .and_then(|v: String| v.parse().ok())
            .unwrap_or(0);

        let (severity, category, outcome) = activity_classification(event_type);
        let mut stmt = conn.prepare_cached(
            "INSERT INTO activity_logs (
                event_type, description, created_at, observed_at,
                severity_text, category, outcome, attributes_json
             ) VALUES (
                ?1, ?2,
                strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                ?3, ?4, ?5, '{}'
             )",
        )?;
        stmt.execute(params![
            event_type,
            description,
            severity,
            category,
            outcome
        ])?;

        self.enforce_activity_retention_internal(conn, keep_count, keep_age_days)
    }

    fn enforce_activity_retention_internal(
        &self,
        conn: &Connection,
        keep_count: i64,
        keep_age_days: i64,
    ) -> Result<()> {
        let keep_count = keep_count.max(0);
        let keep_age_days = keep_age_days.max(0);

        if keep_age_days > 0 {
            let age_modifier = format!("-{keep_age_days} days");
            conn.execute(
                "DELETE FROM activity_logs WHERE datetime(created_at) < datetime('now', ?1)",
                [age_modifier],
            )?;
        }

        if keep_count > 0 {
            let mut purge_stmt = conn.prepare_cached(
                "DELETE FROM activity_logs WHERE id NOT IN (SELECT id FROM activity_logs ORDER BY created_at DESC, id DESC LIMIT ?1)"
            )?;
            purge_stmt.execute(params![keep_count])?;
        }
        Ok(())
    }

    pub fn get_activity_logs(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<ActivityLog>> {
        self.get_activity_logs_filtered(limit, offset, None, None, None)
    }

    pub fn get_activity_logs_filtered(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
        category: Option<&str>,
        severity: Option<&str>,
        event_name: Option<&str>,
    ) -> Result<Vec<ActivityLog>> {
        let conn = self.conn.lock();
        let lim = limit.unwrap_or(100).clamp(1, 100_000);
        let off = offset.unwrap_or(0).max(0);
        let mut stmt = conn.prepare_cached(
            "SELECT id, event_type, description, created_at,
                    COALESCE(observed_at, created_at), severity_text, category, outcome, attributes_json
             FROM activity_logs
             WHERE (?1 IS NULL OR category = ?1)
               AND (?2 IS NULL OR severity_text = ?2)
               AND (?3 IS NULL OR event_type = ?3)
             ORDER BY created_at DESC, id DESC LIMIT ?4 OFFSET ?5"
        )?;
        let log_iter =
            stmt.query_map(params![category, severity, event_name, lim, off], |row| {
                let attributes_json: String = row.get(8)?;
                Ok(ActivityLog {
                    id: row.get(0)?,
                    event_type: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    observed_at: row.get(4)?,
                    severity_text: row.get(5)?,
                    category: row.get(6)?,
                    outcome: row.get(7)?,
                    attributes: serde_json::from_str(&attributes_json)
                        .unwrap_or_else(|_| serde_json::json!({})),
                })
            })?;
        let mut logs = Vec::new();
        for log in log_iter {
            logs.push(log?);
        }
        Ok(logs)
    }

    pub fn export_activity_json(&self) -> Result<String> {
        let entries = self
            .get_activity_logs(Some(i64::MAX), Some(0))?
            .into_iter()
            .map(Self::activity_archive_entry)
            .collect::<Result<Vec<_>>>()?;
        let mut resource = serde_json::Map::new();
        resource.insert("service.name".to_string(), serde_json::json!("Pasted"));
        resource.insert(
            "service.version".to_string(),
            serde_json::json!(env!("CARGO_PKG_VERSION")),
        );
        resource.insert(
            "telemetry.schema".to_string(),
            serde_json::json!("pasted.activity.v1"),
        );
        let archive = ActivityArchive {
            schema_version: 1,
            exported_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            resource,
            entries,
        };
        serde_json::to_string_pretty(&archive)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
    }

    pub fn export_activity_csv(&self) -> Result<String> {
        fn cell(value: &str) -> String {
            let escaped = value.replace('"', "\"\"");
            let neutralized = if matches!(
                value.chars().next(),
                Some('=' | '+' | '-' | '@' | '\t' | '\r')
            ) {
                format!("'{escaped}")
            } else {
                escaped
            };
            format!("\"{neutralized}\"")
        }

        let entries = self
            .get_activity_logs(Some(i64::MAX), Some(0))?
            .into_iter()
            .map(Self::activity_archive_entry)
            .collect::<Result<Vec<_>>>()?;
        let mut csv = String::from(
            "timestamp,observed_timestamp,event_name,severity_text,body,category,outcome,attributes_json\n",
        );
        for entry in entries {
            let category = entry
                .attributes
                .get("pasted.category")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("general");
            let outcome = entry
                .attributes
                .get("pasted.outcome")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown");
            let attributes_json = serde_json::Value::Object(entry.attributes.clone()).to_string();
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                cell(&entry.timestamp),
                cell(&entry.observed_timestamp),
                cell(&entry.event_name),
                cell(&entry.severity_text),
                cell(&entry.body),
                cell(category),
                cell(outcome),
                cell(&attributes_json),
            ));
        }
        Ok(csv)
    }

    fn activity_archive_entry(log: ActivityLog) -> Result<ActivityArchiveEntry> {
        let mut attributes = log.attributes.as_object().cloned().unwrap_or_default();
        attributes.insert(
            "pasted.category".to_string(),
            serde_json::json!(log.category),
        );
        attributes.insert("pasted.outcome".to_string(), serde_json::json!(log.outcome));
        attributes.insert("event.sequence".to_string(), serde_json::json!(log.id));
        Ok(ActivityArchiveEntry {
            timestamp: canonical_activity_timestamp(&log.created_at)?,
            observed_timestamp: canonical_activity_timestamp(&log.observed_at)?,
            event_name: log.event_type,
            severity_text: log.severity_text,
            body: log.description,
            attributes,
        })
    }

    pub fn import_activity_json(&self, json: &str) -> Result<ActivityImportReport> {
        let entries = Self::parse_activity_json_import(json)?;
        self.apply_activity_entries(entries, true)
    }

    pub fn inspect_activity_json(&self, json: &str) -> Result<ActivityImportReport> {
        let entries = Self::parse_activity_json_import(json)?;
        self.apply_activity_entries(entries, false)
    }

    fn parse_activity_json_import(json: &str) -> Result<Vec<ActivityArchiveEntry>> {
        use crate::resource_limits::{MAX_ACTIVITY_IMPORT_BYTES, MAX_ACTIVITY_IMPORT_ROWS};

        ensure_resource_size(json, MAX_ACTIVITY_IMPORT_BYTES, "Activity import")?;
        let archive: ActivityArchive = serde_json::from_str(json).map_err(|error| {
            rusqlite::Error::InvalidParameterName(format!("invalid Activity JSON: {error}"))
        })?;
        if archive.schema_version != 1 {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "unsupported Activity JSON schema version {} (supported: 1)",
                archive.schema_version
            )));
        }
        if archive.entries.len() > MAX_ACTIVITY_IMPORT_ROWS {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "Activity import contains more than {MAX_ACTIVITY_IMPORT_ROWS} entries"
            )));
        }

        Ok(archive.entries)
    }

    pub fn import_activity_csv(&self, csv: &str) -> Result<ActivityImportReport> {
        let entries = Self::parse_activity_csv_import(csv)?;
        self.apply_activity_entries(entries, true)
    }

    pub fn inspect_activity_csv(&self, csv: &str) -> Result<ActivityImportReport> {
        let entries = Self::parse_activity_csv_import(csv)?;
        self.apply_activity_entries(entries, false)
    }

    fn parse_activity_csv_import(csv: &str) -> Result<Vec<ActivityArchiveEntry>> {
        use crate::resource_limits::{MAX_ACTIVITY_IMPORT_BYTES, MAX_ACTIVITY_IMPORT_ROWS};

        ensure_resource_size(csv, MAX_ACTIVITY_IMPORT_BYTES, "Activity CSV import")?;
        let records = Self::parse_csv(csv)?;
        if records.len().saturating_sub(1) > MAX_ACTIVITY_IMPORT_ROWS {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "Activity import contains more than {MAX_ACTIVITY_IMPORT_ROWS} entries"
            )));
        }
        let expected = [
            "timestamp",
            "observed_timestamp",
            "event_name",
            "severity_text",
            "body",
            "category",
            "outcome",
            "attributes_json",
        ];
        if records.first().map(|header| {
            header
                .iter()
                .map(String::as_str)
                .eq(expected.iter().copied())
        }) != Some(true)
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "Activity CSV header does not match the supported export format".to_string(),
            ));
        }

        let mut entries = Vec::with_capacity(records.len().saturating_sub(1));
        for (index, row) in records.into_iter().skip(1).enumerate() {
            if row.len() != expected.len() {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "Activity CSV row {} has {} columns; expected {}",
                    index + 2,
                    row.len(),
                    expected.len()
                )));
            }
            let attributes_value: serde_json::Value =
                serde_json::from_str(&row[7]).map_err(|_| {
                    rusqlite::Error::InvalidParameterName(format!(
                        "Activity CSV row {} has invalid attributes JSON",
                        index + 2
                    ))
                })?;
            let mut attributes = attributes_value.as_object().cloned().ok_or_else(|| {
                rusqlite::Error::InvalidParameterName(format!(
                    "Activity CSV row {} attributes must be a JSON object",
                    index + 2
                ))
            })?;
            attributes.insert(
                "pasted.category".to_string(),
                serde_json::Value::String(row[5].clone()),
            );
            attributes.insert(
                "pasted.outcome".to_string(),
                serde_json::Value::String(row[6].clone()),
            );
            entries.push(ActivityArchiveEntry {
                timestamp: row[0].clone(),
                observed_timestamp: row[1].clone(),
                event_name: row[2].clone(),
                severity_text: row[3].clone(),
                body: row[4].clone(),
                attributes,
            });
        }

        Ok(entries)
    }

    fn apply_activity_entries(
        &self,
        entries: Vec<ActivityArchiveEntry>,
        commit: bool,
    ) -> Result<ActivityImportReport> {
        use crate::resource_limits::{
            MAX_ACTIVITY_ATTRIBUTES_BYTES, MAX_ACTIVITY_DESCRIPTION_BYTES,
            MAX_ACTIVITY_EVENT_TYPE_BYTES,
        };

        let scanned_count = entries.len();
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let mut imported_count = 0usize;
        let mut duplicate_count = 0usize;
        {
            let mut duplicate = tx.prepare_cached(
                "SELECT EXISTS(SELECT 1 FROM activity_logs WHERE event_type = ?1 AND description = ?2 AND created_at = ?3)",
            )?;
            let mut insert = tx.prepare_cached(
                "INSERT INTO activity_logs (
                    event_type, description, created_at, observed_at,
                    severity_text, category, outcome, attributes_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )?;
            for entry in entries {
                let event_type = entry.event_name.trim();
                let description = entry.body.trim();
                if event_type.is_empty()
                    || event_type.len() > MAX_ACTIVITY_EVENT_TYPE_BYTES
                    || !event_type.chars().all(|character| {
                        character.is_ascii_alphanumeric()
                            || matches!(character, '_' | '-' | '.' | ':')
                    })
                {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "Activity import contains an invalid event type".to_string(),
                    ));
                }
                if description.is_empty() || description.len() > MAX_ACTIVITY_DESCRIPTION_BYTES {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "Activity import contains an invalid description".to_string(),
                    ));
                }
                let created_at = chrono::DateTime::parse_from_rfc3339(&entry.timestamp)
                    .map_err(|_| {
                        rusqlite::Error::InvalidParameterName(
                            "Activity import contains an invalid timestamp".to_string(),
                        )
                    })?
                    .with_timezone(&chrono::Utc)
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                let observed_at = chrono::DateTime::parse_from_rfc3339(&entry.observed_timestamp)
                    .map_err(|_| {
                        rusqlite::Error::InvalidParameterName(
                            "Activity import contains an invalid observed timestamp".to_string(),
                        )
                    })?
                    .with_timezone(&chrono::Utc)
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                let severity = entry.severity_text.to_ascii_lowercase();
                if !matches!(severity.as_str(), "info" | "warn" | "error") {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "Activity import contains an unsupported severity".to_string(),
                    ));
                }
                let category = entry
                    .attributes
                    .get("pasted.category")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("general");
                if category.is_empty()
                    || category.len() > 64
                    || !category.chars().all(|character| {
                        character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
                    })
                {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "Activity import contains an invalid category".to_string(),
                    ));
                }
                let outcome = entry
                    .attributes
                    .get("pasted.outcome")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown");
                if !matches!(outcome, "success" | "failure" | "unknown") {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "Activity import contains an unsupported outcome".to_string(),
                    ));
                }
                let attributes_json = serde_json::to_string(&entry.attributes)
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
                if attributes_json.len() > MAX_ACTIVITY_ATTRIBUTES_BYTES {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "Activity import contains oversized attributes".to_string(),
                    ));
                }
                let exists: bool = duplicate
                    .query_row(params![event_type, description, created_at], |row| {
                        row.get(0)
                    })?;
                if exists {
                    duplicate_count += 1;
                    continue;
                }
                insert.execute(params![
                    event_type,
                    description,
                    created_at,
                    observed_at,
                    severity,
                    category,
                    outcome,
                    attributes_json,
                ])?;
                imported_count += 1;
            }
        }

        let keep_count = tx
            .query_row(
                "SELECT value FROM settings WHERE key = 'activityLogCapacity'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(1000);
        let keep_age_days = tx
            .query_row(
                "SELECT value FROM settings WHERE key = 'activityLogAgeDays'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        self.enforce_activity_retention_internal(&tx, keep_count, keep_age_days)?;
        let retained_count = tx.query_row("SELECT COUNT(*) FROM activity_logs", [], |row| {
            row.get::<_, i64>(0)
        })? as usize;
        if commit {
            tx.commit()?;
        } else {
            tx.rollback()?;
        }

        Ok(ActivityImportReport {
            scanned_count,
            imported_count,
            duplicate_count,
            retained_count,
        })
    }

    pub fn clear_activity_logs(&self) -> Result<()> {
        let conn = self.conn.lock();
        let _ = conn.execute("DELETE FROM activity_logs", [])?;
        Ok(())
    }

    pub fn batch_pin_clips(&self, ids: Vec<i64>, pin_state: bool) -> Result<ClipMutationSummary> {
        let requested_count = ids.len();
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let mut changed_ids = Vec::new();
        let mut seen_ids = HashSet::new();
        for id in ids {
            if !seen_ids.insert(id) {
                continue;
            }
            let current = tx
                .query_row(
                    "SELECT is_pinned FROM clips WHERE id = ?1",
                    params![id],
                    |row| row.get::<_, i32>(0),
                )
                .optional()?;
            if current.is_some_and(|value| (value != 0) != pin_state) {
                changed_ids.push(id);
            }
        }
        if pin_state && !changed_ids.is_empty() {
            tx.execute(
                "UPDATE clips SET pin_order = COALESCE(pin_order, 0) + ?1 WHERE is_pinned = 1",
                params![changed_ids.len() as i32],
            )?;
        }
        for (index, id) in changed_ids.iter().enumerate() {
            tx.execute(
                "UPDATE clips SET is_pinned = ?1, pin_order = ?2 WHERE id = ?3",
                params![
                    if pin_state { 1 } else { 0 },
                    if pin_state { index as i32 } else { 0 },
                    id
                ],
            )?;
        }
        tx.commit()?;
        if !changed_ids.is_empty() {
            let event_type = match (pin_state, changed_ids.len()) {
                (true, 1) => "clip_pinned",
                (true, _) => "clips_pinned",
                (false, 1) => "clip_unpinned",
                (false, _) => "clips_unpinned",
            };
            let verb = if pin_state { "Pinned" } else { "Unpinned" };
            let _ = self.log_activity_internal(
                &conn,
                event_type,
                &format!("{} {}", verb, describe_clip_ids(&changed_ids)),
            );
        }
        Ok(ClipMutationSummary::new(
            if pin_state { "pin" } else { "unpin" },
            requested_count,
            changed_ids,
        ))
    }

    pub fn batch_assign_bin_clips(
        &self,
        ids: Vec<i64>,
        bin_id: Option<i64>,
    ) -> Result<ClipMutationSummary> {
        let requested_count = ids.len();
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        if let Some(bin_id) = bin_id {
            let is_manual = tx
                .query_row(
                    "SELECT smart_rule IS NULL FROM bins WHERE id = ?1",
                    params![bin_id],
                    |row| row.get::<_, bool>(0),
                )
                .optional()?
                .unwrap_or(false);
            if !is_manual {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Clips can only be added directly to manual Bins".to_string(),
                ));
            }
        }
        let mut changed_ids = Vec::new();
        for clip_id in ids {
            let is_active = tx
                .query_row(
                    "SELECT 1 FROM clips WHERE id = ?1 AND (is_trashed IS NULL OR is_trashed = 0)",
                    params![clip_id],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?
                .is_some();
            if !is_active {
                continue;
            }
            if let Some(bid) = bin_id {
                let already_assigned = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM clip_bins WHERE clip_id = ?1 AND bin_id = ?2)",
                    params![clip_id, bid],
                    |row| row.get::<_, bool>(0),
                )?;
                if already_assigned {
                    continue;
                }
                tx.execute(
                    "INSERT OR REPLACE INTO clip_bins (clip_id, bin_id) VALUES (?1, ?2)",
                    params![clip_id, bid],
                )?;
                tx.execute(
                    "UPDATE clips SET bin_id = ?1 WHERE id = ?2",
                    params![bid, clip_id],
                )?;
            } else {
                let has_manual_bins = tx.query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM clip_bins membership
                        JOIN bins ON bins.id = membership.bin_id
                        WHERE membership.clip_id = ?1 AND bins.smart_rule IS NULL
                    )",
                    params![clip_id],
                    |row| row.get::<_, bool>(0),
                )?;
                if !has_manual_bins {
                    continue;
                }
                tx.execute(
                    "DELETE FROM clip_bins
                     WHERE clip_id = ?1 AND bin_id IN (
                        SELECT id FROM bins WHERE smart_rule IS NULL
                     )",
                    params![clip_id],
                )?;
                tx.execute(
                    "UPDATE clips SET bin_id = NULL WHERE id = ?1",
                    params![clip_id],
                )?;
            }
            changed_ids.push(clip_id);
        }
        tx.commit()?;
        if !changed_ids.is_empty() {
            let assigned = bin_id.is_some();
            let event_type = match (assigned, changed_ids.len()) {
                (true, 1) => "clip_bin_assigned",
                (true, _) => "clips_bin_assigned",
                (false, 1) => "clip_bin_unassigned",
                (false, _) => "clips_bin_unassigned",
            };
            let _ = self.log_activity_internal(
                &conn,
                event_type,
                &bin_id.map_or_else(
                    || {
                        format!(
                            "Removed {} from all manual Bins",
                            describe_clip_ids(&changed_ids)
                        )
                    },
                    |id| format!("Added {} to Bin #{id}", describe_clip_ids(&changed_ids)),
                ),
            );
        }
        Ok(ClipMutationSummary::new(
            if bin_id.is_some() {
                "assign_bin"
            } else {
                "unassign_bin"
            },
            requested_count,
            changed_ids,
        ))
    }

    pub fn batch_remove_bin_clips(
        &self,
        ids: Vec<i64>,
        bin_id: i64,
    ) -> Result<ClipMutationSummary> {
        let requested_count = ids.len();
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let is_manual = tx
            .query_row(
                "SELECT smart_rule IS NULL FROM bins WHERE id = ?1",
                params![bin_id],
                |row| row.get::<_, bool>(0),
            )
            .optional()?
            .unwrap_or(false);
        if !is_manual {
            return Err(rusqlite::Error::InvalidParameterName(
                "Clips can only be removed directly from manual Bins".to_string(),
            ));
        }
        let mut changed_ids = Vec::new();
        for clip_id in ids {
            let removed = tx.execute(
                "DELETE FROM clip_bins
                 WHERE clip_id = ?1 AND bin_id = ?2
                   AND EXISTS(
                       SELECT 1 FROM clips
                       WHERE id = ?1 AND (is_trashed IS NULL OR is_trashed = 0)
                   )",
                params![clip_id, bin_id],
            )?;
            if removed == 0 {
                continue;
            }
            tx.execute(
                "UPDATE clips
                 SET bin_id = (
                     SELECT membership.bin_id FROM clip_bins membership
                     JOIN bins ON bins.id = membership.bin_id
                     WHERE membership.clip_id = clips.id AND bins.smart_rule IS NULL
                     ORDER BY membership.bin_id ASC LIMIT 1
                 )
                 WHERE id = ?1 AND bin_id = ?2",
                params![clip_id, bin_id],
            )?;
            changed_ids.push(clip_id);
        }
        tx.commit()?;
        if !changed_ids.is_empty() {
            let event_type = if changed_ids.len() == 1 {
                "clip_bin_removed"
            } else {
                "clips_bin_removed"
            };
            let _ = self.log_activity_internal(
                &conn,
                event_type,
                &format!(
                    "Removed {} from Bin #{bin_id}",
                    describe_clip_ids(&changed_ids)
                ),
            );
        }
        Ok(ClipMutationSummary::new(
            "remove_bin",
            requested_count,
            changed_ids,
        ))
    }

    pub fn get_analytics_summary(&self) -> Result<AnalyticsSummary> {
        let conn = self.conn.lock();

        let (total_clips, total_chars): (i64, i64) = conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(LENGTH(text_content)), 0) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0)",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap_or((0, 0));

        let mut source_stmt = conn.prepare(
            "SELECT source, COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) GROUP BY source ORDER BY COUNT(*) DESC LIMIT 8"
        )?;
        let top_sources = source_stmt
            .query_map([], |r| {
                Ok(SourceStat {
                    name: r.get(0)?,
                    count: r.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        let mut type_stmt = conn.prepare(
            "SELECT content_type, COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) GROUP BY content_type"
        )?;
        let content_types = type_stmt
            .query_map([], |r| {
                Ok(TypeStat {
                    content_type: r.get(0)?,
                    count: r.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        let mut daily_stmt = conn.prepare(
            "WITH RECURSIVE recent_days(day) AS (
                SELECT date('now', '-13 days')
                UNION ALL
                SELECT date(day, '+1 day') FROM recent_days WHERE day < date('now')
             )
             SELECT recent_days.day, COUNT(clips.id)
             FROM recent_days
             LEFT JOIN clips
               ON date(clips.created_at) = recent_days.day
              AND (clips.is_trashed IS NULL OR clips.is_trashed = 0)
             GROUP BY recent_days.day
             ORDER BY recent_days.day DESC",
        )?;
        let daily_activity = daily_stmt
            .query_map([], |r| {
                Ok(DailyStat {
                    date: r.get(0)?,
                    count: r.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(AnalyticsSummary {
            total_clips,
            total_chars,
            top_sources,
            content_types,
            daily_activity,
        })
    }

    pub fn get_clip_collection_summary(&self) -> Result<ClipCollectionSummary> {
        let conn = self.conn.lock();
        let (active_count, trash_count, pinned_count, protected_count, noted_count) = conn.query_row(
            "SELECT
                COALESCE(SUM(CASE WHEN COALESCE(is_trashed, 0) = 0 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN is_trashed = 1 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN COALESCE(is_trashed, 0) = 0 AND is_pinned = 1 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN COALESCE(is_trashed, 0) = 0 AND COALESCE(is_protected, 0) = 1 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN COALESCE(is_trashed, 0) = 0 AND TRIM(COALESCE(note, '')) != '' THEN 1 ELSE 0 END), 0)
             FROM clips",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )?;
        let type_counts = conn
            .prepare(
                "SELECT content_type, COUNT(*) FROM clips
                 WHERE COALESCE(is_trashed, 0) = 0
                 GROUP BY content_type ORDER BY content_type",
            )?
            .query_map([], |row| {
                Ok(TypeStat {
                    content_type: row.get(0)?,
                    count: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        let source_counts = conn
            .prepare(
                "SELECT source, COUNT(*) FROM clips
                 WHERE COALESCE(is_trashed, 0) = 0
                 GROUP BY source ORDER BY COUNT(*) DESC, source",
            )?
            .query_map([], |row| {
                Ok(SourceStat {
                    name: row.get(0)?,
                    count: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        Ok(ClipCollectionSummary {
            active_count,
            trash_count,
            pinned_count,
            protected_count,
            noted_count,
            type_counts,
            source_counts,
        })
    }

    pub fn trash_unpinned_clips(&self) -> Result<()> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM clips WHERE is_pinned = 0 AND (is_protected IS NULL OR is_protected = 0) AND (is_trashed IS NULL OR is_trashed = 0)", [], |r| r.get(0)).unwrap_or(0);
        conn.execute(
            "UPDATE clips SET is_trashed = 1, trashed_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE is_pinned = 0 AND (is_protected IS NULL OR is_protected = 0) AND (is_trashed IS NULL OR is_trashed = 0)",
            [],
        )?;
        conn.execute(
            "DELETE FROM clip_bins
             WHERE clip_id IN (SELECT id FROM clips WHERE is_trashed = 1)
               AND bin_id IN (
                   SELECT id FROM bins WHERE COALESCE(bin_type, 'category') != 'tag'
               )",
            [],
        )?;
        conn.execute("UPDATE clips SET bin_id = NULL WHERE is_trashed = 1", [])?;
        let _ = self.log_activity_internal(
            &conn,
            "clips_trashed_all",
            &format!(
                "Moved all unpinned and unprotected clips to Trash ({} items)",
                count
            ),
        );
        let _ = self.enforce_trash_limit_internal(&conn);
        Ok(())
    }

    pub fn purge_unpinned_clips(&self) -> Result<()> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM clips WHERE is_pinned = 0 AND (is_protected IS NULL OR is_protected = 0)", [], |r| r.get(0)).unwrap_or(0);
        conn.execute(
            "DELETE FROM clips WHERE is_pinned = 0 AND (is_protected IS NULL OR is_protected = 0)",
            [],
        )?;
        let _ = self.log_activity_internal(
            &conn,
            "clips_purged_all",
            &format!(
                "Permanently deleted all unpinned and unprotected clips ({} items)",
                count
            ),
        );
        Ok(())
    }

    pub fn clear_all_clips(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM clips WHERE (is_protected IS NULL OR is_protected = 0)",
            [],
        )?;
        Ok(())
    }

    pub fn toggle_protected(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock();
        let current_protected: i32 = conn
            .query_row(
                "SELECT is_protected FROM clips WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        drop(conn);
        let new_protected = current_protected == 0;
        self.batch_protect_clips(vec![id], new_protected)?;
        Ok(new_protected)
    }

    pub fn batch_protect_clips(
        &self,
        ids: Vec<i64>,
        protected_state: bool,
    ) -> Result<ClipMutationSummary> {
        let requested_count = ids.len();
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let mut changed_ids = Vec::new();
        for id in ids {
            let changed = tx.execute(
                "UPDATE clips SET is_protected = ?1
                 WHERE id = ?2 AND COALESCE(is_protected, 0) != ?1",
                params![if protected_state { 1 } else { 0 }, id],
            )?;
            if changed > 0 {
                changed_ids.push(id);
            }
        }
        tx.commit()?;
        if !changed_ids.is_empty() {
            let event_type = if changed_ids.len() == 1 {
                "clip_protected_toggled"
            } else {
                "clips_protected_toggled"
            };
            let verb = if protected_state {
                "Protected"
            } else {
                "Unprotected"
            };
            let _ = self.log_activity_internal(
                &conn,
                event_type,
                &format!("{} {}", verb, describe_clip_ids(&changed_ids)),
            );
        }
        Ok(ClipMutationSummary::new(
            if protected_state {
                "protect"
            } else {
                "unprotect"
            },
            requested_count,
            changed_ids,
        ))
    }

    pub fn toggle_pin(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock();
        let current_pinned: i32 = conn.query_row(
            "SELECT is_pinned FROM clips WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )?;
        drop(conn);
        let new_pinned = current_pinned == 0;
        self.batch_pin_clips(vec![id], new_pinned)?;
        Ok(new_pinned)
    }

    pub fn assign_to_bin(&self, clip_id: i64, bin_id: Option<i64>) -> Result<ClipMutationSummary> {
        self.batch_assign_bin_clips(vec![clip_id], bin_id)
    }

    pub fn add_clip_to_bin(&self, clip_id: i64, bin_id: i64) -> Result<()> {
        let conn = self.conn.lock();
        let is_active = conn
            .query_row(
                "SELECT CASE WHEN is_trashed IS NULL OR is_trashed = 0 THEN 1 ELSE 0 END
             FROM clips WHERE id = ?1",
                params![clip_id],
                |row| row.get::<_, i32>(0),
            )
            .unwrap_or(0)
            != 0;
        if !is_active {
            return Ok(());
        }
        conn.execute(
            "INSERT OR REPLACE INTO clip_bins (clip_id, bin_id) VALUES (?1, ?2)",
            params![clip_id, bin_id],
        )?;
        conn.execute(
            "UPDATE clips SET bin_id = ?1 WHERE id = ?2",
            params![bin_id, clip_id],
        )?;
        Ok(())
    }

    pub fn remove_clip_from_bin(&self, clip_id: i64, bin_id: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM clip_bins WHERE clip_id = ?1 AND bin_id = ?2",
            params![clip_id, bin_id],
        )?;
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    pub fn get_bins(&self) -> Result<Vec<Bin>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT id, name, icon, color, smart_rule, COALESCE(bin_type, 'category'), shortcut, created_at FROM bins ORDER BY id ASC")?;
        let bin_rows: Vec<(
            i64,
            String,
            String,
            String,
            Option<String>,
            String,
            Option<String>,
            String,
        )> = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut bins = Vec::new();
        for (id, name, icon, color, smart_rule, bin_type, shortcut, created_at) in bin_rows {
            let count: i64 = if let Some(ref sr_json) = smart_rule {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(sr_json) {
                    let match_mode = parsed["match"].as_str().unwrap_or("any");
                    let is_and = match_mode == "all";
                    let join_op = if is_and { " AND " } else { " OR " };

                    let mut cond_sqls: Vec<String> = Vec::new();
                    let mut query_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

                    if let Some(conds) = parsed["conditions"].as_array() {
                        for cond in conds {
                            let c_type = cond["type"].as_str().unwrap_or("");
                            let c_val = cond["value"].as_str().unwrap_or("");
                            push_smart_condition(c_type, c_val, &mut cond_sqls, &mut query_params);
                        }
                    } else {
                        let rule_type = parsed["type"].as_str().unwrap_or("");
                        let rule_val = parsed["value"].as_str().unwrap_or("");
                        push_smart_condition(
                            rule_type,
                            rule_val,
                            &mut cond_sqls,
                            &mut query_params,
                        );
                    }

                    if !cond_sqls.is_empty() {
                        let combined = cond_sqls.join(join_op);
                        let sql = format!("SELECT COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) AND (({}) OR bin_id = ? OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?))", combined);
                        query_params.push(Box::new(id));
                        query_params.push(Box::new(id));
                        let param_refs: Vec<&dyn rusqlite::ToSql> =
                            query_params.iter().map(|p| p.as_ref()).collect();
                        conn.query_row(&sql, param_refs.as_slice(), |r| r.get(0))
                            .unwrap_or(0)
                    } else {
                        conn.query_row("SELECT COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) AND (bin_id = ?1 OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?1))", params![id], |r| r.get(0)).unwrap_or(0)
                    }
                } else {
                    conn.query_row("SELECT COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) AND (bin_id = ?1 OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?1))", params![id], |r| r.get(0)).unwrap_or(0)
                }
            } else {
                conn.query_row("SELECT COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) AND (bin_id = ?1 OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?1))", params![id], |r| r.get(0)).unwrap_or(0)
            };

            let clip_order = {
                let mut order_statement = conn.prepare(
                    "SELECT clip_id FROM bin_clip_order WHERE bin_id = ?1 ORDER BY position ASC",
                )?;
                let ordered_ids = order_statement
                    .query_map(params![id], |row| row.get::<_, i64>(0))?
                    .collect::<Result<Vec<_>, _>>()?;
                ordered_ids
            };

            bins.push(Bin {
                id,
                name,
                icon,
                color,
                smart_rule,
                bin_type,
                shortcut,
                clip_count: Some(count),
                clip_order,
                created_at,
            });
        }
        Ok(bins)
    }

    pub fn get_bin(&self, id: i64) -> Result<Bin> {
        self.get_bins()?
            .into_iter()
            .find(|bin| bin.id == id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn update_bin_shortcut(&self, id: i64, shortcut: Option<&str>) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE bins SET shortcut = ?1 WHERE id = ?2",
            params![shortcut, id],
        )?;
        Ok(())
    }

    pub fn get_bin_transform_ref(&self, bin_id: i64) -> Result<Option<String>> {
        let conn = self.conn.lock();
        let transform_id: Option<String> = conn.query_row(
            "SELECT default_transform_id FROM bins WHERE id = ?1",
            params![bin_id],
            |row| row.get(0),
        )?;
        Ok(transform_id.map(|id| format!("transform:{id}")))
    }

    pub fn set_bin_transform_ref(&self, bin_id: i64, transform_ref: Option<&str>) -> Result<()> {
        let transform_id =
            transform_ref.map(|value| value.strip_prefix("transform:").unwrap_or(value));
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE bins SET default_transform_id = ?1 WHERE id = ?2",
            params![transform_id, bin_id],
        )?;
        Ok(())
    }

    pub fn matching_smart_bin_transforms(
        &self,
        content_type: &str,
        text: &str,
        source: &str,
    ) -> Result<Vec<(i64, String)>> {
        let file_paths = if content_type.eq_ignore_ascii_case("file") {
            serde_json::from_str::<Vec<String>>(text).unwrap_or_default()
        } else {
            Vec::new()
        };
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, smart_rule, default_transform_id FROM bins
             WHERE smart_rule IS NOT NULL AND default_transform_id IS NOT NULL ORDER BY id ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        let mut matches = Vec::new();
        for row in rows {
            let (bin_id, rule_json, transform_id) = row?;
            let Ok(rule) = serde_json::from_str::<serde_json::Value>(&rule_json) else {
                continue;
            };
            let condition_matches = |kind: &str, value: &str| match kind {
                "content_type" => content_type.eq_ignore_ascii_case(value),
                "source" => source.to_lowercase().contains(&value.to_lowercase()),
                "contains" => text.to_lowercase().contains(&value.to_lowercase()),
                "origin_kind" => {
                    derived_origin_kind(content_type, source).eq_ignore_ascii_case(value.trim())
                }
                "file_extension" => {
                    let extension = value.trim().trim_start_matches('.').to_lowercase();
                    !extension.is_empty()
                        && file_paths
                            .iter()
                            .any(|path| path.to_lowercase().ends_with(&format!(".{extension}")))
                }
                "file_path" => {
                    let value = value.trim().to_lowercase();
                    !value.is_empty()
                        && file_paths
                            .iter()
                            .any(|path| path.to_lowercase().contains(&value))
                }
                _ => false,
            };
            let matched = if let Some(conditions) = rule["conditions"].as_array() {
                let values = conditions
                    .iter()
                    .map(|condition| {
                        condition_matches(
                            condition["type"].as_str().unwrap_or(""),
                            condition["value"].as_str().unwrap_or(""),
                        )
                    })
                    .collect::<Vec<_>>();
                if rule["match"].as_str() == Some("all") {
                    values.iter().all(|v| *v)
                } else {
                    values.iter().any(|v| *v)
                }
            } else {
                condition_matches(
                    rule["type"].as_str().unwrap_or(""),
                    rule["value"].as_str().unwrap_or(""),
                )
            };
            if matched {
                matches.push((bin_id, format!("transform:{transform_id}")));
            }
        }
        Ok(matches)
    }

    pub fn create_bin_with_type(
        &self,
        name: &str,
        icon: &str,
        color: &str,
        smart_rule: Option<&str>,
        bin_type: &str,
    ) -> Result<Bin> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO bins (name, icon, color, smart_rule, bin_type) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![name, icon, color, smart_rule, bin_type],
        )?;
        let id = conn.last_insert_rowid();
        conn.query_row(
            "SELECT id, name, icon, color, smart_rule, COALESCE(bin_type, 'category'), shortcut, created_at FROM bins WHERE id = ?1",
            params![id],
            |row| {
                Ok(Bin {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    icon: row.get(2)?,
                    color: row.get(3)?,
                    smart_rule: row.get(4)?,
                    bin_type: row.get(5)?,
                    shortcut: row.get(6)?,
                    clip_count: Some(0),
                    clip_order: Vec::new(),
                    created_at: row.get(7)?,
                })
            },
        )
    }

    pub fn create_bin(
        &self,
        name: &str,
        icon: &str,
        color: &str,
        smart_rule: Option<&str>,
    ) -> Result<Bin> {
        self.create_bin_with_type(name, icon, color, smart_rule, "category")
    }

    pub fn reorder_pinned_clips(&self, ids: Vec<i64>) -> Result<()> {
        if ids.len() > 100_000 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Pinned order exceeds Pasted's safety limit".to_string(),
            ));
        }
        let requested = ids
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        if requested.len() != ids.len() {
            return Err(rusqlite::Error::InvalidParameterName(
                "Pinned order contains duplicate clips".to_string(),
            ));
        }
        let current = self
            .get_clips(None, None, true)?
            .into_iter()
            .map(|clip| clip.id)
            .collect::<std::collections::HashSet<_>>();
        if requested != current {
            return Err(rusqlite::Error::InvalidParameterName(
                "Pinned order must contain every current pinned clip exactly once".to_string(),
            ));
        }
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        for (idx, id) in ids.iter().enumerate() {
            tx.execute(
                "UPDATE clips SET pin_order = ?1 WHERE id = ?2",
                params![idx as i32, id],
            )?;
        }
        tx.commit()
    }

    pub fn reorder_bin_clips(&self, bin_id: i64, ids: Vec<i64>) -> Result<()> {
        if ids.len() > 100_000 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Bin order exceeds Pasted's safety limit".to_string(),
            ));
        }
        let unique = ids
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        if unique.len() != ids.len() {
            return Err(rusqlite::Error::InvalidParameterName(
                "Bin order contains duplicate clips".to_string(),
            ));
        }
        let current_ids = self
            .get_clips(None, Some(bin_id), false)?
            .into_iter()
            .map(|clip| clip.id)
            .collect::<std::collections::HashSet<_>>();
        if current_ids != unique {
            return Err(rusqlite::Error::InvalidParameterName(
                "Bin order must contain every current clip exactly once".to_string(),
            ));
        }

        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM bins WHERE id = ?1)",
            params![bin_id],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        tx.execute(
            "DELETE FROM bin_clip_order WHERE bin_id = ?1",
            params![bin_id],
        )?;
        for (position, clip_id) in ids.iter().enumerate() {
            tx.execute(
                "INSERT INTO bin_clip_order (bin_id, clip_id, position) VALUES (?1, ?2, ?3)",
                params![bin_id, clip_id, position as i64],
            )?;
        }
        self.log_activity_internal(
            &tx,
            "bin_clips_reordered",
            &format!("Reordered {} clips in Bin #{bin_id}", ids.len()),
        )?;
        tx.commit()
    }

    #[cfg(test)]
    pub fn get_clip_versions(&self, clip_id: i64) -> Result<Vec<ClipVersion>> {
        self.get_clip_versions_page(clip_id, -1, 0)
    }

    pub fn get_clip_versions_page(
        &self,
        clip_id: i64,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ClipVersion>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, clip_id, text_content, context_json, created_at
             FROM clip_versions WHERE clip_id = ?1
             ORDER BY created_at DESC, id DESC LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(params![clip_id, limit, offset.max(0)], |row| {
            let context_json: Option<String> = row.get(3)?;
            let context = context_json
                .as_deref()
                .and_then(|value| serde_json::from_str::<ClipRevisionContext>(value).ok());
            Ok(ClipVersion {
                id: row.get(0)?,
                clip_id: row.get(1)?,
                text_content: row.get(2)?,
                action_kind: context.as_ref().map(|value| value.action_kind.clone()),
                action_label: context.as_ref().map(|value| value.action_label.clone()),
                restores_organization: context
                    .as_ref()
                    .and_then(|value| value.organization.as_ref())
                    .is_some(),
                created_at: row.get(4)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_clip_version_count(&self, clip_id: i64) -> Result<i64> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT COUNT(*) FROM clip_versions WHERE clip_id = ?1",
            params![clip_id],
            |row| row.get(0),
        )
    }

    pub fn restore_clip_version(&self, clip_id: i64, version_id: i64) -> Result<ClipItem> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let (target_text, context_json): (String, Option<String>) = tx.query_row(
            "SELECT text_content, context_json FROM clip_versions WHERE id = ?1 AND clip_id = ?2",
            params![version_id, clip_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let target_context = context_json
            .as_deref()
            .and_then(|value| serde_json::from_str::<ClipRevisionContext>(value).ok());
        let (current_text, current_bin_id, is_trashed, current_transformation_id): (
            Option<String>,
            Option<i64>,
            i32,
            Option<String>,
        ) = tx
            .query_row(
                "SELECT text_content, bin_id, COALESCE(is_trashed, 0), current_transformation_id FROM clips WHERE id = ?1",
                params![clip_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;
        if is_trashed != 0 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Restore this clip from Trash before restoring a revision".to_string(),
            ));
        }

        let target_bin_id = target_context
            .as_ref()
            .and_then(|context| context.organization.as_ref())
            .map(|organization| organization.category_bin_id);
        let organization_changes = target_bin_id
            .map(|target| target != current_bin_id)
            .unwrap_or(false);
        let target_transformation_id = target_context
            .as_ref()
            .and_then(|context| context.current_transformation_id.clone());
        if current_text.as_deref() == Some(target_text.as_str())
            && !organization_changes
            && current_transformation_id == target_transformation_id
        {
            tx.commit()?;
            return self.get_clip_by_id_internal(&conn, clip_id);
        }

        if let Some(current_text) = current_text {
            let inverse_context = target_bin_id.map(|_| ClipRevisionContext {
                schema_version: 1,
                action_kind: "restore".to_string(),
                action_label: "Before restoring an earlier revision".to_string(),
                organization: Some(ClipRevisionOrganization {
                    category_bin_id: current_bin_id,
                }),
                current_transformation_id: current_transformation_id.clone(),
            });
            let inverse_context = inverse_context.or_else(|| {
                Some(ClipRevisionContext {
                    schema_version: 1,
                    action_kind: "restore".to_string(),
                    action_label: "Before restoring an earlier revision".to_string(),
                    organization: None,
                    current_transformation_id: current_transformation_id.clone(),
                })
            });
            let inverse_json = inverse_context
                .map(|context| serde_json::to_string(&context))
                .transpose()
                .map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))?;
            tx.execute(
                "INSERT INTO clip_versions (clip_id, text_content, context_json) VALUES (?1, ?2, ?3)",
                params![clip_id, current_text, inverse_json],
            )?;
        }

        tx.execute(
            "UPDATE clips SET text_content = ?1, current_transformation_id = ?2 WHERE id = ?3",
            params![target_text, target_transformation_id, clip_id],
        )?;
        if let Some(target_bin_id) = target_bin_id {
            tx.execute(
                "DELETE FROM clip_bins
                 WHERE clip_id = ?1 AND bin_id IN (
                    SELECT id FROM bins WHERE COALESCE(bin_type, 'category') != 'tag'
                 )",
                params![clip_id],
            )?;
            let restored_bin_id = if let Some(bin_id) = target_bin_id {
                let changed = tx.execute(
                    "INSERT OR REPLACE INTO clip_bins (clip_id, bin_id)
                     SELECT ?1, id FROM bins
                     WHERE id = ?2 AND COALESCE(bin_type, 'category') != 'tag'",
                    params![clip_id, bin_id],
                )?;
                (changed > 0).then_some(bin_id)
            } else {
                None
            };
            tx.execute(
                "UPDATE clips SET bin_id = ?1 WHERE id = ?2",
                params![restored_bin_id, clip_id],
            )?;
        }
        Self::prune_clip_versions_internal(&tx, clip_id)?;
        tx.commit()?;
        let _ = self.log_activity_internal(
            &conn,
            "clip_revision_restored",
            &format!("Restored revision #{version_id} for clip #{clip_id}"),
        );
        self.get_clip_by_id_internal(&conn, clip_id)
    }

    pub fn update_bin(
        &self,
        id: i64,
        name: &str,
        icon: &str,
        color: &str,
        smart_rule: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE bins SET name = ?1, icon = ?2, color = ?3, smart_rule = ?4 WHERE id = ?5",
            params![name, icon, color, smart_rule, id],
        )?;
        Ok(())
    }

    pub fn delete_bin(
        &self,
        id: i64,
        disposition: &str,
        destination_bin_id: Option<i64>,
    ) -> Result<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;

        let bin_name: String =
            tx.query_row("SELECT name FROM bins WHERE id = ?1", params![id], |row| {
                row.get(0)
            })?;
        let clip_ids = {
            let mut stmt = tx.prepare(
                "SELECT id FROM clips
                 WHERE (is_trashed IS NULL OR is_trashed = 0)
                   AND (bin_id = ?1 OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?1))",
            )?;
            let ids = stmt
                .query_map(params![id], |row| row.get::<_, i64>(0))?
                .collect::<Result<Vec<_>>>()?;
            ids
        };

        match disposition {
            "keep" => {
                for clip_id in &clip_ids {
                    tx.execute(
                        "UPDATE clips SET bin_id = NULL WHERE id = ?1 AND bin_id = ?2",
                        params![clip_id, id],
                    )?;
                }
            }
            "trash" => {
                for clip_id in &clip_ids {
                    let changed = tx.execute(
                        "UPDATE clips
                         SET is_trashed = 1,
                             trashed_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                         WHERE id = ?1 AND (is_protected IS NULL OR is_protected = 0)",
                        params![clip_id],
                    )?;
                    if changed > 0 {
                        self.clear_category_bin_assignments_internal(&tx, *clip_id)?;
                    } else {
                        tx.execute(
                            "UPDATE clips SET bin_id = NULL WHERE id = ?1 AND bin_id = ?2",
                            params![clip_id, id],
                        )?;
                    }
                }
                self.enforce_trash_limit_internal(&tx)?;
            }
            "move" => {
                let destination_id = destination_bin_id.ok_or_else(|| {
                    rusqlite::Error::InvalidParameterName(
                        "A destination Bin is required when moving clips".to_string(),
                    )
                })?;
                if destination_id == id {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "The destination Bin must be different from the deleted Bin".to_string(),
                    ));
                }
                let destination_exists = tx.query_row(
                    "SELECT EXISTS(
                         SELECT 1 FROM bins
                         WHERE id = ?1
                           AND (smart_rule IS NULL OR TRIM(smart_rule) = '')
                           AND COALESCE(bin_type, 'category') != 'tag'
                     )",
                    params![destination_id],
                    |row| row.get::<_, bool>(0),
                )?;
                if !destination_exists {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "The destination must be another manual Bin".to_string(),
                    ));
                }
                for clip_id in &clip_ids {
                    self.clear_category_bin_assignments_internal(&tx, *clip_id)?;
                    tx.execute(
                        "INSERT OR REPLACE INTO clip_bins (clip_id, bin_id) VALUES (?1, ?2)",
                        params![clip_id, destination_id],
                    )?;
                    tx.execute(
                        "UPDATE clips SET bin_id = ?1 WHERE id = ?2",
                        params![destination_id, clip_id],
                    )?;
                }
            }
            _ => {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Unknown Bin deletion outcome".to_string(),
                ));
            }
        }

        tx.execute("DELETE FROM clip_bins WHERE bin_id = ?1", params![id])?;
        tx.execute("DELETE FROM bins WHERE id = ?1", params![id])?;
        let outcome = match disposition {
            "trash" => "moved its clips to Trash",
            "move" => "moved its clips to another Bin",
            _ => "kept its clips in No Bin",
        };
        self.log_activity_internal(
            &tx,
            "bin_deleted",
            &format!(
                "Deleted Bin \"{}\" and {} ({} clips)",
                bin_name,
                outcome,
                clip_ids.len()
            ),
        )?;
        tx.commit()
    }

    pub fn clear_history(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM clips WHERE is_pinned = 0 AND (is_protected IS NULL OR is_protected = 0)",
            [],
        )?;
        Ok(())
    }

    pub fn export_backup_json(&self) -> Result<String> {
        let clips = self.get_all_clips_for_backup()?;
        let bins = self.get_bins()?;
        let pipelines = Vec::new();
        let operations = self.get_operations()?;
        let saved_transforms = self.get_saved_transforms()?;
        let bin_transforms = bins
            .iter()
            .filter_map(|bin| {
                self.get_bin_transform_ref(bin.id)
                    .ok()
                    .flatten()
                    .map(|transform_ref| BinTransformBinding {
                        bin_id: bin.id,
                        transform_ref,
                    })
            })
            .collect();
        let ocr_metadata = self.get_ocr_backup_metadata()?;
        let content_detectors = self.get_all_content_detectors_for_backup()?;
        let content_types = self.get_content_types(true)?;
        let content_type_groups = self.get_content_type_groups(true)?;

        let payload = BackupPayload {
            version: BACKUP_SCHEMA_VERSION,
            timestamp: chrono::Utc::now().to_rfc3339(),
            clips,
            bins,
            pipelines,
            operations,
            saved_transforms,
            bin_transforms,
            ocr_metadata,
            content_detectors,
            content_types,
            content_type_groups,
        };

        serde_json::to_string_pretty(&payload)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
    }

    pub fn export_clips_json(&self) -> Result<String> {
        let clips = self
            .get_all_clips_for_backup()?
            .into_iter()
            .filter(|clip| !clip.is_trashed)
            .collect::<Vec<_>>();
        serde_json::to_string_pretty(&clips)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
    }

    pub fn export_clips_csv(&self) -> Result<String> {
        fn cell(value: &str) -> String {
            let escaped = value.replace('"', "\"\"");
            let neutralized = if matches!(
                value.chars().next(),
                Some('=' | '+' | '-' | '@' | '\t' | '\r')
            ) {
                format!("'{escaped}")
            } else {
                escaped
            };
            format!("\"{neutralized}\"")
        }

        let clips = self
            .get_clips(None, None, false)?
            .into_iter()
            .filter(|clip| clip.text_content.is_some() && clip.content_type != "image")
            .collect::<Vec<_>>();
        let mut csv = String::from("id,content_type,source,is_pinned,created_at,text_content\n");
        for clip in clips {
            csv.push_str(&format!(
                "{},{},{},{},{},{}\n",
                clip.id,
                cell(&clip.content_type),
                cell(&clip.source),
                clip.is_pinned,
                cell(&clip.created_at),
                cell(clip.text_content.as_deref().unwrap_or_default()),
            ));
        }
        Ok(csv)
    }

    pub fn import_clips_json(&self, json: &str) -> Result<ClipImportReport> {
        let clips = Self::parse_clips_json_import(json)?;
        self.apply_imported_clips(clips, true)
    }

    pub fn inspect_clips_json(&self, json: &str) -> Result<ClipImportReport> {
        let clips = Self::parse_clips_json_import(json)?;
        self.apply_imported_clips(clips, false)
    }

    fn parse_clips_json_import(json: &str) -> Result<Vec<ClipItem>> {
        use crate::resource_limits::{MAX_BACKUP_IMPORT_BYTES, MAX_LIBRARY_ARCHIVE_ROWS};
        ensure_resource_size(json, MAX_BACKUP_IMPORT_BYTES, "Clip JSON import")?;
        let clips: Vec<ClipItem> = serde_json::from_str(json).map_err(|error| {
            rusqlite::Error::InvalidParameterName(format!("invalid clip JSON: {error}"))
        })?;
        if clips.len() > MAX_LIBRARY_ARCHIVE_ROWS {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "Clip import contains more than {MAX_LIBRARY_ARCHIVE_ROWS} records"
            )));
        }
        Ok(clips)
    }

    pub fn import_clips_csv(&self, csv: &str) -> Result<ClipImportReport> {
        let clips = Self::parse_clips_csv_import(csv)?;
        self.apply_imported_clips(clips, true)
    }

    pub fn inspect_clips_csv(&self, csv: &str) -> Result<ClipImportReport> {
        let clips = Self::parse_clips_csv_import(csv)?;
        self.apply_imported_clips(clips, false)
    }

    fn parse_clips_csv_import(csv: &str) -> Result<Vec<ClipItem>> {
        use crate::resource_limits::{MAX_BACKUP_IMPORT_BYTES, MAX_LIBRARY_ARCHIVE_ROWS};
        ensure_resource_size(csv, MAX_BACKUP_IMPORT_BYTES, "Clip CSV import")?;
        let records = Self::parse_csv(csv)?;
        if records.len().saturating_sub(1) > MAX_LIBRARY_ARCHIVE_ROWS {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "Clip import contains more than {MAX_LIBRARY_ARCHIVE_ROWS} records"
            )));
        }
        let expected = [
            "id",
            "content_type",
            "source",
            "is_pinned",
            "created_at",
            "text_content",
        ];
        if records.first().map(|header| {
            header
                .iter()
                .map(String::as_str)
                .eq(expected.iter().copied())
        }) != Some(true)
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "Clip CSV header does not match the supported export format".to_string(),
            ));
        }
        let mut clips = Vec::with_capacity(records.len().saturating_sub(1));
        for (index, row) in records.into_iter().skip(1).enumerate() {
            if row.len() != expected.len() {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "Clip CSV row {} has {} columns; expected {}",
                    index + 2,
                    row.len(),
                    expected.len()
                )));
            }
            let text = row[5].clone();
            if text.is_empty() || row[1] == "image" {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "Clip CSV row {} does not contain an importable text clip",
                    index + 2
                )));
            }
            let mut hasher = Sha256::new();
            hasher.update(text.as_bytes());
            clips.push(ClipItem {
                id: 0,
                content_type: row[1].clone(),
                text_content: Some(text),
                html_content: None,
                image_base64: None,
                image_path: None,
                content_hash: format!("{:x}", hasher.finalize()),
                source: row[2].clone(),
                is_pinned: row[3].parse::<bool>().map_err(|_| {
                    rusqlite::Error::InvalidParameterName(format!(
                        "Clip CSV row {} has an invalid is_pinned value",
                        index + 2
                    ))
                })?,
                is_protected: false,
                is_transformed: false,
                pin_order: 0,
                bin_id: None,
                bin_ids: None,
                note: None,
                is_trashed: false,
                trashed_at: None,
                created_at: row[4].clone(),
                ocr_extractor_ref: None,
                ocr_extractor_name: None,
                ocr_engine_version: None,
            });
        }
        Ok(clips)
    }

    fn parse_csv(csv: &str) -> Result<Vec<Vec<String>>> {
        let mut records = Vec::new();
        let mut record = Vec::new();
        let mut field = String::new();
        let mut quoted = false;
        let mut chars = csv.chars().peekable();
        while let Some(character) = chars.next() {
            match character {
                '"' if quoted && chars.peek() == Some(&'"') => {
                    chars.next();
                    field.push('"');
                }
                '"' => quoted = !quoted,
                ',' if !quoted => {
                    record.push(Self::deneutralize_csv_cell(std::mem::take(&mut field)));
                }
                '\n' if !quoted => {
                    if field.ends_with('\r') {
                        field.pop();
                    }
                    record.push(Self::deneutralize_csv_cell(std::mem::take(&mut field)));
                    records.push(std::mem::take(&mut record));
                }
                other => field.push(other),
            }
        }
        if quoted {
            return Err(rusqlite::Error::InvalidParameterName(
                "CSV contains an unterminated quoted field".to_string(),
            ));
        }
        if !field.is_empty() || !record.is_empty() {
            record.push(Self::deneutralize_csv_cell(field));
            records.push(record);
        }
        Ok(records)
    }

    fn deneutralize_csv_cell(value: String) -> String {
        if value.starts_with("'=")
            || value.starts_with("'+")
            || value.starts_with("'-")
            || value.starts_with("'@")
            || value.starts_with("'\t")
            || value.starts_with("'\r")
        {
            value[1..].to_string()
        } else {
            value
        }
    }

    fn apply_imported_clips(&self, clips: Vec<ClipItem>, commit: bool) -> Result<ClipImportReport> {
        use crate::resource_limits::{MAX_CLIP_NOTE_BYTES, MAX_CLIP_TEXT_BYTES};
        let mut input_hashes = HashSet::new();
        for clip in &clips {
            if clip.content_hash.trim().is_empty()
                || !input_hashes.insert(clip.content_hash.clone())
            {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Clip import contains an empty or duplicate content hash".to_string(),
                ));
            }
            if clip.content_type.trim().is_empty() || clip.content_type.len() > 128 {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Clip import contains an invalid content type".to_string(),
                ));
            }
            if let Some(value) = clip.text_content.as_deref() {
                ensure_resource_size(value, MAX_CLIP_TEXT_BYTES, "Imported clip text")?;
            }
            if let Some(value) = clip.html_content.as_deref() {
                ensure_resource_size(value, MAX_CLIP_TEXT_BYTES, "Imported clip HTML")?;
            }
            if let Some(value) = clip.image_base64.as_deref() {
                ensure_safe_raster_data_url(value, "Imported clip image")?;
            }
            if let Some(value) = clip.note.as_deref() {
                ensure_resource_size(value, MAX_CLIP_NOTE_BYTES, "Imported clip note")?;
            }
        }

        let scanned_count = clips.len();
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let active_count_before: usize = tx.query_row(
            "SELECT COUNT(*) FROM clips WHERE COALESCE(is_trashed, 0) = 0",
            [],
            |row| row.get(0),
        )?;
        let mut imported_count = 0usize;
        for clip in clips {
            imported_count += tx.execute(
                "INSERT OR IGNORE INTO clips (
                    content_type, text_content, html_content, image_base64, image_path,
                    content_hash, source, is_pinned, is_protected, pin_order, note,
                    is_trashed, trashed_at, created_at, ocr_status, ocr_input_hash
                 ) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?8, ?9, ?10, 0, NULL, ?11,
                    CASE WHEN ?1 = 'image' THEN 'never' ELSE 'not_applicable' END,
                    CASE WHEN ?1 = 'image' THEN ?5 ELSE NULL END)",
                params![
                    clip.content_type,
                    clip.text_content,
                    clip.html_content,
                    clip.image_base64,
                    clip.content_hash,
                    clip.source,
                    clip.is_pinned,
                    clip.is_protected,
                    clip.pin_order,
                    clip.note,
                    clip.created_at,
                ],
            )?;
        }
        let current_capacity = tx
            .query_row(
                "SELECT value FROM settings WHERE key = 'keepClipCount'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(1000);
        let required_capacity = active_count_before.saturating_add(imported_count);
        if current_capacity > 0 && required_capacity > current_capacity {
            tx.execute(
                "INSERT INTO settings (key, value) VALUES ('keepClipCount', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                [required_capacity.to_string()],
            )?;
        }
        let duplicate_count = scanned_count.saturating_sub(imported_count);
        tx.execute(
            "INSERT INTO activity_logs (event_type, description) VALUES ('clips_imported', ?1)",
            [format!(
                "Imported {imported_count} clips; skipped {duplicate_count} duplicates"
            )],
        )?;
        if commit {
            tx.commit()?;
        } else {
            tx.rollback()?;
        }
        Ok(ClipImportReport {
            scanned_count,
            imported_count,
            duplicate_count,
        })
    }

    fn parse_library_archive(json_str: &str) -> Result<(BackupPayload, LibraryArchiveInspection)> {
        ensure_resource_size(
            json_str,
            crate::resource_limits::MAX_BACKUP_IMPORT_BYTES,
            "Transfer file",
        )?;
        let payload: BackupPayload = serde_json::from_str(json_str).map_err(|error| {
            rusqlite::Error::InvalidParameterName(format!("invalid transfer JSON: {error}"))
        })?;
        let inspection = Self::preflight_library_archive(&payload)?;
        Ok((payload, inspection))
    }

    fn preflight_library_archive(payload: &BackupPayload) -> Result<LibraryArchiveInspection> {
        use crate::resource_limits::{
            MAX_CLIP_NOTE_BYTES, MAX_CLIP_TEXT_BYTES, MAX_LIBRARY_ARCHIVE_ROWS,
        };

        if !(1..=BACKUP_SCHEMA_VERSION).contains(&payload.version) {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "unsupported transfer schema version {} (supported: 1-{BACKUP_SCHEMA_VERSION})",
                payload.version
            )));
        }
        let total_rows = [
            payload.clips.len(),
            payload.bins.len(),
            payload.pipelines.len(),
            payload.operations.len(),
            payload.saved_transforms.len(),
            payload.bin_transforms.len(),
            payload.ocr_metadata.len(),
            payload.content_detectors.len(),
            payload.content_types.len(),
            payload.content_type_groups.len(),
        ]
        .into_iter()
        .try_fold(0usize, usize::checked_add)
        .ok_or_else(|| {
            rusqlite::Error::InvalidParameterName(
                "Transfer row count exceeds supported limits".to_string(),
            )
        })?;
        if total_rows > MAX_LIBRARY_ARCHIVE_ROWS {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "Transfer file contains more than {MAX_LIBRARY_ARCHIVE_ROWS} records"
            )));
        }
        if payload.content_type_groups.len() > 64 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Transfer file contains more than 64 content type groups".to_string(),
            ));
        }
        if payload.content_types.len() > 256 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Transfer file contains more than 256 content types".to_string(),
            ));
        }
        if payload.content_detectors.len() > 128 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Transfer file contains more than 128 content detectors".to_string(),
            ));
        }

        let unique = |values: Vec<String>, label: &str| -> Result<HashSet<String>> {
            let mut seen = HashSet::with_capacity(values.len());
            for value in values {
                if value.trim().is_empty() || !seen.insert(value.clone()) {
                    return Err(rusqlite::Error::InvalidParameterName(format!(
                        "Transfer file contains an empty or duplicate {label}: {value}"
                    )));
                }
            }
            Ok(seen)
        };
        let unique_ids = |values: Vec<i64>, label: &str| -> Result<HashSet<i64>> {
            let mut seen = HashSet::with_capacity(values.len());
            for value in values {
                if value <= 0 || !seen.insert(value) {
                    return Err(rusqlite::Error::InvalidParameterName(format!(
                        "Transfer file contains an invalid or duplicate {label}: {value}"
                    )));
                }
            }
            Ok(seen)
        };

        let _group_ids = unique(
            payload
                .content_type_groups
                .iter()
                .map(|group| group.id.clone())
                .collect(),
            "content type group ID",
        )?;
        for group in &payload.content_type_groups {
            crate::content_types::validate_content_type_group_input(
                &crate::content_types::ContentTypeGroupInput {
                    id: group.id.clone(),
                    label: group.label.clone(),
                    sort_order: group.sort_order,
                },
            )
            .map_err(rusqlite::Error::InvalidParameterName)?;
        }
        let available_group_ids = payload
            .content_type_groups
            .iter()
            .filter(|group| !group.is_archived)
            .map(|group| group.id.clone())
            .chain(
                crate::content_types::CONTENT_TYPE_GROUP_PRESETS
                    .iter()
                    .map(|preset| preset.id.to_string()),
            )
            .collect::<HashSet<_>>();
        unique(
            payload
                .content_types
                .iter()
                .map(|content_type| content_type.id.clone())
                .collect(),
            "content type ID",
        )?;
        for content_type in &payload.content_types {
            crate::content_types::validate_content_type_input(
                &crate::content_types::ContentTypeInput {
                    id: content_type.id.clone(),
                    label: content_type.label.clone(),
                    icon: content_type.icon.clone(),
                    group: content_type.group.clone(),
                },
            )
            .map_err(rusqlite::Error::InvalidParameterName)?;
            if !available_group_ids.contains(&content_type.group) {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "Transfer content type {} references a missing Group",
                    content_type.id
                )));
            }
        }

        unique(
            payload
                .content_detectors
                .iter()
                .map(|detector| detector.stable_ref.clone())
                .collect(),
            "detector reference",
        )?;
        for detector in &payload.content_detectors {
            crate::content_detection::validate_detector_input(
                &crate::content_detection::DetectorInput {
                    name: detector.name.clone(),
                    content_type: detector.content_type.clone(),
                    description: detector.description.clone(),
                    patterns: detector.patterns.clone(),
                    validator: detector.validator.clone(),
                    enabled: detector.enabled,
                    priority: detector.priority,
                },
            )
            .map_err(rusqlite::Error::InvalidParameterName)?;
        }

        let bin_ids = unique_ids(payload.bins.iter().map(|bin| bin.id).collect(), "Bin ID")?;
        for bin in &payload.bins {
            if !matches!(bin.bin_type.as_str(), "category" | "tag") {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "Transfer Bin {} has an invalid type",
                    bin.id
                )));
            }
            if let Some(rule) = bin.smart_rule.as_deref() {
                serde_json::from_str::<serde_json::Value>(rule).map_err(|error| {
                    rusqlite::Error::InvalidParameterName(format!(
                        "Transfer Bin {} has an invalid smart rule: {error}",
                        bin.id
                    ))
                })?;
            }
        }

        let clip_ids = unique_ids(
            payload.clips.iter().map(|clip| clip.id).collect(),
            "clip ID",
        )?;
        unique(
            payload
                .clips
                .iter()
                .map(|clip| clip.content_hash.clone())
                .collect(),
            "clip content hash",
        )?;
        let image_hashes = payload
            .clips
            .iter()
            .filter(|clip| clip.content_type == "image")
            .map(|clip| clip.content_hash.as_str())
            .collect::<HashSet<_>>();
        for clip in &payload.clips {
            if let Some(text) = clip.text_content.as_deref() {
                ensure_resource_size(text, MAX_CLIP_TEXT_BYTES, "Imported clip text")?;
            }
            if let Some(html) = clip.html_content.as_deref() {
                ensure_resource_size(html, MAX_CLIP_TEXT_BYTES, "Imported clip HTML")?;
            }
            if let Some(image) = clip.image_base64.as_deref() {
                ensure_safe_raster_data_url(image, "Imported clip image")?;
            }
            if let Some(note) = clip.note.as_deref() {
                ensure_resource_size(note, MAX_CLIP_NOTE_BYTES, "Imported clip note")?;
            }
            if clip.bin_id.is_some_and(|id| !bin_ids.contains(&id))
                || clip
                    .bin_ids
                    .as_ref()
                    .is_some_and(|ids| ids.iter().any(|id| !bin_ids.contains(id)))
            {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "Transfer clip {} references a missing Bin",
                    clip.id
                )));
            }
        }
        for bin in &payload.bins {
            let mut ordered = HashSet::new();
            if bin
                .clip_order
                .iter()
                .any(|id| !clip_ids.contains(id) || !ordered.insert(*id))
            {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "Transfer Bin {} contains an invalid clip order",
                    bin.id
                )));
            }
        }

        unique(
            payload
                .ocr_metadata
                .iter()
                .map(|entry| entry.content_hash.clone())
                .collect(),
            "OCR content hash",
        )?;
        for metadata in &payload.ocr_metadata {
            if !image_hashes.contains(metadata.content_hash.as_str())
                || !matches!(
                    metadata.status.as_str(),
                    "complete" | "no_text" | "failed" | "never" | "queued" | "running"
                )
                || metadata
                    .engine_version
                    .as_ref()
                    .is_some_and(|value| value.is_empty() || value.len() > 80)
                || metadata
                    .extractor_ref
                    .as_ref()
                    .is_some_and(|value| value.is_empty() || value.len() > 160)
                || metadata
                    .extractor_name
                    .as_ref()
                    .is_some_and(|value| value.is_empty() || value.len() > 80)
            {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "Transfer file has invalid OCR metadata for {}",
                    metadata.content_hash
                )));
            }
        }

        let custom_operation_refs = payload
            .operations
            .iter()
            .filter(|operation| operation.id >= 0)
            .map(|operation| {
                operation
                    .stable_id
                    .strip_prefix("custom:")
                    .filter(|id| !id.is_empty())
                    .map(|_| operation.stable_id.clone())
                    .ok_or_else(|| {
                        rusqlite::Error::InvalidParameterName(
                            "custom operation in transfer file is missing a stable reference"
                                .to_string(),
                        )
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        let custom_operation_refs = unique(custom_operation_refs, "custom operation reference")?;
        let validate_step = |step: &PipelineStep| -> Result<()> {
            if !matches!(step.failure_policy.as_str(), "stop" | "skip") {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "invalid failure policy: {}",
                    step.failure_policy
                )));
            }
            if let Some(config) = step.config_json.as_deref() {
                serde_json::from_str::<serde_json::Value>(config).map_err(|error| {
                    rusqlite::Error::InvalidParameterName(format!(
                        "invalid step config JSON: {error}"
                    ))
                })?;
            }
            let valid = step
                .operation_ref
                .strip_prefix("builtin:")
                .is_some_and(crate::operation_registry::is_builtin_operation)
                || custom_operation_refs.contains(&step.operation_ref);
            if !valid {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "unknown operation reference: {}",
                    step.operation_ref
                )));
            }
            Ok(())
        };
        unique(
            payload
                .pipelines
                .iter()
                .map(|pipeline| pipeline.stable_ref.clone())
                .collect(),
            "legacy pipeline reference",
        )?;
        for pipeline in &payload.pipelines {
            if pipeline
                .stable_ref
                .strip_prefix("pipeline:")
                .is_none_or(str::is_empty)
                || pipeline.steps.is_empty()
            {
                return Err(rusqlite::Error::InvalidParameterName(
                    "legacy pipeline in transfer file is missing a stable reference or steps"
                        .to_string(),
                ));
            }
            for step in &pipeline.steps {
                validate_step(step)?;
            }
        }
        let transform_refs = unique(
            payload
                .saved_transforms
                .iter()
                .map(|transform| transform.stable_ref.clone())
                .collect(),
            "Transform reference",
        )?;
        for transform in &payload.saved_transforms {
            if transform
                .stable_ref
                .strip_prefix("transform:")
                .is_none_or(str::is_empty)
                || !matches!(transform.authoring_kind.as_str(), "manual" | "intent")
            {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Transform in transfer file has invalid identity metadata".to_string(),
                ));
            }
            transform
                .plan
                .validate()
                .map_err(rusqlite::Error::InvalidParameterName)?;
        }
        for binding in &payload.bin_transforms {
            if !bin_ids.contains(&binding.bin_id)
                || (!transform_refs.contains(&binding.transform_ref)
                    && !payload.pipelines.iter().any(|pipeline| {
                        binding.transform_ref.strip_prefix("transform:")
                            == pipeline.stable_ref.strip_prefix("pipeline:")
                    }))
            {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Transfer file contains an invalid Bin Transform binding".to_string(),
                ));
            }
        }

        Ok(LibraryArchiveInspection {
            schema_version: payload.version,
            clip_count: payload.clips.len(),
            bin_count: payload.bins.len(),
            operation_count: payload
                .operations
                .iter()
                .filter(|item| item.id >= 0)
                .count(),
            transform_count: payload.saved_transforms.len() + payload.pipelines.len(),
            detector_count: payload.content_detectors.len(),
            content_type_count: payload.content_types.len(),
        })
    }

    pub fn inspect_library_archive_json(json_str: &str) -> Result<LibraryArchiveInspection> {
        Self::parse_library_archive(json_str).map(|(_, inspection)| inspection)
    }

    pub fn import_backup_json(&self, json_str: &str) -> Result<usize> {
        let (payload, _) = Self::parse_library_archive(json_str)?;
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let mut bin_id_map = std::collections::HashMap::new();
        if payload.content_type_groups.len() > 64 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Backup contains more than 64 content type groups".to_string(),
            ));
        }
        for group in &payload.content_type_groups {
            crate::content_types::validate_content_type_group_input(
                &crate::content_types::ContentTypeGroupInput {
                    id: group.id.clone(),
                    label: group.label.clone(),
                    sort_order: group.sort_order,
                },
            )
            .map_err(rusqlite::Error::InvalidParameterName)?;
            let is_builtin = crate::content_types::CONTENT_TYPE_GROUP_PRESETS
                .iter()
                .any(|preset| preset.id == group.id);
            tx.execute(
                "INSERT INTO content_type_groups
                    (id, label, sort_order, is_builtin, is_archived)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET
                    label = excluded.label, sort_order = excluded.sort_order,
                    is_archived = CASE WHEN content_type_groups.is_builtin = 1 THEN 0 ELSE excluded.is_archived END,
                    updated_at = CURRENT_TIMESTAMP",
                params![group.id, group.label, group.sort_order, is_builtin, group.is_archived],
            )?;
        }
        if payload.content_types.len() > 256 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Backup contains more than 256 content types".to_string(),
            ));
        }
        for content_type in &payload.content_types {
            crate::content_types::validate_content_type_input(
                &crate::content_types::ContentTypeInput {
                    id: content_type.id.clone(),
                    label: content_type.label.clone(),
                    icon: content_type.icon.clone(),
                    group: content_type.group.clone(),
                },
            )
            .map_err(rusqlite::Error::InvalidParameterName)?;
            let group_exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM content_type_groups WHERE id = ?1 AND is_archived = 0)",
                params![content_type.group],
                |row| row.get(0),
            )?;
            if !group_exists {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "Backup content type {} references a missing or archived Group",
                    content_type.id
                )));
            }
            let is_builtin = crate::content_types::CONTENT_TYPE_PRESETS
                .iter()
                .any(|preset| preset.id == content_type.id);
            tx.execute(
                "INSERT INTO content_types
                    (id, label, icon, group_name, is_builtin, is_archived)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                    label = excluded.label, icon = excluded.icon,
                    group_name = excluded.group_name,
                    is_archived = CASE WHEN content_types.is_builtin = 1 THEN 0 ELSE excluded.is_archived END,
                    updated_at = CURRENT_TIMESTAMP",
                params![
                    content_type.id,
                    content_type.label,
                    content_type.icon,
                    content_type.group,
                    is_builtin,
                    content_type.is_archived,
                ],
            )?;
        }
        if payload.content_detectors.len() > 128 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Backup contains more than 128 content detectors".to_string(),
            ));
        }
        for detector in &payload.content_detectors {
            crate::content_detection::validate_detector_input(
                &crate::content_detection::DetectorInput {
                    name: detector.name.clone(),
                    content_type: detector.content_type.clone(),
                    description: detector.description.clone(),
                    patterns: detector.patterns.clone(),
                    validator: detector.validator.clone(),
                    enabled: detector.enabled,
                    priority: detector.priority,
                },
            )
            .map_err(rusqlite::Error::InvalidParameterName)?;
            let patterns_json = serde_json::to_string(&detector.patterns)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            tx.execute(
                "INSERT INTO content_detectors
                    (stable_ref, name, content_type, description, patterns_json, validator,
                     enabled, priority, is_builtin, is_deleted)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(stable_ref) DO UPDATE SET
                    name = excluded.name, content_type = excluded.content_type,
                    description = excluded.description, patterns_json = excluded.patterns_json,
                    validator = excluded.validator, enabled = excluded.enabled,
                    priority = excluded.priority, is_builtin = excluded.is_builtin,
                    is_deleted = excluded.is_deleted, updated_at = CURRENT_TIMESTAMP",
                params![
                    detector.stable_ref,
                    detector.name,
                    detector.content_type,
                    detector.description,
                    patterns_json,
                    detector.validator,
                    detector.enabled,
                    detector.priority,
                    detector.is_builtin,
                    detector.is_deleted
                ],
            )?;
        }
        let bin_clip_orders = payload
            .bins
            .iter()
            .map(|bin| (bin.id, bin.clip_order.clone()))
            .collect::<std::collections::HashMap<_, _>>();
        let ocr_metadata = payload
            .ocr_metadata
            .iter()
            .map(|entry| (entry.content_hash.clone(), entry.clone()))
            .collect::<std::collections::HashMap<_, _>>();

        for mut bin in payload.bins {
            if let Some(rule) = bin.smart_rule.as_mut() {
                *rule = rule.replace("\"source_app\"", "\"source\"");
            }
            let existing_id = tx.query_row(
                "SELECT id FROM bins WHERE name = ?1 AND COALESCE(bin_type, 'category') = ?2 LIMIT 1",
                params![bin.name, bin.bin_type],
                |row| row.get::<_, i64>(0),
            ).ok();
            let new_id = if let Some(id) = existing_id {
                tx.execute(
                    "UPDATE bins SET icon = ?1, color = ?2, smart_rule = ?3, shortcut = ?4 WHERE id = ?5",
                    params![bin.icon, bin.color, bin.smart_rule, bin.shortcut, id],
                )?;
                id
            } else {
                tx.execute(
                    "INSERT INTO bins (name, icon, color, smart_rule, bin_type, shortcut, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![bin.name, bin.icon, bin.color, bin.smart_rule, bin.bin_type, bin.shortcut, bin.created_at],
                )?;
                tx.last_insert_rowid()
            };
            bin_id_map.insert(bin.id, new_id);
        }

        for operation in payload.operations {
            // Registry built-ins are definitions, not persisted records.
            if operation.id < 0 {
                continue;
            }
            let operation_id = operation.stable_id.strip_prefix("custom:").ok_or_else(|| {
                rusqlite::Error::InvalidParameterName(
                    "custom operation backup is missing a stable reference".to_string(),
                )
            })?;
            let (executor_kind, config_json) =
                Self::operation_storage_fields(&operation.op_type, operation.config.as_deref());
            tx.execute(
                "INSERT INTO custom_operations
                    (id, name, executor_kind, config_json, category, trusted, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    executor_kind = excluded.executor_kind,
                    config_json = excluded.config_json,
                    category = excluded.category,
                    trusted = 0,
                    updated_at = CURRENT_TIMESTAMP",
                params![
                    operation_id,
                    operation.name,
                    executor_kind,
                    config_json,
                    operation.category,
                    operation.created_at
                ],
            )?;
        }

        for pipeline in payload.pipelines {
            let pipeline_id = pipeline
                .stable_ref
                .strip_prefix("pipeline:")
                .ok_or_else(|| {
                    rusqlite::Error::InvalidParameterName(
                        "pipeline backup is missing a stable reference".to_string(),
                    )
                })?;
            let steps = pipeline
                .steps
                .iter()
                .map(|step| PipelineStepInput {
                    operation_ref: step.operation_ref.clone(),
                    config_json: step.config_json.clone(),
                    failure_policy: step.failure_policy.clone(),
                })
                .collect::<Vec<_>>();
            Self::validate_pipeline_steps(&tx, &steps)?;
            let plan_json =
                serde_json::to_string(&Self::manual_transform_plan(&pipeline.name, &steps)?)
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            let collision: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM saved_transforms WHERE id = ?1)",
                params![pipeline_id],
                |row| row.get(0),
            )?;
            let transform_id = if collision {
                tx.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))?
            } else {
                pipeline_id.to_string()
            };
            tx.execute(
                "INSERT INTO saved_transforms
                    (id, name, plan_json, connection_id, shortcut, authoring_kind,
                     revision, created_at, updated_at)
                 VALUES (?1, ?2, ?3, NULL, ?4, 'manual', ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    plan_json = excluded.plan_json,
                    shortcut = excluded.shortcut,
                    authoring_kind = 'manual',
                    revision = excluded.revision,
                    updated_at = excluded.updated_at",
                params![
                    transform_id,
                    pipeline.name,
                    plan_json,
                    pipeline.shortcut,
                    pipeline.revision,
                    pipeline.created_at,
                    pipeline.updated_at
                ],
            )?;
        }

        for transform in payload.saved_transforms {
            let transform_id =
                transform
                    .stable_ref
                    .strip_prefix("transform:")
                    .ok_or_else(|| {
                        rusqlite::Error::InvalidParameterName(
                            "saved Transform backup is missing a stable reference".to_string(),
                        )
                    })?;
            transform
                .plan
                .validate()
                .map_err(rusqlite::Error::InvalidParameterName)?;
            let plan_json = serde_json::to_string(&transform.plan)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            tx.execute(
                "INSERT INTO saved_transforms
                    (id, name, plan_json, connection_id, shortcut, authoring_kind,
                     revision, created_at, updated_at)
                 VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    plan_json = excluded.plan_json,
                    connection_id = NULL,
                    shortcut = excluded.shortcut,
                    authoring_kind = excluded.authoring_kind,
                    revision = excluded.revision,
                    updated_at = excluded.updated_at",
                params![
                    transform_id,
                    transform.name,
                    plan_json,
                    transform.shortcut,
                    transform.authoring_kind,
                    transform.revision,
                    transform.created_at,
                    transform.updated_at
                ],
            )?;
        }

        for binding in payload.bin_transforms {
            let Some(mapped_bin_id) = bin_id_map.get(&binding.bin_id) else {
                continue;
            };
            let transform_id = binding
                .transform_ref
                .strip_prefix("transform:")
                .unwrap_or(&binding.transform_ref);
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM saved_transforms WHERE id = ?1)",
                params![transform_id],
                |row| row.get(0),
            )?;
            if exists {
                tx.execute(
                    "UPDATE bins SET default_transform_id = ?1 WHERE id = ?2",
                    params![transform_id, mapped_bin_id],
                )?;
            }
        }

        let mut imported = 0;
        let mut clip_id_map = std::collections::HashMap::new();
        for clip in payload.clips {
            let old_clip_id = clip.id;
            if let Some(text) = clip.text_content.as_deref() {
                ensure_resource_size(
                    text,
                    crate::resource_limits::MAX_CLIP_TEXT_BYTES,
                    "Imported clip text",
                )?;
            }
            if let Some(html) = clip.html_content.as_deref() {
                ensure_resource_size(
                    html,
                    crate::resource_limits::MAX_CLIP_TEXT_BYTES,
                    "Imported clip HTML",
                )?;
            }
            if let Some(image) = clip.image_base64.as_deref() {
                ensure_safe_raster_data_url(image, "Imported clip image")?;
            }
            if let Some(note) = clip.note.as_deref() {
                ensure_resource_size(
                    note,
                    crate::resource_limits::MAX_CLIP_NOTE_BYTES,
                    "Imported clip note",
                )?;
            }
            let mapped_primary_bin = clip.bin_id.and_then(|id| bin_id_map.get(&id).copied());
            tx.execute(
                "INSERT INTO clips (
                    content_type, text_content, html_content, image_base64, image_path, content_hash,
                    source, is_pinned, is_protected, pin_order, bin_id, note,
                    is_trashed, trashed_at, created_at
                 ) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                 ON CONFLICT(content_hash) DO UPDATE SET
                    content_type = excluded.content_type,
                    text_content = excluded.text_content,
                    html_content = excluded.html_content,
                    image_base64 = excluded.image_base64,
                    source = excluded.source,
                    is_pinned = excluded.is_pinned,
                    is_protected = excluded.is_protected,
                    pin_order = excluded.pin_order,
                    bin_id = excluded.bin_id,
                    note = excluded.note,
                    is_trashed = excluded.is_trashed,
                    trashed_at = excluded.trashed_at,
                    created_at = excluded.created_at",
                params![
                    clip.content_type, clip.text_content, clip.html_content, clip.image_base64,
                    clip.content_hash, clip.source, clip.is_pinned, clip.is_protected,
                    clip.pin_order, mapped_primary_bin, clip.note, clip.is_trashed,
                    clip.trashed_at, clip.created_at,
                ],
            )?;
            let new_clip_id = tx.query_row(
                "SELECT id FROM clips WHERE content_hash = ?1",
                params![clip.content_hash],
                |row| row.get::<_, i64>(0),
            )?;
            clip_id_map.insert(old_clip_id, new_clip_id);
            if clip.content_type == "image" {
                if let Some(metadata) = ocr_metadata.get(&clip.content_hash) {
                    let status = match metadata.status.as_str() {
                        "complete" | "no_text" | "failed" | "never" => metadata.status.as_str(),
                        _ => "never",
                    };
                    tx.execute(
                        "UPDATE clips
                         SET ocr_status = ?1, ocr_input_hash = ?2,
                             ocr_engine_version = ?3, ocr_extractor_ref = ?4,
                             ocr_extractor_name = ?5, ocr_attempted_at = ?6,
                             ocr_error = NULL
                         WHERE id = ?7",
                        params![
                            status,
                            metadata.input_hash.as_deref().unwrap_or(&clip.content_hash),
                            metadata.engine_version.as_deref(),
                            metadata.extractor_ref.as_deref(),
                            metadata.extractor_name.as_deref(),
                            metadata.attempted_at.as_deref(),
                            new_clip_id
                        ],
                    )?;
                }
            }
            tx.execute(
                "DELETE FROM clip_bins WHERE clip_id = ?1",
                params![new_clip_id],
            )?;
            for old_bin_id in clip.bin_ids.unwrap_or_default() {
                if let Some(new_bin_id) = bin_id_map.get(&old_bin_id) {
                    tx.execute(
                        "INSERT OR IGNORE INTO clip_bins (clip_id, bin_id) VALUES (?1, ?2)",
                        params![new_clip_id, new_bin_id],
                    )?;
                }
            }
            if let Some(new_bin_id) = mapped_primary_bin {
                tx.execute(
                    "INSERT OR IGNORE INTO clip_bins (clip_id, bin_id) VALUES (?1, ?2)",
                    params![new_clip_id, new_bin_id],
                )?;
            }
            imported += 1;
        }

        for (old_bin_id, ordered_clip_ids) in bin_clip_orders {
            let Some(new_bin_id) = bin_id_map.get(&old_bin_id) else {
                continue;
            };
            tx.execute(
                "DELETE FROM bin_clip_order WHERE bin_id = ?1",
                params![new_bin_id],
            )?;
            for (position, old_clip_id) in ordered_clip_ids.into_iter().enumerate() {
                let Some(new_clip_id) = clip_id_map.get(&old_clip_id) else {
                    continue;
                };
                tx.execute(
                    "INSERT OR REPLACE INTO bin_clip_order (bin_id, clip_id, position)
                     VALUES (?1, ?2, ?3)",
                    params![new_bin_id, new_clip_id, position as i64],
                )?;
            }
        }

        tx.commit()?;
        Ok(imported)
    }

    fn get_ocr_backup_metadata(&self) -> Result<Vec<OcrBackupMetadata>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT content_hash,
                    CASE WHEN ocr_status IN ('queued', 'running') THEN 'never' ELSE ocr_status END,
                    ocr_input_hash, ocr_engine_version, ocr_extractor_ref,
                    ocr_extractor_name, ocr_attempted_at
             FROM clips WHERE content_type = 'image'",
        )?;
        let metadata = statement
            .query_map([], |row| {
                Ok(OcrBackupMetadata {
                    content_hash: row.get(0)?,
                    status: row.get(1)?,
                    input_hash: row.get(2)?,
                    engine_version: row.get(3)?,
                    extractor_ref: row.get(4)?,
                    extractor_name: row.get(5)?,
                    attempted_at: row.get(6)?,
                })
            })?
            .collect();
        metadata
    }

    fn get_all_clips_for_backup(&self) -> Result<Vec<ClipItem>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, content_type, text_content, html_content, image_base64, image_path,
                    content_hash, source, is_pinned, is_protected, COALESCE(pin_order, 0),
                    bin_id, note, COALESCE(is_trashed, 0), trashed_at, created_at,
                    (SELECT GROUP_CONCAT(bin_id) FROM clip_bins WHERE clip_id = clips.id),
                    current_transformation_id IS NOT NULL,
                    ocr_extractor_ref, ocr_extractor_name, ocr_engine_version
             FROM clips ORDER BY created_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let primary_bin_id: Option<i64> = row.get(11)?;
            let bin_ids_csv: Option<String> = row.get(16)?;
            let mut bin_ids = primary_bin_id.into_iter().collect::<Vec<_>>();
            for value in bin_ids_csv.unwrap_or_default().split(',') {
                if let Ok(id) = value.parse::<i64>() {
                    if !bin_ids.contains(&id) {
                        bin_ids.push(id);
                    }
                }
            }
            Ok(ClipItem {
                id: row.get(0)?,
                content_type: row.get(1)?,
                text_content: row.get(2)?,
                html_content: row.get(3)?,
                image_base64: row.get(4)?,
                image_path: row.get(5)?,
                content_hash: row.get(6)?,
                source: row.get(7)?,
                is_pinned: row.get::<_, i32>(8)? != 0,
                is_protected: row.get::<_, i32>(9)? != 0,
                is_transformed: row.get::<_, i32>(17)? != 0,
                pin_order: row.get(10)?,
                bin_id: primary_bin_id,
                bin_ids: Some(bin_ids),
                note: row.get(12)?,
                is_trashed: row.get::<_, i32>(13)? != 0,
                trashed_at: row.get(14)?,
                created_at: row.get(15)?,
                ocr_extractor_ref: row.get(18)?,
                ocr_extractor_name: row.get(19)?,
                ocr_engine_version: row.get(20)?,
            })
        })?;
        rows.collect()
    }

    fn normalize_json_config(config: Option<&str>) -> String {
        match config {
            Some(value) if serde_json::from_str::<serde_json::Value>(value).is_ok() => {
                value.to_string()
            }
            Some(value) => serde_json::Value::String(value.to_string()).to_string(),
            None => "{}".to_string(),
        }
    }

    fn canonical_executor_kind(operation_type: &str) -> &str {
        match operation_type {
            "shell_script" => "shell",
            "regex" | "cli" | "shell" | "http" | "ai" => operation_type,
            _ => "cli",
        }
    }

    fn operation_storage_fields(op_type: &str, config: Option<&str>) -> (String, String) {
        if crate::operation_registry::is_builtin_operation(op_type) {
            (
                "builtin".to_string(),
                serde_json::json!({
                    "key": op_type,
                    "legacy_config": config.map(|value| Self::normalize_json_config(Some(value))),
                })
                .to_string(),
            )
        } else {
            (
                Self::canonical_executor_kind(op_type).to_string(),
                Self::normalize_json_config(config),
            )
        }
    }

    fn legacy_operation_fields(executor_kind: &str, config_json: &str) -> (String, Option<String>) {
        if executor_kind == "builtin" {
            let value = serde_json::from_str::<serde_json::Value>(config_json).unwrap_or_default();
            let operation_type = value["key"].as_str().unwrap_or("unknown").to_string();
            let config = value.get("legacy_config").and_then(|config| {
                if config.is_null() {
                    None
                } else if let Some(text) = config.as_str() {
                    Some(text.to_string())
                } else {
                    Some(config.to_string())
                }
            });
            (operation_type, config)
        } else {
            let operation_type = if executor_kind == "shell" {
                "shell_script"
            } else {
                executor_kind
            };
            let value = serde_json::from_str::<serde_json::Value>(config_json).ok();
            let config = value.map(|value| {
                value
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| value.to_string())
            });
            (operation_type.to_string(), config)
        }
    }

    pub fn resolve_custom_operation(
        &self,
        operation_ref: &str,
    ) -> Result<Option<ResolvedCustomOperation>> {
        let Some(operation_id) = operation_ref.strip_prefix("custom:") else {
            return Ok(None);
        };
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT executor_kind, config_json, enabled, trusted
             FROM custom_operations WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![operation_id])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        Ok(Some(ResolvedCustomOperation {
            executor_kind: row.get(0)?,
            config_json: row.get(1)?,
            enabled: row.get::<_, i64>(2)? != 0,
            trusted: row.get::<_, i64>(3)? != 0,
        }))
    }

    pub fn begin_transformation_execution(
        &self,
        request: TransformationExecutionStart<'_>,
    ) -> Result<String> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO transformation_executions
                (target_kind, target_ref, target_revision, source_clip_id,
                 trigger_kind, destination_kind, input_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                request.target_kind,
                request.target_ref,
                request.target_revision,
                request.source_clip_id,
                request.trigger_kind,
                request.destination_kind,
                request.input_hash
            ],
        )?;
        conn.query_row(
            "SELECT id FROM transformation_executions WHERE rowid = last_insert_rowid()",
            [],
            |row| row.get(0),
        )
    }

    pub fn finish_transformation_execution(
        &self,
        execution_id: &str,
        duration_ms: i64,
        output_hash: Option<&str>,
        error_summary: Option<&str>,
    ) -> Result<()> {
        let status = if error_summary.is_some() {
            "failed"
        } else {
            "succeeded"
        };
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE transformation_executions
             SET duration_ms = ?1, status = ?2, output_hash = ?3, error_summary = ?4,
                 completed_at = CURRENT_TIMESTAMP
             WHERE id = ?5",
            params![
                duration_ms,
                status,
                output_hash,
                error_summary,
                execution_id
            ],
        )?;
        Ok(())
    }

    pub fn cancel_transformation_execution(
        &self,
        execution_id: &str,
        duration_ms: i64,
    ) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE transformation_executions
             SET duration_ms = ?1, status = 'cancelled', output_hash = NULL,
                 error_summary = NULL, completed_at = CURRENT_TIMESTAMP
             WHERE id = ?2",
            params![duration_ms, execution_id],
        )?;
        Ok(())
    }

    pub fn start_transformation_execution(&self, execution_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE transformation_executions SET status = 'running'
             WHERE id = ?1 AND status = 'queued'",
            params![execution_id],
        )?;
        Ok(())
    }

    pub fn get_clip_transformation_executions(
        &self,
        clip_id: i64,
    ) -> Result<Vec<TransformationExecution>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, target_kind, target_ref, target_revision, source_clip_id,
                    trigger_kind, destination_kind, started_at, completed_at,
                    duration_ms, status, error_summary
             FROM transformation_executions
             WHERE source_clip_id = ?1
             ORDER BY started_at DESC, rowid DESC
             LIMIT 25",
        )?;
        let rows = statement.query_map(params![clip_id], |row| {
            Ok(TransformationExecution {
                id: row.get(0)?,
                target_kind: row.get(1)?,
                target_ref: row.get(2)?,
                target_revision: row.get(3)?,
                source_clip_id: row.get(4)?,
                trigger_kind: row.get(5)?,
                destination_kind: row.get(6)?,
                started_at: row.get(7)?,
                completed_at: row.get(8)?,
                duration_ms: row.get(9)?,
                status: row.get(10)?,
                error_summary: row.get(11)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_pipelines(&self) -> Result<Vec<Pipeline>> {
        let conn = self.conn.lock();
        let refs = {
            let mut statement = conn.prepare(
                "SELECT id FROM saved_transforms
                 WHERE authoring_kind = 'manual' ORDER BY row_id ASC",
            )?;
            let refs = statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>>>()?;
            refs
        };
        refs.into_iter()
            .map(|stable_id| {
                Self::saved_transform_by_id(&conn, &stable_id)
                    .and_then(Self::manual_transform_as_pipeline)
            })
            .collect()
    }

    fn manual_transform_as_pipeline(transform: SavedTransform) -> Result<Pipeline> {
        if transform.authoring_kind != "manual" {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let steps = transform
            .plan
            .steps
            .iter()
            .enumerate()
            .map(|(position, step)| match &step.executor {
                crate::transformation_intent::PlannedExecutor::Deterministic {
                    operation_ref,
                    config_json,
                } => Ok(PipelineStep {
                    position: position as i64,
                    operation_ref: operation_ref.clone(),
                    config_json: config_json.clone(),
                    failure_policy: match step.failure_policy {
                        crate::transformation_intent::StepFailurePolicy::Stop => "stop",
                        crate::transformation_intent::StepFailurePolicy::Skip => "skip",
                    }
                    .to_string(),
                }),
                crate::transformation_intent::PlannedExecutor::Semantic { .. } => {
                    Err(rusqlite::Error::InvalidParameterName(
                        "Manual Transform contains a semantic step".to_string(),
                    ))
                }
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Pipeline {
            id: transform.id,
            stable_ref: transform.stable_ref,
            name: transform.name,
            shortcut: transform.shortcut,
            revision: transform.revision,
            created_at: transform.created_at,
            updated_at: transform.updated_at,
            steps,
        })
    }

    fn saved_transform_by_id(conn: &Connection, transform_id: &str) -> Result<SavedTransform> {
        conn.query_row(
            "SELECT row_id, id, name, plan_json, connection_id, shortcut, authoring_kind, revision, created_at, updated_at
             FROM saved_transforms WHERE id = ?1",
            params![transform_id],
            |row| {
                let stable_id: String = row.get(1)?;
                let plan_json: String = row.get(3)?;
                let plan = serde_json::from_str(&plan_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
                Ok(SavedTransform {
                    id: row.get(0)?,
                    stable_ref: format!("transform:{stable_id}"),
                    name: row.get(2)?,
                    plan,
                    connection_id: row.get(4)?,
                    shortcut: row.get(5)?,
                    authoring_kind: row.get(6)?,
                    revision: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )
    }

    pub fn get_saved_transforms(&self) -> Result<Vec<SavedTransform>> {
        let conn = self.conn.lock();
        let ids = {
            let mut statement = conn
                .prepare("SELECT id FROM saved_transforms ORDER BY updated_at DESC, row_id DESC")?;
            let ids = statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>>>()?;
            ids
        };
        ids.into_iter()
            .map(|id| Self::saved_transform_by_id(&conn, &id))
            .collect()
    }

    pub fn get_intent_transforms(&self) -> Result<Vec<SavedTransform>> {
        Ok(self
            .get_saved_transforms()?
            .into_iter()
            .filter(|transform| transform.authoring_kind == "intent")
            .collect())
    }

    pub fn get_transform_definitions(&self) -> Result<Vec<TransformDefinition>> {
        let mut definitions = self
            .get_saved_transforms()?
            .into_iter()
            .map(TransformDefinition::from)
            .collect::<Vec<_>>();
        definitions.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.name.cmp(&right.name))
        });
        Ok(definitions)
    }

    pub fn resolve_transform_definition(
        &self,
        transform_ref: &str,
    ) -> Result<Option<TransformDefinition>> {
        if transform_ref.starts_with("pipeline:") {
            return self
                .resolve_saved_transform(transform_ref.trim_start_matches("pipeline:"))
                .map(|transform| transform.map(TransformDefinition::from));
        }
        self.resolve_saved_transform(transform_ref)
            .map(|transform| transform.map(TransformDefinition::from))
    }

    pub fn duplicate_transform_definition(
        &self,
        transform_ref: &str,
        name: Option<&str>,
    ) -> Result<TransformDefinition> {
        let definition = self
            .resolve_transform_definition(transform_ref)?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let duplicate_name = name
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("{} Copy", definition.name));
        match definition.authoring_kind {
            TransformAuthoringKind::Intent => {
                let plan = definition.plan.ok_or_else(|| {
                    rusqlite::Error::InvalidParameterName(
                        "Saved Transform has no execution plan".to_string(),
                    )
                })?;
                self.create_saved_transform(
                    &duplicate_name,
                    &plan,
                    definition.connection_id.as_deref(),
                )
                .map(TransformDefinition::from)
            }
            TransformAuthoringKind::Manual => {
                let steps = definition
                    .steps
                    .into_iter()
                    .map(|step| PipelineStepInput {
                        operation_ref: step.operation_ref,
                        config_json: step.config_json,
                        failure_policy: step.failure_policy,
                    })
                    .collect::<Vec<_>>();
                self.create_pipeline(&duplicate_name, &steps, None)
                    .map(TransformDefinition::from)
            }
        }
    }

    pub fn delete_transform_definition(&self, transform_ref: &str) -> Result<()> {
        if transform_ref.starts_with("pipeline:") {
            self.delete_pipeline(transform_ref)
        } else {
            self.delete_saved_transform(transform_ref)
        }
    }

    pub fn resolve_saved_transform(&self, transform_ref: &str) -> Result<Option<SavedTransform>> {
        let transform_id = transform_ref
            .strip_prefix("transform:")
            .unwrap_or(transform_ref);
        let conn = self.conn.lock();
        match Self::saved_transform_by_id(&conn, transform_id) {
            Ok(transform) => Ok(Some(transform)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn create_saved_transform(
        &self,
        name: &str,
        plan: &crate::transformation_intent::TransformationPlan,
        connection_id: Option<&str>,
    ) -> Result<SavedTransform> {
        plan.validate()
            .map_err(rusqlite::Error::InvalidParameterName)?;
        let plan_json = serde_json::to_string(plan).map_err(|error| {
            rusqlite::Error::InvalidParameterName(format!("invalid Transform: {error}"))
        })?;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO saved_transforms (name, plan_json, connection_id, authoring_kind)
             VALUES (?1, ?2, ?3, 'intent')",
            params![name.trim(), plan_json, connection_id],
        )?;
        let row_id = conn.last_insert_rowid();
        let stable_id: String = conn.query_row(
            "SELECT id FROM saved_transforms WHERE row_id = ?1",
            params![row_id],
            |row| row.get(0),
        )?;
        Self::saved_transform_by_id(&conn, &stable_id)
    }

    pub fn update_saved_transform(
        &self,
        transform_ref: &str,
        name: &str,
        plan: &crate::transformation_intent::TransformationPlan,
        connection_id: Option<&str>,
    ) -> Result<SavedTransform> {
        plan.validate()
            .map_err(rusqlite::Error::InvalidParameterName)?;
        let plan_json = serde_json::to_string(plan).map_err(|error| {
            rusqlite::Error::InvalidParameterName(format!("invalid Transform: {error}"))
        })?;
        let transform_id = transform_ref
            .strip_prefix("transform:")
            .unwrap_or(transform_ref);
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE saved_transforms
             SET name = ?1,
                 plan_json = ?2,
                 connection_id = ?3,
                 revision = revision + 1,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?4",
            params![name.trim(), plan_json, connection_id, transform_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Self::saved_transform_by_id(&conn, transform_id)
    }

    pub fn delete_saved_transform(&self, transform_ref: &str) -> Result<()> {
        let transform_id = transform_ref
            .strip_prefix("transform:")
            .unwrap_or(transform_ref);
        let conn = self.conn.lock();
        let changed = conn.execute(
            "DELETE FROM saved_transforms WHERE id = ?1",
            params![transform_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    pub fn apply_transform_output_to_clip(
        &self,
        request: TransformClipApplication<'_>,
    ) -> Result<ClipTransformationProvenance> {
        let TransformClipApplication {
            clip_id,
            transform_ref,
            expected_input,
            output,
            connection_id,
            duration_ms,
            bin_move,
        } = request;
        ensure_resource_size(
            expected_input,
            crate::resource_limits::MAX_TRANSFORM_TEXT_BYTES,
            "Transform input",
        )?;
        ensure_resource_size(
            output,
            crate::resource_limits::MAX_TRANSFORM_TEXT_BYTES,
            "Transform output",
        )?;
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let transform_id = transform_ref
            .strip_prefix("transform:")
            .or_else(|| transform_ref.strip_prefix("pipeline:"))
            .unwrap_or(transform_ref)
            .to_string();
        let (transform_name, transform_revision): (String, i64) = tx.query_row(
            "SELECT name, revision FROM saved_transforms WHERE id = ?1",
            params![transform_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let canonical_transform_ref = format!("transform:{transform_id}");
        let transform_id = Some(transform_id);
        let (current_text, is_trashed, current_transformation_id): (
            Option<String>,
            i32,
            Option<String>,
        ) = tx.query_row(
            "SELECT text_content, COALESCE(is_trashed, 0), current_transformation_id FROM clips WHERE id = ?1",
            params![clip_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        if is_trashed != 0 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Restore this clip before transforming it".to_string(),
            ));
        }
        if current_text.as_deref() != Some(expected_input) {
            return Err(rusqlite::Error::InvalidParameterName(
                "The clip changed after this preview was generated; preview it again".to_string(),
            ));
        }
        if expected_input == output {
            return Err(rusqlite::Error::InvalidParameterName(
                "The Transform did not change the clip".to_string(),
            ));
        }
        let (action_label, organization) =
            if let Some((previous_bin_id, destination_bin_id)) = bin_move {
                let destination_name = tx
                    .query_row(
                        "SELECT name FROM bins WHERE id = ?1",
                        params![destination_bin_id],
                        |row| row.get::<_, String>(0),
                    )
                    .unwrap_or_else(|_| format!("Bin #{destination_bin_id}"));
                (
                    format!("Moved to {destination_name} · Applied {transform_name}"),
                    Some(ClipRevisionOrganization {
                        category_bin_id: previous_bin_id,
                    }),
                )
            } else {
                (format!("Applied {transform_name}"), None)
            };
        if Self::revision_history_enabled_internal(&tx) {
            let context_json = serde_json::to_string(&ClipRevisionContext {
                schema_version: 1,
                action_kind: if organization.is_some() {
                    "transform_bin_drop".to_string()
                } else {
                    "transform".to_string()
                },
                action_label,
                organization,
                current_transformation_id,
            })
            .map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))?;
            tx.execute(
                "INSERT INTO clip_versions (clip_id, text_content, context_json) VALUES (?1, ?2, ?3)",
                params![clip_id, expected_input, context_json],
            )?;
            Self::prune_clip_versions_internal(&tx, clip_id)?;
        }
        let transformation_id: String =
            tx.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))?;
        tx.execute(
            "INSERT INTO clip_transformations
                (id, clip_id, transform_id, transform_ref, transform_name, transform_revision, connection_id, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                transformation_id,
                clip_id,
                transform_id,
                canonical_transform_ref,
                transform_name,
                transform_revision,
                connection_id,
                duration_ms.max(0)
            ],
        )?;
        tx.execute(
            "UPDATE clips SET text_content = ?1, current_transformation_id = ?2 WHERE id = ?3",
            params![output, transformation_id, clip_id],
        )?;
        let created_at: String = tx.query_row(
            "SELECT created_at FROM clip_transformations WHERE rowid = last_insert_rowid()",
            [],
            |row| row.get(0),
        )?;
        tx.commit()?;
        Ok(ClipTransformationProvenance {
            transform_ref: canonical_transform_ref,
            transform_name,
            transform_revision,
            connection_id: connection_id.map(str::to_string),
            duration_ms: duration_ms.max(0),
            created_at,
        })
    }

    pub fn get_clip_transformation_provenance(
        &self,
        clip_id: i64,
    ) -> Result<Option<ClipTransformationProvenance>> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT transformation.transform_ref, transformation.transform_id,
                    transformation.transform_name,
                    transformation.transform_revision, transformation.connection_id,
                    transformation.duration_ms, transformation.created_at
             FROM clips
             JOIN clip_transformations transformation
               ON transformation.id = clips.current_transformation_id
             WHERE clips.id = ?1",
            params![clip_id],
            |row| {
                let transform_ref: Option<String> = row.get(0)?;
                let transform_id: Option<String> = row.get(1)?;
                Ok(ClipTransformationProvenance {
                    transform_ref: transform_ref
                        .or_else(|| transform_id.map(|id| format!("transform:{id}")))
                        .unwrap_or_else(|| "transform:deleted".to_string()),
                    transform_name: row.get(2)?,
                    transform_revision: row.get(3)?,
                    connection_id: row.get(4)?,
                    duration_ms: row.get(5)?,
                    created_at: row.get(6)?,
                })
            },
        );
        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn validate_pipeline_steps(conn: &Connection, steps: &[PipelineStepInput]) -> Result<()> {
        if steps.is_empty() {
            return Err(rusqlite::Error::InvalidParameterName(
                "pipeline requires at least one operation".to_string(),
            ));
        }
        for step in steps {
            if !matches!(step.failure_policy.as_str(), "stop" | "skip") {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "invalid failure policy: {}",
                    step.failure_policy
                )));
            }
            if let Some(config) = &step.config_json {
                serde_json::from_str::<serde_json::Value>(config).map_err(|error| {
                    rusqlite::Error::InvalidParameterName(format!(
                        "invalid step config JSON: {error}"
                    ))
                })?;
            }
            if let Some(key) = step.operation_ref.strip_prefix("builtin:") {
                if !crate::operation_registry::is_builtin_operation(key) {
                    return Err(rusqlite::Error::InvalidParameterName(format!(
                        "unknown operation reference: {}",
                        step.operation_ref
                    )));
                }
            } else if let Some(custom_id) = step.operation_ref.strip_prefix("custom:") {
                let exists: bool = conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM custom_operations WHERE id = ?1)",
                    params![custom_id],
                    |row| row.get(0),
                )?;
                if !exists {
                    return Err(rusqlite::Error::InvalidParameterName(format!(
                        "unknown operation reference: {}",
                        step.operation_ref
                    )));
                }
            } else {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "invalid operation reference: {}",
                    step.operation_ref
                )));
            }
        }
        Ok(())
    }

    fn manual_transform_plan(
        name: &str,
        steps: &[PipelineStepInput],
    ) -> Result<crate::transformation_intent::TransformationPlan> {
        let plan = crate::transformation_intent::TransformationPlan {
            schema_version: crate::transformation_intent::TRANSFORMATION_PLAN_SCHEMA_VERSION,
            intent: format!("Run {}", name.trim()),
            summary: name.trim().to_string(),
            planning_mode: crate::transformation_intent::IntentPlanningMode::Pinned,
            steps: steps
                .iter()
                .map(
                    |step| crate::transformation_intent::PlannedTransformationStep {
                        name: step
                            .operation_ref
                            .strip_prefix("builtin:")
                            .or_else(|| step.operation_ref.strip_prefix("custom:"))
                            .unwrap_or(&step.operation_ref)
                            .replace('_', " "),
                        rationale: "Manually configured Operation".to_string(),
                        scope: crate::transformation_intent::StepExecutionScope::WholeInput,
                        failure_policy: if step.failure_policy == "skip" {
                            crate::transformation_intent::StepFailurePolicy::Skip
                        } else {
                            crate::transformation_intent::StepFailurePolicy::Stop
                        },
                        executor: crate::transformation_intent::PlannedExecutor::Deterministic {
                            operation_ref: step.operation_ref.clone(),
                            config_json: step.config_json.clone(),
                        },
                    },
                )
                .collect(),
        };
        plan.validate()
            .map_err(rusqlite::Error::InvalidParameterName)?;
        Ok(plan)
    }

    pub fn create_pipeline(
        &self,
        name: &str,
        steps: &[PipelineStepInput],
        shortcut: Option<&str>,
    ) -> Result<Pipeline> {
        let conn = self.conn.lock();
        Self::validate_pipeline_steps(&conn, steps)?;
        let plan = Self::manual_transform_plan(name, steps)?;
        let plan_json = serde_json::to_string(&plan).map_err(|error| {
            rusqlite::Error::InvalidParameterName(format!("invalid Transform: {error}"))
        })?;
        conn.execute(
            "INSERT INTO saved_transforms
                (name, plan_json, connection_id, shortcut, authoring_kind)
             VALUES (?1, ?2, NULL, ?3, 'manual')",
            params![name.trim(), plan_json, shortcut],
        )?;
        let stable_id: String = conn.query_row(
            "SELECT id FROM saved_transforms WHERE row_id = last_insert_rowid()",
            [],
            |row| row.get(0),
        )?;
        let pipeline =
            Self::manual_transform_as_pipeline(Self::saved_transform_by_id(&conn, &stable_id)?)?;
        drop(conn);
        let _ = self.log_activity(
            "transform_saved",
            &format!("Created Transform \"{}\"", pipeline.name),
        );
        Ok(pipeline)
    }

    pub fn update_pipeline(
        &self,
        pipeline_ref: &str,
        name: &str,
        steps: &[PipelineStepInput],
        shortcut: Option<&str>,
    ) -> Result<Pipeline> {
        let transform_id = pipeline_ref
            .strip_prefix("transform:")
            .or_else(|| pipeline_ref.strip_prefix("pipeline:"))
            .unwrap_or(pipeline_ref);
        let conn = self.conn.lock();
        Self::validate_pipeline_steps(&conn, steps)?;
        let plan_json =
            serde_json::to_string(&Self::manual_transform_plan(name, steps)?).map_err(|error| {
                rusqlite::Error::InvalidParameterName(format!("invalid Transform: {error}"))
            })?;
        let changed = conn.execute(
            "UPDATE saved_transforms
             SET name = ?1, plan_json = ?2, shortcut = ?3, revision = revision + 1,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?4 AND authoring_kind = 'manual'",
            params![name.trim(), plan_json, shortcut, transform_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let pipeline =
            Self::manual_transform_as_pipeline(Self::saved_transform_by_id(&conn, transform_id)?)?;
        drop(conn);
        let _ = self.log_activity(
            "transform_updated",
            &format!("Updated Transform \"{}\"", pipeline.name),
        );
        Ok(pipeline)
    }

    pub fn update_pipeline_shortcut(
        &self,
        pipeline_ref: &str,
        shortcut: Option<&str>,
    ) -> Result<()> {
        let pipeline_id = pipeline_ref
            .strip_prefix("transform:")
            .or_else(|| pipeline_ref.strip_prefix("pipeline:"))
            .unwrap_or(pipeline_ref);
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE saved_transforms
             SET shortcut = ?1, revision = revision + 1, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?2 AND authoring_kind = 'manual'",
            params![shortcut, pipeline_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        drop(conn);
        let _ = self.log_activity(
            "transform_updated",
            &format!("Updated Transform transform:{pipeline_id}"),
        );
        Ok(())
    }

    pub fn delete_pipeline(&self, pipeline_ref: &str) -> Result<()> {
        let pipeline_id = pipeline_ref
            .strip_prefix("transform:")
            .or_else(|| pipeline_ref.strip_prefix("pipeline:"))
            .unwrap_or(pipeline_ref);
        let conn = self.conn.lock();
        let name = conn
            .query_row(
                "SELECT name FROM saved_transforms WHERE id = ?1 AND authoring_kind = 'manual'",
                params![pipeline_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let changed = conn.execute(
            "DELETE FROM saved_transforms WHERE id = ?1 AND authoring_kind = 'manual'",
            params![pipeline_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        drop(conn);
        let _ = self.log_activity(
            "transform_deleted",
            &format!(
                "Deleted Transform \"{}\"",
                name.unwrap_or_else(|| pipeline_id.to_string())
            ),
        );
        Ok(())
    }

    pub fn get_intelligence_connections(&self) -> Result<Vec<IntelligenceConnection>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, name, provider_kind, endpoint, model, credential_ref,
                    enabled, priority, created_at, updated_at
             FROM intelligence_connections
             ORDER BY priority ASC, row_id ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(IntelligenceConnection {
                id: row.get(0)?,
                name: row.get(1)?,
                provider_kind: row.get(2)?,
                endpoint: row.get(3)?,
                model: row.get(4)?,
                credential_ref: row.get(5)?,
                enabled: row.get::<_, i64>(6)? != 0,
                priority: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_intelligence_connection(&self, id: &str) -> Result<IntelligenceConnection> {
        self.get_intelligence_connections()?
            .into_iter()
            .find(|connection| connection.id == id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn create_intelligence_connection(
        &self,
        name: &str,
        provider_kind: &str,
        endpoint: Option<&str>,
        model: Option<&str>,
        credential_ref: Option<&str>,
    ) -> Result<IntelligenceConnection> {
        if name.trim().is_empty() {
            return Err(rusqlite::Error::InvalidParameterName(
                "Connection name cannot be empty".into(),
            ));
        }
        crate::intelligence_connections::validate_credential_reference(credential_ref)
            .map_err(rusqlite::Error::InvalidParameterName)?;
        let conn = self.conn.lock();
        let priority: i64 = conn.query_row(
            "SELECT COALESCE(MAX(priority), -1) + 1 FROM intelligence_connections",
            [],
            |row| row.get(0),
        )?;
        conn.execute(
            "INSERT INTO intelligence_connections
                (name, provider_kind, endpoint, model, credential_ref, priority)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                name.trim(),
                provider_kind,
                endpoint,
                model,
                credential_ref,
                priority
            ],
        )?;
        let row_id = conn.last_insert_rowid();
        conn.query_row(
            "SELECT id, name, provider_kind, endpoint, model, credential_ref,
                    enabled, priority, created_at, updated_at
             FROM intelligence_connections WHERE row_id = ?1",
            params![row_id],
            |row| {
                Ok(IntelligenceConnection {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    provider_kind: row.get(2)?,
                    endpoint: row.get(3)?,
                    model: row.get(4)?,
                    credential_ref: row.get(5)?,
                    enabled: row.get::<_, i64>(6)? != 0,
                    priority: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )
    }

    pub fn ensure_intelligence_connection_candidate(
        &self,
        name: &str,
        provider_kind: &str,
        endpoint: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock();
        let exists = conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM intelligence_connections
                WHERE provider_kind = ?1
                  AND COALESCE(endpoint, '') = COALESCE(?2, '')
            )",
            params![provider_kind, endpoint],
            |row| row.get::<_, bool>(0),
        )?;
        if exists {
            return Ok(());
        }
        let priority: i64 = conn.query_row(
            "SELECT COALESCE(MAX(priority), -1) + 1 FROM intelligence_connections",
            [],
            |row| row.get(0),
        )?;
        conn.execute(
            "INSERT INTO intelligence_connections
                (name, provider_kind, endpoint, enabled, priority)
             VALUES (?1, ?2, ?3, 0, ?4)",
            params![name.trim(), provider_kind, endpoint, priority],
        )?;
        Ok(())
    }

    pub fn update_intelligence_connection(
        &self,
        request: IntelligenceConnectionUpdate<'_>,
    ) -> Result<()> {
        if request.name.trim().is_empty() {
            return Err(rusqlite::Error::InvalidParameterName(
                "Connection name cannot be empty".into(),
            ));
        }
        crate::intelligence_connections::validate_credential_reference(request.credential_ref)
            .map_err(rusqlite::Error::InvalidParameterName)?;
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE intelligence_connections
             SET name = ?1, provider_kind = ?2, endpoint = ?3, model = ?4,
                 credential_ref = ?5, enabled = ?6, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?7",
            params![
                request.name.trim(),
                request.provider_kind,
                request.endpoint,
                request.model,
                request.credential_ref,
                request.enabled as i64,
                request.id
            ],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    pub fn delete_intelligence_connection(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock();
        let changed = conn.execute(
            "DELETE FROM intelligence_connections WHERE id = ?1",
            params![id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    pub fn reorder_intelligence_connections(&self, ids: &[String]) -> Result<()> {
        let unique = ids.iter().collect::<std::collections::HashSet<_>>();
        if unique.len() != ids.len() {
            return Err(rusqlite::Error::InvalidParameterName(
                "Connection order contains duplicate IDs".into(),
            ));
        }
        let current = self
            .get_intelligence_connections()?
            .into_iter()
            .map(|connection| connection.id)
            .collect::<std::collections::HashSet<_>>();
        if current != ids.iter().cloned().collect() {
            return Err(rusqlite::Error::InvalidParameterName(
                "Connection order must contain every current Connection exactly once".into(),
            ));
        }
        let mut conn = self.conn.lock();
        let transaction = conn.transaction()?;
        for (priority, id) in ids.iter().enumerate() {
            let changed = transaction.execute(
                "UPDATE intelligence_connections SET priority = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![priority as i64, id],
            )?;
            if changed == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
        }
        transaction.commit()
    }

    pub fn get_operations(&self) -> Result<Vec<Operation>> {
        let conn = self.conn.lock();
        let mut operations = crate::operation_registry::BUILTIN_OPERATIONS
            .iter()
            .enumerate()
            .map(|(index, definition)| Operation {
                id: -((index as i64) + 1),
                stable_id: format!("builtin:{}", definition.key),
                name: definition.name.to_string(),
                op_type: definition.key.to_string(),
                config: None,
                category: definition.category_label.to_string(),
                created_at: String::new(),
            })
            .collect::<Vec<_>>();
        let mut stmt = conn.prepare(
            "SELECT row_id, id, name, executor_kind, config_json, category, created_at
             FROM custom_operations ORDER BY row_id ASC",
        )?;
        let op_iter = stmt.query_map([], |row| {
            let operation_id = row.get::<_, String>(1)?;
            let executor_kind = row.get::<_, String>(3)?;
            let config_json = row.get::<_, String>(4)?;
            let (op_type, config) = Self::legacy_operation_fields(&executor_kind, &config_json);
            Ok(Operation {
                id: row.get(0)?,
                stable_id: format!("custom:{operation_id}"),
                name: row.get(2)?,
                op_type,
                config,
                category: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        for o in op_iter {
            operations.push(o?);
        }
        Ok(operations)
    }

    pub fn get_operation(&self, reference: &str) -> Result<Operation> {
        let numeric_id = reference.parse::<i64>().ok();
        self.get_operations()?
            .into_iter()
            .find(|operation| numeric_id == Some(operation.id) || operation.stable_id == reference)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn duplicate_operation(&self, reference: &str, name: Option<&str>) -> Result<Operation> {
        let source = self.get_operation(reference)?;
        let default_name = format!("{} Copy", source.name);
        self.create_operation(
            name.unwrap_or(&default_name),
            &source.op_type,
            source.config.as_deref(),
            Some(&source.category),
        )
    }

    pub fn get_library_items(
        &self,
        kind: Option<&str>,
        include_archived: bool,
    ) -> Result<Vec<crate::library_items::LibraryItemView>> {
        if let Some(kind) = kind {
            if !matches!(
                kind,
                "inspector" | "extractor" | "detector" | "enricher" | "operation" | "transform"
            ) {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Unknown library item kind".into(),
                ));
            }
        }
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT stable_ref, kind, name, description, group_label, icon, enabled,
                    is_builtin, is_archived, sort_order, revision, input_contract,
                    output_contract, created_at, updated_at
             FROM library_items
             WHERE (?1 IS NULL OR kind = ?1) AND (?2 OR is_archived = 0)
             ORDER BY kind, COALESCE(sort_order, 10000), name COLLATE NOCASE",
        )?;
        let rows = statement.query_map(params![kind, include_archived], |row| {
            let item = crate::library_items::LibraryItem {
                stable_ref: row.get(0)?,
                kind: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                group_label: row.get(4)?,
                icon: row.get(5)?,
                enabled: row.get(6)?,
                is_builtin: row.get(7)?,
                is_archived: row.get(8)?,
                sort_order: row.get(9)?,
                revision: row.get(10)?,
                input_contract: row.get(11)?,
                output_contract: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            };
            let analysis_pass = item.analysis_pass();
            let capabilities = item.capabilities();
            Ok(crate::library_items::LibraryItemView {
                item,
                analysis_pass,
                capabilities,
            })
        })?;
        rows.collect()
    }

    pub fn set_library_item_enabled(
        &self,
        kind: &str,
        stable_ref: &str,
        enabled: bool,
    ) -> Result<()> {
        let conn = self.conn.lock();
        let changed = match kind {
            "inspector" | "enricher" => {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Built-in Analysis participants cannot be disabled".to_string(),
                ));
            }
            "extractor" => conn.execute(
                "UPDATE content_extractors
                 SET enabled = ?1, updated_at = CURRENT_TIMESTAMP
                 WHERE stable_ref = ?2 AND is_deleted = 0",
                params![enabled, stable_ref],
            )?,
            "detector" => conn.execute(
                "UPDATE content_detectors
                 SET enabled = ?1, updated_at = CURRENT_TIMESTAMP
                 WHERE stable_ref = ?2 AND is_deleted = 0",
                params![enabled, stable_ref],
            )?,
            "operation" => {
                let Some(operation_id) = stable_ref.strip_prefix("custom:") else {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "Built-in Operations cannot be disabled".to_string(),
                    ));
                };
                conn.execute(
                    "UPDATE custom_operations
                     SET enabled = ?1, updated_at = CURRENT_TIMESTAMP
                     WHERE id = ?2",
                    params![enabled, operation_id],
                )?
            }
            "transform" => {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Transforms do not currently have an enabled state".to_string(),
                ));
            }
            _ => {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Unknown library item kind".to_string(),
                ));
            }
        };
        drop(conn);
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let _ = self.log_activity(
            "library_item_enabled_changed",
            &format!(
                "{} {} {}",
                if enabled { "Enabled" } else { "Disabled" },
                kind,
                stable_ref
            ),
        );
        Ok(())
    }

    pub fn create_operation(
        &self,
        name: &str,
        op_type: &str,
        config: Option<&str>,
        category: Option<&str>,
    ) -> Result<Operation> {
        let conn = self.conn.lock();
        let cat = category.unwrap_or("Custom Operations");
        let (executor_kind, config_json) = Self::operation_storage_fields(op_type, config);
        conn.execute(
            "INSERT INTO custom_operations
                (name, executor_kind, config_json, category, trusted)
             VALUES (?1, ?2, ?3, ?4, 1)",
            params![name, executor_kind, config_json, cat],
        )?;
        let id = conn.last_insert_rowid();
        let stable_id: String = conn.query_row(
            "SELECT id FROM custom_operations WHERE row_id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        let operation = Operation {
            id,
            stable_id: format!("custom:{stable_id}"),
            name: name.to_string(),
            op_type: op_type.to_string(),
            config: config.map(str::to_string),
            category: cat.to_string(),
            created_at: conn.query_row(
                "SELECT created_at FROM custom_operations WHERE row_id = ?1",
                params![id],
                |row| row.get(0),
            )?,
        };
        drop(conn);
        let _ = self.log_activity(
            "operation_created",
            &format!("Created Operation \"{}\"", operation.name),
        );
        Ok(operation)
    }

    pub fn update_operation(
        &self,
        id: i64,
        name: &str,
        op_type: &str,
        config: Option<&str>,
        category: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock();
        let cat = category.unwrap_or("Custom Operations");
        let (executor_kind, config_json) = Self::operation_storage_fields(op_type, config);
        let changed = conn.execute(
            "UPDATE custom_operations
             SET name = ?1, executor_kind = ?2, config_json = ?3, category = ?4,
                 updated_at = CURRENT_TIMESTAMP
             WHERE row_id = ?5",
            params![name, executor_kind, config_json, cat, id],
        )?;
        drop(conn);
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let _ = self.log_activity(
            "operation_updated",
            &format!("Updated Operation \"{}\"", name),
        );
        Ok(())
    }

    pub fn delete_operation(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        let stable_id = conn
            .query_row(
                "SELECT id FROM custom_operations WHERE row_id = ?1",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(stable_id) = stable_id else {
            return Err(rusqlite::Error::InvalidParameterName(
                "Operation not found".to_string(),
            ));
        };
        let operation_ref = format!("custom:{stable_id}");
        let transform_name = conn
            .query_row(
                "SELECT saved_transforms.name
                 FROM saved_transforms, json_each(saved_transforms.plan_json, '$.steps') AS step
                 WHERE json_extract(step.value, '$.executor.kind') = 'deterministic'
                   AND json_extract(step.value, '$.executor.operation_ref') = ?1
                 ORDER BY saved_transforms.name ASC
                 LIMIT 1",
                params![operation_ref],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(transform_name) = transform_name {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "Operation is used by “{transform_name}”. Remove it from that Transform before deleting it."
            )));
        }
        conn.execute(
            "DELETE FROM custom_operations WHERE row_id = ?1",
            params![id],
        )?;
        drop(conn);
        let _ = self.log_activity(
            "operation_deleted",
            &format!("Deleted Operation {operation_ref}"),
        );
        Ok(())
    }

    pub fn configure_clip_retention(&self, keep_count: i64, keep_age_days: i64) -> Result<()> {
        let keep_count = keep_count.clamp(0, 100_000);
        let keep_age_days = keep_age_days.clamp(0, 36_500);
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES ('keepClipCount', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [keep_count.to_string()],
        )?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES ('keepClipAgeDays', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [keep_age_days.to_string()],
        )?;
        self.enforce_clip_retention_internal(&tx, keep_count, keep_age_days)?;
        tx.commit()
    }

    pub fn enforce_clip_retention(&self, keep_count: i64, keep_age_days: i64) -> Result<()> {
        let conn = self.conn.lock();
        self.enforce_clip_retention_internal(
            &conn,
            keep_count.clamp(0, 100_000),
            keep_age_days.clamp(0, 36_500),
        )
    }

    pub fn configure_trash_retention(&self, keep_count: i64, keep_age_days: i64) -> Result<()> {
        let keep_count = keep_count.clamp(0, 100_000);
        let keep_age_days = keep_age_days.clamp(0, 36_500);
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES ('trashCapacityCount', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [keep_count.to_string()],
        )?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES ('trashAgeDays', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [keep_age_days.to_string()],
        )?;
        self.enforce_trash_retention_internal(&tx, keep_count, keep_age_days)?;
        tx.commit()
    }

    pub fn enforce_trash_retention(&self, keep_count: i64, keep_age_days: i64) -> Result<()> {
        let conn = self.conn.lock();
        self.enforce_trash_retention_internal(
            &conn,
            keep_count.clamp(0, 100_000),
            keep_age_days.clamp(0, 36_500),
        )
    }

    pub fn configure_activity_retention(&self, keep_count: i64, keep_age_days: i64) -> Result<()> {
        let keep_count = keep_count.clamp(0, 100_000);
        let keep_age_days = keep_age_days.clamp(0, 36_500);
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES ('activityLogCapacity', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [keep_count.to_string()],
        )?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES ('activityLogAgeDays', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [keep_age_days.to_string()],
        )?;
        self.enforce_activity_retention_internal(&tx, keep_count, keep_age_days)?;
        tx.commit()
    }

    pub fn enforce_activity_retention(&self, keep_count: i64, keep_age_days: i64) -> Result<()> {
        let conn = self.conn.lock();
        self.enforce_activity_retention_internal(
            &conn,
            keep_count.clamp(0, 100_000),
            keep_age_days.clamp(0, 36_500),
        )
    }

    pub fn purge_old_clips(&self, keep_count: i64) -> Result<()> {
        self.enforce_clip_retention(keep_count, 0)
    }

    pub fn enforce_revision_retention(&self, keep_count: i64) -> Result<()> {
        let keep_count = keep_count.max(0);
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES ('revisionHistoryLimit', ?1)
             ON CONFLICT(key) DO UPDATE SET value = ?1",
            params![keep_count.to_string()],
        )?;
        if keep_count > 0 {
            tx.execute(
                "DELETE FROM clip_versions WHERE id IN (
                    SELECT id FROM (
                        SELECT id,
                               ROW_NUMBER() OVER (PARTITION BY clip_id ORDER BY id DESC) AS revision_rank
                        FROM clip_versions
                    ) WHERE revision_rank > ?1
                 )",
                params![keep_count],
            )?;
        }
        tx.commit()
    }

    pub fn save_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn save_settings(&self, values: &std::collections::HashMap<String, String>) -> Result<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        {
            let mut statement = tx.prepare_cached(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = ?2",
            )?;
            for (key, value) in values {
                statement.execute(params![key, value])?;
            }
        }
        tx.commit()
    }

    pub fn delete_setting(&self, key: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            let val: String = row.get(0)?;
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }

    pub fn get_all_settings(&self) -> Result<std::collections::HashMap<String, String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut map = std::collections::HashMap::new();
        for r in rows {
            let (k, v) = r?;
            map.insert(k, v);
        }
        Ok(map)
    }

    pub fn get_content_type_groups(
        &self,
        include_archived: bool,
    ) -> Result<Vec<crate::content_types::ContentTypeGroupDefinition>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, label, sort_order, is_builtin, is_archived
             FROM content_type_groups WHERE ?1 OR is_archived = 0
             ORDER BY is_archived, sort_order, label COLLATE NOCASE",
        )?;
        let groups: Result<Vec<_>> = statement
            .query_map(params![include_archived], |row| {
                Ok(crate::content_types::ContentTypeGroupDefinition {
                    id: row.get(0)?,
                    label: row.get(1)?,
                    sort_order: row.get(2)?,
                    is_builtin: row.get(3)?,
                    is_archived: row.get(4)?,
                    defaults: None,
                })
            })?
            .collect();
        groups.map(|mut groups: Vec<_>| {
            for group in &mut groups {
                if group.is_builtin {
                    group.defaults = crate::content_types::content_type_group_defaults(&group.id);
                }
            }
            groups
        })
    }

    pub fn create_content_type_group(
        &self,
        input: &crate::content_types::ContentTypeGroupInput,
    ) -> Result<crate::content_types::ContentTypeGroupDefinition> {
        crate::content_types::validate_content_type_group_input(input)
            .map_err(rusqlite::Error::InvalidParameterName)?;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO content_type_groups (id, label, sort_order, is_builtin, is_archived) VALUES (?1, ?2, ?3, 0, 0)",
            params![input.id, input.label.trim(), input.sort_order],
        )?;
        drop(conn);
        let created = self
            .get_content_type_groups(true)?
            .into_iter()
            .find(|item| item.id == input.id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let _ = self.log_activity(
            "content_type_group_created",
            &format!("Created content type group \"{}\"", created.label),
        );
        Ok(created)
    }

    pub fn update_content_type_group(
        &self,
        id: &str,
        input: &crate::content_types::ContentTypeGroupInput,
    ) -> Result<crate::content_types::ContentTypeGroupDefinition> {
        if id != input.id {
            return Err(rusqlite::Error::InvalidParameterName(
                "Content type Group IDs cannot be changed".into(),
            ));
        }
        crate::content_types::validate_content_type_group_input(input)
            .map_err(rusqlite::Error::InvalidParameterName)?;
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE content_type_groups SET label = ?1, sort_order = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
            params![input.label.trim(), input.sort_order, id],
        )?;
        drop(conn);
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let updated = self
            .get_content_type_groups(true)?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let _ = self.log_activity(
            "content_type_group_updated",
            &format!("Updated content type group \"{}\"", updated.label),
        );
        Ok(updated)
    }

    pub fn set_content_type_group_archived(&self, id: &str, archived: bool) -> Result<()> {
        let conn = self.conn.lock();
        let (is_builtin, usage_count): (bool, i64) = conn.query_row(
            "SELECT is_builtin, (SELECT COUNT(*) FROM content_types WHERE group_name = ?1) FROM content_type_groups WHERE id = ?1",
            params![id], |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional()?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        if is_builtin {
            return Err(rusqlite::Error::InvalidParameterName(
                "Built-in content type groups cannot be archived".into(),
            ));
        }
        if archived && usage_count > 0 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Move Types out of this Group before archiving it".into(),
            ));
        }
        conn.execute("UPDATE content_type_groups SET is_archived = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2", params![archived, id])?;
        drop(conn);
        let _ = self.log_activity(
            if archived {
                "content_type_group_archived"
            } else {
                "content_type_group_restored"
            },
            &format!(
                "{} content type group {id}",
                if archived { "Archived" } else { "Restored" }
            ),
        );
        Ok(())
    }

    pub fn delete_content_type_group(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock();
        let (is_builtin, usage_count, label): (bool, i64, String) = conn
            .query_row(
                "SELECT is_builtin, (SELECT COUNT(*) FROM content_types WHERE group_name = ?1), label
                 FROM content_type_groups WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        if is_builtin {
            return Err(rusqlite::Error::InvalidParameterName(
                "Built-in content type groups cannot be deleted".into(),
            ));
        }
        if usage_count > 0 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Move Types out of this Group before deleting it".into(),
            ));
        }
        conn.execute("DELETE FROM content_type_groups WHERE id = ?1", params![id])?;
        drop(conn);
        let _ = self.log_activity(
            "content_type_group_deleted",
            &format!("Deleted content type group \"{label}\""),
        );
        Ok(())
    }

    pub fn restore_default_content_type_groups(&self) -> Result<()> {
        let conn = self.conn.lock();
        for preset in crate::content_types::CONTENT_TYPE_GROUP_PRESETS {
            conn.execute(
                "UPDATE content_type_groups SET label = ?1, sort_order = ?2, is_archived = 0, updated_at = CURRENT_TIMESTAMP WHERE id = ?3 AND is_builtin = 1",
                params![preset.label, preset.sort_order, preset.id],
            )?;
        }
        drop(conn);
        let _ = self.log_activity(
            "content_type_groups_restored",
            "Restored built-in content type groups",
        );
        Ok(())
    }

    pub fn get_content_types(
        &self,
        include_archived: bool,
    ) -> Result<Vec<crate::content_types::ContentTypeDefinition>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT types.id, types.label, types.icon, types.group_name, types.is_builtin, types.is_archived
             FROM content_types AS types
             LEFT JOIN content_type_groups AS groups ON groups.id = types.group_name
             WHERE ?1 OR types.is_archived = 0
             ORDER BY types.is_archived, COALESCE(groups.sort_order, 10000), types.is_builtin DESC, types.label COLLATE NOCASE",
        )?;
        let definitions: Result<Vec<_>> = statement
            .query_map(params![include_archived], |row| {
                Ok(crate::content_types::ContentTypeDefinition {
                    id: row.get(0)?,
                    label: row.get(1)?,
                    icon: row.get(2)?,
                    group: row.get(3)?,
                    is_builtin: row.get(4)?,
                    is_archived: row.get(5)?,
                    defaults: None,
                })
            })?
            .collect();
        definitions.map(|mut definitions: Vec<_>| {
            for definition in &mut definitions {
                if definition.is_builtin {
                    definition.defaults =
                        crate::content_types::content_type_defaults(&definition.id);
                }
            }
            definitions
        })
    }

    pub fn create_content_type(
        &self,
        input: &crate::content_types::ContentTypeInput,
    ) -> Result<crate::content_types::ContentTypeDefinition> {
        crate::content_types::validate_content_type_input(input).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                error,
            )))
        })?;
        let conn = self.conn.lock();
        let group_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM content_type_groups WHERE id = ?1 AND is_archived = 0)",
            params![input.group],
            |row| row.get(0),
        )?;
        if !group_exists {
            return Err(rusqlite::Error::InvalidParameterName(
                "Content type Group must exist and be active".into(),
            ));
        }
        conn.execute(
            "INSERT INTO content_types (id, label, icon, group_name, is_builtin, is_archived)
             VALUES (?1, ?2, ?3, ?4, 0, 0)",
            params![input.id, input.label.trim(), input.icon, input.group],
        )?;
        drop(conn);
        let created = self
            .get_content_types(true)?
            .into_iter()
            .find(|item| item.id == input.id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let _ = self.log_activity(
            "content_type_created",
            &format!("Created content type \"{}\"", created.label),
        );
        Ok(created)
    }

    pub fn update_content_type(
        &self,
        id: &str,
        input: &crate::content_types::ContentTypeInput,
    ) -> Result<crate::content_types::ContentTypeDefinition> {
        if id != input.id {
            return Err(rusqlite::Error::InvalidParameterName(
                "Content type IDs cannot be changed".into(),
            ));
        }
        crate::content_types::validate_content_type_input(input).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                error,
            )))
        })?;
        let conn = self.conn.lock();
        let group_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM content_type_groups WHERE id = ?1 AND is_archived = 0)",
            params![input.group],
            |row| row.get(0),
        )?;
        if !group_exists {
            return Err(rusqlite::Error::InvalidParameterName(
                "Content type Group must exist and be active".into(),
            ));
        }
        let changed = conn.execute(
            "UPDATE content_types SET label = ?1, icon = ?2, group_name = ?3,
                    updated_at = CURRENT_TIMESTAMP WHERE id = ?4",
            params![input.label.trim(), input.icon, input.group, id],
        )?;
        drop(conn);
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let updated = self
            .get_content_types(true)?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let _ = self.log_activity(
            "content_type_updated",
            &format!("Updated content type \"{}\"", updated.label),
        );
        Ok(updated)
    }

    pub fn set_content_type_archived(&self, id: &str, archived: bool) -> Result<()> {
        let conn = self.conn.lock();
        let is_builtin = conn
            .query_row(
                "SELECT is_builtin FROM content_types WHERE id = ?1",
                params![id],
                |row| row.get::<_, bool>(0),
            )
            .optional()?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        if is_builtin {
            return Err(rusqlite::Error::InvalidParameterName(
                "Built-in content types cannot be archived".into(),
            ));
        }
        let transaction = conn.unchecked_transaction()?;
        transaction.execute(
            "UPDATE content_types SET is_archived = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![archived, id],
        )?;
        if archived {
            transaction.execute(
                "UPDATE content_detectors SET enabled = 0, updated_at = CURRENT_TIMESTAMP
                 WHERE content_type = ?1 AND is_deleted = 0",
                params![id],
            )?;
        }
        transaction.commit()?;
        drop(conn);
        let _ = self.log_activity(
            if archived {
                "content_type_archived"
            } else {
                "content_type_restored"
            },
            &format!(
                "{} content type {id}",
                if archived { "Archived" } else { "Restored" }
            ),
        );
        Ok(())
    }

    pub fn restore_default_content_types(&self) -> Result<()> {
        let conn = self.conn.lock();
        for preset in crate::content_types::CONTENT_TYPE_PRESETS {
            conn.execute(
                "UPDATE content_types SET label = ?1, icon = ?2, group_name = ?3,
                        is_archived = 0, updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?4 AND is_builtin = 1",
                params![preset.label, preset.icon, preset.group, preset.id],
            )?;
        }
        drop(conn);
        let _ = self.log_activity(
            "content_types_restored",
            "Restored built-in content type metadata",
        );
        Ok(())
    }

    pub fn get_content_extractors(&self) -> Result<Vec<crate::content_extraction::Extractor>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, stable_ref, name, description, engine, input_contract,
                    output_contract, enabled, priority, is_builtin
             FROM content_extractors WHERE is_deleted = 0 ORDER BY priority, id",
        )?;
        let rows = statement.query_map([], |row| {
            let stable_ref = row.get::<_, String>(1)?;
            let engine = row.get::<_, String>(4)?;
            let preset = crate::content_extraction::EXTRACTOR_PRESETS
                .iter()
                .find(|preset| preset.stable_ref == stable_ref);
            let availability = crate::content_extraction::engine_availability(&engine);
            Ok(crate::content_extraction::Extractor {
                id: row.get(0)?,
                stable_ref,
                name: row.get(2)?,
                description: row.get(3)?,
                engine,
                input_contract: row.get(5)?,
                output_contract: row.get(6)?,
                enabled: row.get(7)?,
                priority: row.get(8)?,
                is_builtin: row.get(9)?,
                is_available: availability.is_available,
                unavailable_reason: availability.unavailable_reason,
                defaults: preset.map(|preset| crate::content_extraction::ExtractorInput {
                    name: preset.name.to_string(),
                    description: preset.description.to_string(),
                    enabled: true,
                    priority: preset.priority,
                }),
            })
        })?;
        rows.collect()
    }

    pub fn get_content_extractor(
        &self,
        reference: &str,
    ) -> Result<crate::content_extraction::Extractor> {
        let numeric_id = reference.parse::<i64>().ok();
        self.get_content_extractors()?
            .into_iter()
            .find(|extractor| numeric_id == Some(extractor.id) || extractor.stable_ref == reference)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn create_content_extractor(
        &self,
        input: &crate::content_extraction::ExtractorDefinitionInput,
    ) -> Result<crate::content_extraction::Extractor> {
        crate::content_extraction::validate_extractor_definition(input).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                error,
            )))
        })?;
        let conn = self.conn.lock();
        let extractor_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM content_extractors WHERE is_deleted = 0",
            [],
            |row| row.get(0),
        )?;
        if extractor_count >= 64 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Content Extractors are limited to 64 entries".into(),
            ));
        }
        conn.execute(
            "INSERT INTO content_extractors
                (stable_ref, name, description, engine, input_contract, output_contract,
                 enabled, priority, is_builtin)
             VALUES ('pending', ?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
            params![
                input.name.trim(),
                input.description.trim(),
                input.engine.trim(),
                input.input_contract,
                input.output_contract,
                input.enabled,
                input.priority
            ],
        )?;
        let id = conn.last_insert_rowid();
        conn.execute(
            "UPDATE content_extractors SET stable_ref = ?1 WHERE id = ?2",
            params![format!("extractor:custom:{id}"), id],
        )?;
        drop(conn);
        let created = self.get_content_extractor(&id.to_string())?;
        let _ = self.log_activity(
            "content_extractor_created",
            &format!("Created Extractor \"{}\"", created.name),
        );
        Ok(created)
    }

    pub fn update_content_extractor_definition(
        &self,
        id: i64,
        input: &crate::content_extraction::ExtractorDefinitionInput,
    ) -> Result<crate::content_extraction::Extractor> {
        crate::content_extraction::validate_extractor_definition(input).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                error,
            )))
        })?;
        let current = self.get_content_extractor(&id.to_string())?;
        if current.is_builtin
            && (current.engine != input.engine
                || current.input_contract != input.input_contract
                || current.output_contract != input.output_contract)
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "Built-in Extractor engine and contracts cannot be changed".into(),
            ));
        }
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE content_extractors SET name = ?1, description = ?2, engine = ?3,
                    input_contract = ?4, output_contract = ?5, enabled = ?6,
                    priority = ?7, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?8 AND is_deleted = 0",
            params![
                input.name.trim(),
                input.description.trim(),
                input.engine.trim(),
                input.input_contract,
                input.output_contract,
                input.enabled,
                input.priority,
                id
            ],
        )?;
        drop(conn);
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let updated = self.get_content_extractor(&id.to_string())?;
        let _ = self.log_activity(
            "content_extractor_updated",
            &format!("Updated Extractor \"{}\"", updated.name),
        );
        Ok(updated)
    }

    pub fn duplicate_content_extractor(
        &self,
        reference: &str,
        name: Option<&str>,
    ) -> Result<crate::content_extraction::Extractor> {
        let source = self.get_content_extractor(reference)?;
        self.create_content_extractor(&crate::content_extraction::ExtractorDefinitionInput {
            name: name
                .map(str::to_string)
                .unwrap_or_else(|| format!("{} Copy", source.name)),
            description: source.description,
            engine: source.engine,
            input_contract: source.input_contract,
            output_contract: source.output_contract,
            enabled: source.enabled,
            priority: source.priority.saturating_add(1).min(10_000),
        })
    }

    pub fn delete_content_extractor(&self, id: i64) -> Result<()> {
        let extractor = self.get_content_extractor(&id.to_string())?;
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE content_extractors SET is_deleted = 1, enabled = 0,
                    updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1 AND is_deleted = 0",
            params![id],
        )?;
        drop(conn);
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let _ = self.log_activity(
            "content_extractor_deleted",
            &format!("Deleted Extractor \"{}\"", extractor.name),
        );
        Ok(())
    }

    pub fn update_content_extractor(
        &self,
        id: i64,
        input: &crate::content_extraction::ExtractorInput,
    ) -> Result<crate::content_extraction::Extractor> {
        crate::content_extraction::validate_extractor_input(input).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                error,
            )))
        })?;
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE content_extractors SET name = ?1, description = ?2, enabled = ?3,
                    priority = ?4, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?5 AND is_deleted = 0",
            params![
                input.name.trim(),
                input.description.trim(),
                input.enabled,
                input.priority,
                id
            ],
        )?;
        drop(conn);
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let updated = self
            .get_content_extractors()?
            .into_iter()
            .find(|extractor| extractor.id == id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let _ = self.log_activity(
            "content_extractor_updated",
            &format!("Updated extractor \"{}\"", updated.name),
        );
        Ok(updated)
    }

    pub fn restore_default_content_extractors(&self) -> Result<()> {
        let conn = self.conn.lock();
        for preset in crate::content_extraction::EXTRACTOR_PRESETS {
            conn.execute(
                "INSERT INTO content_extractors
                    (stable_ref, name, description, engine, input_contract, output_contract,
                     enabled, priority, is_builtin, is_deleted)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, 1, 0)
                 ON CONFLICT(stable_ref) DO UPDATE SET
                    name = excluded.name, description = excluded.description,
                    engine = excluded.engine, input_contract = excluded.input_contract,
                    output_contract = excluded.output_contract, enabled = 1,
                    priority = excluded.priority, is_deleted = 0,
                    updated_at = CURRENT_TIMESTAMP",
                params![
                    preset.stable_ref,
                    preset.name,
                    preset.description,
                    preset.engine,
                    preset.input_contract,
                    preset.output_contract,
                    preset.priority
                ],
            )?;
        }
        drop(conn);
        let _ = self.log_activity(
            "content_extractors_restored",
            "Restored shipped extractor defaults",
        );
        Ok(())
    }

    pub fn active_image_text_extractor(
        &self,
    ) -> Result<Option<crate::content_extraction::Extractor>> {
        Ok(self
            .get_content_extractors()?
            .into_iter()
            .find(|extractor| {
                extractor.enabled
                    && extractor.is_available
                    && extractor.supports_contract(
                        crate::analysis_contract::RepresentationKind::ImageBytes,
                        crate::analysis_contract::RepresentationKind::SearchableText,
                    )
            }))
    }

    pub fn get_content_detectors(&self) -> Result<Vec<crate::content_detection::Detector>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, stable_ref, name, content_type, description, patterns_json,
                    validator, enabled, priority, is_builtin, is_deleted
             FROM content_detectors WHERE is_deleted = 0 ORDER BY priority, id",
        )?;
        let rows = statement.query_map([], content_detector_from_row)?;
        rows.collect()
    }

    pub fn get_content_detector(
        &self,
        reference: &str,
    ) -> Result<crate::content_detection::Detector> {
        let numeric_id = reference.parse::<i64>().ok();
        self.get_content_detectors()?
            .into_iter()
            .find(|detector| numeric_id == Some(detector.id) || detector.stable_ref == reference)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn duplicate_content_detector(
        &self,
        reference: &str,
        name: Option<&str>,
    ) -> Result<crate::content_detection::Detector> {
        let source = self.get_content_detector(reference)?;
        self.create_content_detector(&crate::content_detection::DetectorInput {
            name: name
                .map(str::to_string)
                .unwrap_or_else(|| format!("{} Copy", source.name)),
            content_type: source.content_type,
            description: source.description,
            patterns: source.patterns,
            validator: source.validator,
            enabled: source.enabled,
            priority: source.priority.saturating_add(1).min(10_000),
        })
    }

    pub fn apply_content_detector(
        &self,
        clip_id: i64,
        reference: &str,
    ) -> Result<crate::detection_execution::DetectionApplicationResult> {
        let mut conn = self.conn.lock();
        let transaction = conn.transaction()?;
        let no_analyzable_text = || {
            rusqlite::Error::InvalidParameterName("The selected clip has no analyzable text".into())
        };
        let numeric_id = reference.parse::<i64>().ok();
        let detector = transaction.query_row(
            "SELECT id, stable_ref, name, content_type, description, patterns_json,
                    validator, enabled, priority, is_builtin, is_deleted
             FROM content_detectors
             WHERE is_deleted = 0 AND (stable_ref = ?1 OR id = ?2)
             LIMIT 1",
            params![reference, numeric_id],
            content_detector_from_row,
        )?;
        let clip = transaction
            .query_row(
                "SELECT content_type, text_content FROM clips
                 WHERE id = ?1 AND COALESCE(is_trashed, 0) = 0",
                params![clip_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()?;
        let Some((current_type, Some(text))) = clip else {
            return Err(no_analyzable_text());
        };
        if text.trim().is_empty() {
            return Err(no_analyzable_text());
        }
        if matches!(current_type.as_str(), "image" | "file") {
            return Err(rusqlite::Error::InvalidParameterName(
                "Applying a Detector cannot replace a structural image or file type".into(),
            ));
        }
        let analysis = crate::detection_execution::analyze_detector(&text, &detector);
        if !analysis.matched {
            transaction.commit()?;
            return Ok(crate::detection_execution::DetectionApplicationResult::preview(analysis));
        }
        let detected_type = analysis.classification();
        let changed = current_type != detected_type;
        if changed {
            transaction.execute(
                "UPDATE clips SET content_type = ?1 WHERE id = ?2",
                params![detected_type, clip_id],
            )?;
        }
        transaction.commit()?;
        drop(conn);
        if changed {
            let _ = self.log_activity(
                "content_detector_applied",
                &format!("Applied a Detector to clip #{clip_id}"),
            );
        }
        Ok(crate::detection_execution::DetectionApplicationResult::applied(analysis, clip_id))
    }

    fn get_all_content_detectors_for_backup(
        &self,
    ) -> Result<Vec<crate::content_detection::Detector>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, stable_ref, name, content_type, description, patterns_json,
                    validator, enabled, priority, is_builtin, is_deleted
             FROM content_detectors ORDER BY priority, id",
        )?;
        let rows = statement.query_map([], |row| {
            let patterns_json: String = row.get(5)?;
            let patterns = serde_json::from_str(&patterns_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            let stable_ref: String = row.get(1)?;
            let is_builtin: bool = row.get(9)?;
            Ok(crate::content_detection::Detector {
                id: row.get(0)?,
                defaults: is_builtin
                    .then(|| crate::content_detection::detector_defaults(&stable_ref))
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
        })?;
        rows.collect()
    }

    pub fn create_content_detector(
        &self,
        input: &crate::content_detection::DetectorInput,
    ) -> Result<crate::content_detection::Detector> {
        crate::content_detection::validate_detector_input(input).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                error,
            )))
        })?;
        if !self
            .get_content_types(false)?
            .iter()
            .any(|content_type| content_type.id == input.content_type)
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "Detectors must use an active registered content type".into(),
            ));
        }
        let patterns_json = serde_json::to_string(&input.patterns)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let conn = self.conn.lock();
        let detector_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM content_detectors WHERE is_deleted = 0",
            [],
            |row| row.get(0),
        )?;
        if detector_count >= 128 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Content detectors are limited to 128 entries".to_string(),
            ));
        }
        conn.execute(
            "INSERT INTO content_detectors
                (stable_ref, name, content_type, description, patterns_json, validator, enabled, priority, is_builtin)
             VALUES ('pending', ?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
            params![input.name.trim(), input.content_type.trim(), input.description.trim(), patterns_json, input.validator, input.enabled, input.priority],
        )?;
        let id = conn.last_insert_rowid();
        conn.execute(
            "UPDATE content_detectors SET stable_ref = ?1 WHERE id = ?2",
            params![format!("custom-{id}"), id],
        )?;
        drop(conn);
        let detector = self
            .get_content_detectors()?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let _ = self.log_activity(
            "content_detector_created",
            &format!("Created detector \"{}\"", detector.name),
        );
        Ok(detector)
    }

    pub fn update_content_detector(
        &self,
        id: i64,
        input: &crate::content_detection::DetectorInput,
    ) -> Result<crate::content_detection::Detector> {
        crate::content_detection::validate_detector_input(input).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                error,
            )))
        })?;
        if !self
            .get_content_types(false)?
            .iter()
            .any(|content_type| content_type.id == input.content_type)
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "Detectors must use an active registered content type".into(),
            ));
        }
        let patterns_json = serde_json::to_string(&input.patterns)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE content_detectors SET name = ?1, content_type = ?2, description = ?3,
                    patterns_json = ?4, validator = ?5, enabled = ?6, priority = ?7,
                    updated_at = CURRENT_TIMESTAMP
             WHERE id = ?8 AND is_deleted = 0",
            params![
                input.name.trim(),
                input.content_type.trim(),
                input.description.trim(),
                patterns_json,
                input.validator,
                input.enabled,
                input.priority,
                id
            ],
        )?;
        drop(conn);
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let detector = self
            .get_content_detectors()?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let _ = self.log_activity(
            "content_detector_updated",
            &format!("Updated detector \"{}\"", detector.name),
        );
        Ok(detector)
    }

    pub fn delete_content_detector(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        let name = conn
            .query_row(
                "SELECT name FROM content_detectors WHERE id = ?1 AND is_deleted = 0",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if name.is_none() {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        conn.execute(
            "UPDATE content_detectors SET is_deleted = 1, enabled = 0, updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            params![id],
        )?;
        drop(conn);
        let name = name.expect("checked above");
        let _ = self.log_activity(
            "content_detector_deleted",
            &format!("Deleted detector \"{name}\""),
        );
        Ok(())
    }

    pub fn restore_default_content_detectors(&self) -> Result<()> {
        let conn = self.conn.lock();
        for preset in crate::content_detection::DETECTOR_PRESETS {
            let patterns_json = serde_json::to_string(&preset.patterns)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            conn.execute(
                "UPDATE content_detectors SET name = ?1, content_type = ?2, description = ?3,
                        patterns_json = ?4, validator = ?5, enabled = 1, priority = ?6,
                        is_deleted = 0, updated_at = CURRENT_TIMESTAMP WHERE stable_ref = ?7",
                params![
                    preset.name,
                    preset.content_type,
                    preset.description,
                    patterns_json,
                    preset.validator,
                    preset.priority,
                    preset.stable_ref
                ],
            )?;
        }
        drop(conn);
        let _ = self.log_activity(
            "content_detectors_restored",
            "Restored shipped detector defaults",
        );
        Ok(())
    }

    /// Reclassify existing text-backed clips with the current enabled detector order.
    /// Image and file records are intentionally excluded because their types describe
    /// their storage representation rather than detected text semantics.
    pub fn rescan_content_detection(&self) -> Result<ContentDetectionRescanReport> {
        const BATCH_SIZE: i64 = 128;

        let detectors = self.get_content_detectors()?;
        let mut conn = self.conn.lock();
        let transaction = conn.transaction()?;
        let mut last_id = 0i64;
        let mut scanned_count = 0usize;
        let mut changed_count = 0usize;
        let mut failed_count = 0usize;

        loop {
            let clips = {
                let mut statement = transaction.prepare(
                    "SELECT id, content_type, text_content
                     FROM clips
                     WHERE id > ?1 AND text_content IS NOT NULL
                       AND content_type NOT IN ('image', 'file')
                     ORDER BY id ASC
                     LIMIT ?2",
                )?;
                let rows = statement
                    .query_map(params![last_id, BATCH_SIZE], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    })?
                    .collect::<Result<Vec<_>>>()?;
                rows
            };
            if clips.is_empty() {
                break;
            }
            for (id, current_type, text) in clips {
                last_id = id;
                scanned_count += 1;
                if text.trim().is_empty() {
                    failed_count += 1;
                    continue;
                }
                let analysis = crate::detection_execution::analyze_detectors_with_policy(
                    &text,
                    &detectors,
                    crate::analysis_contract::AnalysisPolicy::Rescan,
                    None,
                );
                if analysis.failed() {
                    failed_count += 1;
                    continue;
                }
                let detected_type = analysis.classification();
                if detected_type != current_type {
                    transaction.execute(
                        "UPDATE clips SET content_type = ?1 WHERE id = ?2",
                        params![detected_type, id],
                    )?;
                    changed_count += 1;
                }
            }
        }
        transaction.commit()?;
        drop(conn);

        let report = ContentDetectionRescanReport {
            scanned_count,
            changed_count,
            unchanged_count: scanned_count
                .saturating_sub(changed_count)
                .saturating_sub(failed_count),
            failed_count,
        };
        let _ = self.log_activity(
            "content_detection_history_rescanned",
            &format!(
                "Rescanned {} text clips; reclassified {}; failed {}",
                report.scanned_count, report.changed_count, report.failed_count
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
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn setup_test_db() -> DbState {
        let temp_dir = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_file = temp_dir.join(format!(
            "pasted_test_{}_{:?}.db",
            nanos,
            std::thread::current().id()
        ));
        DbState::new(db_file).expect("Failed to create test DB")
    }

    #[test]
    #[ignore = "run explicitly against a disposable copy of a real Pasted database"]
    fn real_database_library_item_migration_smoke_test() {
        let path = std::env::var("PASTED_MIGRATION_TEST_DB")
            .expect("PASTED_MIGRATION_TEST_DB must point to a disposable database copy");
        let db = DbState::new(PathBuf::from(path)).unwrap();
        let items = db.get_library_items(None, true).unwrap();
        assert!(items.iter().any(|item| item.item.kind == "detector"));
        assert!(items.iter().any(|item| item.item.kind == "extractor"));
        assert!(items.iter().any(|item| item.item.kind == "operation"));
    }

    #[test]
    fn content_extractors_are_versioned_available_and_restorable() {
        let db = setup_test_db();
        let extractors = db.get_content_extractors().unwrap();
        assert_eq!(
            extractors.len(),
            crate::content_extraction::EXTRACTOR_PRESETS.len()
        );
        let apple = extractors
            .iter()
            .find(|extractor| {
                extractor.stable_ref == crate::content_extraction::APPLE_VISION_OCR_REF
            })
            .unwrap();
        assert_eq!(apple.input_contract, "image");
        assert_eq!(apple.output_contract, "searchable_text");
        assert_eq!(apple.is_available, cfg!(target_os = "macos"));
        assert_eq!(
            apple.unavailable_reason.is_some(),
            !cfg!(target_os = "macos")
        );
        let tesseract = extractors
            .iter()
            .find(|extractor| extractor.stable_ref == crate::content_extraction::TESSERACT_OCR_REF)
            .unwrap();
        assert_eq!(
            tesseract.engine,
            crate::content_extraction::TESSERACT_ENGINE
        );
        assert_eq!(tesseract.input_contract, "image");
        assert_eq!(tesseract.output_contract, "searchable_text");
        assert_eq!(
            tesseract.is_available,
            crate::content_extraction::engine_availability(
                crate::content_extraction::TESSERACT_ENGINE
            )
            .is_available
        );

        db.update_content_extractor(
            apple.id,
            &crate::content_extraction::ExtractorInput {
                name: "Local Image Text".into(),
                description: "Customized label".into(),
                enabled: false,
                priority: 42,
            },
        )
        .unwrap();
        let updated = db.get_content_extractor(&apple.stable_ref).unwrap();
        assert_eq!(updated.name, "Local Image Text");
        assert!(!updated.enabled);
        let active = db.active_image_text_extractor().unwrap();
        if tesseract.is_available {
            assert_eq!(
                active
                    .as_ref()
                    .map(|extractor| extractor.stable_ref.as_str()),
                Some(crate::content_extraction::TESSERACT_OCR_REF)
            );
        } else {
            assert!(active.is_none());
        }
        assert!(db
            .get_library_items(Some("extractor"), false)
            .unwrap()
            .iter()
            .any(|item| {
                item.item.stable_ref == apple.stable_ref
                    && item.item.enabled == Some(false)
                    && item.analysis_pass.as_deref() == Some("extract")
            }));

        let custom = db
            .create_content_extractor(&crate::content_extraction::ExtractorDefinitionInput {
                name: "Project OCR".into(),
                description: "Extracts project screenshots".into(),
                engine: crate::content_extraction::APPLE_VISION_ENGINE.into(),
                input_contract: "image".into(),
                output_contract: "searchable_text".into(),
                enabled: true,
                priority: 80,
            })
            .unwrap();
        assert!(!custom.is_builtin);
        assert_eq!(
            db.get_content_extractor(&custom.stable_ref).unwrap().id,
            custom.id
        );
        let duplicate = db
            .duplicate_content_extractor(&custom.stable_ref, Some("Project OCR Copy"))
            .unwrap();
        assert_eq!(duplicate.priority, 81);
        db.update_content_extractor_definition(
            duplicate.id,
            &crate::content_extraction::ExtractorDefinitionInput {
                name: "Project OCR Revised".into(),
                description: duplicate.description.clone(),
                engine: duplicate.engine.clone(),
                input_contract: duplicate.input_contract.clone(),
                output_contract: duplicate.output_contract.clone(),
                enabled: false,
                priority: duplicate.priority,
            },
        )
        .unwrap();
        db.delete_content_extractor(custom.id).unwrap();
        assert!(db.get_content_extractor(&custom.stable_ref).is_err());

        db.restore_default_content_extractors().unwrap();
        let restored_extractors = db.get_content_extractors().unwrap();
        let restored = restored_extractors
            .iter()
            .find(|extractor| extractor.stable_ref == apple.stable_ref)
            .unwrap();
        assert_eq!(restored.name, "Apple Vision OCR");
        assert!(restored.enabled);
        assert_eq!(restored.priority, 10);
        assert!(restored_extractors.iter().any(|extractor| {
            extractor.stable_ref == duplicate.stable_ref
                && extractor.name == "Project OCR Revised"
                && !extractor.enabled
        }));
    }

    #[test]
    fn derived_analysis_classification_is_hash_safe_and_non_destructive() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "image",
                None,
                None,
                Some(crate::resource_limits::TEST_PNG_DATA_URL),
                "analysis-image-hash",
                "Screenshot",
            )
            .unwrap();

        assert!(db
            .record_analysis_classification(
                clip.id,
                &clip.content_hash,
                Some("email"),
                Some("email"),
                "searchable_text",
            )
            .unwrap());
        let classification = db.get_analysis_classification(clip.id).unwrap().unwrap();
        assert_eq!(classification.content_type, "email");
        assert_eq!(classification.source_representation, "searchable_text");
        assert_eq!(db.get_clip_by_id(clip.id).unwrap().content_type, "image");

        assert!(!db
            .record_analysis_classification(
                clip.id,
                "stale-hash",
                Some("credential"),
                Some("credential"),
                "searchable_text",
            )
            .unwrap());
        assert_eq!(
            db.get_analysis_classification(clip.id)
                .unwrap()
                .unwrap()
                .content_type,
            "email"
        );

        db.record_analysis_classification(
            clip.id,
            &clip.content_hash,
            Some("text"),
            None,
            "searchable_text",
        )
        .unwrap();
        assert!(db.get_analysis_classification(clip.id).unwrap().is_none());
    }

    #[test]
    fn content_detectors_are_editable_deletable_restorable_and_backed_up() {
        let source = setup_test_db();
        let shipped = source.get_content_detectors().unwrap();
        assert_eq!(
            shipped.len(),
            crate::content_detection::DETECTOR_PRESETS.len()
        );

        let email = shipped
            .iter()
            .find(|detector| detector.stable_ref == "email")
            .unwrap();
        assert_eq!(
            email
                .defaults
                .as_ref()
                .map(|defaults| defaults.name.as_str()),
            Some("Email Addresses")
        );
        let custom_pattern = r"(?i)^[a-z0-9._%+-]+@example\.test$".to_string();
        source
            .update_content_detector(
                email.id,
                &crate::content_detection::DetectorInput {
                    name: "Example Mail".into(),
                    content_type: "email".into(),
                    description: "Project-specific addresses".into(),
                    patterns: vec![custom_pattern.clone()],
                    validator: None,
                    enabled: true,
                    priority: 7,
                },
            )
            .unwrap();
        source
            .create_content_type(&crate::content_types::ContentTypeInput {
                id: "ticket_id".into(),
                label: "Ticket ID".into(),
                icon: "Hash".into(),
                group: "custom".into(),
            })
            .unwrap();
        let custom = source
            .create_content_detector(&crate::content_detection::DetectorInput {
                name: "Ticket IDs".into(),
                content_type: "ticket_id".into(),
                description: "Internal issue identifiers".into(),
                patterns: vec![r"^PASTE-[0-9]+$".into()],
                validator: None,
                enabled: true,
                priority: 8,
            })
            .unwrap();
        assert!(custom.defaults.is_none());
        source.delete_content_detector(custom.id).unwrap();

        let backup = source.export_backup_json().unwrap();
        let destination = setup_test_db();
        destination.import_backup_json(&backup).unwrap();
        let restored = destination.get_content_detectors().unwrap();
        let restored_email = restored
            .iter()
            .find(|detector| detector.stable_ref == "email")
            .unwrap();
        assert_eq!(restored_email.name, "Example Mail");
        assert_eq!(restored_email.patterns, vec![custom_pattern]);
        assert!(!restored
            .iter()
            .any(|detector| detector.stable_ref == custom.stable_ref));

        destination.restore_default_content_detectors().unwrap();
        let defaults = destination.get_content_detectors().unwrap();
        assert_eq!(
            defaults
                .iter()
                .find(|detector| detector.stable_ref == "email")
                .unwrap()
                .name,
            "Email Addresses"
        );
        assert!(!defaults
            .iter()
            .any(|detector| detector.stable_ref == custom.stable_ref));
    }

    #[test]
    fn a_single_detector_can_be_resolved_duplicated_and_applied() {
        let db = setup_test_db();
        let email = db.get_content_detector("email").unwrap();
        assert_eq!(
            db.get_content_detector(&email.id.to_string()).unwrap().id,
            email.id
        );
        let duplicate = db
            .duplicate_content_detector("email", Some("Email Copy"))
            .unwrap();
        assert_eq!(duplicate.name, "Email Copy");
        assert!(!duplicate.is_builtin);

        let matching = db
            .save_clip(
                "text",
                Some("person@example.com"),
                None,
                None,
                "detector-apply-match",
                "Test",
            )
            .unwrap();
        let applied = db.apply_content_detector(matching.id, "email").unwrap();
        assert!(applied.analysis.matched);
        assert_eq!(applied.application.applied_clip_id, Some(matching.id));
        assert_eq!(
            db.get_clip_by_id(matching.id).unwrap().content_type,
            "email"
        );

        let nonmatching = db
            .save_clip(
                "text",
                Some("plain prose"),
                None,
                None,
                "detector-apply-no-match",
                "Test",
            )
            .unwrap();
        let not_applied = db.apply_content_detector(nonmatching.id, "email").unwrap();
        assert!(!not_applied.analysis.matched);
        assert_eq!(not_applied.application.applied_clip_id, None);
        assert_eq!(
            db.get_clip_by_id(nonmatching.id).unwrap().content_type,
            "text"
        );

        let empty = db
            .save_clip("text", Some(""), None, None, "detector-apply-empty", "Test")
            .unwrap();
        assert!(db
            .apply_content_detector(empty.id, "email")
            .unwrap_err()
            .to_string()
            .contains("no analyzable text"));
        let whitespace = db
            .save_clip(
                "text",
                Some(" \n\t"),
                None,
                None,
                "detector-apply-whitespace",
                "Test",
            )
            .unwrap();
        assert!(db
            .apply_content_detector(whitespace.id, "email")
            .unwrap_err()
            .to_string()
            .contains("no analyzable text"));

        db.delete_content_detector(duplicate.id).unwrap();
        assert!(db.get_content_detector(&duplicate.stable_ref).is_err());
    }

    #[test]
    fn shared_text_capture_hashes_deduplicates_and_classifies() {
        let db = setup_test_db();
        let first = db
            .save_text_clip("person@example.com", "CLI Terminal")
            .unwrap();
        assert_eq!(first.content_type, "email");
        assert_eq!(first.source, "CLI Terminal");
        assert!(!first.content_hash.is_empty());
        let structure = db
            .get_structural_inspection(
                first.id,
                &crate::inspection_execution::inspection_input_hash(&first),
            )
            .unwrap()
            .expect("capture should persist its Analyzer structure");
        assert_eq!(structure.text.unwrap().word_count, 1);

        let duplicate = db
            .save_text_clip("person@example.com", "CLI Terminal")
            .unwrap();
        assert_eq!(duplicate.id, first.id);
        assert_eq!(db.get_clips(None, None, false).unwrap().len(), 1);
    }

    #[test]
    fn duplicate_text_capture_inspects_using_the_stored_source() {
        let db = setup_test_db();
        let first = db.save_text_clip("person@example.com", "Safari").unwrap();
        let duplicate = db
            .save_text_clip("person@example.com", "CLI Terminal")
            .unwrap();
        assert_eq!(duplicate.id, first.id);
        assert_eq!(duplicate.source, "Safari");

        let structure = db
            .get_structural_inspection(
                duplicate.id,
                &crate::inspection_execution::inspection_input_hash(&duplicate),
            )
            .unwrap()
            .expect("duplicate capture should persist structure for the stored clip");
        assert_eq!(
            structure.origin,
            crate::content_inspection::OriginKind::ClipboardContent
        );
    }

    #[test]
    fn text_capture_still_inspects_when_content_detection_is_disabled() {
        let db = setup_test_db();
        db.save_settings(&std::collections::HashMap::from([(
            crate::features::Feature::ContentDetection
                .setting_key()
                .to_string(),
            "false".to_string(),
        )]))
        .unwrap();
        let clip = db
            .save_text_clip("person@example.com", "CLI Terminal")
            .unwrap();
        assert_eq!(clip.content_type, "text");
        assert!(db
            .get_structural_inspection(
                clip.id,
                &crate::inspection_execution::inspection_input_hash(&clip),
            )
            .unwrap()
            .is_some());
    }

    #[test]
    fn content_type_registry_protects_builtin_ids_and_archives_custom_types_safely() {
        let db = setup_test_db();
        let mut payment = db
            .get_content_types(false)
            .unwrap()
            .into_iter()
            .find(|item| item.id == "payment_card")
            .unwrap();
        assert_eq!(
            payment
                .defaults
                .as_ref()
                .map(|defaults| defaults.label.as_str()),
            Some("Payment Card")
        );
        payment.label = "Cards".into();
        payment.icon = "ShieldKeyhole".into();
        db.update_content_type(
            "payment_card",
            &crate::content_types::ContentTypeInput {
                id: payment.id.clone(),
                label: payment.label.clone(),
                icon: payment.icon.clone(),
                group: payment.group.clone(),
            },
        )
        .unwrap();
        assert!(db.set_content_type_archived("payment_card", true).is_err());

        let custom_type = db
            .create_content_type(&crate::content_types::ContentTypeInput {
                id: "ticket_id".into(),
                label: "Ticket ID".into(),
                icon: "Hash".into(),
                group: "custom".into(),
            })
            .unwrap();
        assert!(custom_type.defaults.is_none());
        let detector = db
            .create_content_detector(&crate::content_detection::DetectorInput {
                name: "Tickets".into(),
                content_type: "ticket_id".into(),
                description: String::new(),
                patterns: vec![r"^T-[0-9]+$".into()],
                validator: None,
                enabled: true,
                priority: 5,
            })
            .unwrap();
        db.set_content_type_archived("ticket_id", true).unwrap();
        assert!(db
            .get_content_types(false)
            .unwrap()
            .iter()
            .all(|item| item.id != "ticket_id"));
        assert!(
            !db.get_content_detectors()
                .unwrap()
                .into_iter()
                .find(|item| item.id == detector.id)
                .unwrap()
                .enabled
        );

        db.restore_default_content_types().unwrap();
        assert_eq!(
            db.get_content_types(false)
                .unwrap()
                .into_iter()
                .find(|item| item.id == "payment_card")
                .unwrap()
                .label,
            "Payment Card"
        );
    }

    #[test]
    fn content_type_groups_are_editable_but_cannot_be_archived_while_in_use() {
        let db = setup_test_db();
        let general = db
            .get_content_type_groups(false)
            .unwrap()
            .into_iter()
            .find(|group| group.id == "general")
            .unwrap();
        assert_eq!(
            general
                .defaults
                .as_ref()
                .map(|defaults| defaults.label.as_str()),
            Some("General")
        );
        let custom_group = db
            .create_content_type_group(&crate::content_types::ContentTypeGroupInput {
                id: "work".into(),
                label: "Work".into(),
                sort_order: 15,
            })
            .unwrap();
        assert!(custom_group.defaults.is_none());
        db.create_content_type(&crate::content_types::ContentTypeInput {
            id: "ticket".into(),
            label: "Ticket".into(),
            icon: "Tag".into(),
            group: "work".into(),
        })
        .unwrap();
        assert!(db.set_content_type_group_archived("work", true).is_err());
        db.update_content_type(
            "ticket",
            &crate::content_types::ContentTypeInput {
                id: "ticket".into(),
                label: "Ticket".into(),
                icon: "Tag".into(),
                group: "custom".into(),
            },
        )
        .unwrap();
        db.set_content_type_group_archived("work", true).unwrap();
        assert!(db
            .get_content_type_groups(false)
            .unwrap()
            .iter()
            .all(|group| group.id != "work"));
        assert!(db.set_content_type_group_archived("general", true).is_err());
        let destination = setup_test_db();
        destination
            .import_backup_json(&db.export_backup_json().unwrap())
            .unwrap();
        assert!(destination
            .get_content_type_groups(true)
            .unwrap()
            .iter()
            .any(|group| group.id == "work" && group.is_archived));
        db.delete_content_type_group("work").unwrap();
        assert!(db
            .get_content_type_groups(true)
            .unwrap()
            .iter()
            .all(|group| group.id != "work"));
        assert!(db.delete_content_type_group("general").is_err());
    }

    #[test]
    fn content_detection_rescan_reclassifies_text_but_preserves_structural_types() {
        let db = setup_test_db();
        let card = db
            .save_clip(
                "text",
                Some("4242-4242-4242-4242"),
                None,
                None,
                "card-hash",
                "Test",
            )
            .unwrap();
        let image = db
            .save_clip(
                "image",
                Some("4242-4242-4242-4242"),
                None,
                Some(crate::resource_limits::TEST_PNG_DATA_URL),
                "image-hash",
                "Test",
            )
            .unwrap();
        let empty = db
            .save_clip("code", Some(""), None, None, "empty-hash", "Test")
            .unwrap();
        let whitespace = db
            .save_clip("code", Some(" \n\t"), None, None, "whitespace-hash", "Test")
            .unwrap();

        let report = db.rescan_content_detection().unwrap();
        assert_eq!(report.scanned_count, 3);
        assert_eq!(report.changed_count, 1);
        assert_eq!(report.unchanged_count, 0);
        assert_eq!(report.failed_count, 2);
        assert_eq!(
            db.get_clip_by_id(card.id).unwrap().content_type,
            "payment_card"
        );
        assert_eq!(db.get_clip_by_id(image.id).unwrap().content_type, "image");
        assert_eq!(db.get_clip_by_id(empty.id).unwrap().content_type, "code");
        assert_eq!(
            db.get_clip_by_id(whitespace.id).unwrap().content_type,
            "code"
        );
    }

    #[test]
    fn legacy_source_app_column_migrates_without_losing_filters_or_search() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("pasted_source_migration_{nanos}.db"));
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE clips (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    content_type TEXT NOT NULL,
                    text_content TEXT,
                    html_content TEXT,
                    image_base64 TEXT,
                    content_hash TEXT UNIQUE NOT NULL,
                    source_app TEXT DEFAULT 'Unknown',
                    is_pinned INTEGER DEFAULT 0,
                    bin_id INTEGER,
                    note TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                 );
                 CREATE TABLE bins (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    icon TEXT DEFAULT 'Folder',
                    color TEXT DEFAULT 'default',
                    smart_rule TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                 );
                 INSERT INTO clips
                    (content_type, text_content, content_hash, source_app)
                 VALUES ('text', 'migration-search-token', 'legacy-source-hash', 'Safari');
                 INSERT INTO bins (name, smart_rule)
                 VALUES ('Safari', '{\"type\":\"source_app\",\"value\":\"Safari\"}');",
            )
            .unwrap();
        drop(connection);

        let db = DbState::new(path).unwrap();
        let conn = db.conn.lock();
        assert!(column_exists(&conn, "clips", "source").unwrap());
        assert!(!column_exists(&conn, "clips", "source_app").unwrap());
        let migrated_rule: String = conn
            .query_row(
                "SELECT smart_rule FROM bins WHERE name = 'Safari'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(migrated_rule, r#"{"type":"source","value":"Safari"}"#);
        drop(conn);

        let clips = db
            .get_clips(Some("migration-search-token"), None, false)
            .unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].source, "Safari");
        assert_eq!(db.get_clips(None, Some(1), false).unwrap().len(), 1);

        let backup = db.export_backup_json().unwrap();
        assert!(backup.contains("\"source\": \"Safari\""));
        assert!(!backup.contains("\"source_app\""));

        let mut legacy_backup: serde_json::Value = serde_json::from_str(&backup).unwrap();
        for clip in legacy_backup["clips"].as_array_mut().unwrap() {
            let object = clip.as_object_mut().unwrap();
            let source = object.remove("source").unwrap();
            object.insert("source_app".to_string(), source);
        }
        let destination = setup_test_db();
        destination
            .import_backup_json(&serde_json::to_string(&legacy_backup).unwrap())
            .unwrap();
        assert!(destination
            .get_clips(None, None, false)
            .unwrap()
            .iter()
            .any(|clip| clip.source == "Safari"));
    }

    #[test]
    fn legacy_detection_preferences_migrate_once_into_detector_records() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("pasted_detector_migration_{nanos}.db"));
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO settings (key, value) VALUES ('detectColors', 'false');",
            )
            .unwrap();
        drop(connection);

        let db = DbState::new(path).unwrap();
        let detectors = db.get_content_detectors().unwrap();
        assert!(
            !detectors
                .iter()
                .find(|detector| detector.stable_ref == "color")
                .unwrap()
                .enabled
        );
        assert!(
            detectors
                .iter()
                .find(|detector| detector.stable_ref == "url")
                .unwrap()
                .enabled
        );

        let color = detectors
            .iter()
            .find(|detector| detector.stable_ref == "color")
            .unwrap();
        db.update_content_detector(
            color.id,
            &crate::content_detection::DetectorInput {
                name: color.name.clone(),
                content_type: color.content_type.clone(),
                description: color.description.clone(),
                patterns: color.patterns.clone(),
                validator: color.validator.clone(),
                enabled: true,
                priority: color.priority,
            },
        )
        .unwrap();
        let reopened = DbState::new(db.database_path()).unwrap();
        assert!(
            reopened
                .get_content_detectors()
                .unwrap()
                .iter()
                .find(|detector| detector.stable_ref == "color")
                .unwrap()
                .enabled
        );
    }

    #[test]
    fn relocating_database_preserves_data_and_retains_the_source() {
        let db = setup_test_db();
        let source = db.database_path();
        let destination_directory = std::env::temp_dir().join(format!(
            "pasted_relocation_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&destination_directory).unwrap();
        let destination = destination_directory.join("pasted.db");
        db.save_clip(
            "text",
            Some("Move me without losing me"),
            None,
            None,
            "relocation-test-hash",
            "Test",
        )
        .unwrap();

        let retained = db.relocate_database(destination.clone()).unwrap();

        assert_eq!(retained, source);
        assert_eq!(db.database_path(), destination);
        assert!(retained.is_file());
        assert_eq!(
            db.get_clips(None, None, false).unwrap()[0]
                .text_content
                .as_deref(),
            Some("Move me without losing me")
        );
        let reopened = DbState::new(db.database_path()).unwrap();
        assert_eq!(reopened.get_clips(None, None, false).unwrap().len(), 1);
        let _ = fs::remove_file(retained);
        let _ = fs::remove_dir_all(destination_directory);
    }

    #[test]
    fn relocating_database_never_overwrites_an_existing_target() {
        let db = setup_test_db();
        let destination_directory = std::env::temp_dir().join(format!(
            "pasted_relocation_existing_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&destination_directory).unwrap();
        let destination = destination_directory.join("pasted.db");
        fs::write(&destination, b"keep this file").unwrap();

        assert!(db.relocate_database(destination.clone()).is_err());
        assert_eq!(fs::read(&destination).unwrap(), b"keep this file");
        assert_ne!(db.database_path(), destination);
        let _ = fs::remove_file(db.database_path());
        let _ = fs::remove_dir_all(destination_directory);
    }

    #[test]
    fn factory_reset_removes_user_state_and_restores_first_launch_defaults() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "text",
                Some("Reset me completely"),
                None,
                None,
                "factory-reset-clip",
                "Test",
            )
            .unwrap();
        db.update_clip_note(clip.id, Some("A note to remove"))
            .unwrap();
        db.create_bin_with_type("Personal", "Folder", "default", None, "category")
            .unwrap();
        db.create_content_type(&crate::content_types::ContentTypeInput {
            id: "reset_custom".into(),
            label: "Reset Custom".into(),
            icon: "FileText".into(),
            group: "custom".into(),
        })
        .unwrap();
        db.save_setting("themeMode", "vampire").unwrap();
        {
            let conn = db.conn.lock();
            conn.execute(
                "INSERT INTO activity_logs (event_type, description) VALUES ('test', 'remove me')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO intelligence_connections (id, name, provider_kind) VALUES ('reset-connection', 'Reset', 'cli')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO custom_operations (id, name, executor_kind) VALUES ('reset-operation', 'Reset', 'regex')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO saved_transforms
                    (id, name, plan_json, connection_id, authoring_kind)
                 VALUES
                    ('reset-transform', 'Reset', '{\"steps\":[]}', 'reset-connection', 'intent'),
                    ('reset-manual-transform', 'Reset Manual', '{\"steps\":[]}', NULL, 'manual')",
                [],
            )
            .unwrap();
        }

        let report = db.factory_reset().unwrap();
        assert_eq!(report.clips_deleted, 1);
        assert_eq!(report.bins_deleted, 4);
        assert_eq!(report.transforms_deleted, 3);
        assert_eq!(report.connections_deleted, 1);
        assert_eq!(report.activity_entries_deleted, 3);

        assert!(db.get_clips(None, None, false).unwrap().is_empty());
        assert!(db
            .get_clips(Some("Reset me"), None, false)
            .unwrap()
            .is_empty());
        let default_bins = db.get_bins().unwrap();
        assert_eq!(default_bins.len(), 3);
        assert_eq!(
            default_bins
                .iter()
                .map(|bin| bin.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Screenshots", "Links and web", "Code Snippets"]
        );
        assert_eq!(
            default_bins[0].smart_rule.as_deref(),
            Some("{\"type\":\"origin_kind\",\"value\":\"screenshot\"}")
        );
        assert_eq!(
            default_bins.iter().map(|bin| bin.id).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(db.get_setting("themeMode").unwrap(), None);
        let reset_types = db.get_content_types(true).unwrap();
        assert_eq!(
            reset_types.len(),
            crate::content_types::CONTENT_TYPE_PRESETS.len()
        );
        assert!(!reset_types.iter().any(|item| item.id == "reset_custom"));
        let conn = db.conn.lock();
        for table in [
            "clip_versions",
            "activity_logs",
            "custom_operations",
            "saved_transforms",
            "intelligence_connections",
        ] {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 0, "{table} should be empty after reset");
        }
        drop(conn);
        let reset_registry = db.get_library_items(None, true).unwrap();
        assert!(!reset_registry.iter().any(|item| {
            item.item.stable_ref == "custom:reset-operation"
                || item.item.stable_ref == "transform:reset-manual-transform"
        }));
        assert_eq!(
            reset_registry
                .iter()
                .filter(|item| item.item.kind == "operation" && item.item.is_builtin)
                .count(),
            crate::operation_registry::BUILTIN_OPERATIONS.len()
        );

        let fresh = db
            .save_clip(
                "text",
                Some("Fresh start"),
                None,
                None,
                "factory-reset-fresh",
                "Test",
            )
            .unwrap();
        assert!(fresh.id > 0);
    }

    #[test]
    fn factory_reset_rolls_back_everything_when_a_delete_fails() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "text",
                Some("Do not partially reset me"),
                None,
                None,
                "factory-reset-rollback-clip",
                "Test",
            )
            .unwrap();
        let bin = db
            .create_bin("Keep This Bin", "Folder", "default", None)
            .unwrap();
        db.assign_to_bin(clip.id, Some(bin.id)).unwrap();
        db.save_setting("themeMode", "flux").unwrap();
        {
            let conn = db.conn.lock();
            conn.execute(
                "INSERT INTO activity_logs (event_type, description)
                 VALUES ('test', 'survive a failed reset')",
                [],
            )
            .unwrap();
            conn.execute_batch(
                "CREATE TRIGGER reject_factory_reset_clip_delete
                 BEFORE DELETE ON clips
                 BEGIN
                    SELECT RAISE(ABORT, 'simulated reset failure');
                 END;",
            )
            .unwrap();
        }

        let error = db.factory_reset().unwrap_err();
        assert!(error.to_string().contains("simulated reset failure"));

        let preserved = db.get_clip_by_id(clip.id).unwrap();
        assert_eq!(preserved.bin_id, Some(bin.id));
        assert_eq!(
            db.get_setting("themeMode").unwrap().as_deref(),
            Some("flux")
        );
        assert!(db.get_bins().unwrap().iter().any(|item| item.id == bin.id));
        let conn = db.conn.lock();
        let activity_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM activity_logs", [], |row| row.get(0))
            .unwrap();
        assert!(activity_count > 0);
    }

    #[test]
    fn test_clip_saving_and_retrieval() {
        let db = setup_test_db();
        let clip = db
            .save_clip("text", Some("Hello Rust"), None, None, "hash1", "Safari")
            .unwrap();
        assert!(clip.id > 0);

        let clips = db.get_clips(None, None, false).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].text_content.as_deref(), Some("Hello Rust"));
        assert_eq!(clips[0].source, "Safari");
        assert!(!clips[0].is_pinned);
    }

    #[test]
    fn origin_kind_is_conservative_and_distinguishes_files_and_screenshots() {
        assert_eq!(derived_origin_kind("file", "Finder"), "file_reference");
        assert_eq!(derived_origin_kind("image", "Screenshot"), "screenshot");
        assert_eq!(derived_origin_kind("image", "screencapture"), "screenshot");
        assert_eq!(derived_origin_kind("image", "CleanShot X"), "screenshot");
        assert_eq!(derived_origin_kind("file", "CleanShot X"), "screenshot");
        assert_eq!(derived_origin_kind("image", "Preview"), "clipboard_content");
        assert_eq!(derived_origin_kind("text", "Safari"), "clipboard_content");
        assert_eq!(derived_origin_kind("text", "CLI Terminal"), "command_line");
    }

    #[test]
    fn image_capture_reattribution_is_hash_safe_and_image_only() {
        let db = setup_test_db();
        let image = db
            .save_clip(
                "image",
                None,
                None,
                Some(crate::resource_limits::TEST_PNG_DATA_URL),
                "reattribute-image-hash",
                "Safari",
            )
            .unwrap();
        let file = db
            .save_clip(
                "file",
                Some("[\"/tmp/capture.png\"]"),
                None,
                None,
                "reattribute-file-hash",
                "pasted-app",
            )
            .unwrap();

        assert!(db
            .reattribute_image_capture(image.id, "wrong-hash", "Screenshot")
            .unwrap()
            .is_none());
        assert_eq!(db.get_clip_by_id(image.id).unwrap().source, "Safari");

        let updated = db
            .reattribute_image_capture(image.id, &image.content_hash, "Screenshot")
            .unwrap()
            .unwrap();
        assert_eq!(updated.source, "Screenshot");

        assert!(db
            .reattribute_image_capture(file.id, &file.content_hash, "Screenshot")
            .unwrap()
            .is_none());
        assert_eq!(db.get_clip_by_id(file.id).unwrap().source, "pasted-app");
    }

    #[test]
    fn origin_smart_bins_match_lists_counts_and_transform_automation() {
        let db = setup_test_db();
        let screenshot = db
            .save_clip(
                "image",
                None,
                None,
                Some(crate::resource_limits::TEST_PNG_DATA_URL),
                "origin_screenshot_hash",
                "Screenshot",
            )
            .unwrap();
        let paths = serde_json::json!(["/Users/pasted/Downloads/report.pdf"]).to_string();
        let file = db
            .save_clip(
                "file",
                Some(&paths),
                None,
                None,
                "origin_file_hash",
                "Finder",
            )
            .unwrap();
        let cleanshot_paths =
            serde_json::json!(["/Users/pasted/Desktop/CleanShot 2026-08-07.png"]).to_string();
        let cleanshot_file = db
            .save_clip(
                "file",
                Some(&cleanshot_paths),
                None,
                None,
                "origin_cleanshot_file_hash",
                "CleanShot X",
            )
            .unwrap();
        let clipboard = db
            .save_clip(
                "text",
                Some("ordinary clipboard text"),
                None,
                None,
                "origin_clipboard_hash",
                "Safari",
            )
            .unwrap();

        let screenshot_rule = serde_json::json!({
            "conditions": [{"type": "origin_kind", "operator": "is", "value": "screenshot"}],
            "match": "all"
        })
        .to_string();
        let file_rule = serde_json::json!({
            "conditions": [{"type": "origin_kind", "operator": "is", "value": "file_reference"}],
            "match": "all"
        })
        .to_string();
        let clipboard_rule = serde_json::json!({
            "conditions": [{"type": "origin_kind", "operator": "is", "value": "clipboard_content"}],
            "match": "all"
        })
        .to_string();
        let screenshot_bin = db
            .create_bin("Screenshots", "📸", "default", Some(&screenshot_rule))
            .unwrap();
        let file_bin = db
            .create_bin("File References", "📎", "default", Some(&file_rule))
            .unwrap();
        let clipboard_bin = db
            .create_bin("Clipboard Content", "📋", "default", Some(&clipboard_rule))
            .unwrap();

        let screenshot_clips = db.get_clips(None, Some(screenshot_bin.id), false).unwrap();
        assert_eq!(screenshot_clips.len(), 2);
        assert!(screenshot_clips.iter().any(|clip| clip.id == screenshot.id));
        assert!(screenshot_clips
            .iter()
            .any(|clip| clip.id == cleanshot_file.id));
        assert!(db
            .get_clip_by_id(screenshot.id)
            .unwrap()
            .bin_ids
            .unwrap()
            .contains(&screenshot_bin.id));
        assert!(db
            .assign_to_bin(screenshot.id, Some(screenshot_bin.id))
            .is_err());
        assert_eq!(
            db.get_clips(None, Some(file_bin.id), false).unwrap()[0].id,
            file.id
        );
        assert_eq!(
            db.get_clips(None, Some(clipboard_bin.id), false).unwrap()[0].id,
            clipboard.id
        );
        let bins = db.get_bins().unwrap();
        assert_eq!(
            bins.iter()
                .find(|bin| bin.id == screenshot_bin.id)
                .unwrap()
                .clip_count,
            Some(2)
        );
        for bin_id in [file_bin.id, clipboard_bin.id] {
            assert_eq!(
                bins.iter().find(|bin| bin.id == bin_id).unwrap().clip_count,
                Some(1)
            );
        }

        db.set_bin_transform_ref(screenshot_bin.id, Some("transform:test-origin"))
            .unwrap();
        assert_eq!(
            db.matching_smart_bin_transforms("image", "", "Screenshot")
                .unwrap(),
            vec![(screenshot_bin.id, "transform:test-origin".to_string())]
        );
        assert_eq!(
            db.matching_smart_bin_transforms("file", &cleanshot_paths, "CleanShot X")
                .unwrap(),
            vec![(screenshot_bin.id, "transform:test-origin".to_string())]
        );
        assert!(db
            .matching_smart_bin_transforms("image", "", "Preview")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn file_smart_bins_match_any_selected_path_without_reordering_the_clip() {
        let db = setup_test_db();
        let paths = serde_json::json!([
            "/Users/pasted/Zebra Report.pdf",
            "/Users/pasted/Projects/Alpha Notes.txt"
        ])
        .to_string();
        let clip = db
            .save_clip("file", Some(&paths), None, None, "file_hash", "Finder")
            .unwrap();
        let pdf_rule = serde_json::json!({
            "conditions": [{"type": "file_extension", "operator": "is", "value": "pdf"}],
            "match": "any"
        })
        .to_string();
        let project_rule = serde_json::json!({
            "conditions": [{"type": "file_path", "operator": "contains", "value": "/projects/"}],
            "match": "any"
        })
        .to_string();
        let pdf_bin = db
            .create_bin("PDF Files", "📄", "default", Some(&pdf_rule))
            .unwrap();
        let project_bin = db
            .create_bin("Project Files", "📂", "default", Some(&project_rule))
            .unwrap();

        assert_eq!(
            db.get_clips(None, Some(pdf_bin.id), false).unwrap()[0].id,
            clip.id
        );
        assert_eq!(
            db.get_clips(None, Some(project_bin.id), false).unwrap()[0].id,
            clip.id
        );
        assert_eq!(
            db.get_clip_by_id(clip.id).unwrap().text_content.as_deref(),
            Some(paths.as_str())
        );
        let bins = db.get_bins().unwrap();
        assert_eq!(
            bins.iter()
                .find(|bin| bin.id == pdf_bin.id)
                .unwrap()
                .clip_count,
            Some(1)
        );
    }

    #[test]
    fn clip_lists_defer_image_payloads_to_the_image_endpoint() {
        let db = setup_test_db();
        let image_payload = crate::resource_limits::TEST_PNG_DATA_URL;
        let clip = db
            .save_clip(
                "image",
                None,
                None,
                Some(image_payload),
                "image_hash",
                "Screenshot",
            )
            .unwrap();

        let clips = db.get_clips(None, None, false).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].id, clip.id);
        assert!(clips[0].image_base64.is_none());
        assert_eq!(
            db.get_clip_image(clip.id).unwrap().as_deref(),
            Some(image_payload)
        );
    }

    #[test]
    fn test_protected_clips_immunity() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "text",
                Some("Protected Secret"),
                None,
                None,
                "prot_hash",
                "Keeper",
            )
            .unwrap();

        // Toggle protected
        let is_prot = db.toggle_protected(clip.id).unwrap();
        assert!(is_prot);

        // Attempt delete_clip (should be blocked)
        db.delete_clip(clip.id).unwrap();
        let active = db.get_clips(None, None, false).unwrap();
        assert_eq!(active.len(), 1);
        assert!(active[0].is_protected);

        // Attempt trash_unpinned_clips (should be blocked)
        db.trash_unpinned_clips().unwrap();
        let active_after_trash = db.get_clips(None, None, false).unwrap();
        assert_eq!(active_after_trash.len(), 1);

        // Attempt purge_unpinned_clips (should be blocked)
        db.purge_unpinned_clips().unwrap();
        let active_after_purge = db.get_clips(None, None, false).unwrap();
        assert_eq!(active_after_purge.len(), 1);

        // Every bulk and retention path must preserve protected clips.
        db.clear_history().unwrap();
        db.purge_old_clips(0).unwrap();
        let active_after_clear = db.get_clips(None, None, false).unwrap();
        assert_eq!(active_after_clear.len(), 1);
        assert!(active_after_clear[0].is_protected);

        // Unprotect and verify delete works
        db.toggle_protected(clip.id).unwrap();
        db.delete_clip(clip.id).unwrap();
        assert_eq!(db.get_clips(None, None, false).unwrap().len(), 0);
    }

    #[test]
    fn test_retention_uses_trash_and_excludes_pinned_and_protected_clips() {
        let db = setup_test_db();
        let pinned = db
            .save_clip("text", Some("Pinned"), None, None, "ret-pin", "App")
            .unwrap();
        let protected = db
            .save_clip("text", Some("Protected"), None, None, "ret-prot", "App")
            .unwrap();
        db.toggle_pin(pinned.id).unwrap();
        db.toggle_protected(protected.id).unwrap();

        for index in 0..3 {
            db.save_clip(
                "text",
                Some(&format!("Regular {index}")),
                None,
                None,
                &format!("ret-{index}"),
                "App",
            )
            .unwrap();
        }

        db.purge_old_clips(1).unwrap();

        let active = db.get_clips(None, None, false).unwrap();
        assert_eq!(
            active
                .iter()
                .filter(|clip| !clip.is_pinned && !clip.is_protected)
                .count(),
            1
        );
        assert!(active.iter().any(|clip| clip.id == pinned.id));
        assert!(active.iter().any(|clip| clip.id == protected.id));
        assert_eq!(db.get_trashed_clips().unwrap().len(), 2);
    }

    #[test]
    fn test_retention_without_trash_keeps_requested_unpinned_capacity() {
        let db = setup_test_db();
        db.save_setting("enableTrash", "false").unwrap();
        let pinned = db
            .save_clip("text", Some("Pinned"), None, None, "purge-pin", "App")
            .unwrap();
        db.toggle_pin(pinned.id).unwrap();
        for index in 0..4 {
            db.save_clip(
                "text",
                Some(&format!("Regular {index}")),
                None,
                None,
                &format!("purge-{index}"),
                "App",
            )
            .unwrap();
        }

        db.purge_old_clips(2).unwrap();

        let active = db.get_clips(None, None, false).unwrap();
        assert_eq!(active.iter().filter(|clip| !clip.is_pinned).count(), 2);
        assert!(active.iter().any(|clip| clip.id == pinned.id));
        assert!(db.get_trashed_clips().unwrap().is_empty());
    }

    #[test]
    fn age_retention_uses_trash_and_preserves_pinned_and_protected_clips() {
        let db = setup_test_db();
        let old = db
            .save_clip("text", Some("Old"), None, None, "age-old", "App")
            .unwrap();
        let recent = db
            .save_clip("text", Some("Recent"), None, None, "age-new", "App")
            .unwrap();
        let pinned = db
            .save_clip("text", Some("Pinned"), None, None, "age-pin", "App")
            .unwrap();
        let protected = db
            .save_clip("text", Some("Protected"), None, None, "age-prot", "App")
            .unwrap();
        db.toggle_pin(pinned.id).unwrap();
        db.toggle_protected(protected.id).unwrap();
        {
            let conn = db.conn.lock();
            conn.execute(
                "UPDATE clips SET created_at = datetime('now', '-31 days') WHERE id IN (?1, ?2, ?3)",
                params![old.id, pinned.id, protected.id],
            )
            .unwrap();
        }

        db.configure_clip_retention(0, 30).unwrap();

        let active = db.get_clips(None, None, false).unwrap();
        assert!(!active.iter().any(|clip| clip.id == old.id));
        assert!(active.iter().any(|clip| clip.id == recent.id));
        assert!(active.iter().any(|clip| clip.id == pinned.id));
        assert!(active.iter().any(|clip| clip.id == protected.id));
        assert_eq!(db.get_trashed_clips().unwrap()[0].id, old.id);
    }

    #[test]
    fn unlimited_count_and_forever_age_do_not_remove_clips() {
        let db = setup_test_db();
        let clip = db
            .save_clip("text", Some("Kept"), None, None, "unlimited", "App")
            .unwrap();
        {
            let conn = db.conn.lock();
            conn.execute(
                "UPDATE clips SET created_at = datetime('now', '-100 years') WHERE id = ?1",
                [clip.id],
            )
            .unwrap();
        }

        db.configure_clip_retention(0, 0).unwrap();

        assert_eq!(db.get_clips(None, None, false).unwrap().len(), 1);
        assert!(db.get_trashed_clips().unwrap().is_empty());
    }

    #[test]
    fn history_policy_change_does_not_cascade_into_trash_purging() {
        let db = setup_test_db();
        db.save_setting("trashCapacityCount", "1").unwrap();
        for index in 0..3 {
            db.save_clip(
                "text",
                Some(&format!("Grace {index}")),
                None,
                None,
                &format!("grace-{index}"),
                "App",
            )
            .unwrap();
        }

        db.enforce_clip_retention(1, 0).unwrap();

        assert_eq!(db.get_trashed_clips().unwrap().len(), 2);
        db.enforce_trash_retention(1, 0).unwrap();
        assert_eq!(db.get_trashed_clips().unwrap().len(), 1);
    }

    #[test]
    fn trash_age_retention_purges_old_items_but_preserves_protected_clips() {
        let db = setup_test_db();
        let old = db
            .save_clip("text", Some("Old Trash"), None, None, "trash-age", "App")
            .unwrap();
        let protected = db
            .save_clip(
                "text",
                Some("Protected Trash"),
                None,
                None,
                "trash-protected",
                "App",
            )
            .unwrap();
        let recent = db
            .save_clip(
                "text",
                Some("Recent Trash"),
                None,
                None,
                "trash-recent",
                "App",
            )
            .unwrap();
        db.batch_trash_clips(vec![old.id, protected.id, recent.id])
            .unwrap();
        {
            let conn = db.conn.lock();
            conn.execute(
                "UPDATE clips
                 SET trashed_at = datetime('now', '-31 days'),
                     is_protected = CASE WHEN id = ?2 THEN 1 ELSE 0 END
                 WHERE id IN (?1, ?2)",
                params![old.id, protected.id],
            )
            .unwrap();
        }

        db.configure_trash_retention(0, 30).unwrap();

        let trashed = db.get_trashed_clips().unwrap();
        assert!(!trashed.iter().any(|clip| clip.id == old.id));
        assert!(trashed.iter().any(|clip| clip.id == protected.id));
        assert!(trashed.iter().any(|clip| clip.id == recent.id));
    }

    #[test]
    fn activity_age_retention_removes_old_entries_with_unlimited_count() {
        let db = setup_test_db();
        db.log_activity("app_started", "Old activity").unwrap();
        db.log_activity("app_exit_requested", "Recent activity")
            .unwrap();
        {
            let conn = db.conn.lock();
            conn.execute(
                "UPDATE activity_logs SET created_at = datetime('now', '-31 days')
                 WHERE description = 'Old activity'",
                [],
            )
            .unwrap();
        }

        db.configure_activity_retention(0, 30).unwrap();

        let logs = db.get_activity_logs(None, None).unwrap();
        assert!(!logs.iter().any(|log| log.description == "Old activity"));
        assert!(logs.iter().any(|log| log.description == "Recent activity"));
    }

    #[test]
    fn activity_archive_roundtrip_is_structured_inert_and_deduplicated() {
        let source = setup_test_db();
        source
            .log_activity("transformation_execution_failed", "Transform failed safely")
            .unwrap();
        source
            .log_activity("clip_restored", "Restored one clip")
            .unwrap();

        let json = source.export_activity_json().unwrap();
        let archive: ActivityArchive = serde_json::from_str(&json).unwrap();
        assert_eq!(archive.schema_version, 1);
        assert_eq!(archive.resource["service.name"], "Pasted");
        let failure = archive
            .entries
            .iter()
            .find(|entry| entry.event_name == "transformation_execution_failed")
            .unwrap();
        assert_eq!(failure.severity_text, "error");
        assert_eq!(failure.attributes["pasted.category"], "transformation");
        assert_eq!(failure.attributes["pasted.outcome"], "failure");
        assert!(!json.contains("text_content"));

        let destination = setup_test_db();
        destination.configure_activity_retention(0, 0).unwrap();
        let preview = destination.inspect_activity_json(&json).unwrap();
        assert_eq!(preview.scanned_count, 2);
        assert_eq!(preview.imported_count, 2);
        assert!(destination
            .get_activity_logs(None, None)
            .unwrap()
            .is_empty());
        let first = destination.import_activity_json(&json).unwrap();
        assert_eq!(first.scanned_count, 2);
        assert_eq!(first.imported_count, 2);
        assert_eq!(first.duplicate_count, 0);
        let second = destination.import_activity_json(&json).unwrap();
        assert_eq!(second.imported_count, 0);
        assert_eq!(second.duplicate_count, 2);
        assert_eq!(destination.get_activity_logs(None, None).unwrap().len(), 2);
    }

    #[test]
    fn activity_import_rejects_invalid_records_without_partial_writes() {
        let db = setup_test_db();
        let archive = serde_json::json!({
            "schemaVersion": 1,
            "exportedAt": "2026-08-13T00:00:00Z",
            "resource": { "service.name": "Pasted" },
            "entries": [
                {
                    "timestamp": "2026-08-13T00:00:00Z",
                    "observedTimestamp": "2026-08-13T00:00:00Z",
                    "eventName": "clip_restored",
                    "severityText": "info",
                    "body": "Valid record",
                    "attributes": {}
                },
                {
                    "timestamp": "not-a-time",
                    "observedTimestamp": "2026-08-13T00:00:00Z",
                    "eventName": "clip_restored",
                    "severityText": "info",
                    "body": "Invalid record",
                    "attributes": {}
                }
            ]
        });
        assert!(db.import_activity_json(&archive.to_string()).is_err());
        assert!(db.get_activity_logs(None, None).unwrap().is_empty());
    }

    #[test]
    fn activity_csv_export_has_a_stable_safe_content_contract() {
        let db = setup_test_db();
        {
            let conn = db.conn.lock();
            conn.execute(
                "INSERT INTO activity_logs
                    (event_type, description, created_at, observed_at, severity_text,
                     category, outcome, attributes_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    "transformation_execution_failed",
                    "=SUM(A1:A2), \"unsafe\"",
                    "2026-08-13 12:34:56",
                    "2026-08-13T12:35:00Z",
                    "error",
                    "transformation",
                    "failure",
                    r#"{"attempt":1}"#,
                ],
            )
            .unwrap();
        }

        let csv = db.export_activity_csv().unwrap();
        let mut lines = csv.lines();
        assert_eq!(
            lines.next(),
            Some("timestamp,observed_timestamp,event_name,severity_text,body,category,outcome,attributes_json")
        );
        let row = lines.next().unwrap();
        assert!(row.contains("\"2026-08-13T12:34:56Z\""));
        assert!(row.contains("\"transformation_execution_failed\""));
        assert!(row.contains("\"'=SUM(A1:A2), \"\"unsafe\"\"\""));
        assert!(row.contains("\"error\""));
        assert!(row.contains("\"transformation\",\"failure\""));
        assert!(lines.next().is_none());
        let records = DbState::parse_csv(&csv).unwrap();
        let exported_attributes: serde_json::Value = serde_json::from_str(&records[1][7]).unwrap();
        assert_eq!(exported_attributes["attempt"], 1);
        assert_eq!(exported_attributes["pasted.category"], "transformation");
        assert_eq!(exported_attributes["pasted.outcome"], "failure");
        assert!(exported_attributes["event.sequence"].is_number());

        let destination = setup_test_db();
        destination.configure_activity_retention(0, 0).unwrap();
        let preview = destination.inspect_activity_csv(&csv).unwrap();
        assert_eq!(preview.imported_count, 1);
        assert!(destination
            .get_activity_logs(None, None)
            .unwrap()
            .is_empty());
        let first = destination.import_activity_csv(&csv).unwrap();
        assert_eq!(first.scanned_count, 1);
        assert_eq!(first.imported_count, 1);
        assert_eq!(first.duplicate_count, 0);
        let second = destination.import_activity_csv(&csv).unwrap();
        assert_eq!(second.imported_count, 0);
        assert_eq!(second.duplicate_count, 1);
        let imported = destination.get_activity_logs(None, None).unwrap().remove(0);
        assert_eq!(imported.description, "=SUM(A1:A2), \"unsafe\"");
        assert_eq!(imported.category, "transformation");
        assert_eq!(imported.outcome, "failure");
        assert_eq!(imported.attributes["attempt"], 1);
        assert!(imported.attributes["event.sequence"].is_number());

        let invalid_target = setup_test_db();
        let invalid_csv = format!("{csv}\"broken\",\"row\"");
        assert!(invalid_target.import_activity_csv(&invalid_csv).is_err());
        assert!(invalid_target
            .get_activity_logs(None, None)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn test_clip_pinning_and_notes() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "text",
                Some("Pasted Pin Test"),
                None,
                None,
                "hash2",
                "VSCode",
            )
            .unwrap();

        // Pin clip
        let is_pinned = db.toggle_pin(clip.id).unwrap();
        assert!(is_pinned);

        // Add note
        db.update_clip_note(clip.id, Some("Important note"))
            .unwrap();

        let clips = db.get_clips(None, None, false).unwrap();
        assert!(clips[0].is_pinned);
        assert_eq!(clips[0].note.as_deref(), Some("Important note"));
    }

    #[test]
    fn test_bins_crud() {
        let db = setup_test_db();
        let initial_count = db.get_bins().unwrap().len();

        let bin = db.create_bin("Work", "💼", "#3b82f6", None).unwrap();
        assert!(bin.id > 0);

        let bins = db.get_bins().unwrap();
        assert_eq!(bins.len(), initial_count + 1);

        db.delete_bin(bin.id, "keep", None).unwrap();
        let bins_after = db.get_bins().unwrap();
        assert_eq!(bins_after.len(), initial_count);
    }

    #[test]
    fn deleting_a_bin_can_keep_move_or_trash_its_clips() {
        let db = setup_test_db();

        let keep_bin = db.create_bin("Keep", "📁", "default", None).unwrap();
        let kept = db
            .save_clip("text", Some("kept"), None, None, "keep_hash", "App")
            .unwrap();
        db.assign_to_bin(kept.id, Some(keep_bin.id)).unwrap();
        db.delete_bin(keep_bin.id, "keep", None).unwrap();
        assert_eq!(db.get_clip_by_id(kept.id).unwrap().bin_id, None);

        let source_bin = db.create_bin("Source", "📁", "default", None).unwrap();
        let destination_bin = db.create_bin("Destination", "📁", "default", None).unwrap();
        let moved = db
            .save_clip("text", Some("moved"), None, None, "move_hash", "App")
            .unwrap();
        db.assign_to_bin(moved.id, Some(source_bin.id)).unwrap();
        db.delete_bin(source_bin.id, "move", Some(destination_bin.id))
            .unwrap();
        assert_eq!(
            db.get_clip_by_id(moved.id).unwrap().bin_id,
            Some(destination_bin.id)
        );

        let trash_bin = db.create_bin("Trash", "📁", "default", None).unwrap();
        let trashed = db
            .save_clip("text", Some("trashed"), None, None, "trash_hash", "App")
            .unwrap();
        let protected = db
            .save_clip(
                "text",
                Some("protected"),
                None,
                None,
                "protected_hash",
                "App",
            )
            .unwrap();
        db.assign_to_bin(trashed.id, Some(trash_bin.id)).unwrap();
        db.assign_to_bin(protected.id, Some(trash_bin.id)).unwrap();
        db.toggle_protected(protected.id).unwrap();
        db.delete_bin(trash_bin.id, "trash", None).unwrap();

        assert!(db
            .get_trashed_clips()
            .unwrap()
            .iter()
            .any(|clip| clip.id == trashed.id));
        let protected_after = db.get_clip_by_id(protected.id).unwrap();
        assert!(protected_after.is_protected);
        assert!(!protected_after.is_trashed);
        assert_eq!(protected_after.bin_id, None);
    }

    #[test]
    fn deleting_a_bin_rejects_invalid_move_destinations_atomically() {
        let db = setup_test_db();
        let source_bin = db.create_bin("Source", "📁", "default", None).unwrap();
        let clip = db
            .save_clip("text", Some("clip"), None, None, "clip_hash", "App")
            .unwrap();
        db.assign_to_bin(clip.id, Some(source_bin.id)).unwrap();

        assert!(db.delete_bin(source_bin.id, "move", None).is_err());
        assert!(db
            .get_bins()
            .unwrap()
            .iter()
            .any(|bin| bin.id == source_bin.id));
        assert_eq!(
            db.get_clip_by_id(clip.id).unwrap().bin_id,
            Some(source_bin.id)
        );
    }

    #[test]
    fn test_legacy_container_schema_migrates_to_bins() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_path = std::env::temp_dir().join(format!("pasted_legacy_schema_{nanos}.db"));
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE clips (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    content_type TEXT NOT NULL,
                    text_content TEXT,
                    html_content TEXT,
                    image_base64 TEXT,
                    content_hash TEXT UNIQUE NOT NULL,
                    source TEXT DEFAULT 'Unknown',
                    is_pinned INTEGER DEFAULT 0,
                    board_id INTEGER,
                    note TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE boards (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    icon TEXT DEFAULT 'Folder',
                    color TEXT DEFAULT '#3b82f6',
                    smart_rule TEXT,
                    board_type TEXT DEFAULT 'category',
                    shortcut TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE clip_boards (
                    clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                    board_id INTEGER NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
                    PRIMARY KEY (clip_id, board_id)
                );
                INSERT INTO boards (id, name) VALUES (41, 'Migrated Bin');
                INSERT INTO clips (
                    id, content_type, text_content, content_hash, source, board_id
                ) VALUES (73, 'text', 'Legacy assignment', 'legacy-hash', 'Test', 41);
                INSERT INTO clip_boards (clip_id, board_id) VALUES (73, 41);",
            )
            .unwrap();
        }

        let db = DbState::new(db_path).unwrap();
        let bins = db.get_bins().unwrap();
        let clips = db.get_clips(None, None, false).unwrap();
        assert_eq!(bins.len(), 1);
        assert_eq!(bins[0].name, "Migrated Bin");
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].bin_id, Some(41));
        assert_eq!(clips[0].bin_ids.as_deref(), Some(&[41][..]));

        let conn = db.conn.lock();
        assert!(!table_exists(&conn, "boards").unwrap());
        assert!(!table_exists(&conn, "clip_boards").unwrap());
        assert!(column_exists(&conn, "clips", "bin_id").unwrap());
        assert!(column_exists(&conn, "bins", "bin_type").unwrap());
        assert!(column_exists(&conn, "clip_bins", "bin_id").unwrap());
    }

    #[test]
    fn partial_pre_release_transform_migration_merges_saved_data() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_path = std::env::temp_dir().join(format!("pasted_transform_terms_{nanos}.db"));
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(
                r#"CREATE TABLE bins (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    icon TEXT DEFAULT 'Folder',
                    color TEXT DEFAULT '#3b82f6',
                    smart_rule TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    default_recipe_id TEXT,
                    default_transform_id TEXT
                );
                CREATE TABLE transformation_recipes (
                    row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    id TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    plan_json TEXT NOT NULL,
                    connection_id TEXT,
                    revision INTEGER NOT NULL DEFAULT 1,
                    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE intelligence_connections (
                    row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    id TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    provider_kind TEXT NOT NULL,
                    endpoint TEXT,
                    model TEXT,
                    credential_ref TEXT,
                    enabled INTEGER NOT NULL DEFAULT 1,
                    priority INTEGER NOT NULL DEFAULT 0,
                    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE clip_transformations (
                    id TEXT PRIMARY KEY,
                    clip_id INTEGER NOT NULL,
                    transform_id TEXT REFERENCES transformation_recipes(id) ON DELETE SET NULL,
                    transform_name TEXT NOT NULL,
                    transform_revision INTEGER NOT NULL,
                    connection_id TEXT,
                    duration_ms INTEGER NOT NULL DEFAULT 0,
                    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE saved_transforms (
                    row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    id TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    plan_json TEXT NOT NULL,
                    connection_id TEXT,
                    revision INTEGER NOT NULL DEFAULT 1,
                    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE transformation_executions (
                    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                    target_kind TEXT NOT NULL CHECK (target_kind IN ('operation', 'pipeline')),
                    target_ref TEXT NOT NULL,
                    target_revision INTEGER,
                    source_clip_id INTEGER,
                    trigger_kind TEXT NOT NULL,
                    started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    duration_ms INTEGER,
                    status TEXT NOT NULL DEFAULT 'running',
                    error_summary TEXT,
                    input_hash TEXT NOT NULL,
                    output_hash TEXT
                );
                INSERT INTO transformation_recipes (id, name, plan_json)
                VALUES ('legacy-transform', 'Legacy Markdown',
                    '{"schema_version":1,"intent":"Markdown","summary":"Markdown","planning_mode":"pinned","steps":[]}');
                INSERT INTO bins (name, default_recipe_id)
                VALUES ('Legacy Bin', 'legacy-transform');"#,
            )
            .unwrap();
        }

        let db = DbState::new(db_path).unwrap();
        let transforms = db.get_saved_transforms().unwrap();
        assert_eq!(transforms.len(), 1);
        assert_eq!(transforms[0].stable_ref, "transform:legacy-transform");
        let legacy_bin_id = db
            .get_bins()
            .unwrap()
            .into_iter()
            .find(|bin| bin.name == "Legacy Bin")
            .unwrap()
            .id;
        assert_eq!(
            db.get_bin_transform_ref(legacy_bin_id).unwrap().as_deref(),
            Some("transform:legacy-transform")
        );

        let execution_id = db
            .begin_transformation_execution(TransformationExecutionStart {
                target_kind: "transform",
                target_ref: "transform:legacy-transform",
                target_revision: Some(1),
                source_clip_id: None,
                trigger_kind: "manual",
                destination_kind: "preview",
                input_hash: "input-hash",
            })
            .unwrap();
        db.finish_transformation_execution(&execution_id, 4, Some("output-hash"), None)
            .unwrap();
        let conn = db.conn.lock();
        assert!(!table_exists(&conn, "transformation_recipes").unwrap());
        assert!(column_exists(&conn, "bins", "default_transform_id").unwrap());
        assert!(!column_exists(&conn, "bins", "default_recipe_id").unwrap());
        assert!(column_exists(&conn, "clip_transformations", "transform_id").unwrap());
    }

    #[test]
    fn legacy_pipelines_migrate_atomically_to_canonical_transforms() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_path = std::env::temp_dir().join(format!("pasted_pipeline_merge_{nanos}.db"));
        let (bin_id, clip_id) = {
            let db = DbState::new(db_path.clone()).unwrap();
            let bin_id = db.get_bins().unwrap()[0].id;
            let clip_id = db
                .save_clip(
                    "text",
                    Some("migrate me"),
                    None,
                    None,
                    "pipeline-migration-clip",
                    "Test",
                )
                .unwrap()
                .id;
            (bin_id, clip_id)
        };
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
            conn.execute_batch(
                r#"ALTER TABLE bins ADD COLUMN default_pipeline_id TEXT;
                DROP TABLE transformation_executions;
                CREATE TABLE transformation_executions (
                    id TEXT PRIMARY KEY,
                    target_kind TEXT NOT NULL CHECK (target_kind IN ('operation', 'pipeline')),
                    target_ref TEXT NOT NULL,
                    target_revision INTEGER,
                    source_clip_id INTEGER,
                    trigger_kind TEXT NOT NULL,
                    started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    duration_ms INTEGER,
                    status TEXT NOT NULL DEFAULT 'running',
                    error_summary TEXT,
                    input_hash TEXT NOT NULL,
                    output_hash TEXT
                );
                CREATE TABLE pipelines (
                    row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    id TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    shortcut TEXT,
                    revision INTEGER NOT NULL DEFAULT 1,
                    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE pipeline_steps (
                    row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    pipeline_id TEXT NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
                    position INTEGER NOT NULL,
                    operation_ref TEXT NOT NULL,
                    config_json TEXT,
                    failure_policy TEXT NOT NULL DEFAULT 'stop'
                );
                INSERT INTO saved_transforms
                    (id, name, plan_json, authoring_kind)
                VALUES ('shared-id', 'Existing Intent',
                    '{"schema_version":1,"intent":"Keep","summary":"Keep","planning_mode":"pinned","steps":[{"name":"Trim","rationale":"Keep","scope":"whole_input","failure_policy":"stop","executor":{"kind":"deterministic","operation_ref":"builtin:trim","config_json":null}}]}',
                    'intent');
                INSERT INTO pipelines
                    (id, name, shortcut, revision, created_at, updated_at)
                VALUES ('shared-id', 'Legacy Manual', 'Alt+M', 4,
                    '2026-01-01 00:00:00', '2026-01-02 00:00:00');
                INSERT INTO pipeline_steps
                    (pipeline_id, position, operation_ref, failure_policy)
                VALUES ('shared-id', 0, 'builtin:uppercase', 'skip');
                UPDATE bins SET default_pipeline_id = 'pipeline:shared-id' WHERE id = 1;
                INSERT INTO clip_transformations
                    (id, clip_id, transform_ref, transform_name, transform_revision, duration_ms)
                VALUES ('legacy-provenance', 1, 'pipeline:shared-id', 'Legacy Manual', 4, 3);
                INSERT INTO transformation_executions
                    (id, target_kind, target_ref, target_revision, trigger_kind, input_hash)
                VALUES ('legacy-execution', 'pipeline', 'pipeline:shared-id', 4, 'manual', 'hash');
                INSERT INTO settings (key, value)
                VALUES ('lastExecutedPipelineRef', 'pipeline:shared-id');
                DROP TABLE automation_conditions;
                DROP TABLE automations;
                CREATE TABLE automations (
                    row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    id TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    trigger_kind TEXT NOT NULL,
                    pipeline_id TEXT NOT NULL REFERENCES pipelines(id) ON DELETE RESTRICT,
                    enabled INTEGER NOT NULL DEFAULT 0,
                    trusted INTEGER NOT NULL DEFAULT 0,
                    priority INTEGER NOT NULL DEFAULT 0,
                    action_json TEXT NOT NULL DEFAULT '{}',
                    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE automation_conditions (
                    id TEXT PRIMARY KEY,
                    automation_id TEXT NOT NULL REFERENCES automations(id) ON DELETE CASCADE,
                    position INTEGER NOT NULL,
                    condition_kind TEXT NOT NULL,
                    config_json TEXT NOT NULL
                );
                INSERT INTO automations
                    (id, name, trigger_kind, pipeline_id, enabled, trusted)
                VALUES ('legacy-automation', 'Legacy Automation', 'capture', 'shared-id', 1, 1);
                INSERT INTO automation_conditions
                    (id, automation_id, position, condition_kind, config_json)
                VALUES ('legacy-condition', 'legacy-automation', 0, 'content_type', '{}');"#,
            )
            .unwrap();
            conn.execute(
                "UPDATE bins SET default_pipeline_id = 'pipeline:shared-id' WHERE id = ?1",
                params![bin_id],
            )
            .unwrap();
            conn.execute(
                "UPDATE clip_transformations SET clip_id = ?1 WHERE id = 'legacy-provenance'",
                params![clip_id],
            )
            .unwrap();
            conn.execute(
                "UPDATE clips SET current_transformation_id = 'legacy-provenance' WHERE id = ?1",
                params![clip_id],
            )
            .unwrap();
        }

        let db = DbState::new(db_path).unwrap();
        let transforms = db.get_saved_transforms().unwrap();
        let migrated = transforms
            .iter()
            .find(|transform| transform.name == "Legacy Manual")
            .unwrap();
        assert_ne!(migrated.stable_ref, "transform:shared-id");
        assert_eq!(migrated.authoring_kind, "manual");
        assert_eq!(migrated.shortcut.as_deref(), Some("Alt+M"));
        assert_eq!(migrated.revision, 4);
        assert_eq!(
            migrated.plan.steps[0].failure_policy,
            crate::transformation_intent::StepFailurePolicy::Skip
        );
        assert_eq!(
            db.get_bin_transform_ref(bin_id).unwrap().as_deref(),
            Some(migrated.stable_ref.as_str())
        );
        assert_eq!(
            db.get_clip_transformation_provenance(clip_id)
                .unwrap()
                .unwrap()
                .transform_ref,
            migrated.stable_ref
        );
        assert_eq!(
            db.get_setting("lastExecutedTransformRef")
                .unwrap()
                .as_deref(),
            Some(migrated.stable_ref.as_str())
        );
        assert_eq!(db.get_setting("lastExecutedPipelineRef").unwrap(), None);
        let conn = db.conn.lock();
        let execution: (String, String) = conn
            .query_row(
                "SELECT target_kind, target_ref FROM transformation_executions
                 WHERE id = 'legacy-execution'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            execution,
            ("transform".to_string(), migrated.stable_ref.clone())
        );
        assert_eq!(
            conn.query_row(
                "SELECT transform_id FROM clip_transformations
                 WHERE id = 'legacy-provenance'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
            migrated.stable_ref.trim_start_matches("transform:")
        );
        let automation_transform: String = conn
            .query_row(
                "SELECT transform_id FROM automations WHERE id = 'legacy-automation'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            automation_transform,
            migrated.stable_ref.trim_start_matches("transform:")
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM automation_conditions
                 WHERE automation_id = 'legacy-automation'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            1
        );
        assert!(!table_exists(&conn, "pipelines").unwrap());
        assert!(!table_exists(&conn, "pipeline_steps").unwrap());
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
            0,
            "the migrated database must retain foreign-key integrity"
        );
    }

    #[test]
    fn legacy_pipeline_migration_rolls_back_on_an_orphaned_reference() {
        let db = setup_test_db();
        let conn = db.conn.lock();
        conn.execute_batch(
            "CREATE TABLE pipelines (
                row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                id TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL
             );
             CREATE TABLE pipeline_steps (
                row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                pipeline_id TEXT NOT NULL,
                position INTEGER NOT NULL,
                operation_ref TEXT NOT NULL
             );
             INSERT INTO pipelines (id, name) VALUES ('valid-pipeline', 'Keep Me');
             INSERT INTO pipeline_steps (pipeline_id, position, operation_ref)
             VALUES ('valid-pipeline', 0, 'builtin:trim');
             INSERT INTO settings (key, value)
             VALUES ('lastExecutedPipelineRef', 'pipeline:missing-pipeline');",
        )
        .unwrap();

        let error = migrate_pipelines_to_saved_transforms(&conn)
            .unwrap_err()
            .to_string();
        assert!(error.contains("last-used setting"));
        assert!(table_exists(&conn, "pipelines").unwrap());
        assert!(table_exists(&conn, "pipeline_steps").unwrap());
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM pipelines", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
            1
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM saved_transforms WHERE authoring_kind = 'manual'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            0
        );
        assert_eq!(
            conn.query_row(
                "SELECT value FROM settings WHERE key = 'lastExecutedPipelineRef'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
            "pipeline:missing-pipeline"
        );
        assert!(!column_exists(&conn, "pipelines", "shortcut").unwrap());
        assert!(!column_exists(&conn, "pipeline_steps", "failure_policy").unwrap());

        conn.execute(
            "DELETE FROM settings WHERE key = 'lastExecutedPipelineRef'",
            [],
        )
        .unwrap();
        migrate_pipelines_to_saved_transforms(&conn).unwrap();
        assert!(!table_exists(&conn, "pipelines").unwrap());
        assert!(!table_exists(&conn, "pipeline_steps").unwrap());
        let migrated: (String, String) = conn
            .query_row(
                "SELECT name, authoring_kind FROM saved_transforms
                 WHERE id = 'valid-pipeline'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(migrated, ("Keep Me".to_string(), "manual".to_string()));
    }

    #[test]
    fn test_settings_storage() {
        let db = setup_test_db();
        db.save_setting("hudHotkey", "CmdOrCtrl+Shift+V").unwrap();
        let val = db.get_setting("hudHotkey").unwrap();
        assert_eq!(val.as_deref(), Some("CmdOrCtrl+Shift+V"));
    }

    #[test]
    fn test_clip_search_and_deletion() {
        let db = setup_test_db();
        let clip1 = db
            .save_clip(
                "text",
                Some("Unique Search Secret"),
                None,
                None,
                "h1",
                "Terminal",
            )
            .unwrap();
        let _clip2 = db
            .save_clip("text", Some("Unrelated text"), None, None, "h2", "Finder")
            .unwrap();

        // Search by query
        let search_results = db.get_clips(Some("Secret"), None, false).unwrap();
        assert_eq!(search_results.len(), 1);
        assert_eq!(
            search_results[0].text_content.as_deref(),
            Some("Unique Search Secret")
        );

        // Test distinct apps
        let apps = db.get_distinct_sources().unwrap();
        assert!(apps.contains(&"Terminal".to_string()));
        assert!(apps.contains(&"Finder".to_string()));

        // Delete single clip (moves to trash)
        db.delete_clip(clip1.id).unwrap();
        let after_delete = db.get_clips(None, None, false).unwrap();
        assert_eq!(after_delete.len(), 1);

        // Verify clip is in Trash
        let trashed = db.get_trashed_clips().unwrap();
        assert_eq!(trashed.len(), 1);
        assert_eq!(trashed[0].id, clip1.id);
        assert_eq!(db.get_total_clip_count().unwrap(), 1);

        // Restore clip
        db.restore_clip(clip1.id).unwrap();
        let after_restore = db.get_clips(None, None, false).unwrap();
        assert_eq!(after_restore.len(), 2);
    }

    #[test]
    fn untrusted_clip_and_metadata_text_cannot_become_sql() {
        let db = setup_test_db();
        let hostile = "'); DROP TABLE clips; DELETE FROM bins; -- \" * OR 1=1";
        let hostile_transform = "AI output: '; UPDATE clips SET is_protected = 0; --";
        let hostile_rule = serde_json::json!({
            "type": "contains",
            "value": hostile,
        })
        .to_string();

        let clip = db
            .save_clip("text", Some(hostile), None, None, "hostile-hash", hostile)
            .unwrap();
        db.update_clip_text(clip.id, hostile_transform).unwrap();
        db.update_clip_note(clip.id, Some(hostile)).unwrap();
        let bin = db
            .create_bin(hostile, hostile, hostile, Some(&hostile_rule))
            .unwrap();

        // Search input is also untrusted. It may use FTS syntax internally, but it must
        // remain a bound value and must never alter the surrounding SQL statement.
        let _ = db.get_clips(Some(hostile), None, false).unwrap();

        let conn = db.conn.lock();
        let clip_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM clips", [], |row| row.get(0))
            .unwrap();
        let stored: (String, String, String) = conn
            .query_row(
                "SELECT text_content, source, note FROM clips WHERE id = ?1",
                params![clip.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        let stored_bin_name: String = conn
            .query_row(
                "SELECT name FROM bins WHERE id = ?1",
                params![bin.id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(clip_count, 1);
        assert_eq!(
            stored,
            (hostile_transform.into(), hostile.into(), hostile.into())
        );
        assert_eq!(stored_bin_name, hostile);
    }

    #[test]
    fn oversized_note_updates_are_rejected_without_mutating_the_clip() {
        let db = setup_test_db();
        let clip = db
            .save_clip("text", Some("original"), None, None, "bounded", "Tests")
            .unwrap();
        db.update_clip_note(clip.id, Some("original note")).unwrap();
        let oversized = "x".repeat(crate::resource_limits::MAX_CLIP_NOTE_BYTES + 1);

        assert!(db.update_clip_note(clip.id, Some(&oversized)).is_err());
        let stored = db
            .get_clips(None, None, false)
            .unwrap()
            .into_iter()
            .find(|item| item.id == clip.id)
            .unwrap();
        assert_eq!(stored.note.as_deref(), Some("original note"));
    }

    #[test]
    fn test_trash_and_activity_logging() {
        let db = setup_test_db();
        let clip = db
            .save_clip("text", Some("Trash Me"), None, None, "thash1", "Notes")
            .unwrap();

        // Trash clip
        db.delete_clip(clip.id).unwrap();
        let trashed = db.get_trashed_clips().unwrap();
        assert_eq!(trashed.len(), 1);

        // Empty trash
        db.empty_trash().unwrap();
        assert_eq!(db.get_trashed_clips().unwrap().len(), 0);

        // Check activity logs
        let logs = db.get_activity_logs(None, None).unwrap();
        assert!(logs.len() >= 2); // clip_trashed, trash_emptied
        assert_eq!(logs[0].event_type, "trash_emptied");

        // Clear logs
        db.clear_activity_logs().unwrap();
        assert_eq!(db.get_activity_logs(None, None).unwrap().len(), 0);
    }

    #[test]
    fn test_trashed_clips_are_read_only_and_leave_category_bins() {
        let db = setup_test_db();
        let category = db
            .create_bin("Projects", "Folder", "#3b82f6", None)
            .unwrap();
        let tag = db
            .create_bin_with_type("Keep", "Tag", "#f59e0b", None, "tag")
            .unwrap();
        let clip = db
            .save_clip(
                "text",
                Some("Original searchable text"),
                None,
                None,
                "trash-policy-hash",
                "Tests",
            )
            .unwrap();

        db.update_clip_note(clip.id, Some("Original searchable note"))
            .unwrap();
        db.assign_to_bin(clip.id, Some(category.id)).unwrap();
        db.add_clip_to_bin(clip.id, tag.id).unwrap();
        db.delete_clip(clip.id).unwrap();

        let trashed = db.get_trashed_clips().unwrap();
        assert_eq!(trashed.len(), 1);
        assert_eq!(trashed[0].bin_id, None);
        assert_eq!(trashed[0].note.as_deref(), Some("Original searchable note"));
        let category_after_trash = db
            .get_bins()
            .unwrap()
            .into_iter()
            .find(|bin| bin.id == category.id)
            .unwrap();
        assert_eq!(category_after_trash.clip_count, Some(0));
        {
            let conn = db.conn.lock();
            let category_links: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM clip_bins WHERE clip_id = ?1 AND bin_id = ?2",
                    params![clip.id, category.id],
                    |row| row.get(0),
                )
                .unwrap();
            let tag_links: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM clip_bins WHERE clip_id = ?1 AND bin_id = ?2",
                    params![clip.id, tag.id],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(category_links, 0);
            assert_eq!(tag_links, 1);
        }

        db.assign_to_bin(clip.id, Some(category.id)).unwrap();
        db.update_clip_note(clip.id, Some("Should be ignored"))
            .unwrap();
        db.update_clip_text(clip.id, "Should also be ignored")
            .unwrap();
        let unchanged = db.get_trashed_clips().unwrap();
        assert_eq!(unchanged[0].bin_id, None);
        assert_eq!(
            unchanged[0].note.as_deref(),
            Some("Original searchable note")
        );
        assert_eq!(
            unchanged[0].text_content.as_deref(),
            Some("Original searchable text")
        );

        db.restore_clip(clip.id).unwrap();
        let restored = db.get_clips(None, None, false).unwrap();
        assert_eq!(restored[0].bin_id, None);
        assert!(restored[0].bin_ids.as_ref().unwrap().contains(&tag.id));
        db.assign_to_bin(clip.id, Some(category.id)).unwrap();
        db.update_clip_note(clip.id, Some("Editable after restore"))
            .unwrap();
        let edited = db.get_clips(None, Some(category.id), false).unwrap();
        assert_eq!(edited[0].note.as_deref(), Some("Editable after restore"));
    }

    #[test]
    fn test_pipelines_and_operations_crud() {
        let db = setup_test_db();

        // Built-ins are registry-owned and the old seeded snapshot tables are gone.
        assert!(db.get_pipelines().unwrap().is_empty());
        assert_eq!(
            db.get_library_items(Some("operation"), false)
                .unwrap()
                .iter()
                .filter(|item| item.item.is_builtin)
                .count(),
            crate::operation_registry::BUILTIN_OPERATIONS.len()
        );
        {
            let conn = db.conn.lock();
            assert!(!table_exists(&conn, "operations").unwrap());
            assert!(table_exists(&conn, "custom_operations").unwrap());
            assert!(!table_exists(&conn, "pipelines").unwrap());
            assert!(!table_exists(&conn, "pipeline_steps").unwrap());
            let persisted_builtins: i64 = conn
                .query_row("SELECT COUNT(*) FROM custom_operations", [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(persisted_builtins, 0);
        }

        // Pipeline CRUD
        let pipeline = db
            .create_pipeline(
                "Trim",
                &[PipelineStepInput {
                    operation_ref: "builtin:trim".to_string(),
                    config_json: None,
                    failure_policy: "stop".to_string(),
                }],
                Some("Alt+T"),
            )
            .unwrap();
        assert!(pipeline.id > 0);
        let pipeline_item = db
            .get_library_items(Some("transform"), false)
            .unwrap()
            .into_iter()
            .find(|item| item.item.stable_ref == pipeline.stable_ref)
            .unwrap();
        assert_eq!(pipeline_item.item.input_contract, "text");
        assert!(pipeline_item.capabilities.can_edit);

        let pipelines = db.get_pipelines().unwrap();
        assert_eq!(pipelines[0].name, "Trim");
        assert_eq!(pipelines[0].steps[0].operation_ref, "builtin:trim");

        db.delete_pipeline(&pipeline.stable_ref).unwrap();
        assert!(db.get_pipelines().unwrap().is_empty());
        assert!(db
            .get_library_items(Some("transform"), false)
            .unwrap()
            .is_empty());

        // Operation CRUD
        let op = db
            .create_operation("JSON Prettify", "json_format", None, Some("Format"))
            .unwrap();
        assert!(op.id > 0);
        assert!(db
            .get_library_items(Some("operation"), false)
            .unwrap()
            .iter()
            .any(|item| item.item.stable_ref == op.stable_id && item.capabilities.can_delete));

        db.set_library_item_enabled("operation", &op.stable_id, false)
            .unwrap();
        let disabled = db
            .get_library_items(Some("operation"), false)
            .unwrap()
            .into_iter()
            .find(|item| item.item.stable_ref == op.stable_id)
            .unwrap();
        assert_eq!(disabled.item.enabled, Some(false));
        assert!(
            !db.resolve_custom_operation(&op.stable_id)
                .unwrap()
                .unwrap()
                .enabled
        );
        db.set_library_item_enabled("operation", &op.stable_id, true)
            .unwrap();

        let ops = db.get_operations().unwrap();
        assert!(ops.iter().any(|o| o.name == "JSON Prettify"));
        assert_eq!(db.get_operation(&op.stable_id).unwrap().id, op.id);
        let duplicate = db
            .duplicate_operation(&op.stable_id, Some("JSON Prettify Copy"))
            .unwrap();
        assert_eq!(duplicate.op_type, op.op_type);
        assert_eq!(duplicate.name, "JSON Prettify Copy");
        db.delete_operation(duplicate.id).unwrap();

        db.delete_operation(op.id).unwrap();
        let ops_after = db.get_operations().unwrap();
        assert!(!ops_after.iter().any(|o| o.id == op.id));
        assert!(db
            .get_library_items(Some("operation"), false)
            .unwrap()
            .iter()
            .all(|item| item.item.stable_ref != op.stable_id));
    }

    #[test]
    fn deleting_an_operation_preserves_pipelines_that_depend_on_it() {
        let db = setup_test_db();
        let operation = db
            .create_operation(
                "Reusable cleanup",
                "regex",
                Some(r#"{"pattern":"x","replacement":"y"}"#),
                Some("Custom Operations"),
            )
            .unwrap();
        let pipeline = db
            .create_pipeline(
                "Important Pipeline",
                &[PipelineStepInput {
                    operation_ref: operation.stable_id.clone(),
                    config_json: None,
                    failure_policy: "stop".to_string(),
                }],
                None,
            )
            .unwrap();

        let error = db.delete_operation(operation.id).unwrap_err().to_string();
        assert!(error.contains("Important Pipeline"));
        assert!(db
            .get_operations()
            .unwrap()
            .iter()
            .any(|candidate| candidate.id == operation.id));

        db.delete_pipeline(&pipeline.stable_ref).unwrap();
        db.delete_operation(operation.id).unwrap();
        assert!(!db
            .get_operations()
            .unwrap()
            .iter()
            .any(|candidate| candidate.id == operation.id));
    }

    #[test]
    fn intelligence_connections_store_references_but_not_credentials() {
        let db = setup_test_db();
        let connection = db
            .create_intelligence_connection(
                "Local Ollama",
                "ollama",
                Some("http://127.0.0.1:11434"),
                Some("qwen3"),
                None,
            )
            .unwrap();
        assert_eq!(
            db.get_intelligence_connection(&connection.id).unwrap(),
            connection
        );
        assert_eq!(connection.provider_kind, "ollama");
        assert_eq!(connection.credential_ref, None);

        db.update_intelligence_connection(IntelligenceConnectionUpdate {
            id: &connection.id,
            name: "Local Planner",
            provider_kind: "openai_compatible",
            endpoint: Some("http://127.0.0.1:1234/v1"),
            model: Some("local-model"),
            credential_ref: Some("env:PASTED_AI_API_KEY"),
            enabled: false,
        })
        .unwrap();
        let connections = db.get_intelligence_connections().unwrap();
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].name, "Local Planner");
        assert!(!connections[0].enabled);
        assert_eq!(
            connections[0].credential_ref.as_deref(),
            Some("env:PASTED_AI_API_KEY")
        );

        let fallback = db
            .create_intelligence_connection(
                "Fallback Ollama",
                "ollama",
                Some("http://127.0.0.1:11434"),
                None,
                None,
            )
            .unwrap();
        assert!(db
            .reorder_intelligence_connections(std::slice::from_ref(&connection.id))
            .is_err());
        assert!(db
            .reorder_intelligence_connections(&[connection.id.clone(), connection.id.clone()])
            .is_err());
        db.reorder_intelligence_connections(&[fallback.id.clone(), connection.id.clone()])
            .unwrap();
        let reordered = db.get_intelligence_connections().unwrap();
        assert_eq!(reordered[0].id, fallback.id);
        assert_eq!(reordered[0].priority, 0);
        assert_eq!(reordered[1].id, connection.id);
        assert_eq!(reordered[1].priority, 1);

        db.delete_intelligence_connection(&connection.id).unwrap();
        db.delete_intelligence_connection(&fallback.id).unwrap();
        assert!(db.get_intelligence_connections().unwrap().is_empty());
    }

    #[test]
    fn detected_intelligence_candidates_are_disabled_and_idempotent() {
        let db = setup_test_db();
        db.ensure_intelligence_connection_candidate(
            "Codex CLI",
            "cli",
            Some("/usr/local/bin/codex"),
        )
        .unwrap();
        db.ensure_intelligence_connection_candidate(
            "Codex CLI",
            "cli",
            Some("/usr/local/bin/codex"),
        )
        .unwrap();

        let connections = db.get_intelligence_connections().unwrap();
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].name, "Codex CLI");
        assert!(!connections[0].enabled);
        assert_eq!(connections[0].priority, 0);
    }

    #[test]
    fn test_pipeline_roundtrip_update_and_validation_rollback() {
        let db = setup_test_db();
        let created = db
            .create_pipeline(
                "Normalize",
                &[
                    PipelineStepInput {
                        operation_ref: "builtin:trim".to_string(),
                        config_json: None,
                        failure_policy: "stop".to_string(),
                    },
                    PipelineStepInput {
                        operation_ref: "builtin:wrap_tags".to_string(),
                        config_json: Some(r#""strong""#.to_string()),
                        failure_policy: "stop".to_string(),
                    },
                ],
                Some("Alt+N"),
            )
            .unwrap();
        assert_eq!(created.revision, 1);
        assert_eq!(created.steps.len(), 2);
        assert_eq!(created.steps[0].position, 0);
        assert_eq!(created.steps[0].operation_ref, "builtin:trim");
        assert_eq!(created.steps[1].position, 1);
        assert_eq!(created.steps[1].config_json.as_deref(), Some(r#""strong""#));

        let updated = db
            .update_pipeline(
                &created.stable_ref,
                "Loud Quote",
                &[
                    PipelineStepInput {
                        operation_ref: "builtin:uppercase".to_string(),
                        config_json: None,
                        failure_policy: "stop".to_string(),
                    },
                    PipelineStepInput {
                        operation_ref: "builtin:quote_text".to_string(),
                        config_json: None,
                        failure_policy: "skip".to_string(),
                    },
                ],
                Some("Alt+L"),
            )
            .unwrap();
        assert_eq!(updated.stable_ref, created.stable_ref);
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.revision, 2);
        assert_eq!(updated.name, "Loud Quote");
        assert_eq!(updated.shortcut.as_deref(), Some("Alt+L"));
        assert_eq!(
            updated
                .steps
                .iter()
                .map(|step| (step.position, step.operation_ref.as_str()))
                .collect::<Vec<_>>(),
            vec![(0, "builtin:uppercase"), (1, "builtin:quote_text")]
        );

        let invalid = db.update_pipeline(
            &created.stable_ref,
            "Must Roll Back",
            &[PipelineStepInput {
                operation_ref: "builtin:not-real".to_string(),
                config_json: None,
                failure_policy: "stop".to_string(),
            }],
            None,
        );
        assert!(invalid.is_err());
        let after_failure = db
            .get_pipelines()
            .unwrap()
            .into_iter()
            .find(|pipeline| pipeline.stable_ref == created.stable_ref)
            .unwrap();
        assert_eq!(after_failure.name, "Loud Quote");
        assert_eq!(after_failure.revision, 2);
        assert_eq!(after_failure.steps, updated.steps);

        let too_many_steps = (0..33)
            .map(|_| PipelineStepInput {
                operation_ref: "builtin:trim".to_string(),
                config_json: None,
                failure_policy: "stop".to_string(),
            })
            .collect::<Vec<_>>();
        assert!(db
            .create_pipeline("Too Many Steps", &too_many_steps, None)
            .unwrap_err()
            .to_string()
            .contains("at most 32 steps"));
        assert!(db
            .get_pipelines()
            .unwrap()
            .iter()
            .all(|pipeline| pipeline.name != "Too Many Steps"));
    }

    #[test]
    fn test_pipeline_update_and_delete_report_not_found() {
        let db = setup_test_db();
        let steps = [PipelineStepInput {
            operation_ref: "builtin:trim".to_string(),
            config_json: None,
            failure_policy: "stop".to_string(),
        }];
        assert!(db
            .update_pipeline("pipeline:missing", "Missing", &steps, None)
            .is_err());
        assert!(db.delete_pipeline("pipeline:missing").is_err());
        assert!(db
            .update_pipeline_shortcut("pipeline:missing", Some("Alt+M"))
            .is_err());
    }

    #[test]
    fn test_wal_mode_and_indexing() {
        let db = setup_test_db();
        let conn = db.conn.lock();

        // Verify WAL mode is configured
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert!(
            mode.to_lowercase() == "wal" || mode.to_lowercase() == "memory",
            "journal_mode should be wal or memory (test db), got: {}",
            mode
        );

        // Verify indexes exist
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='index'")
            .unwrap();
        let index_names: Vec<String> = stmt
            .query_map([], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(index_names.contains(&"idx_clips_pinned_created".to_string()));
        assert!(index_names.contains(&"idx_clips_bin_created".to_string()));
        assert!(index_names.contains(&"idx_clips_hash".to_string()));
        assert!(index_names.contains(&"idx_clips_active_timeline".to_string()));
    }

    #[test]
    fn test_fts5_search_indexing() {
        let db = setup_test_db();

        let clip1 = db
            .save_clip(
                "text",
                Some("Supercalifragilisticexpialidocious secret token"),
                None,
                None,
                "HashFTS1",
                "IntelliJ",
            )
            .unwrap();
        let _clip2 = db
            .save_clip(
                "text",
                Some("Unrelated standard content text"),
                None,
                None,
                "HashFTS2",
                "Safari",
            )
            .unwrap();

        let search_res = db
            .get_clips(Some("Supercalifragilisticexpialidocious"), None, false)
            .unwrap();
        assert_eq!(search_res.len(), 1);
        assert_eq!(search_res[0].id, clip1.id);

        db.delete_clip(clip1.id).unwrap();
        let search_after_delete = db
            .get_clips(Some("Supercalifragilisticexpialidocious"), None, false)
            .unwrap();
        assert_eq!(search_after_delete.len(), 0);
    }

    #[test]
    fn test_startup_rebuilds_fts_before_clip_updates() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "text",
                Some("Recoverable noted clip"),
                None,
                None,
                "HashFTSRecovery",
                "Notes",
            )
            .unwrap();
        db.update_clip_note(clip.id, Some("Keep this note"))
            .unwrap();

        {
            let conn = db.conn.lock();
            conn.execute("INSERT INTO clips_fts(clips_fts) VALUES('delete-all')", [])
                .unwrap();
        }

        db.init_tables().unwrap();
        let search_results = db.get_clips(Some("Recoverable"), None, false).unwrap();
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0].id, clip.id);

        assert!(db.toggle_pin(clip.id).unwrap());
        db.update_clip_note(clip.id, Some("Updated note")).unwrap();
        db.delete_clip(clip.id).unwrap();
        assert!(db.get_clips(None, None, false).unwrap().is_empty());
    }

    #[test]
    fn test_unified_taxonomy_and_tags() {
        let db = setup_test_db();
        let tag = db
            .create_bin_with_type("CodeSnippet", "Tag", "#06b6d4", None, "tag")
            .unwrap();
        assert_eq!(tag.bin_type, "tag");

        let bins = db.get_bins().unwrap();
        assert!(bins.iter().any(|b| b.id == tag.id && b.bin_type == "tag"));
    }

    #[test]
    fn test_pin_reordering() {
        let db = setup_test_db();
        let clip1 = db
            .save_clip("text", Some("First Pinned"), None, None, "HashP1", "App")
            .unwrap();
        let clip2 = db
            .save_clip("text", Some("Second Pinned"), None, None, "HashP2", "App")
            .unwrap();
        db.toggle_pin(clip1.id).unwrap();
        db.toggle_pin(clip2.id).unwrap();

        let newly_pinned_first = db.get_clips(None, None, true).unwrap();
        assert_eq!(newly_pinned_first[0].id, clip2.id);
        assert_eq!(newly_pinned_first[1].id, clip1.id);

        assert!(db.reorder_pinned_clips(vec![clip1.id]).is_err());
        assert!(db.reorder_pinned_clips(vec![clip1.id, clip1.id]).is_err());
        db.reorder_pinned_clips(vec![clip1.id, clip2.id]).unwrap();
        let clips = db.get_clips(None, None, true).unwrap();
        assert_eq!(clips[0].id, clip1.id);
        assert_eq!(clips[1].id, clip2.id);
    }

    #[test]
    fn bin_clip_order_is_persistent_validated_and_independent_per_bin() {
        let db = setup_test_db();
        let first = db
            .save_clip("text", Some("First"), None, None, "bin-order-1", "App")
            .unwrap();
        let second = db
            .save_clip("text", Some("Second"), None, None, "bin-order-2", "App")
            .unwrap();
        let manual = db
            .create_bin("Manual Order", "Folder", "default", None)
            .unwrap();
        let smart = db
            .create_bin(
                "Smart Order",
                "Sparkles",
                "default",
                Some(r#"{"type":"content_type","value":"text"}"#),
            )
            .unwrap();

        db.assign_to_bin(first.id, Some(manual.id)).unwrap();
        db.assign_to_bin(second.id, Some(manual.id)).unwrap();
        db.reorder_bin_clips(manual.id, vec![first.id, second.id])
            .unwrap();
        db.reorder_bin_clips(smart.id, vec![second.id, first.id])
            .unwrap();

        let manual_clips = db.get_clips(None, Some(manual.id), false).unwrap();
        let smart_clips = db.get_clips(None, Some(smart.id), false).unwrap();
        assert_eq!(
            manual_clips.iter().map(|clip| clip.id).collect::<Vec<_>>(),
            vec![first.id, second.id]
        );
        assert_eq!(
            smart_clips.iter().map(|clip| clip.id).collect::<Vec<_>>(),
            vec![second.id, first.id]
        );

        let bins = db.get_bins().unwrap();
        assert_eq!(
            bins.iter()
                .find(|bin| bin.id == manual.id)
                .unwrap()
                .clip_order,
            vec![first.id, second.id]
        );
        assert_eq!(
            bins.iter()
                .find(|bin| bin.id == smart.id)
                .unwrap()
                .clip_order,
            vec![second.id, first.id]
        );

        assert!(db.reorder_bin_clips(manual.id, vec![first.id]).is_err());
        assert!(db
            .reorder_bin_clips(manual.id, vec![first.id, first.id])
            .is_err());
        assert_eq!(
            db.get_bins()
                .unwrap()
                .iter()
                .find(|bin| bin.id == manual.id)
                .unwrap()
                .clip_order,
            vec![first.id, second.id]
        );
    }

    #[test]
    fn test_clip_version_history() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "text",
                Some("Original Content"),
                None,
                None,
                "HashV1",
                "App",
            )
            .unwrap();

        db.update_clip_text(clip.id, "Transformed Uppercase Content")
            .unwrap();
        db.update_clip_text(clip.id, "Transformed Uppercase Content")
            .unwrap();
        db.update_clip_text(clip.id, "Final Content").unwrap();

        let versions = db.get_clip_versions(clip.id).unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 2);
        assert_eq!(versions[0].text_content, "Transformed Uppercase Content");
        assert_eq!(versions[1].text_content, "Original Content");

        let updated = db.get_clips(None, None, false).unwrap();
        assert_eq!(updated[0].text_content.as_deref(), Some("Final Content"));

        for index in 0..55 {
            db.update_clip_text(clip.id, &format!("Revision {index}"))
                .unwrap();
        }
        assert_eq!(db.get_clip_versions(clip.id).unwrap().len(), 50);
        assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 50);

        db.purge_clip_permanently(clip.id).unwrap();
        assert!(db.get_clip_versions(clip.id).unwrap().is_empty());
        assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 0);
    }

    #[test]
    fn revision_restore_rejects_versions_from_another_clip_without_mutation() {
        let db = setup_test_db();
        let first = db
            .save_clip(
                "text",
                Some("First original"),
                None,
                None,
                "revision-boundary-first",
                "Test",
            )
            .unwrap();
        let second = db
            .save_clip(
                "text",
                Some("Second original"),
                None,
                None,
                "revision-boundary-second",
                "Test",
            )
            .unwrap();
        db.update_clip_text(first.id, "First current").unwrap();
        db.update_clip_text(second.id, "Second current").unwrap();
        let foreign_version = db.get_clip_versions(second.id).unwrap().remove(0);
        let first_version_count = db.get_clip_version_count(first.id).unwrap();
        let second_version_count = db.get_clip_version_count(second.id).unwrap();

        assert!(db
            .restore_clip_version(first.id, foreign_version.id)
            .is_err());
        assert_eq!(
            db.get_clip_by_id(first.id).unwrap().text_content.as_deref(),
            Some("First current")
        );
        assert_eq!(
            db.get_clip_by_id(second.id)
                .unwrap()
                .text_content
                .as_deref(),
            Some("Second current")
        );
        assert_eq!(
            db.get_clip_version_count(first.id).unwrap(),
            first_version_count
        );
        assert_eq!(
            db.get_clip_version_count(second.id).unwrap(),
            second_version_count
        );
    }

    #[test]
    fn disabled_revision_history_preserves_existing_versions_and_skips_new_snapshots() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "text",
                Some("Original Content"),
                None,
                None,
                "revision-feature-gate",
                "App",
            )
            .unwrap();

        db.update_clip_text(clip.id, "First Edit").unwrap();
        assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 1);

        db.save_setting("enableRevisions", "false").unwrap();
        db.update_clip_text(clip.id, "Irreversible Edit").unwrap();
        assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 1);
        assert_eq!(
            db.get_clip_by_id(clip.id).unwrap().text_content.as_deref(),
            Some("Irreversible Edit")
        );

        db.save_setting("enableRevisions", "true").unwrap();
        db.update_clip_text(clip.id, "History Resumed").unwrap();
        let versions = db.get_clip_versions(clip.id).unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].text_content, "Irreversible Edit");
        assert_eq!(versions[1].text_content, "Original Content");
    }

    #[test]
    fn ocr_state_is_hash_safe_and_follows_the_clip_lifecycle() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "image",
                None,
                None,
                Some(crate::resource_limits::TEST_PNG_DATA_URL),
                "ocr-lifecycle-hash",
                "Screenshot",
            )
            .unwrap();

        let status = db.get_ocr_backfill_status().unwrap();
        assert_eq!(status.total_images, 1);
        assert_eq!(status.eligible_count, 1);

        let candidate = db.claim_next_ocr_candidate().unwrap().unwrap();
        assert_eq!(candidate.clip_id, clip.id);
        assert!(db
            .complete_ocr_attempt(
                clip.id,
                "wrong-hash",
                Some("stale result"),
                "test-engine",
                None,
            )
            .is_ok());
        assert_eq!(
            db.get_clip_by_id(clip.id).unwrap().text_content.as_deref(),
            None
        );

        db.delete_clip(clip.id).unwrap();
        assert!(!db
            .complete_ocr_attempt(
                clip.id,
                &clip.content_hash,
                Some("late result"),
                "test-engine",
                None,
            )
            .unwrap());
        assert_eq!(db.get_ocr_backfill_status().unwrap().total_images, 0);

        db.restore_clip(clip.id).unwrap();
        assert_eq!(db.get_ocr_backfill_status().unwrap().eligible_count, 1);
        db.save_setting("enableOcr", "false").unwrap();
        db.purge_clip_permanently(clip.id).unwrap();
        assert!(db.get_clip_by_id(clip.id).is_err());
        assert_eq!(db.get_ocr_backfill_status().unwrap().total_images, 0);
    }

    #[test]
    fn successful_ocr_records_state_and_revisions_only_when_text_changes() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "image",
                None,
                None,
                Some(crate::resource_limits::TEST_PNG_DATA_URL),
                "ocr-success-hash",
                "Screenshot",
            )
            .unwrap();

        assert!(db
            .complete_ocr_attempt_with_extractor(
                clip.id,
                &clip.content_hash,
                Some("First OCR"),
                OcrExtractorProvenance::identified(
                    "test-engine-v1",
                    "extractor:test-ocr",
                    "Test OCR",
                ),
                None,
            )
            .unwrap());
        let completed_clip = db.get_clip_by_id(clip.id).unwrap();
        assert_eq!(
            completed_clip.ocr_extractor_ref.as_deref(),
            Some("extractor:test-ocr")
        );
        assert_eq!(
            completed_clip.ocr_extractor_name.as_deref(),
            Some("Test OCR")
        );
        assert_eq!(
            completed_clip.ocr_engine_version.as_deref(),
            Some("test-engine-v1")
        );
        assert_eq!(db.get_ocr_backfill_status().unwrap().completed_count, 1);
        assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 0);

        db.force_ocr_running(clip.id, &clip.content_hash).unwrap();
        db.complete_ocr_attempt_with_extractor(
            clip.id,
            &clip.content_hash,
            Some("Improved OCR"),
            OcrExtractorProvenance::identified(
                "test-engine-v2",
                "extractor:test-ocr-v2",
                "Test OCR 2",
            ),
            None,
        )
        .unwrap();
        assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 1);
        assert_eq!(
            db.get_clip_versions(clip.id).unwrap()[0].text_content,
            "First OCR"
        );

        db.force_ocr_running(clip.id, &clip.content_hash).unwrap();
        db.complete_ocr_attempt_with_extractor(
            clip.id,
            &clip.content_hash,
            None,
            OcrExtractorProvenance::identified(
                "failed-engine-v1",
                "extractor:failed-ocr",
                "Failed OCR",
            ),
            Some("recognition_failed"),
        )
        .unwrap();
        let failed_rerun = db.get_clip_by_id(clip.id).unwrap();
        assert_eq!(failed_rerun.text_content.as_deref(), Some("Improved OCR"));
        assert_eq!(
            failed_rerun.ocr_extractor_name.as_deref(),
            Some("Test OCR 2")
        );
        assert_eq!(
            failed_rerun.ocr_engine_version.as_deref(),
            Some("test-engine-v2")
        );
    }

    #[test]
    fn revision_retention_is_configurable_and_can_be_unlimited() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "text",
                Some("Original"),
                None,
                None,
                "revision-policy",
                "App",
            )
            .unwrap();

        db.enforce_revision_retention(10).unwrap();
        for index in 0..18 {
            db.update_clip_text(clip.id, &format!("Limited {index}"))
                .unwrap();
        }
        assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 10);

        db.enforce_revision_retention(0).unwrap();
        for index in 0..60 {
            db.update_clip_text(clip.id, &format!("Unlimited {index}"))
                .unwrap();
        }
        assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 70);

        db.enforce_revision_retention(25).unwrap();
        assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 25);
        let newest = db.get_clip_versions_page(clip.id, 10, 0).unwrap();
        let middle = db.get_clip_versions_page(clip.id, 10, 10).unwrap();
        let oldest = db.get_clip_versions_page(clip.id, 10, 20).unwrap();
        assert_eq!((newest.len(), middle.len(), oldest.len()), (10, 10, 5));
        assert_ne!(newest[0].id, middle[0].id);
    }

    #[test]
    fn test_batch_operations() {
        let db = setup_test_db();
        let clip1 = db
            .save_clip("text", Some("Batch 1"), None, None, "HashB1", "App")
            .unwrap();
        let clip2 = db
            .save_clip("text", Some("Batch 2"), None, None, "HashB2", "App")
            .unwrap();

        db.batch_pin_clips(vec![clip1.id, clip2.id], true).unwrap();
        let pinned = db.get_clips(None, None, true).unwrap();
        assert_eq!(pinned.len(), 2);

        db.batch_trash_clips(vec![clip1.id]).unwrap();
        let trashed = db.get_trashed_clips().unwrap();
        assert_eq!(trashed.len(), 1);
        assert_eq!(trashed[0].id, clip1.id);
    }

    #[test]
    fn restore_all_trashed_clips_restores_every_item_and_reports_a_stable_summary() {
        let db = setup_test_db();
        let first = db
            .save_clip("text", Some("First"), None, None, "restore-all-1", "App")
            .unwrap();
        let second = db
            .save_clip("text", Some("Second"), None, None, "restore-all-2", "App")
            .unwrap();
        let active = db
            .save_clip("text", Some("Active"), None, None, "restore-all-3", "App")
            .unwrap();

        db.batch_trash_clips(vec![first.id, second.id]).unwrap();
        let restored = db.restore_all_trashed_clips().unwrap();

        assert_eq!(restored.action, "restore_all");
        assert_eq!(restored.requested_count, 2);
        assert_eq!(restored.changed_count, 2);
        assert_eq!(restored.skipped_count, 0);
        assert_eq!(restored.clip_ids, vec![first.id, second.id]);
        assert!(db.get_trashed_clips().unwrap().is_empty());
        let active_ids = db
            .get_clips(None, None, false)
            .unwrap()
            .into_iter()
            .map(|clip| clip.id)
            .collect::<Vec<_>>();
        assert!(active_ids.contains(&first.id));
        assert!(active_ids.contains(&second.id));
        assert!(active_ids.contains(&active.id));

        let noop = db.restore_all_trashed_clips().unwrap();
        assert_eq!(noop.requested_count, 0);
        assert_eq!(noop.changed_count, 0);
        assert_eq!(noop.skipped_count, 0);
        assert!(noop.clip_ids.is_empty());

        let logs = db.get_activity_logs(Some(20), None).unwrap();
        assert_eq!(
            logs.iter()
                .filter(|entry| entry.event_type == "clips_restored_all")
                .count(),
            1
        );
    }

    #[test]
    fn clip_mutations_report_changes_skip_noops_and_log_user_actions() {
        let db = setup_test_db();
        let first = db
            .save_clip("text", Some("First"), None, None, "mutation-1", "App")
            .unwrap();
        let second = db
            .save_clip("text", Some("Second"), None, None, "mutation-2", "App")
            .unwrap();
        let bin = db
            .create_bin("Destination", "Folder", "#3b82f6", None)
            .unwrap();

        let pinned = db
            .batch_pin_clips(vec![first.id, second.id, first.id], true)
            .unwrap();
        assert_eq!(pinned.action, "pin");
        assert_eq!(pinned.requested_count, 3);
        assert_eq!(pinned.changed_count, 2);
        assert_eq!(pinned.skipped_count, 1);

        let pin_noop = db.batch_pin_clips(vec![first.id], true).unwrap();
        assert_eq!(pin_noop.changed_count, 0);

        let protected = db.batch_protect_clips(vec![first.id], true).unwrap();
        assert_eq!(protected.changed_count, 1);

        let assigned = db
            .batch_assign_bin_clips(vec![first.id, second.id], Some(bin.id))
            .unwrap();
        assert_eq!(assigned.changed_count, 2);

        let trashed = db.batch_trash_clips(vec![first.id, second.id]).unwrap();
        assert_eq!(trashed.changed_count, 1);
        assert_eq!(trashed.skipped_count, 1);
        assert_eq!(trashed.clip_ids, vec![second.id]);

        let logs = db.get_activity_logs(Some(20), None).unwrap();
        let event_types = logs
            .iter()
            .map(|log| log.event_type.as_str())
            .collect::<Vec<_>>();
        assert!(event_types.contains(&"clips_pinned"));
        assert!(event_types.contains(&"clip_protected_toggled"));
        assert!(event_types.contains(&"clips_bin_assigned"));
        assert!(event_types.contains(&"clip_trashed"));
        assert_eq!(
            event_types
                .iter()
                .filter(|event| **event == "clips_pinned")
                .count(),
            1
        );
    }

    #[test]
    fn test_manual_bin_assignment_is_additive_and_individually_removable() {
        let db = setup_test_db();
        let clip1 = db
            .save_clip("text", Some("Exclusive 1"), None, None, "HashE1", "App")
            .unwrap();
        let clip2 = db
            .save_clip("text", Some("Exclusive 2"), None, None, "HashE2", "App")
            .unwrap();
        let first_bin = db
            .create_bin("First Bin", "Folder", "#3b82f6", None)
            .unwrap();
        let second_bin = db
            .create_bin("Second Bin", "Folder", "#10b981", None)
            .unwrap();
        let tag = db
            .create_bin_with_type("Important", "Tag", "#f59e0b", None, "tag")
            .unwrap();

        assert!(db.toggle_pin(clip1.id).unwrap());
        assert!(db.toggle_protected(clip1.id).unwrap());
        db.assign_to_bin(clip1.id, Some(first_bin.id)).unwrap();
        db.add_clip_to_bin(clip1.id, tag.id).unwrap();
        db.assign_to_bin(clip1.id, Some(second_bin.id)).unwrap();

        assert_eq!(
            db.get_clips(None, Some(first_bin.id), false).unwrap().len(),
            1
        );
        let second_bin_clips = db.get_clips(None, Some(second_bin.id), false).unwrap();
        assert_eq!(second_bin_clips.len(), 1);
        assert_eq!(second_bin_clips[0].id, clip1.id);
        assert!(second_bin_clips[0].is_pinned);
        assert!(second_bin_clips[0].is_protected);
        assert!(second_bin_clips[0]
            .bin_ids
            .as_ref()
            .unwrap()
            .contains(&tag.id));

        db.assign_to_bin(clip1.id, None).unwrap();
        let unassigned = db.get_clips(None, None, false).unwrap();
        let clip1_after_unassign = unassigned.iter().find(|clip| clip.id == clip1.id).unwrap();
        assert_eq!(clip1_after_unassign.bin_id, None);
        assert!(clip1_after_unassign.is_pinned);
        assert!(clip1_after_unassign.is_protected);
        assert!(clip1_after_unassign.bin_ids.as_ref().unwrap().is_empty());

        db.batch_assign_bin_clips(vec![clip1.id, clip2.id], Some(first_bin.id))
            .unwrap();
        db.batch_assign_bin_clips(vec![clip1.id, clip2.id], Some(second_bin.id))
            .unwrap();
        assert_eq!(
            db.get_clips(None, Some(first_bin.id), false).unwrap().len(),
            2
        );
        let batch_assigned = db.get_clips(None, Some(second_bin.id), false).unwrap();
        assert_eq!(batch_assigned.len(), 2);
        let protected_pinned = batch_assigned
            .iter()
            .find(|clip| clip.id == clip1.id)
            .unwrap();
        assert!(protected_pinned.is_pinned);
        assert!(protected_pinned.is_protected);

        let removed = db
            .batch_remove_bin_clips(vec![clip1.id], second_bin.id)
            .unwrap();
        assert_eq!(removed.changed_count, 1);
        let clip1_after_remove = db.get_clip_by_id(clip1.id).unwrap();
        assert!(!clip1_after_remove
            .bin_ids
            .as_ref()
            .unwrap()
            .contains(&second_bin.id));
        assert!(clip1_after_remove
            .bin_ids
            .as_ref()
            .unwrap()
            .contains(&first_bin.id));
    }

    #[test]
    fn test_backup_export_import() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "text",
                Some("Backup Test Item"),
                Some("<strong>Backup Test Item</strong>"),
                None,
                "HashBK1",
                "VSCode",
            )
            .unwrap();
        let trashed = db
            .save_clip("text", Some("In Trash"), None, None, "HashBK2", "Notes")
            .unwrap();
        let bin = db.create_bin("DevBin", "Code", "#3b82f6", None).unwrap();
        let tag = db
            .create_bin_with_type("BackupTag", "Tag", "#f59e0b", None, "tag")
            .unwrap();
        db.assign_to_bin(clip.id, Some(bin.id)).unwrap();
        db.add_clip_to_bin(clip.id, tag.id).unwrap();
        db.update_clip_note(clip.id, Some("Restore this note"))
            .unwrap();
        db.toggle_pin(clip.id).unwrap();
        db.toggle_protected(clip.id).unwrap();
        db.delete_clip(trashed.id).unwrap();
        let backup_pipeline = db
            .create_pipeline(
                "Backup Pipeline",
                &[
                    PipelineStepInput {
                        operation_ref: "builtin:trim".to_string(),
                        config_json: None,
                        failure_policy: "stop".to_string(),
                    },
                    PipelineStepInput {
                        operation_ref: "builtin:uppercase".to_string(),
                        config_json: None,
                        failure_policy: "stop".to_string(),
                    },
                ],
                Some("Alt+B"),
            )
            .unwrap();
        db.create_operation(
            "Backup Operation",
            "uppercase",
            Some("{}"),
            Some("Backup Tools"),
        )
        .unwrap();
        let transform_plan = crate::transformation_intent::TransformationPlan {
            schema_version: crate::transformation_intent::TRANSFORMATION_PLAN_SCHEMA_VERSION,
            intent: "Uppercase".to_string(),
            summary: "Uppercase".to_string(),
            planning_mode: crate::transformation_intent::IntentPlanningMode::Pinned,
            steps: vec![crate::transformation_intent::PlannedTransformationStep {
                name: "Uppercase".to_string(),
                rationale: "Replayable".to_string(),
                scope: crate::transformation_intent::StepExecutionScope::WholeInput,
                failure_policy: Default::default(),
                executor: crate::transformation_intent::PlannedExecutor::Deterministic {
                    operation_ref: "builtin:uppercase".to_string(),
                    config_json: None,
                },
            }],
        };
        let saved_transform = db
            .create_saved_transform("Backup Transform", &transform_plan, None)
            .unwrap();
        db.set_bin_transform_ref(bin.id, Some(&saved_transform.stable_ref))
            .unwrap();

        let json = db.export_backup_json().unwrap();
        assert!(json.contains("Backup Test Item"));
        assert!(json.contains("DevBin"));
        assert!(!json.contains("\"pipelines\""));
        assert!(json.contains("\"authoringKind\": \"manual\""));

        let db2 = setup_test_db();
        let imported_count = db2.import_backup_json(&json).unwrap();
        assert_eq!(imported_count, 2);

        let restored = db2.get_all_clips_for_backup().unwrap();
        let restored_clip = restored
            .iter()
            .find(|item| item.content_hash == "HashBK1")
            .unwrap();
        assert_eq!(
            restored_clip.text_content.as_deref(),
            Some("Backup Test Item")
        );
        assert_eq!(
            restored_clip.html_content.as_deref(),
            Some("<strong>Backup Test Item</strong>")
        );
        assert_eq!(restored_clip.note.as_deref(), Some("Restore this note"));
        assert!(restored_clip.is_pinned);
        assert!(restored_clip.is_protected);
        assert!(!restored_clip.is_trashed);

        let restored_trashed = restored
            .iter()
            .find(|item| item.content_hash == "HashBK2")
            .unwrap();
        assert!(restored_trashed.is_trashed);
        assert!(restored_trashed.trashed_at.is_some());

        let restored_bins = db2.get_bins().unwrap();
        let restored_bin = restored_bins
            .iter()
            .find(|item| item.name == "DevBin")
            .unwrap();
        let restored_tag = restored_bins
            .iter()
            .find(|item| item.name == "BackupTag")
            .unwrap();
        let restored_bin_ids = restored_clip.bin_ids.as_ref().unwrap();
        assert!(restored_bin_ids.contains(&restored_bin.id));
        assert!(restored_bin_ids.contains(&restored_tag.id));
        let restored_pipeline = db2
            .get_pipelines()
            .unwrap()
            .into_iter()
            .find(|item| item.name == "Backup Pipeline")
            .unwrap();
        assert_eq!(restored_pipeline.stable_ref, backup_pipeline.stable_ref);
        assert_eq!(restored_pipeline.shortcut.as_deref(), Some("Alt+B"));
        assert_eq!(restored_pipeline.steps.len(), 2);
        assert_eq!(
            restored_pipeline.steps[1].operation_ref,
            "builtin:uppercase"
        );
        assert_eq!(
            db2.get_saved_transforms()
                .unwrap()
                .into_iter()
                .find(|item| item.name == "Backup Transform")
                .unwrap()
                .stable_ref,
            saved_transform.stable_ref
        );
        assert_eq!(
            db2.get_bin_transform_ref(restored_bin.id)
                .unwrap()
                .as_deref(),
            Some(saved_transform.stable_ref.as_str())
        );
        assert!(db2
            .get_operations()
            .unwrap()
            .iter()
            .any(|item| item.name == "Backup Operation" && item.category == "Backup Tools"));
        let restored_registry = db2.get_library_items(None, false).unwrap();
        assert!(restored_registry
            .iter()
            .any(|item| item.item.stable_ref == backup_pipeline.stable_ref));
        assert!(restored_registry.iter().any(|item| {
            item.item.kind == "operation"
                && item.item.name == "Backup Operation"
                && item.item.group_label.as_deref() == Some("Backup Tools")
        }));
    }

    #[test]
    fn legacy_pipeline_backups_import_as_manual_transforms() {
        let source = setup_test_db();
        let mut payload =
            serde_json::from_str::<serde_json::Value>(&source.export_backup_json().unwrap())
                .unwrap();
        payload["pipelines"] = serde_json::json!([{
            "id": 1,
            "stableRef": "pipeline:legacy-backup",
            "name": "Legacy Backup",
            "shortcut": "Alt+L",
            "revision": 3,
            "createdAt": "2026-01-01 00:00:00",
            "updatedAt": "2026-01-02 00:00:00",
            "steps": [{
                "position": 0,
                "operationRef": "builtin:uppercase",
                "configJson": null,
                "failurePolicy": "skip"
            }]
        }]);

        let destination = setup_test_db();
        destination
            .import_backup_json(&serde_json::to_string(&payload).unwrap())
            .unwrap();
        let imported = destination
            .get_pipelines()
            .unwrap()
            .into_iter()
            .find(|transform| transform.name == "Legacy Backup")
            .unwrap();
        assert_eq!(imported.stable_ref, "transform:legacy-backup");
        assert_eq!(imported.shortcut.as_deref(), Some("Alt+L"));
        assert_eq!(imported.revision, 3);
        assert_eq!(imported.steps[0].failure_policy, "skip");
        assert!(!destination
            .export_backup_json()
            .unwrap()
            .contains("\"pipelines\""));
    }

    #[test]
    fn backup_roundtrip_preserves_bin_clip_order() {
        let source = setup_test_db();
        let first = source
            .save_clip("text", Some("First"), None, None, "backup-order-1", "App")
            .unwrap();
        let second = source
            .save_clip("text", Some("Second"), None, None, "backup-order-2", "App")
            .unwrap();
        let bin = source
            .create_bin("Ordered", "Folder", "default", None)
            .unwrap();
        source.assign_to_bin(first.id, Some(bin.id)).unwrap();
        source.assign_to_bin(second.id, Some(bin.id)).unwrap();
        source
            .reorder_bin_clips(bin.id, vec![first.id, second.id])
            .unwrap();

        let destination = setup_test_db();
        destination
            .import_backup_json(&source.export_backup_json().unwrap())
            .unwrap();
        let restored_bin = destination
            .get_bins()
            .unwrap()
            .into_iter()
            .find(|candidate| candidate.name == "Ordered")
            .unwrap();
        let restored = destination
            .get_clips(None, Some(restored_bin.id), false)
            .unwrap();
        assert_eq!(
            restored
                .iter()
                .map(|clip| clip.text_content.as_deref().unwrap_or_default())
                .collect::<Vec<_>>(),
            vec!["First", "Second"]
        );
    }

    #[test]
    fn test_backup_export_is_not_limited_to_visible_history() {
        let db = setup_test_db();
        for index in 0..501 {
            db.save_clip(
                "text",
                Some(&format!("Backup item {index}")),
                None,
                None,
                &format!("backup-limit-{index}"),
                "App",
            )
            .unwrap();
        }

        let json = db.export_backup_json().unwrap();
        let payload: BackupPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(payload.version, BACKUP_SCHEMA_VERSION);
        assert_eq!(payload.clips.len(), 501);
        assert_eq!(db.get_clips(None, None, false).unwrap().len(), 501);
    }

    #[test]
    fn library_archive_preflight_reports_contents_and_rejects_late_corruption() {
        let source = setup_test_db();
        source.configure_clip_retention(0, 0).unwrap();
        for index in 0..2_000 {
            source
                .save_clip(
                    "text",
                    Some(&format!("Archive item {index}")),
                    None,
                    None,
                    &format!("archive-preflight-{index}"),
                    "Tests",
                )
                .unwrap();
        }
        let json = source.export_backup_json().unwrap();
        let inspection = DbState::inspect_library_archive_json(&json).unwrap();
        assert_eq!(inspection.schema_version, BACKUP_SCHEMA_VERSION);
        assert_eq!(inspection.clip_count, 2_000);
        assert!(inspection.content_type_count > 0);
        assert!(inspection.detector_count > 0);

        let mut corrupted: serde_json::Value = serde_json::from_str(&json).unwrap();
        let clips = corrupted["clips"].as_array_mut().unwrap();
        let duplicate_hash = clips[0]["content_hash"].clone();
        clips.last_mut().unwrap()["content_hash"] = duplicate_hash;
        let corrupted = serde_json::to_string(&corrupted).unwrap();
        let error = DbState::inspect_library_archive_json(&corrupted)
            .unwrap_err()
            .to_string();
        assert!(error.contains("duplicate clip content hash"));

        let destination = setup_test_db();
        destination
            .save_setting("preflightMarker", "unchanged")
            .unwrap();
        let changes_before = destination.conn.lock().total_changes();
        assert!(destination.import_backup_json(&corrupted).is_err());
        assert_eq!(destination.conn.lock().total_changes(), changes_before);
        assert_eq!(
            destination
                .get_setting("preflightMarker")
                .unwrap()
                .as_deref(),
            Some("unchanged")
        );
    }

    #[test]
    fn library_archive_reimport_updates_stable_identities_without_duplicates() {
        let source = setup_test_db();
        let clip = source
            .save_clip(
                "text",
                Some("Idempotent archive clip"),
                None,
                None,
                "idempotent-archive-clip",
                "Tests",
            )
            .unwrap();
        let bin = source
            .create_bin("Archive Bin", "Folder", "default", None)
            .unwrap();
        source.assign_to_bin(clip.id, Some(bin.id)).unwrap();
        source
            .create_operation(
                "Archive Operation",
                "uppercase",
                Some("{}"),
                Some("Archive Tests"),
            )
            .unwrap();
        let plan = crate::transformation_intent::TransformationPlan {
            schema_version: crate::transformation_intent::TRANSFORMATION_PLAN_SCHEMA_VERSION,
            intent: "Trim text".to_string(),
            summary: "Trim text".to_string(),
            planning_mode: crate::transformation_intent::IntentPlanningMode::Pinned,
            steps: vec![crate::transformation_intent::PlannedTransformationStep {
                name: "Trim".to_string(),
                rationale: "Remove surrounding whitespace".to_string(),
                scope: crate::transformation_intent::StepExecutionScope::WholeInput,
                failure_policy: Default::default(),
                executor: crate::transformation_intent::PlannedExecutor::Deterministic {
                    operation_ref: "builtin:trim".to_string(),
                    config_json: None,
                },
            }],
        };
        source
            .create_saved_transform("Archive Transform", &plan, None)
            .unwrap();
        let archive = source.export_backup_json().unwrap();

        let destination = setup_test_db();
        assert_eq!(destination.import_backup_json(&archive).unwrap(), 1);
        let counts_after_first = {
            let conn = destination.conn.lock();
            (
                conn.query_row("SELECT COUNT(*) FROM clips", [], |row| row.get::<_, i64>(0))
                    .unwrap(),
                conn.query_row(
                    "SELECT COUNT(*) FROM bins WHERE name = 'Archive Bin'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
                conn.query_row(
                    "SELECT COUNT(*) FROM custom_operations WHERE name = 'Archive Operation'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
                conn.query_row(
                    "SELECT COUNT(*) FROM saved_transforms WHERE name = 'Archive Transform'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            )
        };
        assert_eq!(counts_after_first, (1, 1, 1, 1));

        assert_eq!(destination.import_backup_json(&archive).unwrap(), 1);
        let counts_after_second = {
            let conn = destination.conn.lock();
            (
                conn.query_row("SELECT COUNT(*) FROM clips", [], |row| row.get::<_, i64>(0))
                    .unwrap(),
                conn.query_row(
                    "SELECT COUNT(*) FROM bins WHERE name = 'Archive Bin'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
                conn.query_row(
                    "SELECT COUNT(*) FROM custom_operations WHERE name = 'Archive Operation'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
                conn.query_row(
                    "SELECT COUNT(*) FROM saved_transforms WHERE name = 'Archive Transform'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            )
        };
        assert_eq!(counts_after_second, counts_after_first);
    }

    #[test]
    fn clip_exports_match_their_documented_json_and_csv_contracts() {
        let db = setup_test_db();
        let active = db
            .save_clip(
                "text",
                Some("=SUM(A1:A2), \"quoted\""),
                Some("<b>preserved in JSON</b>"),
                None,
                "clip-export-active",
                "Editor, Inc.",
            )
            .unwrap();
        db.toggle_pin(active.id).unwrap();
        let trashed = db
            .save_clip(
                "text",
                Some("must not be exported"),
                None,
                None,
                "clip-export-trashed",
                "Tests",
            )
            .unwrap();
        db.delete_clip(trashed.id).unwrap();

        let json = db.export_clips_json().unwrap();
        let clips: Vec<ClipItem> = serde_json::from_str(&json).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].content_hash, "clip-export-active");
        assert_eq!(
            clips[0].html_content.as_deref(),
            Some("<b>preserved in JSON</b>")
        );
        assert!(clips[0].is_pinned);
        assert!(!json.contains("must not be exported"));

        let csv = db.export_clips_csv().unwrap();
        let mut lines = csv.lines();
        assert_eq!(
            lines.next(),
            Some("id,content_type,source,is_pinned,created_at,text_content")
        );
        let row = lines.next().unwrap();
        assert!(row.contains("\"Editor, Inc.\""));
        assert!(row.contains("\"'=SUM(A1:A2), \"\"quoted\"\"\""));
        assert!(row.contains(",true,"));
        assert!(lines.next().is_none());

        let json_target = setup_test_db();
        let json_preview = json_target.inspect_clips_json(&json).unwrap();
        assert_eq!(json_preview.imported_count, 1);
        assert!(json_target.get_all_clips_for_backup().unwrap().is_empty());
        let first_json_import = json_target.import_clips_json(&json).unwrap();
        assert_eq!(first_json_import.scanned_count, 1);
        assert_eq!(first_json_import.imported_count, 1);
        assert_eq!(first_json_import.duplicate_count, 0);
        let second_json_import = json_target.import_clips_json(&json).unwrap();
        assert_eq!(second_json_import.imported_count, 0);
        assert_eq!(second_json_import.duplicate_count, 1);
        let imported_json_clip = json_target.get_all_clips_for_backup().unwrap().remove(0);
        assert_eq!(
            imported_json_clip.html_content.as_deref(),
            Some("<b>preserved in JSON</b>")
        );
        assert!(imported_json_clip.is_pinned);

        let csv_target = setup_test_db();
        let csv_preview = csv_target.inspect_clips_csv(&csv).unwrap();
        assert_eq!(csv_preview.imported_count, 1);
        assert!(csv_target.get_clips(None, None, false).unwrap().is_empty());
        let first_csv_import = csv_target.import_clips_csv(&csv).unwrap();
        assert_eq!(first_csv_import.imported_count, 1);
        assert_eq!(first_csv_import.duplicate_count, 0);
        let second_csv_import = csv_target.import_clips_csv(&csv).unwrap();
        assert_eq!(second_csv_import.imported_count, 0);
        assert_eq!(second_csv_import.duplicate_count, 1);
        let imported_csv_clip = csv_target.get_clips(None, None, false).unwrap().remove(0);
        assert_eq!(
            imported_csv_clip.text_content.as_deref(),
            Some("=SUM(A1:A2), \"quoted\"")
        );
        assert_eq!(imported_csv_clip.source, "Editor, Inc.");

        let invalid_target = setup_test_db();
        let invalid_csv = format!("{csv}\n\"broken\",\"row\"");
        assert!(invalid_target.import_clips_csv(&invalid_csv).is_err());
        assert!(invalid_target
            .get_clips(None, None, false)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn clip_json_import_round_trips_stored_images() {
        let source = setup_test_db();
        source
            .save_clip(
                "image",
                Some("recognized text"),
                None,
                Some(crate::resource_limits::TEST_PNG_DATA_URL),
                "clip-image-export-hash",
                "Screenshot",
            )
            .unwrap();
        let json = source.export_clips_json().unwrap();

        let target = setup_test_db();
        let report = target.import_clips_json(&json).unwrap();
        assert_eq!(report.imported_count, 1);
        let imported = target.get_all_clips_for_backup().unwrap().remove(0);
        assert_eq!(imported.content_type, "image");
        assert_eq!(imported.text_content.as_deref(), Some("recognized text"));
        assert_eq!(
            imported.image_base64.as_deref(),
            Some(crate::resource_limits::TEST_PNG_DATA_URL)
        );
        assert_eq!(imported.content_hash, "clip-image-export-hash");
    }

    #[test]
    fn raster_image_boundaries_reject_active_content_without_mutation() {
        let malicious = "data:image/png;base64,PHN2ZyBvbmxvYWQ9ImFsZXJ0KDEpIj48L3N2Zz4=";
        let direct = setup_test_db();
        assert!(direct
            .save_clip(
                "image",
                None,
                None,
                Some(malicious),
                "malicious-direct-image",
                "Tests",
            )
            .is_err());
        assert!(direct.get_all_clips_for_backup().unwrap().is_empty());

        let source = setup_test_db();
        source
            .save_clip(
                "image",
                None,
                None,
                Some(crate::resource_limits::TEST_PNG_DATA_URL),
                "malicious-import-image",
                "Tests",
            )
            .unwrap();
        let mut payload: serde_json::Value =
            serde_json::from_str(&source.export_clips_json().unwrap()).unwrap();
        payload[0]["image_base64"] = malicious.into();
        let payload = serde_json::to_string(&payload).unwrap();
        let target = setup_test_db();
        assert!(target.inspect_clips_json(&payload).is_err());
        assert!(target.import_clips_json(&payload).is_err());
        assert!(target.get_all_clips_for_backup().unwrap().is_empty());

        let legacy = source.get_all_clips_for_backup().unwrap().remove(0);
        source
            .conn
            .lock()
            .execute(
                "UPDATE clips SET image_base64 = ?1 WHERE id = ?2",
                params![malicious, legacy.id],
            )
            .unwrap();
        assert_eq!(source.get_clip_image(legacy.id).unwrap(), None);
        assert_eq!(
            source
                .get_all_clips_for_backup()
                .unwrap()
                .remove(0)
                .image_base64
                .as_deref(),
            Some(malicious)
        );
    }

    #[test]
    fn insights_summary_is_strictly_read_only() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "text",
                Some("Read-only insight"),
                None,
                None,
                "insights-read-only",
                "",
            )
            .unwrap();
        let changes_before = db.conn.lock().total_changes();
        let before = db.get_clip_by_id(clip.id).unwrap();
        let summary = db.get_analytics_summary().unwrap();
        let after = db.get_clip_by_id(clip.id).unwrap();

        assert_eq!(summary.total_clips, 1);
        assert_eq!(db.conn.lock().total_changes(), changes_before);
        assert_eq!(after.source, before.source);
        assert_eq!(after.content_hash, before.content_hash);
    }

    #[test]
    fn backup_roundtrip_preserves_completed_ocr_lifecycle_state() {
        let source = setup_test_db();
        let clip = source
            .save_clip(
                "image",
                None,
                None,
                Some(crate::resource_limits::TEST_PNG_DATA_URL),
                "ocr-backup-hash",
                "Screenshot",
            )
            .unwrap();
        assert!(source
            .complete_ocr_attempt_with_extractor(
                clip.id,
                "ocr-backup-hash",
                Some("Recovered words"),
                OcrExtractorProvenance::identified(
                    "vision-test-v1",
                    "extractor:test-vision",
                    "Test Vision OCR",
                ),
                None,
            )
            .unwrap());

        let backup = source.export_backup_json().unwrap();
        let destination = setup_test_db();
        assert_eq!(destination.import_backup_json(&backup).unwrap(), 1);

        let status = destination.get_ocr_backfill_status().unwrap();
        assert_eq!(status.total_images, 1);
        assert_eq!(status.completed_count, 1);
        assert_eq!(status.eligible_count, 0);

        let restored_payload: BackupPayload =
            serde_json::from_str(&destination.export_backup_json().unwrap()).unwrap();
        assert_eq!(restored_payload.ocr_metadata.len(), 1);
        assert_eq!(restored_payload.ocr_metadata[0].status, "complete");
        assert_eq!(
            restored_payload.ocr_metadata[0].engine_version.as_deref(),
            Some("vision-test-v1")
        );
        assert_eq!(
            restored_payload.ocr_metadata[0].extractor_ref.as_deref(),
            Some("extractor:test-vision")
        );
        assert_eq!(
            restored_payload.ocr_metadata[0].extractor_name.as_deref(),
            Some("Test Vision OCR")
        );
    }

    #[test]
    fn backup_import_rejects_unknown_schema_without_mutating_data() {
        let source = setup_test_db();
        source
            .save_clip(
                "text",
                Some("future data"),
                None,
                None,
                "future-backup-item",
                "Test",
            )
            .unwrap();
        let mut payload: serde_json::Value =
            serde_json::from_str(&source.export_backup_json().unwrap()).unwrap();
        payload["version"] = serde_json::json!(BACKUP_SCHEMA_VERSION + 1);

        let destination = setup_test_db();
        let error = destination
            .import_backup_json(&serde_json::to_string(&payload).unwrap())
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("unsupported transfer schema version"));
        assert!(destination.get_clips(None, None, false).unwrap().is_empty());
    }

    #[test]
    fn backup_import_rolls_back_earlier_writes_when_valid_payload_fails_midway() {
        let source = setup_test_db();
        source
            .create_bin("Imported Bin", "Folder", "default", None)
            .unwrap();
        source
            .create_operation(
                "Imported Operation",
                "uppercase",
                Some("{}"),
                Some("Import Test"),
            )
            .unwrap();
        let mut payload: serde_json::Value =
            serde_json::from_str(&source.export_backup_json().unwrap()).unwrap();
        let custom_operation = payload["operations"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|operation| {
                operation["stable_id"]
                    .as_str()
                    .is_some_and(|stable_id| stable_id.starts_with("custom:"))
            })
            .unwrap();
        custom_operation["stable_id"] = serde_json::json!("invalid-operation-reference");

        let destination = setup_test_db();
        let existing = destination
            .save_clip(
                "text",
                Some("Destination must survive"),
                None,
                None,
                "backup-rollback-existing",
                "Test",
            )
            .unwrap();
        destination.save_setting("themeMode", "warm").unwrap();
        let bins_before = destination
            .get_bins()
            .unwrap()
            .into_iter()
            .map(|bin| (bin.id, bin.name))
            .collect::<Vec<_>>();

        let error = destination
            .import_backup_json(&serde_json::to_string(&payload).unwrap())
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("custom operation in transfer file is missing a stable reference"));
        assert_eq!(
            destination
                .get_clip_by_id(existing.id)
                .unwrap()
                .text_content
                .as_deref(),
            Some("Destination must survive")
        );
        assert_eq!(
            destination.get_setting("themeMode").unwrap().as_deref(),
            Some("warm")
        );
        assert_eq!(
            destination
                .get_bins()
                .unwrap()
                .into_iter()
                .map(|bin| (bin.id, bin.name))
                .collect::<Vec<_>>(),
            bins_before
        );
        assert!(!destination
            .get_operations()
            .unwrap()
            .iter()
            .any(|operation| operation.name == "Imported Operation"));
    }

    #[test]
    fn test_saved_transform_roundtrip_and_delete() {
        let db = setup_test_db();
        let connection = db
            .create_intelligence_connection(
                "Codex CLI",
                "cli",
                Some("/usr/local/bin/codex"),
                None,
                None,
            )
            .unwrap();
        let plan = crate::transformation_intent::TransformationPlan {
            schema_version: crate::transformation_intent::TRANSFORMATION_PLAN_SCHEMA_VERSION,
            intent: "Convert this text to Markdown".to_string(),
            summary: "Convert text to Markdown".to_string(),
            planning_mode: crate::transformation_intent::IntentPlanningMode::Pinned,
            steps: vec![crate::transformation_intent::PlannedTransformationStep {
                name: "Convert to Markdown".to_string(),
                rationale: "Structure requires interpretation".to_string(),
                scope: crate::transformation_intent::StepExecutionScope::WholeInput,
                failure_policy: Default::default(),
                executor: crate::transformation_intent::PlannedExecutor::Semantic {
                    instructions: "Return clean Markdown".to_string(),
                    output_schema: None,
                    model_policy: crate::transformation_intent::ModelPolicy::Balanced,
                },
            }],
        };
        let transform = db
            .create_saved_transform("Markdown", &plan, Some(connection.id.as_str()))
            .unwrap();
        assert!(transform.stable_ref.starts_with("transform:"));
        assert_eq!(
            transform.connection_id.as_deref(),
            Some(connection.id.as_str())
        );
        assert_eq!(transform.plan, plan);
        assert_eq!(db.get_saved_transforms().unwrap().len(), 1);
        assert_eq!(
            db.resolve_saved_transform(&transform.stable_ref)
                .unwrap()
                .unwrap()
                .name,
            "Markdown"
        );
        let mut updated_plan = plan.clone();
        updated_plan.summary = "Convert text to concise Markdown".to_string();
        let updated = db
            .update_saved_transform(
                &transform.stable_ref,
                "Concise Markdown",
                &updated_plan,
                Some(connection.id.as_str()),
            )
            .unwrap();
        assert_eq!(updated.stable_ref, transform.stable_ref);
        assert_eq!(updated.name, "Concise Markdown");
        assert_eq!(updated.revision, transform.revision + 1);
        assert_eq!(updated.plan, updated_plan);
        db.delete_saved_transform(&transform.stable_ref).unwrap();
        assert!(db.get_saved_transforms().unwrap().is_empty());
    }

    #[test]
    fn test_transform_preview_applies_atomically_with_revision_and_provenance() {
        let db = setup_test_db();
        let clip = db
            .save_clip("text", Some("hello"), None, None, "transform-clip", "Test")
            .unwrap();
        let plan = crate::transformation_intent::TransformationPlan {
            schema_version: crate::transformation_intent::TRANSFORMATION_PLAN_SCHEMA_VERSION,
            intent: "Uppercase".to_string(),
            summary: "Uppercase text".to_string(),
            planning_mode: crate::transformation_intent::IntentPlanningMode::Pinned,
            steps: vec![crate::transformation_intent::PlannedTransformationStep {
                name: "Uppercase".to_string(),
                rationale: "Replayable".to_string(),
                scope: crate::transformation_intent::StepExecutionScope::WholeInput,
                failure_policy: Default::default(),
                executor: crate::transformation_intent::PlannedExecutor::Deterministic {
                    operation_ref: "builtin:uppercase".to_string(),
                    config_json: None,
                },
            }],
        };
        let transform = db.create_saved_transform("Uppercase", &plan, None).unwrap();
        let provenance = db
            .apply_transform_output_to_clip(TransformClipApplication {
                clip_id: clip.id,
                transform_ref: &transform.stable_ref,
                expected_input: "hello",
                output: "HELLO",
                connection_id: None,
                duration_ms: 12,
                bin_move: None,
            })
            .unwrap();
        assert_eq!(provenance.transform_name, "Uppercase");
        assert_eq!(provenance.duration_ms, 12);
        assert_eq!(
            db.get_clip_versions(clip.id).unwrap()[0].text_content,
            "hello"
        );
        assert_eq!(
            db.get_clip_transformation_provenance(clip.id)
                .unwrap()
                .unwrap()
                .transform_ref,
            transform.stable_ref
        );
        let current = db
            .get_clips(None, None, false)
            .unwrap()
            .into_iter()
            .find(|item| item.id == clip.id)
            .unwrap();
        assert_eq!(current.text_content.as_deref(), Some("HELLO"));

        let stale = db.apply_transform_output_to_clip(TransformClipApplication {
            clip_id: clip.id,
            transform_ref: &transform.stable_ref,
            expected_input: "hello",
            output: "ANOTHER RESULT",
            connection_id: None,
            duration_ms: 5,
            bin_move: None,
        });
        assert!(stale
            .unwrap_err()
            .to_string()
            .contains("changed after this preview"));
        assert_eq!(db.get_clip_versions(clip.id).unwrap().len(), 1);
    }

    #[test]
    fn manually_built_transform_applies_with_revision_and_stable_provenance() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "text",
                Some("hello"),
                None,
                None,
                "manual-transform-clip",
                "Test",
            )
            .unwrap();
        let pipeline = db
            .create_pipeline(
                "Uppercase Locally",
                &[PipelineStepInput {
                    operation_ref: "builtin:uppercase".to_string(),
                    config_json: None,
                    failure_policy: "stop".to_string(),
                }],
                None,
            )
            .unwrap();
        assert!(db.get_intent_transforms().unwrap().is_empty());

        let definitions = db.get_transform_definitions().unwrap();
        assert_eq!(
            definitions
                .iter()
                .filter(|item| item.stable_ref == pipeline.stable_ref)
                .count(),
            1,
            "canonical definitions must not duplicate manual Transforms"
        );
        let definition = definitions
            .iter()
            .find(|item| item.stable_ref == pipeline.stable_ref)
            .unwrap();
        assert_eq!(definition.authoring_kind, TransformAuthoringKind::Manual);
        assert_eq!(definition.execution_character, "replayable");

        let provenance = db
            .apply_transform_output_to_clip(TransformClipApplication {
                clip_id: clip.id,
                transform_ref: &pipeline.stable_ref,
                expected_input: "hello",
                output: "HELLO",
                connection_id: None,
                duration_ms: 4,
                bin_move: None,
            })
            .unwrap();
        assert_eq!(provenance.transform_ref, pipeline.stable_ref);
        assert_eq!(
            db.get_clip_versions(clip.id).unwrap()[0].text_content,
            "hello"
        );
        assert_eq!(
            db.get_clip_transformation_provenance(clip.id)
                .unwrap()
                .unwrap()
                .transform_ref,
            pipeline.stable_ref
        );
        let stored: (Option<String>, Option<String>) = db
            .conn
            .lock()
            .query_row(
                "SELECT transform_id, transform_ref FROM clip_transformations WHERE clip_id = ?1",
                params![clip.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            stored,
            (
                Some(
                    pipeline
                        .stable_ref
                        .trim_start_matches("transform:")
                        .to_string()
                ),
                Some(pipeline.stable_ref.clone())
            )
        );

        db.delete_pipeline(&pipeline.stable_ref).unwrap();
        assert_eq!(
            db.get_clip_transformation_provenance(clip.id)
                .unwrap()
                .unwrap()
                .transform_ref,
            pipeline.stable_ref
        );
    }

    #[test]
    fn transformation_provenance_migration_backfills_stable_refs() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "text",
                Some("hello"),
                None,
                None,
                "provenance-migration-clip",
                "Test",
            )
            .unwrap();
        let plan = crate::transformation_intent::TransformationPlan {
            schema_version: crate::transformation_intent::TRANSFORMATION_PLAN_SCHEMA_VERSION,
            intent: "Uppercase".to_string(),
            summary: "Uppercase text".to_string(),
            planning_mode: crate::transformation_intent::IntentPlanningMode::Pinned,
            steps: vec![crate::transformation_intent::PlannedTransformationStep {
                name: "Uppercase".to_string(),
                rationale: "Replayable".to_string(),
                scope: crate::transformation_intent::StepExecutionScope::WholeInput,
                failure_policy: Default::default(),
                executor: crate::transformation_intent::PlannedExecutor::Deterministic {
                    operation_ref: "builtin:uppercase".to_string(),
                    config_json: None,
                },
            }],
        };
        let transform = db.create_saved_transform("Uppercase", &plan, None).unwrap();
        db.apply_transform_output_to_clip(TransformClipApplication {
            clip_id: clip.id,
            transform_ref: &transform.stable_ref,
            expected_input: "hello",
            output: "HELLO",
            connection_id: None,
            duration_ms: 1,
            bin_move: None,
        })
        .unwrap();
        let path = db.path.lock().clone();
        {
            let conn = db.conn.lock();
            conn.execute("DROP INDEX idx_clip_transformations_ref", [])
                .unwrap();
            conn.execute(
                "ALTER TABLE clip_transformations DROP COLUMN transform_ref",
                [],
            )
            .unwrap();
        }
        drop(db);

        let migrated = DbState::new(path).unwrap();
        assert_eq!(
            migrated
                .get_clip_transformation_provenance(clip.id)
                .unwrap()
                .unwrap()
                .transform_ref,
            transform.stable_ref
        );
    }

    #[test]
    fn transform_bin_drop_revision_restores_content_and_previous_bin_only() {
        let db = setup_test_db();
        let source_bin = db.create_bin("Source", "📥", "#111111", None).unwrap();
        let destination_bin = db.create_bin("Markdown", "📝", "#222222", None).unwrap();
        let tag = db
            .create_bin_with_type("Important", "⭐", "#333333", None, "tag")
            .unwrap();
        let clip = db
            .save_clip("text", Some("hello"), None, None, "compound-undo", "Test")
            .unwrap();
        db.add_clip_to_bin(clip.id, tag.id).unwrap();
        db.assign_to_bin(clip.id, Some(source_bin.id)).unwrap();
        let plan = crate::transformation_intent::TransformationPlan {
            schema_version: crate::transformation_intent::TRANSFORMATION_PLAN_SCHEMA_VERSION,
            intent: "Uppercase".to_string(),
            summary: "Uppercase text".to_string(),
            planning_mode: crate::transformation_intent::IntentPlanningMode::Pinned,
            steps: vec![crate::transformation_intent::PlannedTransformationStep {
                name: "Uppercase".to_string(),
                rationale: "Replayable".to_string(),
                scope: crate::transformation_intent::StepExecutionScope::WholeInput,
                failure_policy: Default::default(),
                executor: crate::transformation_intent::PlannedExecutor::Deterministic {
                    operation_ref: "builtin:uppercase".to_string(),
                    config_json: None,
                },
            }],
        };
        let transform = db.create_saved_transform("Uppercase", &plan, None).unwrap();

        db.assign_to_bin(clip.id, Some(destination_bin.id)).unwrap();
        db.apply_transform_output_to_clip(TransformClipApplication {
            clip_id: clip.id,
            transform_ref: &transform.stable_ref,
            expected_input: "hello",
            output: "HELLO",
            connection_id: None,
            duration_ms: 3,
            bin_move: Some((Some(source_bin.id), destination_bin.id)),
        })
        .unwrap();
        let version = db.get_clip_versions(clip.id).unwrap().remove(0);
        assert_eq!(
            version.action_label.as_deref(),
            Some("Moved to Markdown · Applied Uppercase")
        );
        assert!(version.restores_organization);

        let restored = db.restore_clip_version(clip.id, version.id).unwrap();
        assert_eq!(restored.text_content.as_deref(), Some("hello"));
        assert_eq!(restored.bin_id, Some(source_bin.id));
        assert!(!restored.is_transformed);
        assert!(db
            .get_clip_transformation_provenance(clip.id)
            .unwrap()
            .is_none());
        assert!(restored.bin_ids.unwrap_or_default().contains(&tag.id));

        let inverse = db.get_clip_versions(clip.id).unwrap().remove(0);
        assert_eq!(inverse.text_content, "HELLO");
        assert!(inverse.restores_organization);
        let redone = db.restore_clip_version(clip.id, inverse.id).unwrap();
        assert_eq!(redone.text_content.as_deref(), Some("HELLO"));
        assert_eq!(redone.bin_id, Some(destination_bin.id));
        assert!(redone.is_transformed);
        assert_eq!(
            db.get_clip_transformation_provenance(clip.id)
                .unwrap()
                .unwrap()
                .transform_name,
            "Uppercase"
        );
    }

    #[test]
    fn clip_collection_pages_and_summary_cover_active_and_trashed_clips() {
        let db = setup_test_db();
        let empty = db.get_clip_collection_summary().unwrap();
        assert_eq!(empty.active_count, 0);
        assert_eq!(empty.trash_count, 0);

        let clips = (0..6)
            .map(|index| {
                db.save_clip(
                    if index % 2 == 0 { "text" } else { "link" },
                    Some(&format!("clip {index}")),
                    None,
                    None,
                    &format!("paged-clip-{index}"),
                    if index < 4 { "Editor" } else { "Browser" },
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        db.toggle_pin(clips[0].id).unwrap();
        db.toggle_protected(clips[1].id).unwrap();
        db.update_clip_note(clips[2].id, Some("Remember this"))
            .unwrap();
        db.delete_clip(clips[5].id).unwrap();
        db.delete_clip(clips[4].id).unwrap();

        let first = db
            .get_clips_page(None, None, false, Some(2), Some(0))
            .unwrap();
        let second = db
            .get_clips_page(None, None, false, Some(2), Some(2))
            .unwrap();
        assert_eq!(first.len(), 2);
        assert_eq!(second.len(), 2);
        assert!(first
            .iter()
            .all(|left| second.iter().all(|right| left.id != right.id)));
        assert_eq!(
            db.get_trashed_clips_page(Some(1), Some(0)).unwrap().len(),
            1
        );
        assert_eq!(
            db.get_trashed_clips_page(Some(1), Some(1)).unwrap().len(),
            1
        );

        let summary = db.get_clip_collection_summary().unwrap();
        assert_eq!(summary.active_count, 4);
        assert_eq!(summary.trash_count, 2);
        assert_eq!(summary.pinned_count, 1);
        assert_eq!(summary.protected_count, 1);
        assert_eq!(summary.noted_count, 1);
        assert_eq!(
            summary
                .type_counts
                .iter()
                .map(|item| item.count)
                .sum::<i64>(),
            4
        );
        assert_eq!(
            summary
                .source_counts
                .iter()
                .map(|item| item.count)
                .sum::<i64>(),
            4
        );
    }

    #[test]
    fn full_backup_round_trip_covers_every_durable_table_and_interface_state() {
        let db = setup_test_db();
        let active_path = db.database_path();
        let backup_path = active_path.with_extension("pastedbackup");
        let clip = db
            .save_clip(
                "text",
                Some("complete backup marker"),
                None,
                None,
                "full-backup-marker",
                "Tests",
            )
            .unwrap();
        db.update_clip_text(clip.id, "updated backup marker")
            .unwrap();
        db.record_analysis_classification(
            clip.id,
            &clip.content_hash,
            Some("prose"),
            Some("prose"),
            "original_text",
        )
        .unwrap();
        db.save_setting("fullBackupSetting", "preserved").unwrap();
        db.log_activity("app_started", "Complete backup test")
            .unwrap();
        db.create_intelligence_connection(
            "Backup Connection",
            "openai_compatible",
            Some("http://127.0.0.1:1234/v1"),
            Some("local-model"),
            Some("keychain:pasted:test"),
        )
        .unwrap();
        let extractor = db.get_content_extractors().unwrap().remove(0);
        db.update_content_extractor(
            extractor.id,
            &crate::content_extraction::ExtractorInput {
                name: "Backup Extractor Marker".into(),
                description: extractor.description,
                enabled: false,
                priority: 77,
            },
        )
        .unwrap();

        let client_state = r#"{"version":1,"localStorage":{"pasted_sidebar_width":"280"}}"#;
        let window_state = r#"{"main":{"width":1200,"height":800}}"#;
        let report = db
            .create_full_backup(&backup_path, Some(client_state), Some(window_state))
            .unwrap();
        assert!(report.size_bytes > 0);
        let inspection = db.inspect_full_backup(&backup_path).unwrap();
        assert_eq!(inspection.format_version, FULL_BACKUP_FORMAT_VERSION);
        assert_eq!(inspection.created_at, report.created_at);
        assert_eq!(inspection.size_bytes, report.size_bytes);

        let table_names = |connection: &Connection| -> Vec<String> {
            let mut statement = connection
                .prepare(
                    "SELECT name FROM sqlite_master
                     WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
                     ORDER BY name",
                )
                .unwrap();
            statement
                .query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<Vec<_>>>()
                .unwrap()
        };
        let source_tables = table_names(&db.conn.lock());
        let backup_connection = Connection::open(&backup_path).unwrap();
        let backup_tables = table_names(&backup_connection);
        for table in source_tables {
            assert!(
                backup_tables.contains(&table),
                "full backup omitted durable table {table}"
            );
        }
        assert!(backup_tables.contains(&"pasted_backup_manifest".to_string()));
        drop(backup_connection);

        db.save_setting("fullBackupSetting", "mutated").unwrap();
        db.save_clip(
            "text",
            Some("post-backup marker"),
            None,
            None,
            "post-backup-marker",
            "Tests",
        )
        .unwrap();
        let (restore_report, restored_client_state, restored_window_state) = db
            .restore_full_backup(&backup_path, Some("{}"), Some("{}"))
            .unwrap();

        assert_eq!(restored_client_state.as_deref(), Some(client_state));
        assert_eq!(restored_window_state.as_deref(), Some(window_state));
        assert_eq!(
            db.get_setting("fullBackupSetting").unwrap().as_deref(),
            Some("preserved")
        );
        assert_eq!(db.get_all_clips_for_backup().unwrap().len(), 1);
        assert!(!db.get_clip_versions(clip.id).unwrap().is_empty());
        assert_eq!(
            db.get_analysis_classification(clip.id)
                .unwrap()
                .unwrap()
                .content_type,
            "prose"
        );
        assert!(db
            .get_activity_logs(None, None)
            .unwrap()
            .iter()
            .any(|entry| entry.event_type == "app_started"));
        assert_eq!(db.get_intelligence_connections().unwrap().len(), 1);
        let restored_extractor = db.get_content_extractor(&extractor.stable_ref).unwrap();
        assert_eq!(restored_extractor.name, "Backup Extractor Marker");
        assert!(!restored_extractor.enabled);
        assert_eq!(restored_extractor.priority, 77);
        assert!(Path::new(&restore_report.recovery_path).is_file());
        assert_eq!(
            db.consume_pending_full_restore_client_state()
                .unwrap()
                .as_deref(),
            Some(client_state)
        );
        assert!(db
            .consume_pending_full_restore_client_state()
            .unwrap()
            .is_none());

        let _ = fs::remove_file(backup_path);
        let _ = fs::remove_file(restore_report.recovery_path);
    }

    #[test]
    fn full_restore_rejects_invalid_embedded_state_before_replacing_library() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("pasted-invalid-backup-{unique}"));
        fs::create_dir_all(&directory).unwrap();
        let db = DbState::new(directory.join("library.db")).unwrap();
        let backup_path = db.database_path().with_extension("pastedbackup");
        db.save_setting("liveStateMarker", "untouched").unwrap();
        db.create_full_backup(&backup_path, Some("{}"), Some("{}"))
            .unwrap();
        let backup = Connection::open(&backup_path).unwrap();
        backup
            .execute(
                "UPDATE pasted_backup_manifest SET client_state_json = 'not-json'",
                [],
            )
            .unwrap();
        let _ = backup.pragma_update(None, "wal_checkpoint", "TRUNCATE");
        drop(backup);

        assert!(db.inspect_full_backup(&backup_path).is_err());
        assert!(db
            .restore_full_backup(&backup_path, Some("{}"), Some("{}"))
            .is_err());
        assert_eq!(
            db.get_setting("liveStateMarker").unwrap().as_deref(),
            Some("untouched")
        );
        let recovery_count = fs::read_dir(&directory)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("Pasted_Pre_Restore_")
            })
            .count();
        assert_eq!(recovery_count, 0);
        let _ = fs::remove_file(backup_path);
        drop(db);
        let _ = fs::remove_dir_all(directory);
    }
}
