use super::*;

const MIGRATION_KEY: &str = "analysisTransformCanonicalTimestampsV1";

fn prepare_registered_migration() -> (crate::db::DbState, i64) {
    let db = crate::db::tests::setup_test_db();
    let clip_id = db.save_text_clip("person@example.com", "Tests").unwrap().id;
    db.conn
        .lock()
        .execute(
            "DELETE FROM schema_migrations WHERE key = ?1",
            [MIGRATION_KEY],
        )
        .unwrap();
    (db, clip_id)
}

#[test]
fn registered_migration_records_success_only_after_normalization() {
    let (db, clip_id) = prepare_registered_migration();
    let conn = db.conn.lock();
    conn.execute(
        "UPDATE clip_analysis_classifications
         SET updated_at = '2026-08-16 23:45:00' WHERE clip_id = ?1",
        [clip_id],
    )
    .unwrap();

    crate::db::schema::run_registered_migrations(&conn).unwrap();

    let (timestamp, applied): (String, bool) = conn
        .query_row(
            "SELECT classifications.updated_at,
                    EXISTS(SELECT 1 FROM schema_migrations WHERE key = ?1)
             FROM clip_analysis_classifications AS classifications
             WHERE classifications.clip_id = ?2",
            params![MIGRATION_KEY, clip_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(timestamp, "2026-08-16T23:45:00Z");
    assert!(applied);
}

#[test]
fn registered_migration_does_not_record_or_partially_apply_a_failure() {
    let (db, clip_id) = prepare_registered_migration();
    let conn = db.conn.lock();
    conn.execute(
        "INSERT INTO saved_transforms
            (id, name, plan_json, created_at, updated_at)
         VALUES ('rollback-transform', 'Rollback', '{}',
                 '2026-08-16 23:45:00', '2026-08-16 23:46:00')",
        [],
    )
    .unwrap();
    conn.execute(
        "UPDATE clip_analysis_classifications
         SET updated_at = 'not-a-timestamp' WHERE clip_id = ?1",
        [clip_id],
    )
    .unwrap();

    assert!(crate::db::schema::run_registered_migrations(&conn).is_err());
    let (created_at, applied): (String, bool) = conn
        .query_row(
            "SELECT created_at,
                    EXISTS(SELECT 1 FROM schema_migrations WHERE key = ?1)
             FROM saved_transforms WHERE id = 'rollback-transform'",
            [MIGRATION_KEY],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(created_at, "2026-08-16 23:45:00");
    assert!(!applied);
}
