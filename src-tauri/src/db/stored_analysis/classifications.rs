use rusqlite::{params, Result};

use super::super::{AnalysisClassification, DbState};

#[cfg(test)]
mod tests;

impl DbState {
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
                     start_offset, end_offset, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7,
                         strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
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
}
