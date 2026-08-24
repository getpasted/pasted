use rusqlite::{params, OptionalExtension, Result, Row};

use super::super::{DbState, StoredExtractionAttempt};

#[cfg(test)]
mod tests;

fn extraction_attempt_from_row(row: &Row<'_>) -> Result<StoredExtractionAttempt> {
    let result_json: String = row.get(0)?;
    let failure_class = row
        .get::<_, Option<String>>(4)?
        .map(|value| serde_json::from_value(serde_json::Value::String(value)))
        .transpose()
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
    Ok(StoredExtractionAttempt {
        observation: serde_json::from_str(&result_json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
        run_id: row.get(1)?,
        run_at: row.get(2)?,
        input_fingerprint: row.get(3)?,
        failure_class,
        retry_after: row.get(5)?,
    })
}

impl DbState {
    pub(super) fn analysis_attempt_limit_internal(conn: &rusqlite::Connection) -> i64 {
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'analysisAttemptsPerClip'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(10)
        .max(0)
    }

    pub(super) fn prune_analysis_attempts_internal(
        conn: &rusqlite::Connection,
        clip_id: i64,
        participant_ref: &str,
    ) -> Result<()> {
        let limit = Self::analysis_attempt_limit_internal(conn);
        if limit == 0 {
            return Ok(());
        }
        conn.execute(
            "DELETE FROM clip_extraction_attempts
             WHERE clip_id = ?1 AND participant_ref = ?2 AND id NOT IN (
                SELECT id FROM clip_extraction_attempts
                WHERE clip_id = ?1 AND participant_ref = ?2
                ORDER BY id DESC LIMIT ?3
             )",
            params![clip_id, participant_ref, limit],
        )?;
        Ok(())
    }

    pub fn enforce_analysis_attempt_retention(&self, keep_count: i64) -> Result<()> {
        let keep_count = keep_count.clamp(0, 10_000);
        let mut conn = self.conn.lock();
        let transaction = conn.transaction()?;
        transaction.execute(
            "INSERT INTO settings (key, value) VALUES ('analysisAttemptsPerClip', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [keep_count.to_string()],
        )?;
        if keep_count > 0 {
            transaction.execute(
                "DELETE FROM clip_extraction_attempts WHERE id IN (
                    SELECT id FROM (
                        SELECT id, ROW_NUMBER() OVER (
                            PARTITION BY clip_id, participant_ref ORDER BY id DESC
                        ) AS attempt_rank
                        FROM clip_extraction_attempts
                    ) WHERE attempt_rank > ?1
                 )",
                [keep_count],
            )?;
        }
        transaction.commit()
    }

    pub fn get_extraction_history(
        &self,
        clip_id: i64,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<StoredExtractionAttempt>> {
        let limit = limit.clamp(1, 101);
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT attempts.result_json, attempts.run_id, attempts.run_at,
                    attempts.input_fingerprint, attempts.failure_class, attempts.retry_after
             FROM clip_extraction_attempts AS attempts
             WHERE attempts.clip_id = ?1
             ORDER BY (
                        SELECT MAX(run_order.id)
                        FROM clip_extraction_attempts AS run_order
                        WHERE run_order.clip_id = attempts.clip_id
                          AND run_order.run_id = attempts.run_id
                      ) DESC,
                      attempts.priority,
                      attempts.participant_ref
             LIMIT ?2 OFFSET ?3",
        )?;
        let attempts = statement
            .query_map(
                params![clip_id, limit as i64, offset as i64],
                extraction_attempt_from_row,
            )?
            .collect();
        attempts
    }

    pub fn get_latest_extraction_attempt(
        &self,
        clip_id: i64,
        participant_ref: &str,
        input_fingerprint: &str,
    ) -> Result<Option<StoredExtractionAttempt>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT result_json, run_id, run_at, input_fingerprint, failure_class, retry_after
             FROM clip_extraction_attempts
             WHERE clip_id = ?1 AND participant_ref = ?2 AND input_fingerprint = ?3
             ORDER BY id DESC LIMIT 1",
            params![clip_id, participant_ref, input_fingerprint],
            extraction_attempt_from_row,
        )
        .optional()
    }
}
