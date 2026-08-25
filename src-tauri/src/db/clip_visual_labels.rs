use rusqlite::{params, OptionalExtension, Result};
use serde::{Deserialize, Serialize};

use super::DbState;

pub(super) const fn search_condition() -> &'static str {
    "(EXISTS (
        SELECT 1 FROM clip_visual_label_overrides AS manual
        WHERE manual.clip_id = clips.id AND manual.operation = 'add'
          AND LOWER(manual.label) LIKE ? ESCAPE '\\'
    ) OR EXISTS (
        SELECT 1 FROM clip_analysis_results AS extracted,
             json_each(extracted.result_json, '$.labels') AS label
        WHERE extracted.clip_id = clips.id
          AND extracted.content_hash = clips.content_hash
          AND extracted.input_hash = clips.content_hash
          AND json_extract(extracted.result_json, '$.outcome') = 'produced'
          AND LOWER(json_extract(label.value, '$.value')) LIKE ? ESCAPE '\\'
          AND NOT EXISTS (
              SELECT 1 FROM clip_visual_label_overrides AS suppressed
              WHERE suppressed.clip_id = clips.id
                AND suppressed.operation = 'suppress'
                AND LOWER(suppressed.label) =
                    LOWER(json_extract(label.value, '$.value'))
          )
    ))"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VisualLabelSource {
    Detected,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveVisualLabel {
    pub value: String,
    pub confidence_basis_points: Option<u16>,
    pub source: VisualLabelSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveVisualLabels {
    pub clip_id: i64,
    pub labels: Vec<EffectiveVisualLabel>,
    pub has_overrides: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VisualLabelOverrideArchive {
    pub clip_id: i64,
    pub label: String,
    pub operation: String,
}

impl DbState {
    pub(in crate::db) fn get_visual_label_overrides_for_backup(
        &self,
    ) -> Result<Vec<VisualLabelOverrideArchive>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT clip_id, label, operation FROM clip_visual_label_overrides
             ORDER BY clip_id, label COLLATE NOCASE",
        )?;
        let overrides = statement
            .query_map([], |row| {
                Ok(VisualLabelOverrideArchive {
                    clip_id: row.get(0)?,
                    label: row.get(1)?,
                    operation: row.get(2)?,
                })
            })?
            .collect();
        overrides
    }

    pub fn get_effective_visual_labels(&self, clip_id: i64) -> Result<EffectiveVisualLabels> {
        let detected = self.detected_visual_labels(clip_id)?;
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT label, operation FROM clip_visual_label_overrides
             WHERE clip_id = ?1 ORDER BY updated_at, label COLLATE NOCASE",
        )?;
        let overrides = statement
            .query_map([clip_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>>>()?;
        Ok(effective_labels(clip_id, detected, overrides))
    }

    pub fn add_visual_label(&self, clip_id: i64, label: &str) -> Result<EffectiveVisualLabels> {
        let label = validate_label(label)?;
        self.require_active_clip(clip_id)?;
        let detected = self.detected_visual_labels(clip_id)?;
        let is_detected = detected
            .iter()
            .any(|candidate| candidate.value.eq_ignore_ascii_case(&label));
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let current = visual_label_override_operation(&tx, clip_id, &label)?;
        let changes = if is_detected {
            current.as_deref() == Some("suppress")
        } else {
            current.as_deref() != Some("add")
        };
        if changes {
            Self::snapshot_derived_revision_internal(
                &tx,
                clip_id,
                "visual_label_edit",
                "Before editing Visual Labels",
            )?;
        } else {
            tx.commit()?;
            drop(conn);
            return self.get_effective_visual_labels(clip_id);
        }
        if is_detected {
            tx.execute(
                "DELETE FROM clip_visual_label_overrides
                 WHERE clip_id = ?1 AND label = ?2 COLLATE NOCASE",
                params![clip_id, label],
            )?;
        } else {
            tx.execute(
                "INSERT INTO clip_visual_label_overrides(clip_id, label, operation, updated_at)
                 VALUES (?1, ?2, 'add', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                 ON CONFLICT(clip_id, label) DO UPDATE SET
                    label = excluded.label, operation = 'add', updated_at = excluded.updated_at",
                params![clip_id, label],
            )?;
        }
        tx.commit()?;
        drop(conn);
        self.get_effective_visual_labels(clip_id)
    }

    pub fn remove_visual_label(&self, clip_id: i64, label: &str) -> Result<EffectiveVisualLabels> {
        let label = validate_label(label)?;
        self.require_active_clip(clip_id)?;
        let detected = self.detected_visual_labels(clip_id)?;
        let is_detected = detected
            .iter()
            .any(|candidate| candidate.value.eq_ignore_ascii_case(&label));
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let current = visual_label_override_operation(&tx, clip_id, &label)?;
        let changes = if is_detected {
            current.as_deref() != Some("suppress")
        } else {
            current.as_deref() == Some("add")
        };
        if changes {
            Self::snapshot_derived_revision_internal(
                &tx,
                clip_id,
                "visual_label_edit",
                "Before editing Visual Labels",
            )?;
        }
        if is_detected {
            tx.execute(
                "INSERT INTO clip_visual_label_overrides(clip_id, label, operation, updated_at)
                 VALUES (?1, ?2, 'suppress', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                 ON CONFLICT(clip_id, label) DO UPDATE SET
                    label = excluded.label, operation = 'suppress', updated_at = excluded.updated_at",
                params![clip_id, label],
            )?;
        } else {
            tx.execute(
                "DELETE FROM clip_visual_label_overrides
                 WHERE clip_id = ?1 AND label = ?2 COLLATE NOCASE AND operation = 'add'",
                params![clip_id, label],
            )?;
        }
        tx.commit()?;
        drop(conn);
        self.get_effective_visual_labels(clip_id)
    }

    pub fn reset_visual_labels(&self, clip_id: i64) -> Result<EffectiveVisualLabels> {
        self.require_active_clip(clip_id)?;
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let has_overrides: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM clip_visual_label_overrides WHERE clip_id = ?1)",
            [clip_id],
            |row| row.get(0),
        )?;
        if has_overrides {
            Self::snapshot_derived_revision_internal(
                &tx,
                clip_id,
                "visual_label_edit",
                "Before resetting Visual Labels",
            )?;
        }
        tx.execute(
            "DELETE FROM clip_visual_label_overrides WHERE clip_id = ?1",
            [clip_id],
        )?;
        tx.commit()?;
        drop(conn);
        self.get_effective_visual_labels(clip_id)
    }

    fn detected_visual_labels(
        &self,
        clip_id: i64,
    ) -> Result<Vec<crate::content_extraction::VisualLabel>> {
        Ok(self
            .get_extraction_observations(clip_id)?
            .into_iter()
            .flat_map(|stored| match stored.observation.outcome {
                crate::content_extraction::ExtractionOutcome::Produced { labels, .. } => labels,
                _ => Vec::new(),
            })
            .collect())
    }

    fn require_active_clip(&self, clip_id: i64) -> Result<()> {
        let exists = self
            .conn
            .lock()
            .query_row(
                "SELECT 1 FROM clips WHERE id = ?1 AND COALESCE(is_trashed, 0) = 0",
                [clip_id],
                |_| Ok(()),
            )
            .optional()?;
        exists.ok_or(rusqlite::Error::QueryReturnedNoRows)
    }
}

fn visual_label_override_operation(
    conn: &rusqlite::Connection,
    clip_id: i64,
    label: &str,
) -> Result<Option<String>> {
    conn.query_row(
        "SELECT operation FROM clip_visual_label_overrides
         WHERE clip_id = ?1 AND label = ?2 COLLATE NOCASE",
        params![clip_id, label],
        |row| row.get(0),
    )
    .optional()
}

fn validate_label(label: &str) -> Result<String> {
    let label = label.split_whitespace().collect::<Vec<_>>().join(" ");
    let valid = !label.is_empty()
        && label.len() <= crate::content_extraction::MAX_VISUAL_LABEL_BYTES
        && !label.chars().any(char::is_control);
    valid.then_some(label).ok_or_else(|| {
        rusqlite::Error::InvalidParameterName(
            "Visual Labels require 1–120 characters without control characters".into(),
        )
    })
}

pub(super) fn effective_labels(
    clip_id: i64,
    detected: Vec<crate::content_extraction::VisualLabel>,
    overrides: Vec<(String, String)>,
) -> EffectiveVisualLabels {
    let suppressed = overrides
        .iter()
        .filter(|(_, operation)| operation == "suppress")
        .map(|(label, _)| label)
        .collect::<Vec<_>>();
    let mut labels = detected
        .into_iter()
        .filter(|label| {
            !suppressed
                .iter()
                .any(|suppressed| suppressed.eq_ignore_ascii_case(&label.value))
        })
        .map(|label| EffectiveVisualLabel {
            value: label.value,
            confidence_basis_points: label.confidence_basis_points,
            source: VisualLabelSource::Detected,
        })
        .collect::<Vec<_>>();
    for (value, operation) in &overrides {
        if operation == "add"
            && !labels
                .iter()
                .any(|label| label.value.eq_ignore_ascii_case(value))
            && labels.len() < crate::content_extraction::MAX_VISUAL_LABELS
        {
            labels.push(EffectiveVisualLabel {
                value: value.clone(),
                confidence_basis_points: None,
                source: VisualLabelSource::Manual,
            });
        }
    }
    EffectiveVisualLabels {
        clip_id,
        labels,
        has_overrides: !overrides.is_empty(),
    }
}
