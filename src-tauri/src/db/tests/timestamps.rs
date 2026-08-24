use super::*;

#[test]
fn canonical_timestamps_preserve_instants_and_reject_malformed_imports() {
    assert_eq!(
        canonical_utc_timestamp("2026-08-16 23:45:00", "Test").unwrap(),
        "2026-08-16T23:45:00Z"
    );
    assert_eq!(
        canonical_utc_timestamp("2026-08-16T18:45:00-05:00", "Test").unwrap(),
        "2026-08-16T23:45:00Z"
    );
    assert!(canonical_utc_timestamp("tomorrow sometime", "Test").is_err());

    let source = setup_test_db();
    source
        .save_clip(
            "text",
            Some("Timestamp import"),
            None,
            None,
            "timestamp-import",
            "Tests",
        )
        .unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(&source.export_clips_json().unwrap()).unwrap();
    payload[0]["created_at"] = serde_json::json!("2026-08-16 23:45:00");

    let target = setup_test_db();
    target
        .import_clips_json(&serde_json::to_string(&payload).unwrap())
        .unwrap();
    assert_eq!(
        target.get_clips(None, false).unwrap()[0].created_at,
        "2026-08-16T23:45:00Z"
    );

    payload[0]["created_at"] = serde_json::json!("not-a-timestamp");
    let invalid_target = setup_test_db();
    assert!(invalid_target
        .inspect_clips_json(&serde_json::to_string(&payload).unwrap())
        .is_err());
    assert!(invalid_target.get_clips(None, false).unwrap().is_empty());

    source
        .create_pipeline(
            "Timestamp Transform",
            &[PipelineStepInput {
                operation_ref: "builtin:trim".into(),
                config_json: None,
                failure_policy: "stop".into(),
            }],
            None,
        )
        .unwrap();
    let mut archive: serde_json::Value =
        serde_json::from_str(&source.export_backup_json().unwrap()).unwrap();
    archive["bins"][0]["created_at"] = serde_json::json!("2026-08-16T18:45:00-05:00");
    archive["saved_transforms"][0]["createdAt"] = serde_json::json!("2026-08-16 23:45:00");
    let mut custom_operation = archive["operations"][0].clone();
    custom_operation["id"] = serde_json::json!(12345);
    custom_operation["stable_id"] = serde_json::json!("custom:timezone-test");
    custom_operation["name"] = serde_json::json!("Timezone Test");
    custom_operation["created_at"] = serde_json::json!("2026-08-16 23:45:00");
    archive["operations"]
        .as_array_mut()
        .unwrap()
        .push(custom_operation);
    let (normalized, _) =
        DbState::parse_library_archive(&serde_json::to_string(&archive).unwrap()).unwrap();
    assert_eq!(normalized.bins[0].created_at, "2026-08-16T23:45:00Z");
    assert_eq!(
        normalized.saved_transforms[0].created_at,
        "2026-08-16T23:45:00Z"
    );
    assert_eq!(
        normalized
            .operations
            .iter()
            .find(|operation| operation.id == 12345)
            .unwrap()
            .created_at,
        "2026-08-16T23:45:00Z"
    );

    archive["bins"][0]["created_at"] = serde_json::json!("not-a-timestamp");
    assert!(DbState::parse_library_archive(&serde_json::to_string(&archive).unwrap()).is_err());
}

#[test]
fn timestamp_migration_normalizes_legacy_timeline_values() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "text",
            Some("Legacy time"),
            None,
            None,
            "legacy-time",
            "Tests",
        )
        .unwrap();
    db.log_activity("app_started", "Tested legacy timestamp")
        .unwrap();
    let classified = db.save_text_clip("person@example.com", "Tests").unwrap();
    let conn = db.conn.lock();
    conn.execute(
        "UPDATE clips SET created_at = '2026-08-16 23:45:00' WHERE id = ?1",
        [clip.id],
    )
    .unwrap();
    conn.execute(
        "UPDATE activity_logs SET created_at = '2026-08-16 23:46:00'",
        [],
    )
    .unwrap();
    conn.execute(
        "UPDATE clip_analysis_classifications
             SET updated_at = '2026-08-16 23:47:00' WHERE clip_id = ?1",
        [classified.id],
    )
    .unwrap();
    conn.execute(
        "DELETE FROM schema_migrations WHERE key = 'canonicalUtcTimestampsV1'",
        [],
    )
    .unwrap();

    migrate_canonical_timestamps(&conn).unwrap();
    migrate_analysis_transform_timestamps(&conn).unwrap();
    let clip_timestamp: String = conn
        .query_row(
            "SELECT created_at FROM clips WHERE id = ?1",
            [clip.id],
            |row| row.get(0),
        )
        .unwrap();
    let activity_timestamp: String = conn
        .query_row("SELECT created_at FROM activity_logs LIMIT 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    let classification_timestamp: String = conn
        .query_row(
            "SELECT updated_at FROM clip_analysis_classifications WHERE clip_id = ?1",
            [classified.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(clip_timestamp, "2026-08-16T23:45:00Z");
    assert_eq!(activity_timestamp, "2026-08-16T23:46:00Z");
    assert_eq!(classification_timestamp, "2026-08-16T23:47:00Z");
}
