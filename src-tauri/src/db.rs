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
mod operations;
mod search_indexes;
pub use clip_names::clip_name_input_limit;
mod clip_protection;
mod clip_queries;
mod clip_revisions;
mod retention;
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
pub use operations::{Operation, ResolvedCustomOperation};
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

fn insert_default_bins(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT INTO bins (name, icon, color, smart_rule) VALUES ('Images', '🖼️', '#ec4899', '{\"version\":1,\"conditions\":[{\"type\":\"clip_type\",\"operator\":\"is\",\"value\":\"image\"}],\"match\":\"any\"}')",
        [],
    )?;
    conn.execute(
        "INSERT INTO bins (name, icon, color, smart_rule) VALUES ('Links and Web', 'Link', '#3b82f6', '{\"version\":1,\"conditions\":[{\"type\":\"content_type\",\"operator\":\"is\",\"value\":\"link\"}],\"match\":\"any\"}')",
        [],
    )?;
    conn.execute(
        "INSERT INTO bins (name, icon, color, smart_rule) VALUES ('Code Snippets', 'Code', '#10b981', '{\"version\":1,\"conditions\":[{\"type\":\"content_type\",\"operator\":\"is\",\"value\":\"code\"}],\"match\":\"any\"}')",
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

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<()> {
    if !column_exists(conn, table, column)? {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
            [],
        )?;
    }
    Ok(())
}

fn migrate_app_exclusion_hotkey_setting(conn: &Connection) -> Result<()> {
    if !table_exists(conn, "settings")? {
        return Ok(());
    }
    let stored: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'blacklistApps'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let Some(stored) = stored else {
        return Ok(());
    };
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&stored) else {
        return Ok(());
    };
    let Some(entries) = value.as_array_mut() else {
        return Ok(());
    };
    let mut changed = false;
    for entry in entries {
        let Some(rule) = entry.as_object_mut() else {
            continue;
        };
        let legacy = rule.remove("ignoreShortcuts");
        if let Some(legacy) = legacy {
            rule.entry("ignoreHotkeys").or_insert(legacy);
            changed = true;
        }
    }
    if changed {
        let serialized = serde_json::to_string(&value)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        conn.execute(
            "UPDATE settings SET value = ?1 WHERE key = 'blacklistApps'",
            params![serialized],
        )?;
    }
    Ok(())
}

struct NamedMigration {
    key: &'static str,
    apply: fn(&Connection) -> Result<()>,
}

fn run_named_migrations(conn: &Connection, migrations: &[NamedMigration]) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            key TEXT PRIMARY KEY,
            applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    for migration in migrations {
        let applied: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE key = ?1)",
            [migration.key],
            |row| row.get(0),
        )?;
        if applied {
            continue;
        }
        let transaction = conn.unchecked_transaction()?;
        (migration.apply)(&transaction)?;
        transaction.execute(
            "INSERT INTO schema_migrations (key) VALUES (?1)",
            [migration.key],
        )?;
        transaction.commit()?;
    }
    Ok(())
}

fn migrate_transform_activity_terminology(conn: &Connection) -> Result<()> {
    conn.execute(
        "UPDATE activity_logs
         SET event_type = replace(event_type, 'recipe_', 'transform_'),
             description = replace(replace(description, 'Recipes', 'Transforms'), 'Recipe', 'Transform')
         WHERE event_type LIKE '%recipe%' OR description LIKE '%Recipe%'",
        [],
    )?;
    Ok(())
}

