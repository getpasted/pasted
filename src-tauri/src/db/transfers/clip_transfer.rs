use std::collections::HashSet;

use rusqlite::{params, Result};
use sha2::{Digest, Sha256};

use super::super::{
    canonical_utc_timestamp, clip_names, ensure_resource_size, ensure_safe_raster_data_url,
    normalize_imported_clip_types, replace_imported_content_types, sqlite_count, ClipImportReport,
    ClipItem, DbState,
};

impl DbState {
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
            .get_clips(None, false)?
            .into_iter()
            .filter(|clip| clip.text_content.is_some() && clip.content_type != "image")
            .collect::<Vec<_>>();
        let mut csv =
            String::from("id,content_type,source,is_pinned,created_at,name,text_content\n");
        for clip in clips {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                clip.id,
                cell(&clip.content_type),
                cell(&clip.source),
                clip.is_pinned,
                cell(&clip.created_at),
                cell(clip.name.as_deref().unwrap_or_default()),
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
            "name",
            "text_content",
        ];
        let legacy_expected = [
            "id",
            "content_type",
            "source",
            "is_pinned",
            "created_at",
            "text_content",
        ];
        let current_header = records.first().map(|header| {
            header
                .iter()
                .map(String::as_str)
                .eq(expected.iter().copied())
        }) == Some(true);
        let legacy_header = records.first().map(|header| {
            header
                .iter()
                .map(String::as_str)
                .eq(legacy_expected.iter().copied())
        }) == Some(true);
        if !current_header && !legacy_header {
            return Err(rusqlite::Error::InvalidParameterName(
                "Clip CSV header does not match the supported export format".to_string(),
            ));
        }
        let mut clips = Vec::with_capacity(records.len().saturating_sub(1));
        for (index, row) in records.into_iter().skip(1).enumerate() {
            let expected_columns = if current_header {
                expected.len()
            } else {
                legacy_expected.len()
            };
            if row.len() != expected_columns {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "Clip CSV row {} has {} columns; expected {}",
                    index + 2,
                    row.len(),
                    expected_columns
                )));
            }
            let text_index = if current_header { 6 } else { 5 };
            let text = row[text_index].clone();
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
                name: if current_header {
                    clip_names::normalize_clip_name(Some(&row[5]))?
                } else {
                    None
                },
                content_type: row[1].clone(),
                content_types: Vec::new(),
                file_formats: Vec::new(),
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
                is_explicitly_protected: Some(false),
                protecting_bin_ids: Vec::new(),
                is_concealed: false,
                is_explicitly_concealed: Some(false),
                is_explicitly_revealed: false,
                concealing_bin_ids: Vec::new(),
                concealing_content_types: Vec::new(),
                shortcut: None,
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

    pub(in crate::db) fn parse_csv(csv: &str) -> Result<Vec<Vec<String>>> {
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

    fn apply_imported_clips(
        &self,
        mut clips: Vec<ClipItem>,
        commit: bool,
    ) -> Result<ClipImportReport> {
        use crate::resource_limits::{MAX_CLIP_NOTE_BYTES, MAX_CLIP_TEXT_BYTES};
        let mut input_hashes = HashSet::new();
        for clip in &mut clips {
            normalize_imported_clip_types(clip)?;
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
            clip.name = clip_names::normalize_clip_name(clip.name.as_deref())?;
            clip.created_at = canonical_utc_timestamp(&clip.created_at, "Clip import")?;
        }

        let scanned_count = clips.len();
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let active_count_before: usize = tx.query_row(
            "SELECT COUNT(*) FROM clips WHERE COALESCE(is_trashed, 0) = 0",
            [],
            sqlite_count,
        )?;
        let mut imported_count = 0usize;
        for clip in clips {
            let inserted = tx.execute(
                "INSERT OR IGNORE INTO clips (
                    content_type, text_content, html_content, image_base64, image_path,
                    content_hash, source, is_pinned, is_protected, is_concealed, is_revealed, pin_order, note, name,
                    is_trashed, trashed_at, created_at, ocr_status, ocr_input_hash
                 ) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 0, NULL, ?14,
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
                    clip.is_explicitly_concealed.unwrap_or(clip.is_concealed),
                    clip.is_explicitly_revealed,
                    clip.pin_order,
                    clip.note,
                    clip.name,
                    clip.created_at,
                ],
            )?;
            imported_count += inserted;
            if inserted > 0 && !clip.content_types.is_empty() {
                replace_imported_content_types(
                    &tx,
                    tx.last_insert_rowid(),
                    &clip.content_hash,
                    &clip.content_type,
                    &clip.content_types,
                )?;
            }
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
        self.log_activity_internal(
            &tx,
            "clips_imported",
            &format!("Imported {imported_count} clips; skipped {duplicate_count} duplicates"),
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
}
