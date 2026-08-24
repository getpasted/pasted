use rusqlite::{params, Result};

use super::super::{DbState, ExtractionAttemptContext, StoredExtractionObservation};

#[cfg(test)]
mod tests;

impl DbState {
    pub fn record_extraction_observations(
        &self,
        clip_id: i64,
        content_hash: &str,
        observations: &[crate::content_analysis::ExtractionObservation],
    ) -> Result<bool> {
        let contexts = crate::analysis_attempt_policy::legacy_contexts(content_hash, observations);
        self.record_extraction_observations_with_context(
            clip_id,
            content_hash,
            observations,
            observations,
            &contexts,
        )
    }

    pub fn record_extraction_observations_with_context(
        &self,
        clip_id: i64,
        content_hash: &str,
        observations: &[crate::content_analysis::ExtractionObservation],
        attempt_observations: &[crate::content_analysis::ExtractionObservation],
        contexts: &[ExtractionAttemptContext],
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
                Ok((observation.extractor_ref.as_str(), json))
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
        for (participant_ref, result_json) in serialized {
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
        }
        super::attempt_writes::append_extraction_attempts(
            &transaction,
            clip_id,
            content_hash,
            &run_id,
            &run_at,
            attempt_observations,
            contexts,
        )?;
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
}
