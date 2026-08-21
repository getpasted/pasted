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
use clip_search::{clip_search_feature_policy, parse_clip_search};
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

#[derive(Clone, Copy)]
struct SmartBinFeaturePolicy {
    clip_types: bool,
    content_types: bool,
    file_formats: bool,
    sources: bool,
}

fn smart_bin_feature_policy(conn: &Connection) -> Result<SmartBinFeaturePolicy> {
    conn.query_row(
        "SELECT
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableClipTypes' AND value IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableTypes' AND value IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableFileFormats' AND value IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableSources' AND value IN ('false', '0'))",
        [],
        |row| {
            Ok(SmartBinFeaturePolicy {
                clip_types: row.get(0)?,
                content_types: row.get(1)?,
                file_formats: row.get(2)?,
                sources: row.get(3)?,
            })
        },
    )
}

fn push_smart_condition(
    kind: &str,
    operator: &str,
    value: &str,
    features: SmartBinFeaturePolicy,
    conditions: &mut Vec<String>,
    parameters: &mut Vec<Box<dyn ToSql>>,
) {
    let value = value.trim();
    if value.is_empty() {
        return;
    }
    let enabled = match kind {
        "clip_type" => features.clip_types,
        "content_type" => features.content_types,
        "file_format" => features.file_formats,
        "source" => features.sources,
        _ => true,
    };
    if !enabled {
        conditions.push("0".into());
        return;
    }
    let contains = operator == "contains"
        || (operator.is_empty() && matches!(kind, "source" | "contains" | "file_path"));
    let condition = match kind {
        "clip_type" => {
            if contains {
                parameters.push(Box::new(format!(
                    "%{}%",
                    escape_like_literal(&value.to_lowercase())
                )));
                "LOWER(content_type) LIKE ? ESCAPE '\\'".to_string()
            } else {
                parameters.push(Box::new(value.to_lowercase()));
                "LOWER(content_type) = ?".to_string()
            }
        }
        "content_type" => {
            parameters.push(Box::new(if contains {
                format!("%{}%", escape_like_literal(&value.to_lowercase()))
            } else {
                value.to_lowercase()
            }));
            "EXISTS (
                SELECT 1 FROM clip_analysis_classifications AS classified
                WHERE classified.clip_id = clips.id
                  AND classified.input_hash = clips.content_hash
                  AND LOWER(classified.content_type) "
                .to_string()
                + if contains {
                    "LIKE ? ESCAPE '\\'"
                } else {
                    "= ?"
                }
                + "
            )"
        }
        "file_format" => {
            parameters.push(Box::new(
                crate::content_inspection::FILE_FORMAT_INSPECTOR_REF.to_string(),
            ));
            parameters.push(Box::new(
                crate::analysis_contract::ANALYSIS_CONTRACT_VERSION,
            ));
            parameters.push(Box::new(if contains {
                format!("%{}%", escape_like_literal(&value.to_lowercase()))
            } else {
                value.to_lowercase()
            }));
            "EXISTS (
                SELECT 1
                FROM clip_analysis_results AS formats,
                     json_each(formats.result_json, '$.formats') AS detected
                WHERE formats.clip_id = clips.id
                  AND formats.participant_ref = ?
                  AND formats.content_hash = clips.content_hash
                  AND formats.input_hash = clips.content_hash
                  AND formats.format_version = ?
                  AND LOWER(json_extract(detected.value, '$.format')) "
                .to_string()
                + if contains {
                    "LIKE ? ESCAPE '\\'"
                } else {
                    "= ?"
                }
                + "
            )"
        }
        "origin_kind" => {
            parameters.push(Box::new(value.to_lowercase()));
            "CASE WHEN content_type IN ('image', 'file') AND (LOWER(source) LIKE '%screenshot%' OR LOWER(source) LIKE '%screencapture%' OR LOWER(source) LIKE '%cleanshot%') THEN 'screenshot' WHEN content_type = 'file' THEN 'file_reference' WHEN LOWER(source) IN ('cli terminal', 'pasted cli') THEN 'command_line' ELSE 'clipboard_content' END = ?".to_string()
        }
        "source" => {
            if contains {
                parameters.push(Box::new(format!(
                    "%{}%",
                    escape_like_literal(&value.to_lowercase())
                )));
                "LOWER(source) LIKE ? ESCAPE '\\'".to_string()
            } else {
                parameters.push(Box::new(value.to_lowercase()));
                "LOWER(source) = ?".to_string()
            }
        }
        "contains" => {
            let pattern = format!("%{}%", value);
            parameters.push(Box::new(pattern.clone()));
            parameters.push(Box::new(pattern));
            "(text_content LIKE ? OR EXISTS (
                SELECT 1 FROM clip_searchable_text AS extracted
                WHERE extracted.clip_id = clips.id
                  AND extracted.input_hash = clips.content_hash
                  AND extracted.searchable_text LIKE ?
            ))"
            .to_string()
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
    let features = smart_bin_feature_policy(conn)?;
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
        let parsed = crate::smart_bins::parse_rule_json(&smart_rule).ok();
        if let Some(rule) = parsed.as_ref() {
            for condition in &rule.conditions {
                push_smart_condition(
                    &condition.target,
                    &condition.operator,
                    &condition.value,
                    features,
                    &mut conditions,
                    &mut parameters,
                );
            }
        }
        let join = if parsed.as_ref().is_some_and(|rule| rule.match_mode == "all") {
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

    for clip in clips.iter_mut() {
        let bin_ids = clip.bin_ids.get_or_insert_with(Vec::new);
        for bin_id in memberships.remove(&clip.id).unwrap_or_default() {
            if !bin_ids.contains(&bin_id) {
                bin_ids.push(bin_id);
            }
        }
    }
    append_clip_content_types(conn, clips)?;
    append_clip_file_formats(conn, clips)?;
    append_clip_protection(conn, clips)?;
    append_clip_concealment(conn, clips)?;
    append_clip_names(conn, clips)?;
    Ok(())
}

