use super::*;

pub(super) fn finalize_content_registry(conn: &Connection) -> Result<()> {
    let legacy_type_ids = {
        let mut statement = conn.prepare(
            "SELECT content_type FROM content_classifiers
             UNION SELECT content_type FROM clips
             ORDER BY content_type",
        )?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>>>()?;
        ids
    };
    for id in legacy_type_ids {
        if crate::content_types::is_structural_clip_type_id(&id) {
            continue;
        }
        conn.execute(
            "INSERT OR IGNORE INTO content_types
                (id, label, icon, group_name, is_builtin, is_archived)
             VALUES (?1, ?2, 'FileText', 'custom', 0, 0)",
            params![id, crate::content_types::fallback_label(&id)],
        )?;
    }
    let classifier_migration_applied: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE key = 'contentClassifierRegistryV1')",
        [],
        |row| row.get(0),
    )?;
    if !classifier_migration_applied {
        for (setting_key, stable_ref) in [
            ("detectColors", "color"),
            ("detectLinks", "url"),
            ("detectCode", "code"),
        ] {
            let disabled: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM settings WHERE key = ?1 AND value = 'false')",
                params![setting_key],
                |row| row.get(0),
            )?;
            if disabled {
                conn.execute(
                    "UPDATE content_classifiers SET enabled = 0 WHERE stable_ref = ?1",
                    params![stable_ref],
                )?;
            }
        }
        conn.execute(
            "INSERT INTO schema_migrations (key) VALUES ('contentClassifierRegistryV1')",
            [],
        )?;
    }
    Ok(())
}
