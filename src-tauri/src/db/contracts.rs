use super::*;

mod clip_version;
pub use clip_version::ClipVersion;

pub(super) const BACKUP_SCHEMA_VERSION: u32 = 14;
pub(super) const MAX_BACKUP_INTERFACE_STATE_BYTES: usize = 1024 * 1024;

pub(super) struct ClipSaveInput<'a> {
    pub(super) content_type: &'a str,
    pub(super) text_content: Option<&'a str>,
    pub(super) html_content: Option<&'a str>,
    pub(super) image_base64: Option<&'a str>,
    pub(super) content_hash: &'a str,
    pub(super) source: &'a str,
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

    pub(super) fn engine_only(engine_version: &'a str) -> Self {
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

impl ClipMutationSummary {
    pub(super) fn new(action: &str, requested_count: usize, clip_ids: Vec<i64>) -> Self {
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
pub(super) struct ClipRevisionContext {
    pub(super) schema_version: i64,
    pub(super) action_kind: String,
    pub(super) action_label: String,
    pub(super) organization: Option<ClipRevisionOrganization>,
    #[serde(default)]
    pub(super) current_transformation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) derived_state: Option<super::clip_revision_state::ClipRevisionDerivedState>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(super) struct ClipRevisionOrganization {
    pub(super) category_bin_id: Option<i64>,
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

pub(super) struct FullBackupManifest {
    pub(super) format_version: i64,
    pub(super) created_at: String,
    pub(super) client_state_json: Option<String>,
    pub(super) window_state_json: Option<String>,
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
    pub visual_label_overrides: Vec<super::clip_visual_labels::VisualLabelOverrideArchive>,
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
    pub(super) path: Mutex<PathBuf>,
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
