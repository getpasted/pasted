use super::super::*;

pub(crate) fn migrate_smart_paste_feature_setting(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value)
         SELECT 'enableSmartPaste', value FROM settings WHERE key = 'enableTransformations'",
        [],
    )?;
    Ok(())
}
#[cfg(test)]
mod smart_paste_tests {
    use super::*;

    #[test]
    fn migration_preserves_old_disabled_setting_and_new_explicit_choice() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO settings VALUES ('enableTransformations', 'false');",
        )
        .unwrap();
        migrate_smart_paste_feature_setting(&conn).unwrap();
        let migrated: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'enableSmartPaste'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(migrated, "false");
        conn.execute(
            "UPDATE settings SET value = 'true' WHERE key = 'enableSmartPaste'",
            [],
        )
        .unwrap();
        migrate_smart_paste_feature_setting(&conn).unwrap();
        let explicit: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'enableSmartPaste'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(explicit, "true");
    }
}
