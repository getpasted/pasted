use rusqlite::{params, OptionalExtension, Result};

use super::super::DbState;

impl DbState {
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

    pub fn record_file_format_inspection(
        &self,
        clip_id: i64,
        content_hash: &str,
        inspection: &crate::content_inspection::FileFormatInspection,
    ) -> Result<bool> {
        let result_json = serde_json::to_string(inspection)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        if result_json.len() > 64 * 1024 {
            return Err(rusqlite::Error::InvalidParameterName(
                "File Format inspection metadata exceeds its safety limit".into(),
            ));
        }
        let conn = self.conn.lock();
        let changed = conn.execute(
            "INSERT INTO clip_analysis_results
                (clip_id, participant_ref, content_hash, input_hash, format_version, result_json)
             SELECT id, ?1, content_hash, content_hash, ?2, ?3 FROM clips
             WHERE id = ?4 AND content_hash = ?5 AND COALESCE(is_trashed, 0) = 0
             ON CONFLICT(clip_id, participant_ref) DO UPDATE SET
                content_hash = excluded.content_hash,
                input_hash = excluded.input_hash,
                format_version = excluded.format_version,
                result_json = excluded.result_json,
                updated_at = CURRENT_TIMESTAMP",
            params![
                crate::content_inspection::FILE_FORMAT_INSPECTOR_REF,
                crate::analysis_contract::ANALYSIS_CONTRACT_VERSION,
                result_json,
                clip_id,
                content_hash,
            ],
        )?;
        Ok(changed == 1)
    }

    pub fn get_file_format_inspection(
        &self,
        clip_id: i64,
        content_hash: &str,
    ) -> Result<Option<crate::content_inspection::FileFormatInspection>> {
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
                    crate::content_inspection::FILE_FORMAT_INSPECTOR_REF,
                    content_hash,
                    crate::analysis_contract::ANALYSIS_CONTRACT_VERSION,
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(result_json.and_then(|json| serde_json::from_str(&json).ok()))
    }
}
