use std::collections::HashSet;

use rusqlite::{params, Result};
use sha2::{Digest, Sha256};

use super::{
    append_clip_concealment, append_clip_content_types, append_clip_names, canonical_utc_timestamp,
    clip_names, ensure_resource_size, ensure_safe_raster_data_url, normalize_imported_clip_types,
    normalize_library_archive_timestamps, replace_imported_content_types,
    retire_structural_content_type_entries, sqlite_count, BackupPayload, BinTransformBinding,
    ClipImportReport, ClipItem, DbState, LibraryArchiveInspection, OcrBackupMetadata, PipelineStep,
    PipelineStepInput, BACKUP_SCHEMA_VERSION,
};

impl DbState {
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
        let content_classifiers = self.get_all_content_classifiers_for_backup()?;
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
            content_classifiers,
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

    pub(super) fn parse_csv(csv: &str) -> Result<Vec<Vec<String>>> {
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

    pub(super) fn parse_library_archive(
        json_str: &str,
    ) -> Result<(BackupPayload, LibraryArchiveInspection)> {
        ensure_resource_size(
            json_str,
            crate::resource_limits::MAX_BACKUP_IMPORT_BYTES,
            "Transfer file",
        )?;
        let mut payload: BackupPayload = serde_json::from_str(json_str).map_err(|error| {
            rusqlite::Error::InvalidParameterName(format!("invalid transfer JSON: {error}"))
        })?;
        normalize_library_archive_timestamps(&mut payload)?;
        payload.content_types.retain(|content_type| {
            !crate::content_types::is_structural_clip_type_id(&content_type.id)
        });
        for clip in &mut payload.clips {
            normalize_imported_clip_types(clip)?;
            clip.shortcut = clip
                .shortcut
                .take()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            if clip
                .shortcut
                .as_ref()
                .is_some_and(|value| value.len() > 256)
            {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Transfer clip contains an invalid shortcut".into(),
                ));
            }
            if clip.shortcut.is_some() {
                clip.is_protected = true;
                clip.is_explicitly_protected = Some(true);
            }
        }
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
            payload.content_classifiers.len(),
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
        if payload.content_classifiers.len() > 128 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Transfer file contains more than 128 content classifiers".to_string(),
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
                    conceal_clips: content_type.conceal_clips.unwrap_or_else(|| {
                        crate::content_types::CONTENT_TYPE_PRESETS
                            .iter()
                            .find(|preset| preset.id == content_type.id)
                            .is_some_and(|preset| preset.conceal_clips())
                    }),
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
                .content_classifiers
                .iter()
                .map(|classifier| classifier.stable_ref.clone())
                .collect(),
            "classifier reference",
        )?;
        for classifier in &payload.content_classifiers {
            crate::content_classification::validate_classifier_input(
                &crate::content_classification::ClassifierInput {
                    name: classifier.name.clone(),
                    content_type: classifier.content_type.clone(),
                    description: classifier.description.clone(),
                    patterns: classifier.patterns.clone(),
                    validator: classifier.validator.clone(),
                    enabled: classifier.enabled,
                    priority: classifier.priority,
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
                crate::smart_bins::parse_rule_json(rule).map_err(|error| {
                    rusqlite::Error::InvalidParameterName(format!(
                        "Transfer Bin {} has an invalid smart rule: {error}",
                        bin.id
                    ))
                })?;
                if bin.protect_clips {
                    return Err(rusqlite::Error::InvalidParameterName(format!(
                        "Transfer Smart Bin {} cannot apply inherited protection",
                        bin.id
                    )));
                }
                if bin.conceal_clips {
                    return Err(rusqlite::Error::InvalidParameterName(format!(
                        "Transfer Smart Bin {} cannot apply inherited concealment",
                        bin.id
                    )));
                }
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
            classifier_count: payload.content_classifiers.len(),
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
                    conceal_clips: content_type.conceal_clips.unwrap_or_else(|| {
                        crate::content_types::CONTENT_TYPE_PRESETS
                            .iter()
                            .find(|preset| preset.id == content_type.id)
                            .is_some_and(|preset| preset.conceal_clips())
                    }),
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
                    (id, label, icon, group_name, is_builtin, is_archived, conceal_clips)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET
                    label = excluded.label, icon = excluded.icon,
                    group_name = excluded.group_name,
                    conceal_clips = excluded.conceal_clips,
                    is_archived = CASE WHEN content_types.is_builtin = 1 THEN 0 ELSE excluded.is_archived END,
                    updated_at = CURRENT_TIMESTAMP",
                params![
                    content_type.id,
                    content_type.label,
                    content_type.icon,
                    content_type.group,
                    is_builtin,
                    content_type.is_archived,
                    content_type.conceal_clips.unwrap_or_else(|| {
                        crate::content_types::CONTENT_TYPE_PRESETS
                            .iter()
                            .find(|preset| preset.id == content_type.id)
                            .is_some_and(|preset| preset.conceal_clips())
                    }),
                ],
            )?;
        }
        if payload.content_classifiers.len() > 128 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Backup contains more than 128 content classifiers".to_string(),
            ));
        }
        for classifier in &payload.content_classifiers {
            crate::content_classification::validate_classifier_input(
                &crate::content_classification::ClassifierInput {
                    name: classifier.name.clone(),
                    content_type: classifier.content_type.clone(),
                    description: classifier.description.clone(),
                    patterns: classifier.patterns.clone(),
                    validator: classifier.validator.clone(),
                    enabled: classifier.enabled,
                    priority: classifier.priority,
                },
            )
            .map_err(rusqlite::Error::InvalidParameterName)?;
            let patterns_json = serde_json::to_string(&classifier.patterns)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            tx.execute(
                "INSERT INTO content_classifiers
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
                    classifier.stable_ref,
                    classifier.name,
                    classifier.content_type,
                    classifier.description,
                    patterns_json,
                    classifier.validator,
                    classifier.enabled,
                    classifier.priority,
                    classifier.is_builtin,
                    classifier.is_deleted
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
                *rule = crate::smart_bins::normalize_rule_json(rule)
                    .map_err(rusqlite::Error::InvalidParameterName)?;
            }
            let existing_id = tx.query_row(
                "SELECT id FROM bins WHERE name = ?1 AND COALESCE(bin_type, 'category') = ?2 LIMIT 1",
                params![bin.name, bin.bin_type],
                |row| row.get::<_, i64>(0),
            ).ok();
            let new_id = if let Some(id) = existing_id {
                tx.execute(
                    "UPDATE bins SET icon = ?1, color = ?2, smart_rule = ?3, shortcut = ?4,
                                     protect_clips = ?5, conceal_clips = ?6 WHERE id = ?7",
                    params![
                        bin.icon,
                        bin.color,
                        bin.smart_rule,
                        bin.shortcut,
                        bin.protect_clips,
                        bin.conceal_clips,
                        id
                    ],
                )?;
                id
            } else {
                tx.execute(
                    "INSERT INTO bins (name, icon, color, smart_rule, bin_type, shortcut, protect_clips, conceal_clips, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![bin.name, bin.icon, bin.color, bin.smart_rule, bin.bin_type, bin.shortcut, bin.protect_clips, bin.conceal_clips, bin.created_at],
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
            let clip_name = clip_names::normalize_clip_name(clip.name.as_deref())?;
            let mapped_primary_bin = clip.bin_id.and_then(|id| bin_id_map.get(&id).copied());
            tx.execute(
                "INSERT INTO clips (
                    content_type, text_content, html_content, image_base64, image_path, content_hash,
                    source, is_pinned, is_protected, is_concealed, is_revealed, pin_order, bin_id, note,
                    name, is_trashed, trashed_at, created_at, shortcut
                 ) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
                 ON CONFLICT(content_hash) DO UPDATE SET
                    content_type = excluded.content_type,
                    text_content = excluded.text_content,
                    html_content = excluded.html_content,
                    image_base64 = excluded.image_base64,
                    source = excluded.source,
                    is_pinned = excluded.is_pinned,
                    is_protected = excluded.is_protected,
                    is_concealed = excluded.is_concealed,
                    is_revealed = excluded.is_revealed,
                    pin_order = excluded.pin_order,
                    bin_id = excluded.bin_id,
                    note = excluded.note,
                    name = excluded.name,
                    is_trashed = excluded.is_trashed,
                    trashed_at = excluded.trashed_at,
                    created_at = excluded.created_at,
                    shortcut = excluded.shortcut",
                params![
                    clip.content_type, clip.text_content, clip.html_content, clip.image_base64,
                    clip.content_hash, clip.source, clip.is_pinned,
                    clip.is_explicitly_protected.unwrap_or(clip.is_protected),
                    clip.is_explicitly_concealed.unwrap_or(clip.is_concealed),
                    clip.is_explicitly_revealed, clip.pin_order, mapped_primary_bin, clip.note,
                    clip_name, clip.is_trashed, clip.trashed_at, clip.created_at, clip.shortcut,
                ],
            )?;
            let new_clip_id = tx.query_row(
                "SELECT id FROM clips WHERE content_hash = ?1",
                params![clip.content_hash],
                |row| row.get::<_, i64>(0),
            )?;
            clip_id_map.insert(old_clip_id, new_clip_id);
            if !clip.content_types.is_empty() {
                replace_imported_content_types(
                    &tx,
                    new_clip_id,
                    &clip.content_hash,
                    &clip.content_type,
                    &clip.content_types,
                )?;
            }
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

        retire_structural_content_type_entries(&tx)?;
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

    pub(super) fn get_all_clips_for_backup(&self) -> Result<Vec<ClipItem>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, content_type, text_content, html_content, image_base64, image_path,
                    content_hash, source, is_pinned, is_protected, COALESCE(pin_order, 0),
                    bin_id, note, COALESCE(is_trashed, 0), trashed_at, created_at,
                    (SELECT GROUP_CONCAT(bin_id) FROM clip_bins WHERE clip_id = clips.id),
                    current_transformation_id IS NOT NULL,
                    ocr_extractor_ref, ocr_extractor_name, ocr_engine_version, shortcut
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
                shortcut: row.get(21)?,
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
        let mut clips = rows.collect::<Result<Vec<_>>>()?;
        append_clip_content_types(&conn, &mut clips)?;
        append_clip_concealment(&conn, &mut clips)?;
        append_clip_names(&conn, &mut clips)?;
        Ok(clips)
    }
}
