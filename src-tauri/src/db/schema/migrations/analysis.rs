use super::super::*;

pub(crate) fn migrate_multi_type_classifications(conn: &Connection) -> Result<()> {
    if !table_exists(conn, "clip_analysis_classifications")?
        || column_exists(conn, "clip_analysis_classifications", "start_offset")?
    {
        return Ok(());
    }
    let reference_column =
        if column_exists(conn, "clip_analysis_classifications", "classifier_ref")? {
            "classifier_ref"
        } else if column_exists(conn, "clip_analysis_classifications", "detector_ref")? {
            "detector_ref"
        } else {
            return Err(rusqlite::Error::InvalidParameterName(
                "Legacy classifications have no participant reference".into(),
            ));
        };
    let transaction = conn.unchecked_transaction()?;
    transaction.execute_batch(&format!(
        "DROP TABLE IF EXISTS clip_analysis_classifications_multi;
         CREATE TABLE clip_analysis_classifications_multi (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
            content_type TEXT NOT NULL,
            classifier_ref TEXT NOT NULL,
            source_representation TEXT NOT NULL
                CHECK (source_representation IN ('original_text', 'searchable_text')),
            input_hash TEXT NOT NULL,
            start_offset INTEGER,
            end_offset INTEGER,
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
            CHECK (
                (start_offset IS NULL AND end_offset IS NULL)
                OR (start_offset >= 0 AND end_offset > start_offset)
            )
         );
         INSERT INTO clip_analysis_classifications_multi
            (clip_id, content_type, classifier_ref, source_representation, input_hash,
             start_offset, end_offset, updated_at)
         SELECT clip_id, content_type, {reference_column}, source_representation, input_hash,
                NULL, NULL, updated_at
         FROM clip_analysis_classifications;
         DROP TABLE clip_analysis_classifications;
         ALTER TABLE clip_analysis_classifications_multi
            RENAME TO clip_analysis_classifications;"
    ))?;
    transaction.commit()
}

pub(crate) fn migrate_legacy_semantic_clip_types(conn: &Connection) -> Result<()> {
    let transaction = conn.unchecked_transaction()?;
    transaction.execute(
        "INSERT INTO clip_analysis_classifications
            (clip_id, content_type, classifier_ref, source_representation, input_hash,
             start_offset, end_offset)
         SELECT clips.id, clips.content_type,
                COALESCE(
                    (SELECT classifiers.stable_ref
                     FROM content_classifiers AS classifiers
                     WHERE classifiers.content_type = clips.content_type
                       AND classifiers.is_deleted = 0
                     ORDER BY classifiers.priority, classifiers.id
                     LIMIT 1),
                    'legacy:' || clips.content_type
                ),
                'original_text', clips.content_hash, NULL, NULL
         FROM clips
         WHERE clips.content_type NOT IN ('text', 'image', 'file')
           AND TRIM(clips.content_type) != ''
           AND NOT EXISTS (
                SELECT 1 FROM clip_analysis_classifications AS existing
                WHERE existing.clip_id = clips.id
                  AND existing.input_hash = clips.content_hash
                  AND existing.content_type = clips.content_type
           )",
        [],
    )?;
    transaction.execute(
        "UPDATE clips
         SET content_type = 'text'
         WHERE content_type NOT IN ('text', 'image', 'file')
           AND TRIM(content_type) != ''",
        [],
    )?;
    transaction.commit()
}

pub(crate) fn retire_structural_content_type_entries(conn: &Connection) -> Result<()> {
    if table_exists(conn, "bins")? && column_exists(conn, "bins", "smart_rule")? {
        let rules = {
            let mut statement =
                conn.prepare("SELECT id, smart_rule FROM bins WHERE smart_rule IS NOT NULL")?;
            let rules = statement
                .query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>>>()?;
            rules
        };
        for (id, rule_json) in rules {
            let Ok(mut rule) = serde_json::from_str::<serde_json::Value>(&rule_json) else {
                continue;
            };
            let mut changed = false;
            let mut migrate_condition = |condition: &mut serde_json::Value| {
                let is_legacy_structural = condition["type"].as_str() == Some("content_type")
                    && condition["value"]
                        .as_str()
                        .is_some_and(crate::content_types::is_structural_clip_type_id);
                if is_legacy_structural {
                    condition["type"] = serde_json::Value::String("clip_type".into());
                    changed = true;
                }
            };
            if let Some(conditions) = rule["conditions"].as_array_mut() {
                for condition in conditions {
                    migrate_condition(condition);
                }
            } else {
                migrate_condition(&mut rule);
            }
            if changed {
                conn.execute(
                    "UPDATE bins SET smart_rule = ?1 WHERE id = ?2",
                    params![
                        serde_json::to_string(&rule).map_err(|error| {
                            rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                        })?,
                        id
                    ],
                )?;
            }
        }
    }
    conn.execute(
        "DELETE FROM content_types
         WHERE id IN ('text', 'image', 'file')
           AND NOT EXISTS (
                SELECT 1 FROM content_classifiers
                WHERE content_classifiers.content_type = content_types.id
                  AND content_classifiers.is_deleted = 0
           )",
        [],
    )?;
    Ok(())
}

pub(crate) fn migrate_analysis_terminology_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "DROP TRIGGER IF EXISTS library_items_detector_insert;
         DROP TRIGGER IF EXISTS library_items_detector_update;
         DROP TRIGGER IF EXISTS library_items_detector_delete;",
    )?;
    let has_legacy_classifiers = table_exists(conn, "content_detectors")?;
    let has_classifiers = table_exists(conn, "content_classifiers")?;
    if has_legacy_classifiers && !has_classifiers {
        conn.execute(
            "ALTER TABLE content_detectors RENAME TO content_classifiers",
            [],
        )?;
    } else if has_legacy_classifiers {
        let transaction = conn.unchecked_transaction()?;
        transaction.execute_batch(
            "INSERT OR IGNORE INTO content_classifiers
                (stable_ref, name, content_type, description, patterns_json, validator,
                 enabled, priority, is_builtin, is_deleted, created_at, updated_at)
             SELECT stable_ref, name, content_type, description, patterns_json, validator,
                    enabled, priority, is_builtin, is_deleted, created_at, updated_at
             FROM content_detectors;
             DROP TABLE content_detectors;",
        )?;
        transaction.commit()?;
    }
    conn.execute("DROP INDEX IF EXISTS idx_content_detectors_order", [])?;

    if table_exists(conn, "settings")? {
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value)
             SELECT 'enableContentClassification', value
             FROM settings WHERE key = 'enableContentDetection'",
            [],
        )?;
        conn.execute(
            "DELETE FROM settings WHERE key = 'enableContentDetection'",
            [],
        )?;
    }
    if table_exists(conn, "schema_migrations")? {
        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (key)
             SELECT 'contentClassifierRegistryV1'
             FROM schema_migrations WHERE key = 'contentDetectorRegistryV1'",
            [],
        )?;
        conn.execute(
            "DELETE FROM schema_migrations WHERE key = 'contentDetectorRegistryV1'",
            [],
        )?;
    }
    Ok(())
}
