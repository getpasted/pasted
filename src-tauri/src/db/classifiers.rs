use std::collections::HashSet;

use rusqlite::{params, OptionalExtension, Result};

use super::{content_classifier_from_row, ContentClassificationRescanReport, DbState};

impl DbState {
    pub fn get_content_classifiers(
        &self,
    ) -> Result<Vec<crate::content_classification::Classifier>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, stable_ref, name, content_type, description, patterns_json,
                    validator, enabled, priority, is_builtin, is_deleted
             FROM content_classifiers WHERE is_deleted = 0 ORDER BY priority, id",
        )?;
        let rows = statement.query_map([], content_classifier_from_row)?;
        rows.collect()
    }

    pub fn get_content_classifier(
        &self,
        reference: &str,
    ) -> Result<crate::content_classification::Classifier> {
        let numeric_id = reference.parse::<i64>().ok();
        self.get_content_classifiers()?
            .into_iter()
            .find(|classifier| {
                numeric_id == Some(classifier.id) || classifier.stable_ref == reference
            })
            .ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn duplicate_content_classifier(
        &self,
        reference: &str,
        name: Option<&str>,
    ) -> Result<crate::content_classification::Classifier> {
        let source = self.get_content_classifier(reference)?;
        self.create_content_classifier(&crate::content_classification::ClassifierInput {
            name: name
                .map(str::to_string)
                .unwrap_or_else(|| format!("{} Copy", source.name)),
            content_type: source.content_type,
            description: source.description,
            patterns: source.patterns,
            validator: source.validator,
            enabled: source.enabled,
            priority: source.priority.saturating_add(1).min(10_000),
        })
    }

    pub fn apply_content_classifier(
        &self,
        clip_id: i64,
        reference: &str,
    ) -> Result<crate::classification_execution::ClassificationApplicationResult> {
        let mut conn = self.conn.lock();
        let transaction = conn.transaction()?;
        let no_analyzable_text = || {
            rusqlite::Error::InvalidParameterName("The selected clip has no analyzable text".into())
        };
        let numeric_id = reference.parse::<i64>().ok();
        let classifier = transaction.query_row(
            "SELECT id, stable_ref, name, content_type, description, patterns_json,
                    validator, enabled, priority, is_builtin, is_deleted
             FROM content_classifiers
             WHERE is_deleted = 0 AND (stable_ref = ?1 OR id = ?2)
             LIMIT 1",
            params![reference, numeric_id],
            content_classifier_from_row,
        )?;
        let clip = transaction
            .query_row(
                "SELECT clips.content_type,
                        CASE
                          WHEN clips.content_type = 'file' THEN extracted.searchable_text
                          ELSE clips.text_content
                        END,
                        clips.content_hash,
                        CASE WHEN clips.content_type IN ('image', 'file')
                             THEN 'searchable_text' ELSE 'original_text' END
                 FROM clips
                 LEFT JOIN clip_searchable_text AS extracted
                   ON extracted.clip_id = clips.id
                  AND extracted.input_hash = clips.content_hash
                 WHERE id = ?1 AND COALESCE(is_trashed, 0) = 0",
                params![clip_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?;
        let Some((_clip_type, Some(text), content_hash, source_representation)) = clip else {
            return Err(no_analyzable_text());
        };
        if text.trim().is_empty() {
            return Err(no_analyzable_text());
        }
        let analysis = crate::classification_execution::analyze_classifier(&text, &classifier);
        let existing_count: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM clip_analysis_classifications
             WHERE clip_id = ?1 AND classifier_ref = ?2 AND input_hash = ?3",
            params![clip_id, classifier.stable_ref, content_hash],
            |row| row.get(0),
        )?;
        transaction.execute(
            "DELETE FROM clip_analysis_classifications
             WHERE clip_id = ?1 AND classifier_ref = ?2",
            params![clip_id, classifier.stable_ref],
        )?;
        for matched in &analysis.matches {
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
                    content_hash,
                    i64::try_from(matched.start_offset).map_err(|_| no_analyzable_text())?,
                    i64::try_from(matched.end_offset).map_err(|_| no_analyzable_text())?
                ],
            )?;
        }
        let changed = analysis.matched || existing_count > 0;
        transaction.commit()?;
        drop(conn);
        if changed {
            let _ = self.log_activity(
                "content_classifier_applied",
                &format!("Applied a Classifier to clip #{clip_id}"),
            );
        }
        Ok(if analysis.matched || existing_count > 0 {
            crate::classification_execution::ClassificationApplicationResult::applied(
                analysis, clip_id,
            )
        } else {
            crate::classification_execution::ClassificationApplicationResult::preview(analysis)
        })
    }

    pub(super) fn get_all_content_classifiers_for_backup(
        &self,
    ) -> Result<Vec<crate::content_classification::Classifier>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, stable_ref, name, content_type, description, patterns_json,
                    validator, enabled, priority, is_builtin, is_deleted
             FROM content_classifiers ORDER BY priority, id",
        )?;
        let rows = statement.query_map([], |row| {
            let patterns_json: String = row.get(5)?;
            let patterns = serde_json::from_str(&patterns_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            let stable_ref: String = row.get(1)?;
            let is_builtin: bool = row.get(9)?;
            Ok(crate::content_classification::Classifier {
                id: row.get(0)?,
                defaults: is_builtin
                    .then(|| crate::content_classification::classifier_defaults(&stable_ref))
                    .flatten(),
                stable_ref,
                name: row.get(2)?,
                content_type: row.get(3)?,
                description: row.get(4)?,
                patterns,
                validator: row.get(6)?,
                enabled: row.get(7)?,
                priority: row.get(8)?,
                is_builtin,
                is_deleted: row.get(10)?,
            })
        })?;
        rows.collect()
    }

    pub fn create_content_classifier(
        &self,
        input: &crate::content_classification::ClassifierInput,
    ) -> Result<crate::content_classification::Classifier> {
        crate::content_classification::validate_classifier_input(input).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                error,
            )))
        })?;
        if !self
            .get_content_types(false)?
            .iter()
            .any(|content_type| content_type.id == input.content_type)
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "Classifiers must use an active registered content type".into(),
            ));
        }
        let patterns_json = serde_json::to_string(&input.patterns)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let conn = self.conn.lock();
        let classifier_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM content_classifiers WHERE is_deleted = 0",
            [],
            |row| row.get(0),
        )?;
        if classifier_count >= 128 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Content classifiers are limited to 128 entries".to_string(),
            ));
        }
        conn.execute(
            "INSERT INTO content_classifiers
                (stable_ref, name, content_type, description, patterns_json, validator, enabled, priority, is_builtin)
             VALUES ('pending', ?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
            params![input.name.trim(), input.content_type.trim(), input.description.trim(), patterns_json, input.validator, input.enabled, input.priority],
        )?;
        let id = conn.last_insert_rowid();
        conn.execute(
            "UPDATE content_classifiers SET stable_ref = ?1 WHERE id = ?2",
            params![format!("custom-{id}"), id],
        )?;
        drop(conn);
        let classifier = self
            .get_content_classifiers()?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let _ = self.log_activity(
            "content_classifier_created",
            &format!("Created classifier \"{}\"", classifier.name),
        );
        Ok(classifier)
    }

    pub fn update_content_classifier(
        &self,
        id: i64,
        input: &crate::content_classification::ClassifierInput,
    ) -> Result<crate::content_classification::Classifier> {
        crate::content_classification::validate_classifier_input(input).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                error,
            )))
        })?;
        if !self
            .get_content_types(false)?
            .iter()
            .any(|content_type| content_type.id == input.content_type)
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "Classifiers must use an active registered content type".into(),
            ));
        }
        let patterns_json = serde_json::to_string(&input.patterns)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let conn = self.conn.lock();
        let previous_enabled = conn
            .query_row(
                "SELECT enabled FROM content_classifiers WHERE id = ?1 AND is_deleted = 0",
                params![id],
                |row| row.get::<_, bool>(0),
            )
            .optional()?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let changed = conn.execute(
            "UPDATE content_classifiers SET name = ?1, content_type = ?2, description = ?3,
                    patterns_json = ?4, validator = ?5, enabled = ?6, priority = ?7,
                    updated_at = CURRENT_TIMESTAMP
             WHERE id = ?8 AND is_deleted = 0",
            params![
                input.name.trim(),
                input.content_type.trim(),
                input.description.trim(),
                patterns_json,
                input.validator,
                input.enabled,
                input.priority,
                id
            ],
        )?;
        drop(conn);
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let classifier = self
            .get_content_classifiers()?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        self.log_analysis_participant_update(
            "classifier",
            &classifier.stable_ref,
            &classifier.name,
            previous_enabled,
            classifier.enabled,
        );
        Ok(classifier)
    }

    pub fn delete_content_classifier(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        let name = conn
            .query_row(
                "SELECT name FROM content_classifiers WHERE id = ?1 AND is_deleted = 0",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if name.is_none() {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        conn.execute(
            "UPDATE content_classifiers SET is_deleted = 1, enabled = 0, updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            params![id],
        )?;
        drop(conn);
        let name = name.expect("checked above");
        let _ = self.log_activity(
            "content_classifier_deleted",
            &format!("Deleted classifier \"{name}\""),
        );
        Ok(())
    }

    pub fn restore_default_content_classifiers(&self) -> Result<()> {
        let conn = self.conn.lock();
        for preset in crate::content_classification::CLASSIFIER_PRESETS {
            let patterns_json = serde_json::to_string(&preset.patterns)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            conn.execute(
                "UPDATE content_classifiers SET name = ?1, content_type = ?2, description = ?3,
                        patterns_json = ?4, validator = ?5, enabled = 1, priority = ?6,
                        is_deleted = 0, updated_at = CURRENT_TIMESTAMP WHERE stable_ref = ?7",
                params![
                    preset.name,
                    preset.content_type,
                    preset.description,
                    patterns_json,
                    preset.validator,
                    preset.priority,
                    preset.stable_ref
                ],
            )?;
        }
        drop(conn);
        let _ = self.log_activity(
            "content_classifiers_restored",
            "Restored shipped classifier defaults",
        );
        Ok(())
    }

    /// Reclassify every current original-text or searchable-text representation while
    /// preserving each clip's physical Text, Image, or File type.
    pub fn rescan_content_classification(&self) -> Result<ContentClassificationRescanReport> {
        const BATCH_SIZE: i64 = 128;

        let classifiers = self.get_content_classifiers()?;
        let mut conn = self.conn.lock();
        let transaction = conn.transaction()?;
        let mut last_id = 0i64;
        let mut scanned_count = 0usize;
        let mut changed_count = 0usize;
        let mut failed_count = 0usize;

        loop {
            let clips = {
                let mut statement = transaction.prepare(
                    "SELECT clips.id, clips.content_hash, clips.content_type,
                            CASE
                              WHEN clips.content_type = 'file' THEN extracted.searchable_text
                              ELSE clips.text_content
                            END,
                            CASE WHEN clips.content_type IN ('image', 'file')
                                 THEN 'searchable_text' ELSE 'original_text' END
                     FROM clips
                     LEFT JOIN clip_searchable_text AS extracted
                       ON extracted.clip_id = clips.id
                      AND extracted.input_hash = clips.content_hash
                     WHERE clips.id > ?1
                       AND CASE
                             WHEN clips.content_type = 'file' THEN extracted.searchable_text
                             ELSE clips.text_content
                           END IS NOT NULL
                     ORDER BY clips.id ASC
                     LIMIT ?2",
                )?;
                let rows = statement
                    .query_map(params![last_id, BATCH_SIZE], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                        ))
                    })?
                    .collect::<Result<Vec<_>>>()?;
                rows
            };
            if clips.is_empty() {
                break;
            }
            for (id, content_hash, content_type, text, source_representation) in clips {
                last_id = id;
                scanned_count += 1;
                if !matches!(content_type.as_str(), "text" | "image" | "file") {
                    transaction.execute(
                        "UPDATE clips SET content_type = 'text' WHERE id = ?1",
                        params![id],
                    )?;
                }
                if text.trim().is_empty() {
                    failed_count += 1;
                    continue;
                }
                let analysis = crate::classification_execution::analyze_classifiers_with_policy(
                    &text,
                    &classifiers,
                    crate::analysis_contract::AnalysisPolicy::Rescan,
                    None,
                );
                if analysis.failed() {
                    failed_count += 1;
                    continue;
                }
                let existing = {
                    let mut statement = transaction.prepare(
                        "SELECT classifier_ref, content_type, start_offset, end_offset
                         FROM clip_analysis_classifications
                         WHERE clip_id = ?1 AND input_hash = ?2",
                    )?;
                    let rows = statement
                        .query_map(params![id, content_hash], |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, Option<i64>>(2)?,
                                row.get::<_, Option<i64>>(3)?,
                            ))
                        })?
                        .collect::<Result<HashSet<_>>>()?;
                    rows
                };
                let has_stale_matches: bool = transaction.query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM clip_analysis_classifications
                        WHERE clip_id = ?1 AND input_hash != ?2
                    )",
                    params![id, content_hash],
                    |row| row.get(0),
                )?;
                let incoming = analysis
                    .matches
                    .iter()
                    .map(|matched| {
                        (
                            matched.classifier_ref.clone(),
                            matched.content_type.clone(),
                            i64::try_from(matched.start_offset).ok(),
                            i64::try_from(matched.end_offset).ok(),
                        )
                    })
                    .collect::<HashSet<_>>();
                if incoming != existing || has_stale_matches {
                    transaction.execute(
                        "DELETE FROM clip_analysis_classifications WHERE clip_id = ?1",
                        params![id],
                    )?;
                    for matched in &analysis.matches {
                        transaction.execute(
                            "INSERT INTO clip_analysis_classifications
                                (clip_id, content_type, classifier_ref, source_representation,
                                 input_hash, start_offset, end_offset)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                            params![
                                id,
                                matched.content_type,
                                matched.classifier_ref,
                                source_representation,
                                content_hash,
                                i64::try_from(matched.start_offset).map_err(|_| {
                                    rusqlite::Error::InvalidParameterName(
                                        "Classification offset exceeds its safety limit".into(),
                                    )
                                })?,
                                i64::try_from(matched.end_offset).map_err(|_| {
                                    rusqlite::Error::InvalidParameterName(
                                        "Classification offset exceeds its safety limit".into(),
                                    )
                                })?
                            ],
                        )?;
                    }
                    changed_count += 1;
                }
            }
        }
        transaction.commit()?;
        drop(conn);

        let report = ContentClassificationRescanReport {
            scanned_count,
            changed_count,
            unchanged_count: scanned_count
                .saturating_sub(changed_count)
                .saturating_sub(failed_count),
            failed_count,
        };
        let _ = self.log_activity(
            "content_classification_history_rescanned",
            &format!(
                "Rescanned {} clips; updated matches for {}; failed {}",
                report.scanned_count, report.changed_count, report.failed_count
            ),
        );
        Ok(report)
    }
}
