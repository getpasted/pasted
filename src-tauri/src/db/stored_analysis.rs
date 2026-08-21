use rusqlite::{params, OptionalExtension, Result};

use super::{
    ensure_resource_size, AnalysisClassification, ClipRevisionContext, ClipSearchableText, DbState,
    OcrBackfillStatus, OcrCandidate, OcrExtractorProvenance, StoredExtractionAttempt,
    StoredExtractionObservation,
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

    pub fn replace_analysis_classifications(
        &self,
        clip_id: i64,
        input_hash: &str,
        matches: &[crate::content_classification::ClassificationMatch],
        source_representation: &str,
    ) -> Result<bool> {
        if !matches!(source_representation, "original_text" | "searchable_text") {
            return Err(rusqlite::Error::InvalidParameterName(
                "Unknown analysis source representation".into(),
            ));
        }
        if matches.len() > crate::content_classification::MAX_CLASSIFICATION_MATCHES_PER_CLIP
            || matches.iter().any(|matched| {
                matched.content_type.len() > 80
                    || matched.classifier_ref.len() > 160
                    || matched.end_offset <= matched.start_offset
                    || i64::try_from(matched.start_offset).is_err()
                    || i64::try_from(matched.end_offset).is_err()
            })
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "Analysis classification metadata exceeds its safety limit".into(),
            ));
        }
        let mut conn = self.conn.lock();
        let transaction = conn.transaction()?;
        let clip_matches: bool = transaction.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM clips
                WHERE id = ?1 AND content_hash = ?2 AND COALESCE(is_trashed, 0) = 0
            )",
            params![clip_id, input_hash],
            |row| row.get(0),
        )?;
        if !clip_matches {
            return Ok(false);
        }
        transaction.execute(
            "DELETE FROM clip_analysis_classifications WHERE clip_id = ?1",
            params![clip_id],
        )?;
        for matched in matches {
            transaction.execute(
                "INSERT INTO clip_analysis_classifications
                    (clip_id, content_type, classifier_ref, source_representation, input_hash,
                     start_offset, end_offset)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    clip_id,
                    matched.content_type,
                    matched.classifier_ref,
                    source_representation,
                    input_hash,
                    i64::try_from(matched.start_offset).expect("validated classification offset"),
                    i64::try_from(matched.end_offset).expect("validated classification offset")
                ],
            )?;
        }
        transaction.commit()?;
        Ok(true)
    }

    pub fn get_analysis_classifications(
        &self,
        clip_id: i64,
    ) -> Result<Vec<AnalysisClassification>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT classifications.id, classifications.clip_id,
                    classifications.content_type, classifications.classifier_ref,
                    COALESCE(classifiers.name, classifications.classifier_ref),
                    COALESCE(classifiers.priority, 10000),
                    classifications.source_representation, classifications.input_hash,
                    classifications.start_offset, classifications.end_offset,
                    classifications.updated_at
             FROM clip_analysis_classifications AS classifications
             JOIN clips ON clips.id = classifications.clip_id
             LEFT JOIN content_classifiers AS classifiers
               ON classifiers.stable_ref = classifications.classifier_ref
             WHERE classifications.clip_id = ?1
               AND classifications.input_hash = clips.content_hash
             ORDER BY COALESCE(classifiers.priority, 10000), classifications.start_offset,
                      classifications.id",
        )?;
        let rows = statement
            .query_map(params![clip_id], |row| {
                let start_offset = row
                    .get::<_, Option<i64>>(8)?
                    .and_then(|value| usize::try_from(value).ok());
                let end_offset = row
                    .get::<_, Option<i64>>(9)?
                    .and_then(|value| usize::try_from(value).ok());
                Ok(AnalysisClassification {
                    id: row.get(0)?,
                    clip_id: row.get(1)?,
                    content_type: row.get(2)?,
                    classifier_ref: row.get(3)?,
                    classifier_name: row.get(4)?,
                    priority: row.get(5)?,
                    source_representation: row.get(6)?,
                    input_hash: row.get(7)?,
                    start_offset,
                    end_offset,
                    updated_at: row.get(10)?,
                })
            })?
            .collect();
        rows
    }

    pub fn replace_clip_searchable_text(
        &self,
        clip_id: i64,
        input_hash: &str,
        extractor: &crate::content_extraction::Extractor,
        searchable_text: Option<&str>,
    ) -> Result<bool> {
        if input_hash.len() > 128
            || extractor.stable_ref.len() > 160
            || extractor.name.len() > 80
            || extractor.engine.len() > 80
            || searchable_text
                .is_some_and(|text| text.len() > crate::resource_limits::MAX_OCR_TEXT_BYTES)
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "Searchable extraction exceeds its safety limit".into(),
            ));
        }
        let mut conn = self.conn.lock();
        let transaction = conn.transaction()?;
        let clip_matches: bool = transaction.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM clips
                WHERE id = ?1 AND content_hash = ?2 AND content_type = 'file'
                  AND COALESCE(is_trashed, 0) = 0
            )",
            params![clip_id, input_hash],
            |row| row.get(0),
        )?;
        if !clip_matches {
            transaction.rollback()?;
            return Ok(false);
        }
        if let Some(searchable_text) = searchable_text {
            transaction.execute(
                "INSERT INTO clip_searchable_text
                    (clip_id, extractor_ref, extractor_name, engine, input_hash, searchable_text)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(clip_id) DO UPDATE SET
                    extractor_ref = excluded.extractor_ref,
                    extractor_name = excluded.extractor_name,
                    engine = excluded.engine,
                    input_hash = excluded.input_hash,
                    searchable_text = excluded.searchable_text,
                    updated_at = CURRENT_TIMESTAMP",
                params![
                    clip_id,
                    extractor.stable_ref,
                    extractor.name,
                    extractor.engine,
                    input_hash,
                    searchable_text,
                ],
            )?;
        } else {
            transaction.execute(
                "DELETE FROM clip_searchable_text WHERE clip_id = ?1",
                params![clip_id],
            )?;
        }
        transaction.commit()?;
        Ok(true)
    }

    pub fn get_clip_searchable_text(&self, clip_id: i64) -> Result<Option<ClipSearchableText>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT extracted.clip_id, extracted.extractor_ref, extracted.extractor_name,
                    extracted.engine, extracted.input_hash, extracted.searchable_text,
                    extracted.updated_at
             FROM clip_searchable_text AS extracted
             JOIN clips ON clips.id = extracted.clip_id
             WHERE extracted.clip_id = ?1
               AND extracted.input_hash = clips.content_hash
               AND clips.content_type = 'file'
               AND COALESCE(clips.is_trashed, 0) = 0",
            params![clip_id],
            |row| {
                Ok(ClipSearchableText {
                    clip_id: row.get(0)?,
                    extractor_ref: row.get(1)?,
                    extractor_name: row.get(2)?,
                    engine: row.get(3)?,
                    input_hash: row.get(4)?,
                    searchable_text: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )
        .optional()
    }
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
                    (clip_id, participant_ref, content_hash, input_hash, format_version, result_json)
                 VALUES (?1, ?2, ?3, ?3, ?4, ?5)",
                params![
                    clip_id,
                    participant_ref,
                    content_hash,
                    crate::analysis_contract::ANALYSIS_CONTRACT_VERSION,
                    result_json,
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
