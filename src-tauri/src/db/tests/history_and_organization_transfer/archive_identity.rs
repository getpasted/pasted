use super::super::*;

#[test]
fn library_archive_preflight_reports_contents_and_rejects_late_corruption() {
    let source = setup_test_db();
    source.configure_clip_retention(0, 0).unwrap();
    for index in 0..2_000 {
        source
            .save_clip(
                "text",
                Some(&format!("Archive item {index}")),
                None,
                None,
                &format!("archive-preflight-{index}"),
                "Tests",
            )
            .unwrap();
    }
    let json = source.export_backup_json().unwrap();
    let inspection = DbState::inspect_library_archive_json(&json).unwrap();
    assert_eq!(inspection.schema_version, BACKUP_SCHEMA_VERSION);
    assert_eq!(inspection.clip_count, 2_000);
    assert!(inspection.content_type_count > 0);
    assert!(inspection.classifier_count > 0);

    let mut corrupted: serde_json::Value = serde_json::from_str(&json).unwrap();
    let clips = corrupted["clips"].as_array_mut().unwrap();
    let duplicate_hash = clips[0]["content_hash"].clone();
    clips.last_mut().unwrap()["content_hash"] = duplicate_hash;
    let corrupted = serde_json::to_string(&corrupted).unwrap();
    let error = DbState::inspect_library_archive_json(&corrupted)
        .unwrap_err()
        .to_string();
    assert!(error.contains("duplicate clip content hash"));

    let destination = setup_test_db();
    destination
        .save_setting("preflightMarker", "unchanged")
        .unwrap();
    let changes_before = destination.conn.lock().total_changes();
    assert!(destination.import_backup_json(&corrupted).is_err());
    assert_eq!(destination.conn.lock().total_changes(), changes_before);
    assert_eq!(
        destination
            .get_setting("preflightMarker")
            .unwrap()
            .as_deref(),
        Some("unchanged")
    );
}

#[test]
fn library_archive_reimport_updates_stable_identities_without_duplicates() {
    let source = setup_test_db();
    let clip = save_plain_test_clip(
        &source,
        "text",
        "Idempotent archive clip",
        "idempotent-archive-clip",
        "Tests",
    );
    let bin = source
        .create_bin("Archive Bin", "Folder", "default", None)
        .unwrap();
    source.assign_to_bin(clip.id, Some(bin.id)).unwrap();
    source
        .create_operation(
            "Archive Operation",
            "uppercase",
            Some("{}"),
            Some("Archive Tests"),
        )
        .unwrap();
    let plan = crate::transformation_intent::TransformationPlan {
        schema_version: crate::transformation_intent::TRANSFORMATION_PLAN_SCHEMA_VERSION,
        intent: "Trim text".to_string(),
        summary: "Trim text".to_string(),
        planning_mode: crate::transformation_intent::IntentPlanningMode::Pinned,
        steps: vec![crate::transformation_intent::PlannedTransformationStep {
            name: "Trim".to_string(),
            rationale: "Remove surrounding whitespace".to_string(),
            scope: crate::transformation_intent::StepExecutionScope::WholeInput,
            failure_policy: Default::default(),
            executor: crate::transformation_intent::PlannedExecutor::Deterministic {
                operation_ref: "builtin:trim".to_string(),
                config_json: None,
            },
        }],
    };
    source
        .create_saved_transform("Archive Transform", &plan, None)
        .unwrap();
    let archive = source.export_backup_json().unwrap();

    let destination = setup_test_db();
    assert_eq!(destination.import_backup_json(&archive).unwrap(), 1);
    let counts_after_first = {
        let conn = destination.conn.lock();
        (
            conn.query_row("SELECT COUNT(*) FROM clips", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            conn.query_row(
                "SELECT COUNT(*) FROM bins WHERE name = 'Archive Bin'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            conn.query_row(
                "SELECT COUNT(*) FROM custom_operations WHERE name = 'Archive Operation'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            conn.query_row(
                "SELECT COUNT(*) FROM saved_transforms WHERE name = 'Archive Transform'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        )
    };
    assert_eq!(counts_after_first, (1, 1, 1, 1));

    assert_eq!(destination.import_backup_json(&archive).unwrap(), 1);
    let counts_after_second = {
        let conn = destination.conn.lock();
        (
            conn.query_row("SELECT COUNT(*) FROM clips", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            conn.query_row(
                "SELECT COUNT(*) FROM bins WHERE name = 'Archive Bin'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            conn.query_row(
                "SELECT COUNT(*) FROM custom_operations WHERE name = 'Archive Operation'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            conn.query_row(
                "SELECT COUNT(*) FROM saved_transforms WHERE name = 'Archive Transform'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        )
    };
    assert_eq!(counts_after_second, counts_after_first);
}