fn backfill_current_transformation(conn: &Connection) -> Result<()> {
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
    Ok(())
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

fn migrate_multi_type_classifications(conn: &Connection) -> Result<()> {
    if !table_exists(conn, "clip_analysis_classifications")?
        || column_exists(conn, "clip_analysis_classifications", "start_offset")?
    {
        return Ok(());
    }
    let reference_column =
        if column_exists(conn, "clip_analysis_classifications", "classifier_ref")? {
            "classifier_ref"
        } else if column_exists(conn, "clip_analysis_classifications", "detector_ref")? {
            "detector_ref"
        } else {
            return Err(rusqlite::Error::InvalidParameterName(
                "Legacy classifications have no participant reference".into(),
            ));
        };
    let transaction = conn.unchecked_transaction()?;
    transaction.execute_batch(&format!(
        "DROP TABLE IF EXISTS clip_analysis_classifications_multi;
         CREATE TABLE clip_analysis_classifications_multi (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
            content_type TEXT NOT NULL,
            classifier_ref TEXT NOT NULL,
            source_representation TEXT NOT NULL
                CHECK (source_representation IN ('original_text', 'searchable_text')),
            input_hash TEXT NOT NULL,
            start_offset INTEGER,
            end_offset INTEGER,
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
            CHECK (
                (start_offset IS NULL AND end_offset IS NULL)
                OR (start_offset >= 0 AND end_offset > start_offset)
            )
         );
         INSERT INTO clip_analysis_classifications_multi
            (clip_id, content_type, classifier_ref, source_representation, input_hash,
             start_offset, end_offset, updated_at)
         SELECT clip_id, content_type, {reference_column}, source_representation, input_hash,
                NULL, NULL, updated_at
         FROM clip_analysis_classifications;
         DROP TABLE clip_analysis_classifications;
         ALTER TABLE clip_analysis_classifications_multi
            RENAME TO clip_analysis_classifications;"
    ))?;
    transaction.commit()
}

fn migrate_legacy_semantic_clip_types(conn: &Connection) -> Result<()> {
    let transaction = conn.unchecked_transaction()?;
    transaction.execute(
        "INSERT INTO clip_analysis_classifications
            (clip_id, content_type, classifier_ref, source_representation, input_hash,
             start_offset, end_offset)
         SELECT clips.id, clips.content_type,
                COALESCE(
                    (SELECT classifiers.stable_ref
                     FROM content_classifiers AS classifiers
                     WHERE classifiers.content_type = clips.content_type
                       AND classifiers.is_deleted = 0
                     ORDER BY classifiers.priority, classifiers.id
                     LIMIT 1),
                    'legacy:' || clips.content_type
                ),
                'original_text', clips.content_hash, NULL, NULL
         FROM clips
         WHERE clips.content_type NOT IN ('text', 'image', 'file')
           AND TRIM(clips.content_type) != ''
           AND NOT EXISTS (
                SELECT 1 FROM clip_analysis_classifications AS existing
                WHERE existing.clip_id = clips.id
                  AND existing.input_hash = clips.content_hash
                  AND existing.content_type = clips.content_type
           )",
        [],
    )?;
    transaction.execute(
        "UPDATE clips
         SET content_type = 'text'
         WHERE content_type NOT IN ('text', 'image', 'file')
           AND TRIM(content_type) != ''",
        [],
    )?;
    transaction.commit()
}

fn retire_structural_content_type_entries(conn: &Connection) -> Result<()> {
    if table_exists(conn, "bins")? && column_exists(conn, "bins", "smart_rule")? {
        let rules = {
            let mut statement =
                conn.prepare("SELECT id, smart_rule FROM bins WHERE smart_rule IS NOT NULL")?;
            let rules = statement
                .query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>>>()?;
            rules
        };
        for (id, rule_json) in rules {
            let Ok(mut rule) = serde_json::from_str::<serde_json::Value>(&rule_json) else {
                continue;
            };
            let mut changed = false;
            let mut migrate_condition = |condition: &mut serde_json::Value| {
                let is_legacy_structural = condition["type"].as_str() == Some("content_type")
                    && condition["value"]
                        .as_str()
                        .is_some_and(crate::content_types::is_structural_clip_type_id);
                if is_legacy_structural {
                    condition["type"] = serde_json::Value::String("clip_type".into());
                    changed = true;
                }
            };
            if let Some(conditions) = rule["conditions"].as_array_mut() {
                for condition in conditions {
                    migrate_condition(condition);
                }
            } else {
                migrate_condition(&mut rule);
            }
            if changed {
                conn.execute(
                    "UPDATE bins SET smart_rule = ?1 WHERE id = ?2",
                    params![
                        serde_json::to_string(&rule).map_err(|error| {
                            rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                        })?,
                        id
                    ],
                )?;
            }
        }
    }
    conn.execute(
        "DELETE FROM content_types
         WHERE id IN ('text', 'image', 'file')
           AND NOT EXISTS (
                SELECT 1 FROM content_classifiers
                WHERE content_classifiers.content_type = content_types.id
                  AND content_classifiers.is_deleted = 0
           )",
        [],
    )?;
    Ok(())
}

fn migrate_analysis_terminology_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "DROP TRIGGER IF EXISTS library_items_detector_insert;
         DROP TRIGGER IF EXISTS library_items_detector_update;
         DROP TRIGGER IF EXISTS library_items_detector_delete;",
    )?;
    let has_legacy_classifiers = table_exists(conn, "content_detectors")?;
    let has_classifiers = table_exists(conn, "content_classifiers")?;
    if has_legacy_classifiers && !has_classifiers {
        conn.execute(
            "ALTER TABLE content_detectors RENAME TO content_classifiers",
            [],
        )?;
    } else if has_legacy_classifiers {
        let transaction = conn.unchecked_transaction()?;
        transaction.execute_batch(
            "INSERT OR IGNORE INTO content_classifiers
                (stable_ref, name, content_type, description, patterns_json, validator,
                 enabled, priority, is_builtin, is_deleted, created_at, updated_at)
             SELECT stable_ref, name, content_type, description, patterns_json, validator,
                    enabled, priority, is_builtin, is_deleted, created_at, updated_at
             FROM content_detectors;
             DROP TABLE content_detectors;",
        )?;
        transaction.commit()?;
    }
    conn.execute("DROP INDEX IF EXISTS idx_content_detectors_order", [])?;

    if table_exists(conn, "settings")? {
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value)
             SELECT 'enableContentClassification', value
             FROM settings WHERE key = 'enableContentDetection'",
            [],
        )?;
        conn.execute(
            "DELETE FROM settings WHERE key = 'enableContentDetection'",
            [],
        )?;
    }
    if table_exists(conn, "schema_migrations")? {
        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (key)
             SELECT 'contentClassifierRegistryV1'
             FROM schema_migrations WHERE key = 'contentDetectorRegistryV1'",
            [],
        )?;
        conn.execute(
            "DELETE FROM schema_migrations WHERE key = 'contentDetectorRegistryV1'",
            [],
        )?;
    }
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

/// Opens a Pasted-owned SQLite database and applies the shared connection policy.
/// Keep keying or storage-engine setup here so the GUI, CLI, backup, restore, and
/// library relocation paths cannot silently diverge.
pub fn open_pasted_database(path: &Path) -> Result<Connection> {
    let connection = Connection::open(path)?;
    configure_connection(&connection)?;
    Ok(connection)
}