fn append_clip_content_types(conn: &Connection, clips: &mut [ClipItem]) -> Result<()> {
    if clips.is_empty() {
        return Ok(());
    }
    let requested_ids = clips.iter().map(|clip| clip.id).collect::<HashSet<_>>();
    let ids_json = serde_json::to_string(&requested_ids.iter().copied().collect::<Vec<_>>())
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    let mut by_clip = HashMap::<i64, Vec<String>>::new();
    let mut statement = conn.prepare(
        "SELECT classifications.clip_id, classifications.content_type
         FROM clip_analysis_classifications AS classifications
         LEFT JOIN content_classifiers AS classifiers
           ON classifiers.stable_ref = classifications.classifier_ref
         JOIN clips ON clips.id = classifications.clip_id
         WHERE classifications.clip_id IN (
             SELECT CAST(value AS INTEGER) FROM json_each(?1)
         ) AND classifications.input_hash = clips.content_hash
         GROUP BY classifications.clip_id, classifications.content_type
         ORDER BY classifications.clip_id, MIN(COALESCE(classifiers.priority, 10000)),
                  classifications.content_type COLLATE NOCASE",
    )?;
    for row in statement.query_map(params![ids_json], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })? {
        let (clip_id, content_type) = row?;
        if requested_ids.contains(&clip_id) {
            by_clip.entry(clip_id).or_default().push(content_type);
        }
    }
    for clip in clips {
        clip.content_types = by_clip.remove(&clip.id).unwrap_or_default();
    }
    Ok(())
}

fn append_clip_file_formats(conn: &Connection, clips: &mut [ClipItem]) -> Result<()> {
    if clips.is_empty() {
        return Ok(());
    }
    let requested_ids = clips.iter().map(|clip| clip.id).collect::<HashSet<_>>();
    let ids_json = serde_json::to_string(&requested_ids.iter().copied().collect::<Vec<_>>())
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    let mut by_clip = HashMap::<i64, Vec<String>>::new();
    let mut statement = conn.prepare(
        "SELECT results.clip_id, LOWER(json_extract(detected.value, '$.format'))
         FROM clip_analysis_results AS results
         JOIN clips ON clips.id = results.clip_id,
              json_each(results.result_json, '$.formats') AS detected
         WHERE results.clip_id IN (SELECT CAST(value AS INTEGER) FROM json_each(?1))
           AND results.participant_ref = ?2
           AND results.content_hash = clips.content_hash
           AND results.input_hash = clips.content_hash
           AND results.format_version = ?3
         ORDER BY results.clip_id, CAST(json_extract(detected.value, '$.format') AS TEXT) COLLATE NOCASE",
    )?;
    for row in statement.query_map(
        params![
            ids_json,
            crate::content_inspection::FILE_FORMAT_INSPECTOR_REF,
            crate::analysis_contract::ANALYSIS_CONTRACT_VERSION,
        ],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
    )? {
        let (clip_id, format) = row?;
        if requested_ids.contains(&clip_id) {
            by_clip.entry(clip_id).or_default().push(format);
        }
    }
    for clip in clips {
        clip.file_formats = by_clip.remove(&clip.id).unwrap_or_default();
    }
    Ok(())
}

