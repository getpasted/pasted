use rusqlite::{params, OptionalExtension, Result};

use super::super::{DbState, OcrBackfillStatus, OcrCandidate};

impl DbState {
    pub fn get_ocr_backfill_status(&self) -> Result<OcrBackfillStatus> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT COUNT(*),
                SUM(ocr_status = 'never'), SUM(ocr_status = 'queued'),
                SUM(ocr_status = 'running'), SUM(ocr_status = 'complete'),
                SUM(ocr_status = 'no_text'), SUM(ocr_status = 'failed')
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
                "SELECT id, content_hash, image_base64 FROM clips
                 WHERE content_type = 'image' AND ocr_status = 'never'
                   AND COALESCE(is_trashed, 0) = 0 AND image_base64 IS NOT NULL
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
        Ok(conn.execute(
            "UPDATE clips SET ocr_status = 'running', ocr_error = NULL
             WHERE id = ?1 AND content_hash = ?2 AND content_type = 'image'
               AND COALESCE(is_trashed, 0) = 0",
            params![clip_id, content_hash],
        )? > 0)
    }

    pub fn reset_ocr_work(&self, clip_id: Option<i64>, content_hash: Option<&str>) -> Result<()> {
        let conn = self.conn.lock();
        match (clip_id, content_hash) {
            (Some(id), Some(hash)) => conn.execute(
                "UPDATE clips SET ocr_status = 'never', ocr_error = NULL
                 WHERE id = ?1 AND content_hash = ?2 AND content_type = 'image'
                   AND ocr_status IN ('queued', 'running')",
                params![id, hash],
            )?,
            _ => conn.execute(
                "UPDATE clips SET ocr_status = 'never', ocr_error = NULL
                 WHERE content_type = 'image' AND ocr_status IN ('queued', 'running')",
                [],
            )?,
        };
        Ok(())
    }

    pub fn reset_failed_ocr(&self) -> Result<usize> {
        self.conn.lock().execute(
            "UPDATE clips SET ocr_status = 'never', ocr_error = NULL
             WHERE content_type = 'image' AND ocr_status = 'failed'
               AND COALESCE(is_trashed, 0) = 0",
            [],
        )
    }
}
