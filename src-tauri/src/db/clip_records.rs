use super::*;

mod smart_bin_policy;
mod visual_label_condition;

pub(super) use smart_bin_policy::SmartBinFeaturePolicy;
pub(super) fn smart_bin_feature_policy(conn: &Connection) -> Result<SmartBinFeaturePolicy> {
    smart_bin_policy::load(conn)
}

pub(super) fn push_smart_condition(
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
        "visual_label" => visual_label_condition::build(contains, value, parameters),
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

pub(super) fn append_smart_bin_memberships(
    conn: &Connection,
    clips: &mut [ClipItem],
) -> Result<()> {
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

pub(super) fn append_clip_content_types(conn: &Connection, clips: &mut [ClipItem]) -> Result<()> {
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

pub(super) fn append_clip_file_formats(conn: &Connection, clips: &mut [ClipItem]) -> Result<()> {
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

pub(super) fn append_clip_protection(conn: &Connection, clips: &mut [ClipItem]) -> Result<()> {
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

pub(super) fn clip_item_from_row(row: &Row<'_>) -> Result<ClipItem> {
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
pub(super) const MAX_CLIP_SEARCH_QUERY_BYTES: usize = 4 * 1024;
pub(super) const MAX_CLIP_SEARCH_FILTERS: usize = 32;
pub(super) const MAX_CLIP_SEARCH_TERMS: usize = 32;
pub(super) const MAX_CLIP_SEARCH_OFFSET: usize = 10_000_000;

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

pub(super) fn normalize_imported_clip_types(clip: &mut ClipItem) -> Result<()> {
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

pub(super) fn replace_imported_content_types(
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