fn append_clip_protection(conn: &Connection, clips: &mut [ClipItem]) -> Result<()> {
    if clips.is_empty() {
        return Ok(());
    }
    let ids = clips.iter().map(|clip| clip.id).collect::<Vec<_>>();
    let ids_json = serde_json::to_string(&ids)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    let mut protection = HashMap::<i64, (bool, Vec<i64>)>::new();
    let mut statement = conn.prepare(
        "SELECT clip_id, is_protected, protecting_bin_ids
         FROM effective_clip_protection
         WHERE clip_id IN (SELECT CAST(value AS INTEGER) FROM json_each(?1))",
    )?;
    for row in statement.query_map(params![ids_json], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i32>(1)? != 0,
            row.get::<_, Option<String>>(2)?,
        ))
    })? {
        let (clip_id, is_protected, bin_ids) = row?;
        let bin_ids = bin_ids
            .unwrap_or_default()
            .split(',')
            .filter_map(|value| value.parse::<i64>().ok())
            .collect();
        protection.insert(clip_id, (is_protected, bin_ids));
    }
    for clip in clips {
        clip.is_explicitly_protected = Some(clip.is_protected);
        if let Some((is_protected, bin_ids)) = protection.remove(&clip.id) {
            clip.is_protected = is_protected;
            clip.protecting_bin_ids = bin_ids;
        }
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClipItem {
    pub id: i64,
    #[serde(default)]
    pub name: Option<String>,
    pub content_type: String, // Physical Clip Type: "text", "image", or "file".
    #[serde(default)]
    pub content_types: Vec<String>,
    #[serde(default)]
    pub file_formats: Vec<String>,
    pub text_content: Option<String>,
    pub html_content: Option<String>,
    pub image_base64: Option<String>,
    pub image_path: Option<String>,
    pub content_hash: String,
    #[serde(alias = "source_app")]
    pub source: String,
    pub is_pinned: bool,
    /// Effective protection, including explicit, shortcut, and inherited Bin protection.
    pub is_protected: bool,
    /// The durable per-clip protection bit. Absent in legacy transfer archives.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_explicitly_protected: Option<bool>,
    #[serde(default)]
    pub protecting_bin_ids: Vec<i64>,
    /// Effective concealment from the clip, a Content Type, or a manual Bin.
    #[serde(default)]
    pub is_concealed: bool,
    /// The durable per-clip concealment bit. Absent in legacy transfer archives.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_explicitly_concealed: Option<bool>,
    /// A durable per-clip reveal overrides inherited concealment.
    #[serde(default)]
    pub is_explicitly_revealed: bool,
    #[serde(default)]
    pub concealing_bin_ids: Vec<i64>,
    #[serde(default)]
    pub concealing_content_types: Vec<String>,
    #[serde(default)]
    #[serde(rename = "hotkey", alias = "shortcut")]
    pub shortcut: Option<String>,
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

fn clip_item_from_row(row: &Row<'_>) -> Result<ClipItem> {
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
        name: None,
        content_type: row.get(1)?,
        content_types: Vec::new(),
        file_formats: Vec::new(),
        text_content: row.get(2)?,
        html_content: row.get(3)?,
        image_base64: row.get(4)?,
        image_path: row.get(5)?,
        content_hash: row.get(6)?,
        source: row.get(7)?,
        is_pinned: row.get::<_, i32>(8)? != 0,
        is_protected: row.get::<_, i32>(9)? != 0,
        is_explicitly_protected: Some(row.get::<_, i32>(9)? != 0),
        protecting_bin_ids: Vec::new(),
        is_concealed: false,
        is_explicitly_concealed: None,
        is_explicitly_revealed: false,
        concealing_bin_ids: Vec::new(),
        concealing_content_types: Vec::new(),
        shortcut: row.get(21).unwrap_or(None),
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
}

pub const DEFAULT_CLIP_SEARCH_PAGE_SIZE: usize = 100;
pub const MAX_CLIP_SEARCH_PAGE_SIZE: usize = 500;
const MAX_CLIP_SEARCH_QUERY_BYTES: usize = 4 * 1024;
const MAX_CLIP_SEARCH_FILTERS: usize = 32;
const MAX_CLIP_SEARCH_TERMS: usize = 32;
const MAX_CLIP_SEARCH_OFFSET: usize = 10_000_000;

/// Authoritative Search request shared by the app, Quick HUD, and CLI.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct ClipSearchRequest {
    pub query: String,
    pub clip_types: Vec<String>,
    pub content_types: Vec<String>,
    pub file_formats: Vec<String>,
    pub sources: Vec<String>,
    pub trash: bool,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClipSearchResult {
    pub schema_version: u32,
    pub items: Vec<ClipItem>,
    pub total_count: usize,
    pub limit: usize,
    pub offset: usize,
}

fn normalize_imported_clip_types(clip: &mut ClipItem) -> Result<()> {
    if !matches!(clip.content_type.as_str(), "text" | "image" | "file") {
        clip.content_types.push(clip.content_type.clone());
        clip.content_type = "text".into();
    }
    clip.content_types.sort();
    clip.content_types.dedup();
    if clip.content_types.len() > crate::content_classification::MAX_CLASSIFICATION_MATCHES_PER_CLIP
        || clip.content_types.iter().any(|content_type| {
            content_type.is_empty()
                || content_type.len() > 80
                || !content_type.chars().all(|character| {
                    character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
                })
        })
    {
        return Err(rusqlite::Error::InvalidParameterName(
            "Imported Content Types exceed their safety limit".into(),
        ));
    }
    Ok(())
}

fn replace_imported_content_types(
    conn: &Connection,
    clip_id: i64,
    content_hash: &str,
    clip_type: &str,
    content_types: &[String],
) -> Result<()> {
    conn.execute(
        "DELETE FROM clip_analysis_classifications WHERE clip_id = ?1",
        [clip_id],
    )?;
    let source_representation = if matches!(clip_type, "image" | "file") {
        "searchable_text"
    } else {
        "original_text"
    };
    for content_type in content_types {
        let classifier_ref = conn
            .query_row(
                "SELECT stable_ref FROM content_classifiers
                 WHERE content_type = ?1 AND is_deleted = 0
                 ORDER BY priority, id LIMIT 1",
                [content_type],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_else(|| format!("transfer:{content_type}"));
        conn.execute(
            "INSERT INTO clip_analysis_classifications
                (clip_id, content_type, classifier_ref, source_representation, input_hash,
                 start_offset, end_offset)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL)",
            params![
                clip_id,
                content_type,
                classifier_ref,
                source_representation,
                content_hash
            ],
        )?;
    }
    Ok(())
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
    pub fn get_total_clip_count(&self) -> Result<i64> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT COUNT(*) FROM clips WHERE is_trashed IS NULL OR is_trashed = 0",
            [],
            |r| r.get(0),
        )
    }

    pub fn search_clips(&self, request: &ClipSearchRequest) -> Result<ClipSearchResult> {
        if request.query.len() > MAX_CLIP_SEARCH_QUERY_BYTES {
            return Err(rusqlite::Error::InvalidParameterName(
                "Search query exceeds its safety limit".into(),
            ));
        }
        if request.offset > MAX_CLIP_SEARCH_OFFSET {
            return Err(rusqlite::Error::InvalidParameterName(
                "Search offset exceeds its safety limit".into(),
            ));
        }
        if request.limit > MAX_CLIP_SEARCH_PAGE_SIZE {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "Search limit must not exceed {MAX_CLIP_SEARCH_PAGE_SIZE}"
            )));
        }
        let requested_filter_count = request.clip_types.len()
            + request.content_types.len()
            + request.file_formats.len()
            + request.sources.len();
        if requested_filter_count > MAX_CLIP_SEARCH_FILTERS {
            return Err(rusqlite::Error::InvalidParameterName(
                "Search filters exceed their safety limit".into(),
            ));
        }
        let validate_filter = |value: &String| {
            !value.trim().is_empty() && value.len() <= 256 && !value.contains('\0')
        };
        if request
            .clip_types
            .iter()
            .chain(&request.content_types)
            .chain(&request.file_formats)
            .chain(&request.sources)
            .any(|value| !validate_filter(value))
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "Search filter is empty or exceeds its safety limit".into(),
            ));
        }

        let limit = if request.limit == 0 {
            DEFAULT_CLIP_SEARCH_PAGE_SIZE
        } else {
            request.limit
        };
        let offset = request.offset;
        let mut parsed = parse_clip_search(&request.query);
        parsed.clip_types.extend(
            request
                .clip_types
                .iter()
                .map(|value| value.trim().to_lowercase()),
        );
        parsed.content_types.extend(
            request
                .content_types
                .iter()
                .map(|value| value.trim().to_lowercase()),
        );
        parsed.file_formats.extend(
            request
                .file_formats
                .iter()
                .map(|value| value.trim().to_lowercase()),
        );
        parsed.sources.extend(
            request
                .sources
                .iter()
                .map(|value| value.trim().to_lowercase()),
        );
        parsed.requires_trashed |= request.trash;
        let parsed_filter_count = parsed.clip_types.len()
            + parsed.content_types.len()
            + parsed.file_formats.len()
            + parsed.sources.len();
        if parsed_filter_count > MAX_CLIP_SEARCH_FILTERS
            || parsed.terms.len() > MAX_CLIP_SEARCH_TERMS
            || parsed.terms.iter().any(|term| term.len() > 256)
            || parsed
                .clip_types
                .iter()
                .chain(&parsed.content_types)
                .chain(&parsed.file_formats)
                .chain(&parsed.sources)
                .any(|value| value.len() > 256 || value.contains('\0'))
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "Search terms or filters exceed their safety limit".into(),
            ));
        }

        let conn = self.conn.lock();
        let features = clip_search_feature_policy(&conn)?;
        let gated_filter = (!features.clip_types && !parsed.clip_types.is_empty())
            || (!features.content_types && !parsed.content_types.is_empty())
            || (!features.file_formats && !parsed.file_formats.is_empty())
            || (!features.sources && !parsed.sources.is_empty())
            || (!features.notes && parsed.requires_note)
            || (!features.naming && parsed.requires_named)
            || (!features.pinning && parsed.requires_pinned)
            || (!features.protection && parsed.requires_protected)
            || (!features.trash && parsed.requires_trashed);
        if parsed.incomplete || gated_filter {
            return Ok(ClipSearchResult {
                schema_version: 1,
                items: Vec::new(),
                total_count: 0,
                limit,
                offset,
            });
        }

        let mut clauses = vec![if parsed.requires_trashed {
            "COALESCE(clips.is_trashed, 0) = 1".to_string()
        } else {
            "COALESCE(clips.is_trashed, 0) = 0".to_string()
        }];
        let mut parameters: Vec<Box<dyn ToSql>> = Vec::new();
        if parsed.requires_note {
            clauses.push("TRIM(COALESCE(clips.note, '')) <> ''".into());
        }
        if parsed.requires_named {
            clauses.push("TRIM(COALESCE(clips.name, '')) <> ''".into());
        }
        if parsed.requires_pinned {
            clauses.push("COALESCE(clips.is_pinned, 0) = 1".into());
        }
        if parsed.requires_protected {
            clauses.push("clips.id IN (SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1)".into());
        }
        for value in &parsed.clip_types {
            clauses.push("LOWER(clips.content_type) LIKE ? ESCAPE '\\'".into());
            parameters.push(Box::new(format!("%{}%", escape_like_literal(value))));
        }
        for value in &parsed.sources {
            clauses.push("LOWER(clips.source) LIKE ? ESCAPE '\\'".into());
            parameters.push(Box::new(format!("%{}%", escape_like_literal(value))));
        }
        for value in &parsed.content_types {
            clauses.push(
                "EXISTS (SELECT 1 FROM clip_analysis_classifications AS classified
                         WHERE classified.clip_id = clips.id
                           AND classified.input_hash = clips.content_hash
                           AND LOWER(classified.content_type) LIKE ? ESCAPE '\\')"
                    .into(),
            );
            parameters.push(Box::new(format!("%{}%", escape_like_literal(value))));
        }
        for value in &parsed.file_formats {
            clauses.push(
                "EXISTS (SELECT 1 FROM clip_analysis_results AS formats,
                                      json_each(formats.result_json, '$.formats') AS detected
                         WHERE formats.clip_id = clips.id
                           AND formats.participant_ref = ?
                           AND formats.content_hash = clips.content_hash
                           AND formats.input_hash = clips.content_hash
                           AND formats.format_version = ?
                           AND LOWER(CAST(json_extract(detected.value, '$.format') AS TEXT)) LIKE ? ESCAPE '\\')"
                    .into(),
            );
            parameters.push(Box::new(
                crate::content_inspection::FILE_FORMAT_INSPECTOR_REF.to_string(),
            ));
            parameters.push(Box::new(
                crate::analysis_contract::ANALYSIS_CONTRACT_VERSION,
            ));
            parameters.push(Box::new(format!("%{}%", escape_like_literal(value))));
        }
        if parsed.regex.is_none() && parsed.regex_fallback.is_none() {
            for term in &parsed.terms {
                let indexed_fts_like =
                    term.chars().count() >= 3 && !term.contains(['%', '_', '\\']);
                let fts_like = if indexed_fts_like {
                    "LIKE ?"
                } else {
                    "LIKE ? ESCAPE '\\'"
                };
                let mut fields = vec![
                    format!(
                        "clips.id IN (SELECT rowid FROM clips_fts
                                           WHERE text_content {fts_like})"
                    ),
                    format!(
                        "(clips.id IN (SELECT rowid FROM clip_searchable_text_fts
                                            WHERE searchable_text {fts_like})
                      AND EXISTS (SELECT 1 FROM clip_searchable_text AS extracted
                                  WHERE extracted.clip_id = clips.id
                                    AND extracted.input_hash = clips.content_hash))"
                    ),
                ];
                if features.sources {
                    fields.push(format!(
                        "clips.id IN (SELECT rowid FROM clips_fts WHERE source {fts_like})"
                    ));
                }
                if features.notes {
                    fields.push(format!(
                        "clips.id IN (SELECT rowid FROM clips_fts WHERE note {fts_like})"
                    ));
                }
                if features.naming {
                    fields.push(format!(
                        "clips.id IN (SELECT rowid FROM clips_fts WHERE name {fts_like})"
                    ));
                }
                if features.clip_types {
                    fields.push("LOWER(clips.content_type) LIKE ? ESCAPE '\\'".into());
                }
                if features.content_types {
                    fields.push(
                        "EXISTS (SELECT 1 FROM clip_analysis_classifications AS classified
                                 WHERE classified.clip_id = clips.id
                                   AND classified.input_hash = clips.content_hash
                                   AND LOWER(classified.content_type) LIKE ? ESCAPE '\\')"
                            .into(),
                    );
                }
                if features.file_formats {
                    fields.push(
                        "EXISTS (SELECT 1 FROM clip_analysis_results AS formats,
                                              json_each(formats.result_json, '$.formats') AS detected
                                 WHERE formats.clip_id = clips.id
                                   AND formats.participant_ref = ?
                                   AND formats.content_hash = clips.content_hash
                                   AND formats.input_hash = clips.content_hash
                                   AND formats.format_version = ?
                                   AND LOWER(CAST(json_extract(detected.value, '$.format') AS TEXT)) LIKE ? ESCAPE '\\')"
                            .into(),
                    );
                }
                clauses.push(format!("({})", fields.join(" OR ")));
                let pattern = format!("%{}%", escape_like_literal(term));
                parameters.push(Box::new(pattern.clone()));
                parameters.push(Box::new(pattern.clone()));
                if features.sources {
                    parameters.push(Box::new(pattern.clone()));
                }
                if features.notes {
                    parameters.push(Box::new(pattern.clone()));
                }
                if features.naming {
                    parameters.push(Box::new(pattern.clone()));
                }
                if features.clip_types {
                    parameters.push(Box::new(pattern.clone()));
                }
                if features.content_types {
                    parameters.push(Box::new(pattern.clone()));
                }
                if features.file_formats {
                    parameters.push(Box::new(
                        crate::content_inspection::FILE_FORMAT_INSPECTOR_REF.to_string(),
                    ));
                    parameters.push(Box::new(
                        crate::analysis_contract::ANALYSIS_CONTRACT_VERSION,
                    ));
                    parameters.push(Box::new(pattern));
                }
            }
        }

        let where_clause = clauses.join(" AND ");
        let parameter_refs = parameters
            .iter()
            .map(|parameter| parameter.as_ref())
            .collect::<Vec<&dyn ToSql>>();
        let regex_pattern = parsed.regex.as_ref().or(parsed.regex_fallback.as_ref());

        let (matching_ids, total_count) = if let Some(pattern) = regex_pattern {
            let regex = parsed.regex.as_ref().map(|_| {
                RegexBuilder::new(pattern)
                    .case_insensitive(true)
                    .build()
                    .expect("validated Search regular expression")
            });
            let mut statement = conn.prepare(&format!(
                "SELECT clips.id, clips.content_type, clips.text_content, clips.html_content,
                        clips.image_base64, clips.image_path, clips.content_hash, clips.source,
                        clips.is_pinned, clips.is_protected, COALESCE(clips.pin_order, 0),
                        clips.bin_id, clips.note, COALESCE(clips.is_trashed, 0), clips.trashed_at,
                        clips.created_at,
                        (SELECT GROUP_CONCAT(bin_id) FROM clip_bins WHERE clip_id = clips.id),
                        clips.current_transformation_id IS NOT NULL,
                        clips.ocr_extractor_ref, clips.ocr_extractor_name, clips.ocr_engine_version,
                        clips.shortcut,
                        COALESCE((SELECT extracted.searchable_text
                                  FROM clip_searchable_text AS extracted
                                  WHERE extracted.clip_id = clips.id
                                    AND extracted.input_hash = clips.content_hash), '')
                 FROM clips WHERE {where_clause}
                 ORDER BY clips.created_at DESC, clips.id DESC"
            ))?;
            let candidates = statement
                .query_map(parameter_refs.as_slice(), |row| {
                    Ok((clip_item_from_row(row)?, row.get::<_, String>(22)?))
                })?
                .collect::<Result<Vec<_>>>()?;
            let (mut candidate_clips, extracted_texts): (Vec<_>, Vec<_>) =
                candidates.into_iter().unzip();
            append_clip_content_types(&conn, &mut candidate_clips)?;
            append_clip_file_formats(&conn, &mut candidate_clips)?;
            append_clip_protection(&conn, &mut candidate_clips)?;
            append_clip_names(&conn, &mut candidate_clips)?;
            let mut matching = Vec::new();
            for (clip, extracted_text) in candidate_clips.into_iter().zip(extracted_texts) {
                let mut values = vec![clip.text_content.as_deref().unwrap_or(""), &extracted_text];
                if features.sources {
                    values.push(&clip.source);
                }
                if features.notes {
                    values.push(clip.note.as_deref().unwrap_or(""));
                }
                if features.naming {
                    values.push(clip.name.as_deref().unwrap_or(""));
                }
                if features.clip_types {
                    values.push(&clip.content_type);
                }
                if features.content_types {
                    values.extend(clip.content_types.iter().map(String::as_str));
                }
                if features.file_formats {
                    values.extend(clip.file_formats.iter().map(String::as_str));
                }
                let matches = if let Some(regex) = &regex {
                    values.iter().any(|value| regex.is_match(value))
                } else {
                    values
                        .iter()
                        .any(|value| value.to_lowercase().contains(pattern))
                };
                if matches {
                    matching.push(clip.id);
                }
            }
            let total = matching.len();
            (
                matching.into_iter().skip(offset).take(limit).collect(),
                total,
            )
        } else {
            let total = conn.query_row(
                &format!("SELECT COUNT(*) FROM clips WHERE {where_clause}"),
                parameter_refs.as_slice(),
                sqlite_count,
            )?;
            let mut paged_parameters = parameters;
            paged_parameters.push(Box::new(limit as i64));
            paged_parameters.push(Box::new(offset as i64));
            let paged_refs = paged_parameters
                .iter()
                .map(|parameter| parameter.as_ref())
                .collect::<Vec<&dyn ToSql>>();
            let mut statement = conn.prepare(&format!(
                "SELECT clips.id FROM clips WHERE {where_clause}
                 ORDER BY clips.created_at DESC, clips.id DESC LIMIT ? OFFSET ?"
            ))?;
            let ids = statement
                .query_map(paged_refs.as_slice(), |row| row.get(0))?
                .collect::<Result<Vec<_>>>()?;
            (ids, total)
        };

        let mut items = Self::get_clips_by_ids_internal(&conn, &matching_ids)?;
        for item in &mut items {
            item.html_content = None;
            item.image_base64 = None;
        }
        Ok(ClipSearchResult {
            schema_version: 1,
            items,
            total_count,
            limit,
            offset,
        })
    }

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
