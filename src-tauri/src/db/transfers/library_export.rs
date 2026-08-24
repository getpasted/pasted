use rusqlite::Result;

use super::super::{
    append_clip_concealment, append_clip_content_types, append_clip_names, BackupPayload,
    BinTransformBinding, ClipItem, DbState, OcrBackupMetadata, BACKUP_SCHEMA_VERSION,
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

    pub(in crate::db) fn get_all_clips_for_backup(&self) -> Result<Vec<ClipItem>> {
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
