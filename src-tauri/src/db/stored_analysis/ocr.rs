use rusqlite::{params, OptionalExtension, Result};

use super::super::{ensure_resource_size, DbState, OcrExtractorProvenance};

impl DbState {
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
        self.complete_ocr_attempt_with_extractor_and_revision(
            clip_id,
            content_hash,
            recognized_text,
            provenance,
            error,
            false,
        )
    }

    pub(crate) fn complete_ocr_attempt_with_extractor_and_revision(
        &self,
        clip_id: i64,
        content_hash: &str,
        recognized_text: Option<&str>,
        provenance: OcrExtractorProvenance<'_>,
        error: Option<&str>,
        derived_state_changes: bool,
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
        let text_changes = status == "complete"
            && previous_text.as_deref() != Some(recognized_text.unwrap_or_default());
        if error.is_none()
            && (text_changes || derived_state_changes)
            && Self::revision_history_enabled_internal(&tx)
        {
            Self::snapshot_derived_revision_internal(
                &tx,
                clip_id,
                "extraction",
                "Before extracting again",
            )?;
        }
        if status == "complete" {
            let recognized_text = recognized_text.unwrap_or_default();
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
        self.complete_or_reset_ocr_attempt_with_extractor_and_revision(
            clip_id,
            content_hash,
            recognized_text,
            provenance,
            error,
            false,
        )
    }

    pub(crate) fn complete_or_reset_ocr_attempt_with_extractor_and_revision(
        &self,
        clip_id: i64,
        content_hash: &str,
        recognized_text: Option<&str>,
        provenance: OcrExtractorProvenance<'_>,
        error: Option<&str>,
        derived_state_changes: bool,
    ) -> Result<bool> {
        let result = self.complete_ocr_attempt_with_extractor_and_revision(
            clip_id,
            content_hash,
            recognized_text,
            provenance,
            error,
            derived_state_changes,
        );
        if result.is_err() {
            let _ = self.reset_ocr_work(Some(clip_id), Some(content_hash));
        }
        result
    }
}
