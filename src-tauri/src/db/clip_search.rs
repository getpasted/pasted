use super::*;

mod request_validation;
mod term_fields;

pub(super) fn validate_search_request(request: &ClipSearchRequest) -> Result<()> {
    request_validation::validate(request)
}

#[derive(Debug, Default)]
pub(super) struct ParsedClipSearch {
    pub clip_ids: Vec<i64>,
    pub sources: Vec<String>,
    pub clip_types: Vec<String>,
    pub content_types: Vec<String>,
    pub file_formats: Vec<String>,
    pub terms: Vec<String>,
    pub requires_note: bool,
    pub requires_named: bool,
    pub requires_pinned: bool,
    pub requires_protected: bool,
    pub requires_trashed: bool,
    pub incomplete: bool,
    pub regex: Option<String>,
    pub regex_fallback: Option<String>,
}

#[derive(Clone, Copy)]
pub(super) struct ClipSearchFeaturePolicy {
    pub clip_types: bool,
    pub content_types: bool,
    pub file_formats: bool,
    pub sources: bool,
    pub notes: bool,
    pub naming: bool,
    pub pinning: bool,
    pub protection: bool,
    pub trash: bool,
}

fn tokenize(query: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut quote = None;
    for character in query.chars() {
        if let Some(expected) = quote {
            if character == expected {
                quote = None;
            } else {
                token.push(character);
            }
        } else if matches!(character, '\'' | '"') {
            quote = Some(character);
        } else if character.is_whitespace() {
            if !token.is_empty() {
                tokens.push(std::mem::take(&mut token));
            }
        } else {
            token.push(character);
        }
    }
    if !token.is_empty() {
        tokens.push(token);
    }
    tokens
}

pub(super) fn parse_clip_search(query: &str) -> ParsedClipSearch {
    let trimmed = query.trim();
    let mut parsed = ParsedClipSearch::default();
    if trimmed
        .get(..6)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("regex:"))
    {
        let pattern = &trimmed[6..];
        if pattern.trim().is_empty() {
            parsed.incomplete = true;
        } else if RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()
            .is_ok()
        {
            parsed.regex = Some(pattern.to_string());
        } else {
            parsed.regex_fallback = Some(pattern.to_lowercase());
        }
        return parsed;
    }
    for token in tokenize(trimmed) {
        let lower = token.to_lowercase();
        let push_filter = |values: &mut Vec<String>, value: &str, incomplete: &mut bool| {
            if value.is_empty() {
                *incomplete = true;
            } else {
                values.push(value.to_string());
            }
        };
        if let Some(value) = lower.strip_prefix("id:") {
            let values = value.split(',').collect::<Vec<_>>();
            if values.is_empty() || values.iter().any(|value| value.is_empty()) {
                parsed.incomplete = true;
            } else {
                for value in values {
                    match value.parse::<i64>() {
                        Ok(id) if id > 0 => parsed.clip_ids.push(id),
                        _ => parsed.incomplete = true,
                    }
                }
            }
        } else if let Some(value) = lower.strip_prefix("source:") {
            push_filter(&mut parsed.sources, value.trim(), &mut parsed.incomplete);
        } else if let Some(value) = lower.strip_prefix("clip:") {
            push_filter(&mut parsed.clip_types, value.trim(), &mut parsed.incomplete);
        } else if let Some(value) = lower.strip_prefix("content:") {
            push_filter(
                &mut parsed.content_types,
                value.trim(),
                &mut parsed.incomplete,
            );
        } else if let Some(value) = lower.strip_prefix("format:") {
            push_filter(
                &mut parsed.file_formats,
                value.trim(),
                &mut parsed.incomplete,
            );
        } else if lower == "has:note" {
            parsed.requires_note = true;
        } else if lower == "is:named" || lower == "has:name" {
            parsed.requires_named = true;
        } else if lower == "is:pinned" {
            parsed.requires_pinned = true;
        } else if lower == "is:protected" {
            parsed.requires_protected = true;
        } else if lower == "is:trashed" {
            parsed.requires_trashed = true;
        } else if !lower.is_empty() {
            parsed.terms.push(lower);
        }
    }
    parsed
}

pub(super) fn clip_search_feature_policy(conn: &Connection) -> Result<ClipSearchFeaturePolicy> {
    conn.query_row(
        "SELECT
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableClipTypes' AND LOWER(TRIM(value)) IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableTypes' AND LOWER(TRIM(value)) IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableFileFormats' AND LOWER(TRIM(value)) IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableSources' AND LOWER(TRIM(value)) IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableNotes' AND LOWER(TRIM(value)) IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableNaming' AND LOWER(TRIM(value)) IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enablePinning' AND LOWER(TRIM(value)) IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableProtection' AND LOWER(TRIM(value)) IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableTrash' AND LOWER(TRIM(value)) IN ('false', '0'))",
        [],
        |row| Ok(ClipSearchFeaturePolicy {
            clip_types: row.get(0)?, content_types: row.get(1)?, file_formats: row.get(2)?,
            sources: row.get(3)?, notes: row.get(4)?, naming: row.get(5)?,
            pinning: row.get(6)?, protection: row.get(7)?, trash: row.get(8)?,
        }),
    )
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
        validate_search_request(request)?;

        let limit = if request.limit == 0 {
            DEFAULT_CLIP_SEARCH_PAGE_SIZE
        } else {
            request.limit
        };
        let offset = request.offset;
        let mut parsed = parse_clip_search(&request.query);
        parsed.clip_ids.extend(request.clip_ids.iter().copied());
        parsed.clip_ids.sort_unstable();
        parsed.clip_ids.dedup();
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
            || parsed.clip_ids.len() > MAX_CLIP_SEARCH_IDS
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
        if !parsed.clip_ids.is_empty() {
            clauses.push("clips.id IN (SELECT CAST(value AS INTEGER) FROM json_each(?))".into());
            parameters.push(Box::new(serde_json::to_string(&parsed.clip_ids).map_err(
                |error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)),
            )?));
        }
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
                let mut fields = term_fields::base(fts_like);
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
                term_fields::push_visual_label_parameters(&mut parameters, &pattern);
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
}
