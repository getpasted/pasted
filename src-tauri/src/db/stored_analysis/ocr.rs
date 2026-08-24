use rusqlite::{params, OptionalExtension, Result};

use super::super::{
    ensure_resource_size, ClipRevisionContext, DbState, OcrBackfillStatus, OcrCandidate,
    OcrExtractorProvenance,
};

impl DbState {
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
}
