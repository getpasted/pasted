use rusqlite::{params, Result, Transaction};

use super::super::{AnalysisFailureClass, DbState, ExtractionAttemptContext};

pub(super) fn append_extraction_attempts(
    transaction: &Transaction<'_>,
    clip_id: i64,
    content_hash: &str,
    run_id: &str,
    run_at: &str,
    observations: &[crate::content_analysis::ExtractionObservation],
    contexts: &[ExtractionAttemptContext],
) -> Result<()> {
    for observation in observations {
        let result_json = serde_json::to_string(observation)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        if result_json.len() > crate::resource_limits::MAX_OCR_TEXT_BYTES.saturating_add(8 * 1024) {
            return Err(rusqlite::Error::InvalidParameterName(
                "Extractor result exceeds its safety limit".into(),
            ));
        }
        let input_fingerprint = contexts
            .iter()
            .find(|context| context.participant_ref == observation.extractor_ref)
            .map(|context| context.input_fingerprint.as_str())
            .ok_or_else(|| {
                rusqlite::Error::InvalidParameterName("Extractor attempt context is missing".into())
            })?;
        let failure_class = crate::analysis_attempt_policy::failure_class(&observation.outcome);
        let retry_after = if failure_class == Some(AnalysisFailureClass::Transient) {
            let previous_failures: i64 = transaction.query_row(
                "SELECT COUNT(*) FROM clip_extraction_attempts
                 WHERE clip_id = ?1 AND participant_ref = ?2 AND input_fingerprint = ?3
                   AND failure_class = 'transient'
                   AND id > COALESCE((
                        SELECT MAX(id) FROM clip_extraction_attempts
                        WHERE clip_id = ?1 AND participant_ref = ?2
                          AND input_fingerprint = ?3
                          AND (failure_class IS NULL OR failure_class != 'transient')
                   ), 0)",
                params![clip_id, observation.extractor_ref, input_fingerprint],
                |row| row.get(0),
            )?;
            crate::analysis_attempt_policy::retry_after(
                run_at,
                usize::try_from(previous_failures).unwrap_or_default() + 1,
            )
        } else {
            None
        };
        transaction.execute(
            "INSERT INTO clip_extraction_attempts
                (clip_id, run_id, participant_ref, content_hash, priority, result_json, run_at,
                 input_fingerprint, failure_class, retry_after)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                clip_id,
                run_id,
                observation.extractor_ref,
                content_hash,
                observation.priority,
                result_json,
                run_at,
                input_fingerprint,
                failure_class.as_ref().map(AnalysisFailureClass::as_str),
                retry_after,
            ],
        )?;
        DbState::prune_analysis_attempts_internal(
            transaction,
            clip_id,
            &observation.extractor_ref,
        )?;
    }
    Ok(())
}