fn open_pasted_database_read_only(path: &Path) -> Result<Connection> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    connection.set_db_config(rusqlite::config::DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true)?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    let _ = connection.pragma_update(None, "temp_store", "MEMORY");
    Ok(connection)
}

impl DbState {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let conn = open_pasted_database(&db_path)?;
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
        let mut destination = open_pasted_database(&temporary)?;
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
        let replacement = open_pasted_database(&target_path)?;
        *source = replacement;
        *self.path.lock() = target_path;
        Ok(previous_path)
    }

    pub fn switch_to_database(&self, database_path: PathBuf) -> Result<()> {
        let replacement = open_pasted_database(&database_path)?;
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
                created_at DATETIME DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
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

        // Every additive migration distinguishes an existing column from a real
        // SQLite failure. Never discard ALTER TABLE errors during startup.
        add_column_if_missing(&conn, "clips", "note", "TEXT")?;
        add_column_if_missing(&conn, "clips", "name", "TEXT")?;
        add_column_if_missing(&conn, "clips", "is_trashed", "INTEGER DEFAULT 0")?;
        add_column_if_missing(&conn, "clips", "trashed_at", "DATETIME")?;
        add_column_if_missing(&conn, "clips", "is_protected", "INTEGER DEFAULT 0")?;
        add_column_if_missing(&conn, "clips", "is_concealed", "INTEGER NOT NULL DEFAULT 0")?;
        add_column_if_missing(&conn, "clips", "is_revealed", "INTEGER NOT NULL DEFAULT 0")?;
        add_column_if_missing(&conn, "clips", "shortcut", "TEXT")?;
        add_column_if_missing(&conn, "clips", "image_path", "TEXT")?;
        add_column_if_missing(&conn, "clips", "pin_order", "INTEGER DEFAULT 0")?;
        add_column_if_missing(&conn, "clips", "current_transformation_id", "TEXT")?;
        add_column_if_missing(
            &conn,
            "clips",
            "ocr_status",
            "TEXT NOT NULL DEFAULT 'not_applicable'",
        )?;
        add_column_if_missing(&conn, "clips", "ocr_input_hash", "TEXT")?;
        add_column_if_missing(&conn, "clips", "ocr_engine_version", "TEXT")?;
        add_column_if_missing(&conn, "clips", "ocr_extractor_ref", "TEXT")?;
        add_column_if_missing(&conn, "clips", "ocr_extractor_name", "TEXT")?;
        add_column_if_missing(&conn, "clips", "ocr_attempted_at", "DATETIME")?;
        add_column_if_missing(&conn, "clips", "ocr_error", "TEXT")?;
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
        add_column_if_missing(&conn, "bins", "smart_rule", "TEXT")?;
        add_column_if_missing(&conn, "bins", "bin_type", "TEXT DEFAULT 'category'")?;
        add_column_if_missing(&conn, "bins", "shortcut", "TEXT")?;
        add_column_if_missing(&conn, "bins", "protect_clips", "INTEGER NOT NULL DEFAULT 0")?;
        add_column_if_missing(&conn, "bins", "conceal_clips", "INTEGER NOT NULL DEFAULT 0")?;

        migrate_clip_source_schema(&conn)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS clip_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                text_content TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clip_versions_clip_id ON clip_versions(clip_id, created_at DESC)",
            [],
        )?;
        add_column_if_missing(&conn, "clip_versions", "context_json", "TEXT")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS clip_analysis_classifications (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                content_type TEXT NOT NULL,
                classifier_ref TEXT NOT NULL,
                source_representation TEXT NOT NULL
                    CHECK (source_representation IN ('original_text', 'searchable_text')),
                input_hash TEXT NOT NULL,
                start_offset INTEGER,
                end_offset INTEGER,
                updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                CHECK (
                    (start_offset IS NULL AND end_offset IS NULL)
                    OR (start_offset >= 0 AND end_offset > start_offset)
                )
            )",
            [],
        )?;
        migrate_multi_type_classifications(&conn)?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clip_analysis_classification_type
             ON clip_analysis_classifications(content_type, clip_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clip_analysis_classification_clip
             ON clip_analysis_classifications(clip_id, input_hash, classifier_ref, start_offset)",
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
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS clip_extraction_attempts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                run_id TEXT NOT NULL,
                participant_ref TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                priority INTEGER NOT NULL,
                result_json TEXT NOT NULL,
                run_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_clip_extraction_attempts_history
                ON clip_extraction_attempts (clip_id, run_at DESC, id DESC, priority, participant_ref);",
        )?;
        conn.execute(
            "INSERT INTO clip_extraction_attempts
                (clip_id, run_id, participant_ref, content_hash, priority, result_json, run_at)
             SELECT results.clip_id,
                    'migrated-' || results.clip_id,
                    results.participant_ref,
                    results.content_hash,
                    CAST(json_extract(results.result_json, '$.priority') AS INTEGER),
                    results.result_json,
                    COALESCE(
                        strftime('%Y-%m-%dT%H:%M:%SZ', results.updated_at),
                        strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                    )
             FROM clip_analysis_results AS results
             WHERE results.participant_ref LIKE 'extractor:%'
               AND NOT EXISTS (
                    SELECT 1 FROM clip_extraction_attempts AS attempts
                    WHERE attempts.clip_id = results.clip_id
               )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS clip_searchable_text (
                clip_id INTEGER PRIMARY KEY REFERENCES clips(id) ON DELETE CASCADE,
                extractor_ref TEXT NOT NULL,
                extractor_name TEXT NOT NULL,
                engine TEXT NOT NULL,
                input_hash TEXT NOT NULL,
                searchable_text TEXT NOT NULL,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
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
            "CREATE INDEX IF NOT EXISTS idx_clips_named_created ON clips (created_at DESC)
             WHERE name IS NOT NULL AND TRIM(name) != ''",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_shortcut ON clips (shortcut)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_active_timeline ON clips (is_trashed, is_pinned DESC, created_at DESC)",
            [],
        );

        search_indexes::ensure_search_indexes(&conn);

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

        // One shared contract protects clips from every cleanup and destructive path.
        // Smart-rule matches are intentionally excluded: only durable manual membership
        // can apply inherited protection.
        conn.execute_batch(
            "DROP VIEW IF EXISTS effective_clip_protection;
             CREATE VIEW effective_clip_protection AS
             SELECT clips.id AS clip_id,
                    CASE WHEN COALESCE(clips.is_protected, 0) = 1
                              OR NULLIF(TRIM(clips.shortcut), '') IS NOT NULL
                              OR EXISTS (
                                  SELECT 1 FROM bins
                                  WHERE COALESCE(bins.protect_clips, 0) = 1
                                    AND (bins.id = clips.bin_id OR EXISTS (
                                        SELECT 1 FROM clip_bins
                                        WHERE clip_bins.clip_id = clips.id
                                          AND clip_bins.bin_id = bins.id
                                    ))
                              )
                         THEN 1 ELSE 0 END AS is_protected,
                    (SELECT GROUP_CONCAT(protecting.id)
                     FROM bins AS protecting
                     WHERE COALESCE(protecting.protect_clips, 0) = 1
                       AND (protecting.id = clips.bin_id OR EXISTS (
                           SELECT 1 FROM clip_bins
                           WHERE clip_bins.clip_id = clips.id
                             AND clip_bins.bin_id = protecting.id
                       ))) AS protecting_bin_ids
             FROM clips;",
        )?;

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
                created_at DATETIME DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                observed_at DATETIME DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
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
                      OR event_type LIKE 'classifier_%' OR event_type LIKE 'content_%' THEN 'organization'
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
        migrate_analysis_terminology_schema(&conn)?;

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
                conceal_clips INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_content_types_order
                ON content_types (is_archived, is_builtin DESC, group_name, label);
            CREATE TABLE IF NOT EXISTS content_classifiers (
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
            CREATE INDEX IF NOT EXISTS idx_content_classifiers_order
                ON content_classifiers (is_deleted, enabled, priority, id);
            CREATE TABLE IF NOT EXISTS content_extractors (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                stable_ref TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                engine TEXT NOT NULL,
                executable_path TEXT,
                model_path TEXT,
                input_contract TEXT NOT NULL,
                output_contract TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                priority INTEGER NOT NULL DEFAULT 100,
                revision INTEGER NOT NULL DEFAULT 1,
                shipped_revision INTEGER,
                shipped_definition_json TEXT,
                recipe_json TEXT,
                shipped_recipe_json TEXT,
                is_builtin INTEGER NOT NULL DEFAULT 0,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_content_extractors_order
                ON content_extractors (is_deleted, enabled, priority, id);",
        )?;
        configure_content_type_schema(&conn)?;
        if !column_exists(&conn, "content_extractors", "model_path")? {
            conn.execute(
                "ALTER TABLE content_extractors ADD COLUMN model_path TEXT",
                [],
            )?;
        }
        if !column_exists(&conn, "content_extractors", "executable_path")? {
            conn.execute(
                "ALTER TABLE content_extractors ADD COLUMN executable_path TEXT",
                [],
            )?;
        }
        if !column_exists(&conn, "content_extractors", "revision")? {
            conn.execute(
                "ALTER TABLE content_extractors ADD COLUMN revision INTEGER NOT NULL DEFAULT 1",
                [],
            )?;
        }
        if !column_exists(&conn, "content_extractors", "shipped_revision")? {
            conn.execute(
                "ALTER TABLE content_extractors ADD COLUMN shipped_revision INTEGER",
                [],
            )?;
        }
        if !column_exists(&conn, "content_extractors", "shipped_definition_json")? {
            conn.execute(
                "ALTER TABLE content_extractors ADD COLUMN shipped_definition_json TEXT",
                [],
            )?;
        }
        if !column_exists(&conn, "content_extractors", "recipe_json")? {
            conn.execute(
                "ALTER TABLE content_extractors ADD COLUMN recipe_json TEXT",
                [],
            )?;
        }
        if !column_exists(&conn, "content_extractors", "shipped_recipe_json")? {
            conn.execute(
                "ALTER TABLE content_extractors ADD COLUMN shipped_recipe_json TEXT",
                [],
            )?;
        }
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS extractor_authoring_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                extractor_id INTEGER NOT NULL,
                source TEXT NOT NULL,
                provider TEXT,
                model TEXT,
                original_prompt TEXT,
                manifest_version INTEGER NOT NULL DEFAULT 1,
                created_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                updated_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                FOREIGN KEY (extractor_id) REFERENCES content_extractors(id)
            );
            CREATE INDEX IF NOT EXISTS idx_extractor_authoring_sessions
                ON extractor_authoring_sessions (extractor_id, created_at, id);
            CREATE TABLE IF NOT EXISTS extractor_authoring_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                sequence INTEGER NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                structured_content_json TEXT,
                created_at DATETIME NOT NULL,
                FOREIGN KEY (session_id) REFERENCES extractor_authoring_sessions(id),
                UNIQUE (session_id, sequence)
            );
            CREATE TABLE IF NOT EXISTS extractor_recipe_revisions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                extractor_id INTEGER NOT NULL,
                revision INTEGER NOT NULL,
                recipe_json TEXT NOT NULL,
                recipe_hash TEXT NOT NULL,
                authoring_session_id INTEGER,
                created_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                FOREIGN KEY (extractor_id) REFERENCES content_extractors(id),
                FOREIGN KEY (authoring_session_id) REFERENCES extractor_authoring_sessions(id),
                UNIQUE (extractor_id, revision)
            );",
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
                    (id, label, icon, group_name, is_builtin, is_archived, conceal_clips)
                 VALUES (?1, ?2, ?3, ?4, 1, 0, ?5)",
                params![
                    preset.id,
                    preset.label,
                    preset.icon,
                    preset.group,
                    preset.conceal_clips()
                ],
            )?;
        }
        for preset in crate::content_classification::CLASSIFIER_PRESETS {
            let patterns_json = serde_json::to_string(&preset.patterns)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            conn.execute(
                "INSERT OR IGNORE INTO content_classifiers
                    (stable_ref, name, content_type, description, patterns_json, validator, enabled, priority, is_builtin)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, 1)",
                params![preset.stable_ref, preset.name, preset.content_type, preset.description, patterns_json, preset.validator, preset.priority],
            )?;
        }
        create_effective_view(&conn)?;
        migrate_legacy_semantic_clip_types(&conn)?;
        retire_structural_content_type_entries(&conn)?;
        for preset in crate::content_extraction::EXTRACTOR_PRESETS {
            conn.execute(
                "INSERT OR IGNORE INTO content_extractors
                    (stable_ref, name, description, engine, executable_path, model_path,
                     input_contract, output_contract, enabled, priority, revision,
                     shipped_revision, shipped_definition_json, is_builtin)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, 1, ?10, ?11, 1)",
                params![
                    preset.stable_ref,
                    preset.name,
                    preset.description,
                    preset.engine,
                    preset.executable_path,
                    preset.model_path,
                    preset.input_contract,
                    preset.output_contract,
                    preset.priority,
                    preset.revision,
                    serde_json::to_string(&preset.definition()).map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                    })?
                ],
            )?;
            conn.execute(
                "UPDATE content_extractors
                 SET shipped_revision = COALESCE(shipped_revision, ?1),
                     shipped_definition_json = COALESCE(shipped_definition_json, ?2)
                 WHERE stable_ref = ?3 AND is_builtin = 1",
                params![
                    preset.revision,
                    serde_json::to_string(&preset.definition()).map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                    })?,
                    preset.stable_ref,
                ],
            )?;
            let shipped = conn.query_row(
                "SELECT shipped_revision, shipped_definition_json,
                        name, description, engine, executable_path, model_path,
                        input_contract, output_contract, enabled, priority
                 FROM content_extractors WHERE stable_ref = ?1 AND is_builtin = 1",
                params![preset.stable_ref],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        crate::content_extraction::ExtractorDefinitionInput {
                            name: row.get(2)?,
                            description: row.get(3)?,
                            engine: row.get(4)?,
                            executable_path: row.get(5)?,
                            model_path: row.get(6)?,
                            input_contract: row.get(7)?,
                            output_contract: row.get(8)?,
                            enabled: row.get(9)?,
                            priority: row.get(10)?,
                        },
                    ))
                },
            )?;
            if shipped.0 < preset.revision {
                let previous = serde_json::from_str::<
                    crate::content_extraction::ExtractorDefinitionInput,
                >(&shipped.1)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
                let next = preset.definition();
                let effective = crate::content_extraction::merge_shipped_definition(
                    &shipped.2, &previous, &next,
                );
                conn.execute(
                    "UPDATE content_extractors
                     SET name = ?1, description = ?2, engine = ?3, executable_path = ?4,
                         model_path = ?5, input_contract = ?6, output_contract = ?7,
                         enabled = ?8, priority = ?9, revision = revision + 1,
                         shipped_revision = ?10, shipped_definition_json = ?11,
                         updated_at = CURRENT_TIMESTAMP
                     WHERE stable_ref = ?12 AND is_builtin = 1",
                    params![
                        effective.name,
                        effective.description,
                        effective.engine,
                        effective.executable_path,
                        effective.model_path,
                        effective.input_contract,
                        effective.output_contract,
                        effective.enabled,
                        effective.priority,
                        preset.revision,
                        serde_json::to_string(&next).map_err(|error| {
                            rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                        })?,
                        preset.stable_ref,
                    ],
                )?;
            }
            let recipe = preset.recipe();
            crate::extractor_recipe::validate_recipe(&recipe).map_err(|error| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    error,
                )))
            })?;
            let recipe_json = serde_json::to_string(&recipe)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            let (current_recipe, previous_shipped_recipe) = conn.query_row(
                "SELECT recipe_json, shipped_recipe_json
                 FROM content_extractors WHERE stable_ref = ?1 AND is_builtin = 1",
                params![preset.stable_ref],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                    ))
                },
            )?;
            let effective_recipe = match (current_recipe, previous_shipped_recipe) {
                (Some(current), Some(previous)) => {
                    let matches_previous = match (
                        serde_json::from_str::<crate::extractor_recipe::ExtractorRecipe>(&current),
                        serde_json::from_str::<crate::extractor_recipe::ExtractorRecipe>(&previous),
                    ) {
                        (Ok(current), Ok(previous)) => current == previous,
                        _ => current == previous,
                    };
                    if matches_previous {
                        recipe_json.clone()
                    } else {
                        current
                    }
                }
                (Some(current), None) => current,
                _ => recipe_json.clone(),
            };
            let effective_recipe =
                serde_json::from_str::<crate::extractor_recipe::ExtractorRecipe>(&effective_recipe)
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            let effective_recipe = crate::content_extraction::migrate_builtin_recipe_compatibility(
                preset.stable_ref,
                &effective_recipe,
                shipped.2.model_path.as_deref(),
            );
            crate::extractor_recipe::validate_recipe(&effective_recipe).map_err(|error| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    error,
                )))
            })?;
            let effective_recipe = serde_json::to_string(&effective_recipe)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            conn.execute(
                "UPDATE content_extractors
                 SET recipe_json = ?1, shipped_recipe_json = ?2
                 WHERE stable_ref = ?3 AND is_builtin = 1",
                params![effective_recipe, recipe_json, preset.stable_ref],
            )?;
        }
        {
            let legacy = {
                let mut statement = conn.prepare(
                    "SELECT id, name, description, engine, executable_path, model_path,
                            input_contract, output_contract, enabled, priority, revision
                     FROM content_extractors WHERE recipe_json IS NULL",
                )?;
                let rows = statement
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, i64>(10)?,
                            crate::content_extraction::ExtractorDefinitionInput {
                                name: row.get(1)?,
                                description: row.get(2)?,
                                engine: row.get(3)?,
                                executable_path: row.get(4)?,
                                model_path: row.get(5)?,
                                input_contract: row.get(6)?,
                                output_contract: row.get(7)?,
                                enabled: row.get(8)?,
                                priority: row.get(9)?,
                            },
                        ))
                    })?
                    .collect::<Result<Vec<_>>>()?;
                rows
            };
            for (id, revision, definition) in legacy {
                let recipe = crate::content_extraction::recipe_for_legacy_definition(&definition);
                crate::extractor_recipe::validate_recipe(&recipe).map_err(|error| {
                    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        error,
                    )))
                })?;
                let recipe_json = serde_json::to_string(&recipe)
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
                let recipe_hash = recipe.hash().map_err(|error| {
                    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(error)))
                })?;
                conn.execute(
                    "UPDATE content_extractors SET recipe_json = ?1 WHERE id = ?2",
                    params![recipe_json, id],
                )?;
                conn.execute(
                    "INSERT OR IGNORE INTO extractor_recipe_revisions
                        (extractor_id, revision, recipe_json, recipe_hash)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![id, revision, recipe_json, recipe_hash],
                )?;
            }
        }
        {
            let recipes = {
                let mut statement = conn.prepare(
                    "SELECT id, revision, recipe_json FROM content_extractors
                     WHERE recipe_json IS NOT NULL",
                )?;
                let rows = statement
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    })?
                    .collect::<Result<Vec<_>>>()?;
                rows
            };
            for (id, revision, recipe_json) in recipes {
                let recipe =
                    serde_json::from_str::<crate::extractor_recipe::ExtractorRecipe>(&recipe_json)
                        .map_err(|error| {
                            rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                        })?;
                let recipe_hash = recipe.hash().map_err(|error| {
                    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(error)))
                })?;
                conn.execute(
                    "INSERT OR IGNORE INTO extractor_recipe_revisions
                        (extractor_id, revision, recipe_json, recipe_hash)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![id, revision, recipe_json, recipe_hash],
                )?;
            }
        }
        let legacy_type_ids = {
            let mut statement = conn.prepare(
                "SELECT content_type FROM content_classifiers
                 UNION SELECT content_type FROM clips
                 ORDER BY content_type",
            )?;
            let ids = statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>>>()?;
            ids
        };
        for id in legacy_type_ids {
            if crate::content_types::is_structural_clip_type_id(&id) {
                continue;
            }
            conn.execute(
                "INSERT OR IGNORE INTO content_types
                    (id, label, icon, group_name, is_builtin, is_archived)
                 VALUES (?1, ?2, 'FileText', 'custom', 0, 0)",
                params![id, crate::content_types::fallback_label(&id)],
            )?;
        }
        let classifier_migration_applied: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE key = 'contentClassifierRegistryV1')",
            [],
            |row| row.get(0),
        )?;
        if !classifier_migration_applied {
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
                        "UPDATE content_classifiers SET enabled = 0 WHERE stable_ref = ?1",
                        params![stable_ref],
                    )?;
                }
            }
            conn.execute(
                "INSERT INTO schema_migrations (key) VALUES ('contentClassifierRegistryV1')",
                [],
            )?;
        }
        Self::init_library_items(&conn)?;
        migrate_canonical_timestamps(&conn)?;
        migrate_analysis_classification_timestamps(&conn)?;

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
            DROP TRIGGER IF EXISTS library_items_classifier_insert;
            DROP TRIGGER IF EXISTS library_items_classifier_update;
            DROP TRIGGER IF EXISTS library_items_classifier_delete;
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
                kind TEXT NOT NULL CHECK (kind IN ('capture', 'inspector', 'extractor', 'classifier', 'suggestion', 'operation', 'transform')),
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
            VALUES ('capture:clip-type-v1', 'capture', 'Clip Type',
                    'Assigns exactly one structural Text, Image, or Files type from the captured clipboard representation.',
                    'Capture', 'Shapes', NULL, 1, 0, 0, 1,
                    'clipboard_representation', 'clip_type', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            VALUES ('capture:source-attribution-v1', 'capture', 'Source Attribution',
                    'Records the application associated with a clipboard capture and resolves its icon when shown.',
                    'Capture', 'AppWindow', NULL, 1, 0, 10, 1,
                    'clipboard_event', 'source_attribution', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

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
            VALUES ('inspector:file-format-v1', 'inspector', 'File Format',
                    'Identifies referenced file formats from bounded byte signatures.',
                    'Content Analysis', 'FileType2', NULL, 1, 0, 10, 1,
                    'file_references', 'file_formats', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            VALUES ('inspector:media-metadata-v1', 'inspector', 'Media Metadata',
                    'Reads bounded audio and video metadata locally.',
                    'Content Analysis', 'FileAudio', NULL, 1, 0, 20, 1,
                    'file_references', 'media_metadata', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            VALUES ('suggestion:smart-actions-v1', 'suggestion', 'Smart Actions',
                    'Suggests saved Transforms from content-free analysis signals.',
                    'Content Analysis', 'Lightbulb', NULL, 1, 0, 0, 1,
                    'analyzable_text+structural_metadata', 'suggestions',
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
            SELECT classifiers.stable_ref, 'classifier', classifiers.name, classifiers.description,
                   groups.label, types.icon, classifiers.enabled, classifiers.is_builtin,
                   classifiers.is_deleted, classifiers.priority, 1, 'text',
                   'set_type:' || classifiers.content_type, classifiers.created_at, classifiers.updated_at
            FROM content_classifiers AS classifiers
            LEFT JOIN content_types AS types ON types.id = classifiers.content_type
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
            DROP TRIGGER IF EXISTS library_items_classifier_insert;
            DROP TRIGGER IF EXISTS library_items_classifier_update;
            DROP TRIGGER IF EXISTS library_items_classifier_delete;
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
            CREATE TRIGGER library_items_classifier_insert AFTER INSERT ON content_classifiers BEGIN
              DELETE FROM library_items WHERE stable_ref=NEW.stable_ref;
              INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled, is_builtin,
                 is_archived, sort_order, revision, input_contract, output_contract, created_at, updated_at)
              SELECT NEW.stable_ref, 'classifier', NEW.name, NEW.description, groups.label,
                     types.icon, NEW.enabled, NEW.is_builtin, NEW.is_deleted, NEW.priority,
                     1, 'text', 'set_type:' || NEW.content_type, NEW.created_at, NEW.updated_at
              FROM content_types AS types LEFT JOIN content_type_groups AS groups ON groups.id=types.group_name
              WHERE types.id=NEW.content_type;
            END;
            CREATE TRIGGER library_items_classifier_update AFTER UPDATE ON content_classifiers BEGIN
              DELETE FROM library_items WHERE stable_ref=OLD.stable_ref OR stable_ref=NEW.stable_ref;
              INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled, is_builtin,
                 is_archived, sort_order, revision, input_contract, output_contract, created_at, updated_at)
              SELECT NEW.stable_ref, 'classifier', NEW.name, NEW.description, groups.label,
                     types.icon, NEW.enabled, NEW.is_builtin, NEW.is_deleted, NEW.priority,
                     1, 'text', 'set_type:' || NEW.content_type, NEW.created_at, NEW.updated_at
              FROM content_types AS types LEFT JOIN content_type_groups AS groups ON groups.id=types.group_name
              WHERE types.id=NEW.content_type;
            END;
            CREATE TRIGGER library_items_classifier_delete AFTER DELETE ON content_classifiers BEGIN
              DELETE FROM library_items WHERE stable_ref=OLD.stable_ref;
            END;
            CREATE TRIGGER library_items_content_type_update AFTER UPDATE ON content_types BEGIN
              UPDATE library_items SET
                icon=NEW.icon,
                group_label=(SELECT label FROM content_type_groups WHERE id=NEW.group_name),
                output_contract='set_type:'||NEW.id,
                updated_at=CURRENT_TIMESTAMP
              WHERE kind='classifier' AND stable_ref IN (
                SELECT stable_ref FROM content_classifiers WHERE content_type=NEW.id
              );
            END;
            CREATE TRIGGER library_items_content_group_update AFTER UPDATE ON content_type_groups BEGIN
              UPDATE library_items SET group_label=NEW.label,updated_at=CURRENT_TIMESTAMP
              WHERE kind='classifier' AND stable_ref IN (
                SELECT classifiers.stable_ref FROM content_classifiers AS classifiers
                JOIN content_types AS types ON types.id=classifiers.content_type
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
            clips_deleted: transaction.query_row("SELECT COUNT(*) FROM clips", [], sqlite_count)?,
            bins_deleted: transaction.query_row("SELECT COUNT(*) FROM bins", [], sqlite_count)?,
            transforms_deleted: transaction.query_row(
                "SELECT (SELECT COUNT(*) FROM saved_transforms) + (SELECT COUNT(*) FROM custom_operations)",
                [],
                sqlite_count,
            )?,
            connections_deleted: transaction.query_row(
                "SELECT COUNT(*) FROM intelligence_connections",
                [],
                sqlite_count,
            )?,
            activity_entries_deleted: transaction.query_row(
                "SELECT COUNT(*) FROM activity_logs",
                [],
                sqlite_count,
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
             DELETE FROM extractor_authoring_messages;
             DELETE FROM extractor_recipe_revisions;
             DELETE FROM extractor_authoring_sessions;
             DELETE FROM content_extractors;
             DELETE FROM content_classifiers;
             DELETE FROM content_types;
             DELETE FROM content_type_groups;
             DELETE FROM settings;",
        )?;
        transaction.execute(
            "DELETE FROM sqlite_sequence WHERE name IN (
                'clips', 'bins', 'clip_versions', 'activity_logs', 'custom_operations',
                'saved_transforms', 'automations', 'intelligence_connections',
                'extractor_authoring_sessions', 'extractor_authoring_messages',
                'extractor_recipe_revisions'
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
                    (id, label, icon, group_name, is_builtin, is_archived, conceal_clips)
                 VALUES (?1, ?2, ?3, ?4, 1, 0, ?5)",
                params![
                    preset.id,
                    preset.label,
                    preset.icon,
                    preset.group,
                    preset.conceal_clips()
                ],
            )?;
        }
        for preset in crate::content_classification::CLASSIFIER_PRESETS {
            let patterns_json = serde_json::to_string(&preset.patterns)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            transaction.execute(
                "INSERT INTO content_classifiers
                    (stable_ref, name, content_type, description, patterns_json, validator, enabled, priority, is_builtin)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, 1)",
                params![preset.stable_ref, preset.name, preset.content_type, preset.description, patterns_json, preset.validator, preset.priority],
            )?;
        }
        for preset in crate::content_extraction::EXTRACTOR_PRESETS {
            let recipe = preset.recipe();
            let recipe_json = serde_json::to_string(&recipe)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            let recipe_hash = recipe.hash().map_err(invalid_extractor_input)?;
            transaction.execute(
                "INSERT INTO content_extractors
                    (stable_ref, name, description, engine, executable_path, model_path,
                     input_contract, output_contract, enabled, priority, revision,
                     shipped_revision, shipped_definition_json, recipe_json,
                     shipped_recipe_json, is_builtin)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, 1, ?10, ?11, ?12, ?12, 1)",
                params![
                    preset.stable_ref,
                    preset.name,
                    preset.description,
                    preset.engine,
                    preset.executable_path,
                    preset.model_path,
                    preset.input_contract,
                    preset.output_contract,
                    preset.priority,
                    preset.revision,
                    serde_json::to_string(&preset.definition()).map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                    })?,
                    recipe_json,
                ],
            )?;
            let extractor_id = transaction.last_insert_rowid();
            transaction.execute(
                "INSERT INTO extractor_recipe_revisions
                    (extractor_id, revision, recipe_json, recipe_hash)
                 VALUES (?1, 1, ?2, ?3)",
                params![extractor_id, recipe_json, recipe_hash],
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
        run_named_migrations(
            conn,
            &[
                NamedMigration {
                    key: "appExclusionHotkeysV1",
                    apply: migrate_app_exclusion_hotkey_setting,
                },
                NamedMigration {
                    key: "transformTerminologyV1",
                    apply: migrate_transform_activity_terminology,
                },
                NamedMigration {
                    key: "currentTransformationBackfillV1",
                    apply: backfill_current_transformation,
                },
            ],
        )?;

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
        let include_classifiers =
            crate::features::is_enabled(self, crate::features::Feature::ContentClassification);
        let analysis = crate::analysis_execution::analyze_text(
            self,
            text,
            Some(source),
            crate::analysis_execution::AnalyzerOptions {
                policy: crate::analysis_contract::AnalysisPolicy::Capture,
                include_extractor: false,
                include_classifiers,
                include_suggestions: false,
            },
        )
        .ok();
        let classification_matches = analysis
            .as_ref()
            .map(|result| result.analysis.result.classification_matches.clone())
            .unwrap_or_default();
        let structure = analysis
            .as_ref()
            .and_then(|result| result.analysis.result.structure.as_ref());
        let content_hash = crate::clipboard_fingerprint::text(text);
        let clip = self.save_clip_with_structure(
            ClipSaveInput {
                content_type: "text",
                text_content: Some(text),
                html_content: None,
                image_base64: None,
                content_hash: &content_hash,
                source,
            },
            structure,
        )?;
        if include_classifiers {
            self.replace_analysis_classifications(
                clip.id,
                &clip.content_hash,
                &classification_matches,
                "original_text",
            )?;
            return self.get_clip_by_id(clip.id);
        }
        Ok(clip)
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
            let created_at = clip
                .created_at
                .as_deref()
                .map(|value| canonical_utc_timestamp(value, "External history"))
                .transpose()?;
            let changed = transaction.execute(
                "INSERT OR IGNORE INTO clips
                    (content_type, text_content, content_hash, source, ocr_status, created_at)
                 VALUES ('text', ?1, ?2, ?3, 'not_applicable', COALESCE(?4, strftime('%Y-%m-%dT%H:%M:%SZ', 'now')))",
                params![clip.text, clip.content_hash, clip.source, created_at],
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

        self.log_activity_internal(
            &transaction,
            "external_history_imported",
            &format!(
                "Imported {imported_count} clips from {source_label}; skipped {duplicate_count} duplicates"
            ),
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
