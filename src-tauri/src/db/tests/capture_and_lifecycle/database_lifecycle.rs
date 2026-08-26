use super::super::*;

#[test]
fn relocating_database_preserves_data_and_retains_the_source() {
    let db = setup_test_db();
    let source = db.database_path();
    let destination_directory = std::env::temp_dir().join(format!(
        "pasted_relocation_{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&destination_directory).unwrap();
    let destination = destination_directory.join("pasted.db");
    save_plain_test_clip(
        &db,
        "text",
        "Move me without losing me",
        "relocation-test-hash",
        "Test",
    );

    let retained = db.relocate_database(destination.clone()).unwrap();

    assert_eq!(retained, source);
    assert_eq!(db.database_path(), destination);
    assert!(retained.is_file());
    assert_eq!(
        db.get_clips(None, false).unwrap()[0]
            .text_content
            .as_deref(),
        Some("Move me without losing me")
    );
    let reopened = DbState::new(db.database_path()).unwrap();
    assert_eq!(reopened.get_clips(None, false).unwrap().len(), 1);
    let _ = fs::remove_file(retained);
    let _ = fs::remove_dir_all(destination_directory);
}

#[test]
fn relocating_database_never_overwrites_an_existing_target() {
    let db = setup_test_db();
    let destination_directory = std::env::temp_dir().join(format!(
        "pasted_relocation_existing_{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&destination_directory).unwrap();
    let destination = destination_directory.join("pasted.db");
    fs::write(&destination, b"keep this file").unwrap();

    assert!(db.relocate_database(destination.clone()).is_err());
    assert_eq!(fs::read(&destination).unwrap(), b"keep this file");
    assert_ne!(db.database_path(), destination);
    let _ = fs::remove_file(db.database_path());
    let _ = fs::remove_dir_all(destination_directory);
}

#[test]
fn factory_reset_removes_user_state_and_restores_first_launch_defaults() {
    let db = setup_test_db();
    let clip = save_plain_test_clip(
        &db,
        "text",
        "Reset me completely",
        "factory-reset-clip",
        "Test",
    );
    db.update_clip_note(clip.id, Some("A note to remove"))
        .unwrap();
    db.create_bin_with_type("Personal", "Folder", "default", None, "category")
        .unwrap();
    db.create_content_type(&crate::content_types::ContentTypeInput {
        id: "reset_custom".into(),
        label: "Reset Custom".into(),
        icon: "FileText".into(),
        group: "custom".into(),
        conceal_clips: false,
    })
    .unwrap();
    db.save_setting("themeMode", "vampire").unwrap();
    {
        let conn = db.conn.lock();
        conn.execute(
            "INSERT INTO activity_logs (event_type, description) VALUES ('test', 'remove me')",
            [],
        )
        .unwrap();
        conn.execute(
                "INSERT INTO intelligence_connections (id, name, provider_kind) VALUES ('reset-connection', 'Reset', 'cli')",
                [],
            )
            .unwrap();
        conn.execute(
                "INSERT INTO custom_operations (id, name, executor_kind) VALUES ('reset-operation', 'Reset', 'regex')",
                [],
            )
            .unwrap();
        conn.execute(
            "INSERT INTO saved_transforms
                    (id, name, plan_json, connection_id, authoring_kind)
                 VALUES
                    ('reset-transform', 'Reset', '{\"steps\":[]}', 'reset-connection', 'intent'),
                    ('reset-manual-transform', 'Reset Manual', '{\"steps\":[]}', NULL, 'manual')",
            [],
        )
        .unwrap();
    }

    let report = db.factory_reset().unwrap();
    assert_eq!(report.clips_deleted, 1);
    assert_eq!(report.bins_deleted, 3);
    assert_eq!(report.transforms_deleted, 3);
    assert_eq!(report.connections_deleted, 1);
    assert_eq!(report.activity_entries_deleted, 3);

    assert!(db.get_clips(None, false).unwrap().is_empty());
    assert!(search_test_clips(&db, "Reset me").is_empty());
    let default_bins = db.get_bins().unwrap();
    assert_eq!(default_bins.len(), 2);
    assert_eq!(
        default_bins
            .iter()
            .map(|bin| bin.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Projects", "From Browsers"]
    );
    assert!(default_bins.iter().all(|bin| bin.color == "#6b7280"));
    assert_eq!(default_bins[0].smart_rule, None);
    assert!(default_bins[1]
        .smart_rule
        .as_deref()
        .is_some_and(|rule| rule.contains("Safari")
            && rule.contains("Firefox")
            && rule.contains("Brave")));
    assert_eq!(
        default_bins.iter().map(|bin| bin.id).collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(db.get_setting("themeMode").unwrap(), None);
    let reset_types = db.get_content_types(true).unwrap();
    assert_eq!(
        reset_types.len(),
        crate::content_types::CONTENT_TYPE_PRESETS.len()
    );
    assert!(!reset_types.iter().any(|item| item.id == "reset_custom"));
    let conn = db.conn.lock();
    for table in [
        "clip_versions",
        "activity_logs",
        "custom_operations",
        "saved_transforms",
        "intelligence_connections",
    ] {
        let count: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "{table} should be empty after reset");
    }
    drop(conn);
    let reset_registry = db.get_library_items(None, true).unwrap();
    assert!(!reset_registry.iter().any(|item| {
        item.item.stable_ref == "custom:reset-operation"
            || item.item.stable_ref == "transform:reset-manual-transform"
    }));
    assert_eq!(
        reset_registry
            .iter()
            .filter(|item| item.item.kind == "operation" && item.item.is_builtin)
            .count(),
        crate::operation_registry::BUILTIN_OPERATIONS.len()
    );

    let fresh = save_plain_test_clip(&db, "text", "Fresh start", "factory-reset-fresh", "Safari");
    assert!(fresh.id > 0);
    assert_eq!(db.get_bins().unwrap()[1].clip_count, Some(1));
}

#[test]
fn factory_reset_rolls_back_everything_when_a_delete_fails() {
    let db = setup_test_db();
    let clip = save_plain_test_clip(
        &db,
        "text",
        "Do not partially reset me",
        "factory-reset-rollback-clip",
        "Test",
    );
    let bin = db
        .create_bin("Keep This Bin", "Folder", "default", None)
        .unwrap();
    db.assign_to_bin(clip.id, Some(bin.id)).unwrap();
    db.save_setting("themeMode", "flux").unwrap();
    {
        let conn = db.conn.lock();
        conn.execute(
            "INSERT INTO activity_logs (event_type, description)
                 VALUES ('test', 'survive a failed reset')",
            [],
        )
        .unwrap();
        conn.execute_batch(
            "CREATE TRIGGER reject_factory_reset_clip_delete
                 BEFORE DELETE ON clips
                 BEGIN
                    SELECT RAISE(ABORT, 'simulated reset failure');
                 END;",
        )
        .unwrap();
    }

    let error = db.factory_reset().unwrap_err();
    assert!(error.to_string().contains("simulated reset failure"));

    let preserved = db.get_clip_by_id(clip.id).unwrap();
    assert_eq!(preserved.bin_id, Some(bin.id));
    assert_eq!(
        db.get_setting("themeMode").unwrap().as_deref(),
        Some("flux")
    );
    assert!(db.get_bins().unwrap().iter().any(|item| item.id == bin.id));
    let conn = db.conn.lock();
    let activity_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM activity_logs", [], |row| row.get(0))
        .unwrap();
    assert!(activity_count > 0);
}
