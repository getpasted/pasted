use rusqlite::{params, Connection, Result, Transaction};
use serde::{Deserialize, Serialize};

use super::{ClipRevisionContext, DbState};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ClipRevisionDerivedState {
    pub(super) ocr: OcrRevisionState,
    #[serde(default)]
    extraction_results: Vec<RevisionAnalysisResult>,
    #[serde(default)]
    classifications: Vec<RevisionClassification>,
    #[serde(default)]
    visual_label_overrides: Vec<RevisionVisualLabelOverride>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct OcrRevisionState {
    status: String,
    input_hash: Option<String>,
    engine_version: Option<String>,
    extractor_ref: Option<String>,
    extractor_name: Option<String>,
    attempted_at: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RevisionAnalysisResult {
    participant_ref: String,
    content_hash: String,
    input_hash: String,
    format_version: i64,
    result_json: String,
    updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RevisionClassification {
    content_type: String,
    classifier_ref: String,
    source_representation: String,
    input_hash: String,
    start_offset: Option<i64>,
    end_offset: Option<i64>,
    updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RevisionVisualLabelOverride {
    label: String,
    operation: String,
    updated_at: String,
}

impl ClipRevisionDerivedState {
    pub(super) fn capture(conn: &Connection, clip_id: i64) -> Result<Self> {
        let (text, mut ocr) = conn.query_row(
            "SELECT text_content, ocr_status, ocr_input_hash, ocr_engine_version,
                    ocr_extractor_ref, ocr_extractor_name, ocr_attempted_at, ocr_error
             FROM clips WHERE id = ?1",
            [clip_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    OcrRevisionState {
                        status: row.get(1)?,
                        input_hash: row.get(2)?,
                        engine_version: row.get(3)?,
                        extractor_ref: row.get(4)?,
                        extractor_name: row.get(5)?,
                        attempted_at: row.get(6)?,
                        error: row.get(7)?,
                    },
                ))
            },
        )?;
        let extraction_results = query_extraction_results(conn, clip_id)?;
        if matches!(ocr.status.as_str(), "queued" | "running") {
            ocr.status = stable_ocr_status(text.as_deref(), &ocr, extraction_results.is_empty());
        }
        Ok(Self {
            ocr,
            extraction_results,
            classifications: query_classifications(conn, clip_id)?,
            visual_label_overrides: query_visual_label_overrides(conn, clip_id)?,
        })
    }

    pub(super) fn original(content_hash: &str) -> Self {
        Self {
            ocr: OcrRevisionState {
                status: "never".into(),
                input_hash: Some(content_hash.into()),
                engine_version: None,
                extractor_ref: None,
                extractor_name: None,
                attempted_at: None,
                error: None,
            },
            extraction_results: Vec::new(),
            classifications: Vec::new(),
            visual_label_overrides: Vec::new(),
        }
    }

    pub(super) fn legacy_text(content_hash: &str) -> Self {
        let mut state = Self::original(content_hash);
        state.ocr.status = "complete".into();
        state.ocr.engine_version = Some("legacy".into());
        state.ocr.extractor_name = Some("Legacy OCR".into());
        state
    }

    pub(super) fn is_original(&self, text: Option<&str>) -> bool {
        text.is_none_or(str::is_empty)
            && self.extraction_results.is_empty()
            && self.ocr.extractor_ref.is_none()
            && self.ocr.attempted_at.is_none()
    }

    pub(super) fn extraction_results_match(
        &self,
        observations: &[crate::content_analysis::ExtractionObservation],
    ) -> Result<bool> {
        let mut incoming = observations
            .iter()
            .map(|observation| {
                serde_json::to_string(observation)
                    .map(|json| (observation.extractor_ref.as_str(), json))
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
            })
            .collect::<Result<Vec<_>>>()?;
        incoming.sort_by(|left, right| left.0.cmp(right.0));
        Ok(incoming.len() == self.extraction_results.len()
            && incoming.iter().zip(&self.extraction_results).all(
                |((participant_ref, result_json), stored)| {
                    *participant_ref == stored.participant_ref && *result_json == stored.result_json
                },
            ))
    }

    pub(super) fn effective_visual_labels(
        &self,
        clip_id: i64,
    ) -> super::clip_visual_labels::EffectiveVisualLabels {
        let detected = self
            .extraction_results
            .iter()
            .filter_map(|row| {
                serde_json::from_str::<crate::content_analysis::ExtractionObservation>(
                    &row.result_json,
                )
                .ok()
            })
            .flat_map(|observation| match observation.outcome {
                crate::content_extraction::ExtractionOutcome::Produced { labels, .. } => labels,
                _ => Vec::new(),
            })
            .collect();
        let overrides = self
            .visual_label_overrides
            .iter()
            .map(|row| (row.label.clone(), row.operation.clone()))
            .collect();
        super::clip_visual_labels::effective_labels(clip_id, detected, overrides)
    }

    pub(super) fn restore(&self, tx: &Transaction<'_>, clip_id: i64) -> Result<()> {
        tx.execute(
            "UPDATE clips SET ocr_status = ?1, ocr_input_hash = ?2,
                    ocr_engine_version = ?3, ocr_extractor_ref = ?4,
                    ocr_extractor_name = ?5, ocr_attempted_at = ?6, ocr_error = ?7
             WHERE id = ?8",
            params![
                self.ocr.status,
                self.ocr.input_hash,
                self.ocr.engine_version,
                self.ocr.extractor_ref,
                self.ocr.extractor_name,
                self.ocr.attempted_at,
                self.ocr.error,
                clip_id,
            ],
        )?;
        restore_extraction_results(tx, clip_id, &self.extraction_results)?;
        restore_classifications(tx, clip_id, &self.classifications)?;
        restore_visual_label_overrides(tx, clip_id, &self.visual_label_overrides)
    }
}

impl DbState {
    pub(crate) fn current_extraction_results_match(
        &self,
        clip_id: i64,
        observations: &[crate::content_analysis::ExtractionObservation],
    ) -> Result<bool> {
        ClipRevisionDerivedState::capture(&self.conn.lock(), clip_id)?
            .extraction_results_match(observations)
    }

    pub(super) fn snapshot_derived_revision_internal(
        tx: &Transaction<'_>,
        clip_id: i64,
        action_kind: &str,
        action_label: &str,
    ) -> Result<()> {
        if !Self::revision_history_enabled_internal(tx) {
            return Ok(());
        }
        let (text, current_transformation_id): (Option<String>, Option<String>) = tx.query_row(
            "SELECT text_content, current_transformation_id FROM clips WHERE id = ?1",
            [clip_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let derived_state = ClipRevisionDerivedState::capture(tx, clip_id)?;
        let is_original = derived_state.is_original(text.as_deref());
        let context = ClipRevisionContext {
            schema_version: 2,
            action_kind: if is_original { "original" } else { action_kind }.into(),
            action_label: if is_original {
                "Original"
            } else {
                action_label
            }
            .into(),
            organization: None,
            current_transformation_id,
            derived_state: Some(derived_state),
        };
        let context_json = serde_json::to_string(&context)
            .map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))?;
        tx.execute(
            "INSERT INTO clip_versions (clip_id, text_content, context_json)
             VALUES (?1, ?2, ?3)",
            params![clip_id, text.as_deref().unwrap_or_default(), context_json],
        )?;
        Self::prune_clip_versions_internal(tx, clip_id)
    }
}

fn stable_ocr_status(
    text: Option<&str>,
    ocr: &OcrRevisionState,
    extraction_results_empty: bool,
) -> String {
    if ocr.attempted_at.is_none() && ocr.extractor_ref.is_none() && extraction_results_empty {
        "never"
    } else if ocr.error.is_some() {
        "failed"
    } else if text.is_some_and(|value| !value.trim().is_empty()) {
        "complete"
    } else {
        "no_text"
    }
    .into()
}

fn query_extraction_results(
    conn: &Connection,
    clip_id: i64,
) -> Result<Vec<RevisionAnalysisResult>> {
    let mut statement = conn.prepare(
        "SELECT participant_ref, content_hash, input_hash, format_version, result_json, updated_at
         FROM clip_analysis_results
         WHERE clip_id = ?1 AND participant_ref LIKE 'extractor:%'
         ORDER BY participant_ref",
    )?;
    let rows = statement
        .query_map([clip_id], |row| {
            Ok(RevisionAnalysisResult {
                participant_ref: row.get(0)?,
                content_hash: row.get(1)?,
                input_hash: row.get(2)?,
                format_version: row.get(3)?,
                result_json: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?
        .collect();
    rows
}

fn query_classifications(conn: &Connection, clip_id: i64) -> Result<Vec<RevisionClassification>> {
    let mut statement = conn.prepare(
        "SELECT content_type, classifier_ref, source_representation, input_hash,
                start_offset, end_offset, updated_at
         FROM clip_analysis_classifications WHERE clip_id = ?1 ORDER BY id",
    )?;
    let rows = statement
        .query_map([clip_id], |row| {
            Ok(RevisionClassification {
                content_type: row.get(0)?,
                classifier_ref: row.get(1)?,
                source_representation: row.get(2)?,
                input_hash: row.get(3)?,
                start_offset: row.get(4)?,
                end_offset: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?
        .collect();
    rows
}

fn query_visual_label_overrides(
    conn: &Connection,
    clip_id: i64,
) -> Result<Vec<RevisionVisualLabelOverride>> {
    let mut statement = conn.prepare(
        "SELECT label, operation, updated_at FROM clip_visual_label_overrides
         WHERE clip_id = ?1 ORDER BY label COLLATE NOCASE",
    )?;
    let rows = statement
        .query_map([clip_id], |row| {
            Ok(RevisionVisualLabelOverride {
                label: row.get(0)?,
                operation: row.get(1)?,
                updated_at: row.get(2)?,
            })
        })?
        .collect();
    rows
}

fn restore_extraction_results(
    tx: &Transaction<'_>,
    clip_id: i64,
    rows: &[RevisionAnalysisResult],
) -> Result<()> {
    tx.execute(
        "DELETE FROM clip_analysis_results
         WHERE clip_id = ?1 AND participant_ref LIKE 'extractor:%'",
        [clip_id],
    )?;
    for row in rows {
        tx.execute(
            "INSERT INTO clip_analysis_results
                (clip_id, participant_ref, content_hash, input_hash, format_version,
                 result_json, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                clip_id,
                row.participant_ref,
                row.content_hash,
                row.input_hash,
                row.format_version,
                row.result_json,
                row.updated_at,
            ],
        )?;
    }
    Ok(())
}

fn restore_classifications(
    tx: &Transaction<'_>,
    clip_id: i64,
    rows: &[RevisionClassification],
) -> Result<()> {
    tx.execute(
        "DELETE FROM clip_analysis_classifications WHERE clip_id = ?1",
        [clip_id],
    )?;
    for row in rows {
        tx.execute(
            "INSERT INTO clip_analysis_classifications
                (clip_id, content_type, classifier_ref, source_representation, input_hash,
                 start_offset, end_offset, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                clip_id,
                row.content_type,
                row.classifier_ref,
                row.source_representation,
                row.input_hash,
                row.start_offset,
                row.end_offset,
                row.updated_at,
            ],
        )?;
    }
    Ok(())
}

fn restore_visual_label_overrides(
    tx: &Transaction<'_>,
    clip_id: i64,
    rows: &[RevisionVisualLabelOverride],
) -> Result<()> {
    tx.execute(
        "DELETE FROM clip_visual_label_overrides WHERE clip_id = ?1",
        [clip_id],
    )?;
    for row in rows {
        tx.execute(
            "INSERT INTO clip_visual_label_overrides (clip_id, label, operation, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![clip_id, row.label, row.operation, row.updated_at],
        )?;
    }
    Ok(())
}
