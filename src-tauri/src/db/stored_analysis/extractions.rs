use rusqlite::{params, Result};

use super::super::{DbState, StoredExtractionAttempt, StoredExtractionObservation};

#[cfg(test)]
mod tests;

impl DbState {
    pub fn record_extraction_observations(
        &self,
        clip_id: i64,
        content_hash: &str,
        observations: &[crate::content_analysis::ExtractionObservation],
    ) -> Result<bool> {
        if observations.len() > crate::content_extraction::MAX_ACTIVE_EXTRACTORS_PER_INPUT {
            return Err(rusqlite::Error::InvalidParameterName(
                "Too many Extractor results".into(),
            ));
        }
        let serialized = observations
            .iter()
            .map(|observation| {
                let json = serde_json::to_string(observation)
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
                if json.len() > crate::resource_limits::MAX_OCR_TEXT_BYTES.saturating_add(8 * 1024)
                {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "Extractor result exceeds its safety limit".into(),
                    ));
                }
                Ok((
                    observation.extractor_ref.as_str(),
                    observation.priority,
                    json,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let mut conn = self.conn.lock();
        let transaction = conn.transaction()?;
        let clip_matches: bool = transaction.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM clips
                WHERE id = ?1 AND content_hash = ?2 AND COALESCE(is_trashed, 0) = 0
            )",
            params![clip_id, content_hash],
            |row| row.get(0),
        )?;
        if !clip_matches {
            transaction.rollback()?;
            return Ok(false);
        }
        transaction.execute(
            "DELETE FROM clip_analysis_results
             WHERE clip_id = ?1 AND participant_ref LIKE 'extractor:%'",
            [clip_id],
        )?;
        let (run_id, run_at): (String, String) = transaction.query_row(
            "SELECT lower(hex(randomblob(16))), strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        for (participant_ref, priority, result_json) in serialized {
            transaction.execute(
                "INSERT INTO clip_analysis_results
                    (clip_id, participant_ref, content_hash, input_hash, format_version,
                     result_json, updated_at)
                 VALUES (?1, ?2, ?3, ?3, ?4, ?5, ?6)",
                params![
                    clip_id,
                    participant_ref,
                    content_hash,
                    crate::analysis_contract::ANALYSIS_CONTRACT_VERSION,
                    result_json,
                    run_at,
                ],
            )?;
            transaction.execute(
                "INSERT INTO clip_extraction_attempts
                    (clip_id, run_id, participant_ref, content_hash, priority, result_json, run_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    clip_id,
                    run_id,
                    participant_ref,
                    content_hash,
                    priority,
                    result_json,
                    run_at,
                ],
            )?;
        }
        transaction.commit()?;
        Ok(true)
    }

    pub fn get_extraction_observations(
        &self,
        clip_id: i64,
    ) -> Result<Vec<StoredExtractionObservation>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT results.result_json, results.updated_at
             FROM clip_analysis_results AS results
             JOIN clips ON clips.id = results.clip_id
             WHERE results.clip_id = ?1
               AND results.participant_ref LIKE 'extractor:%'
               AND results.content_hash = clips.content_hash
               AND results.input_hash = clips.content_hash
               AND results.format_version = ?2
               AND COALESCE(clips.is_trashed, 0) = 0
             ORDER BY CAST(json_extract(results.result_json, '$.priority') AS INTEGER),
                      results.participant_ref",
        )?;
        let observations = statement
            .query_map(
                params![clip_id, crate::analysis_contract::ANALYSIS_CONTRACT_VERSION],
                |row| {
                    let result_json: String = row.get(0)?;
                    let observation = serde_json::from_str(&result_json).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?;
                    Ok(StoredExtractionObservation {
                        observation,
                        updated_at: row.get(1)?,
                    })
                },
            )?
            .collect();
        observations
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
            "SELECT attempts.result_json, attempts.run_id, attempts.run_at
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
            .query_map(params![clip_id, limit as i64, offset as i64], |row| {
                let result_json: String = row.get(0)?;
                let observation = serde_json::from_str(&result_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
                Ok(StoredExtractionAttempt {
                    observation,
                    run_id: row.get(1)?,
                    run_at: row.get(2)?,
                })
            })?
            .collect();
        attempts
    }
}
