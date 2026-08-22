use super::*;
#[test]
fn test_backup_export_import() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "text",
            Some("Backup Test Item"),
            Some("<strong>Backup Test Item</strong>"),
            None,
            "HashBK1",
            "VSCode",
        )
        .unwrap();
    let trashed = db
        .save_clip("text", Some("In Trash"), None, None, "HashBK2", "Notes")
        .unwrap();
    db.replace_analysis_classifications(
        clip.id,
        &clip.content_hash,
        &[
            crate::content_classification::ClassificationMatch {
                classifier_ref: "email".into(),
                classifier_name: "Email Addresses".into(),
                content_type: "email".into(),
                priority: 20,
                start_offset: 0,
                end_offset: 6,
            },
            crate::content_classification::ClassificationMatch {
                classifier_ref: "url".into(),
                classifier_name: "Web Links".into(),
                content_type: "link".into(),
                priority: 30,
                start_offset: 7,
                end_offset: 11,
            },
        ],
        "original_text",
    )
    .unwrap();
    let bin = db.create_bin("DevBin", "Code", "#3b82f6", None).unwrap();
    let tag = db
        .create_bin_with_type("BackupTag", "Tag", "#f59e0b", None, "tag")
        .unwrap();
    db.assign_to_bin(clip.id, Some(bin.id)).unwrap();
    db.add_clip_to_bin(clip.id, tag.id).unwrap();
    db.update_clip_note(clip.id, Some("Restore this note"))
        .unwrap();
    db.toggle_pin(clip.id).unwrap();
    db.toggle_protected(clip.id).unwrap();
    db.delete_clip(trashed.id).unwrap();
    let backup_pipeline = db
        .create_pipeline(
            "Backup Pipeline",
            &[
                PipelineStepInput {
                    operation_ref: "builtin:trim".to_string(),
                    config_json: None,
                    failure_policy: "stop".to_string(),
                },
                PipelineStepInput {
                    operation_ref: "builtin:uppercase".to_string(),
                    config_json: None,
                    failure_policy: "stop".to_string(),
                },
            ],
            Some("Alt+B"),
        )
        .unwrap();
    db.create_operation(
        "Backup Operation",
        "uppercase",
        Some("{}"),
        Some("Backup Tools"),
    )
    .unwrap();
    let transform_plan = crate::transformation_intent::TransformationPlan {
        schema_version: crate::transformation_intent::TRANSFORMATION_PLAN_SCHEMA_VERSION,
        intent: "Uppercase".to_string(),
        summary: "Uppercase".to_string(),
        planning_mode: crate::transformation_intent::IntentPlanningMode::Pinned,
        steps: vec![crate::transformation_intent::PlannedTransformationStep {
            name: "Uppercase".to_string(),
            rationale: "Replayable".to_string(),
            scope: crate::transformation_intent::StepExecutionScope::WholeInput,
            failure_policy: Default::default(),
            executor: crate::transformation_intent::PlannedExecutor::Deterministic {
                operation_ref: "builtin:uppercase".to_string(),
                config_json: None,
            },
        }],
    };
    let saved_transform = db
        .create_saved_transform("Backup Transform", &transform_plan, None)
        .unwrap();
    db.set_bin_transform_ref(bin.id, Some(&saved_transform.stable_ref))
        .unwrap();

    let json = db.export_backup_json().unwrap();
    assert!(json.contains("Backup Test Item"));
    assert!(json.contains("DevBin"));
    assert!(!json.contains("\"pipelines\""));
    assert!(json.contains("\"authoringKind\": \"manual\""));

    let db2 = setup_test_db();
    let imported_count = db2.import_backup_json(&json).unwrap();
    assert_eq!(imported_count, 2);

    let restored = db2.get_all_clips_for_backup().unwrap();
    let restored_clip = restored
        .iter()
        .find(|item| item.content_hash == "HashBK1")
        .unwrap();
    assert_eq!(
        restored_clip.text_content.as_deref(),
        Some("Backup Test Item")
    );
    assert_eq!(
        restored_clip.html_content.as_deref(),
        Some("<strong>Backup Test Item</strong>")
    );
    assert_eq!(restored_clip.note.as_deref(), Some("Restore this note"));
    assert!(restored_clip.is_pinned);
    assert!(restored_clip.is_protected);
    assert!(!restored_clip.is_trashed);
    assert_eq!(restored_clip.content_type, "text");
    assert_eq!(restored_clip.content_types, vec!["link", "email"]);

    let restored_trashed = restored
        .iter()
        .find(|item| item.content_hash == "HashBK2")
        .unwrap();
    assert!(restored_trashed.is_trashed);
    assert!(restored_trashed.trashed_at.is_some());

    let restored_bins = db2.get_bins().unwrap();
    let restored_bin = restored_bins
        .iter()
        .find(|item| item.name == "DevBin")
        .unwrap();
    let restored_tag = restored_bins
        .iter()
        .find(|item| item.name == "BackupTag")
        .unwrap();
    let restored_bin_ids = restored_clip.bin_ids.as_ref().unwrap();
    assert!(restored_bin_ids.contains(&restored_bin.id));
    assert!(restored_bin_ids.contains(&restored_tag.id));
    let restored_pipeline = db2
        .get_pipelines()
        .unwrap()
        .into_iter()
        .find(|item| item.name == "Backup Pipeline")
        .unwrap();
    assert_eq!(restored_pipeline.stable_ref, backup_pipeline.stable_ref);
    assert_eq!(restored_pipeline.shortcut.as_deref(), Some("Alt+B"));
    assert_eq!(restored_pipeline.steps.len(), 2);
    assert_eq!(
        restored_pipeline.steps[1].operation_ref,
        "builtin:uppercase"
    );
    assert_eq!(
        db2.get_saved_transforms()
            .unwrap()
            .into_iter()
            .find(|item| item.name == "Backup Transform")
            .unwrap()
            .stable_ref,
        saved_transform.stable_ref
    );
    assert_eq!(
        db2.get_bin_transform_ref(restored_bin.id)
            .unwrap()
            .as_deref(),
        Some(saved_transform.stable_ref.as_str())
    );
    assert!(db2
        .get_operations()
        .unwrap()
        .iter()
        .any(|item| item.name == "Backup Operation" && item.category == "Backup Tools"));
    let restored_registry = db2.get_library_items(None, false).unwrap();
    assert!(restored_registry
        .iter()
        .any(|item| item.item.stable_ref == backup_pipeline.stable_ref));
    assert!(restored_registry.iter().any(|item| {
        item.item.kind == "operation"
            && item.item.name == "Backup Operation"
            && item.item.group_label.as_deref() == Some("Backup Tools")
    }));
}

#[test]
fn legacy_pipeline_backups_import_as_manual_transforms() {
    let source = setup_test_db();
    let mut payload =
        serde_json::from_str::<serde_json::Value>(&source.export_backup_json().unwrap()).unwrap();
    payload["pipelines"] = serde_json::json!([{
        "id": 1,
        "stableRef": "pipeline:legacy-backup",
        "name": "Legacy Backup",
        "shortcut": "Alt+L",
        "revision": 3,
        "createdAt": "2026-01-01 00:00:00",
        "updatedAt": "2026-01-02 00:00:00",
        "steps": [{
            "position": 0,
            "operationRef": "builtin:uppercase",
            "configJson": null,
            "failurePolicy": "skip"
        }]
    }]);

    let destination = setup_test_db();
    destination
        .import_backup_json(&serde_json::to_string(&payload).unwrap())
        .unwrap();
    let imported = destination
        .get_pipelines()
        .unwrap()
        .into_iter()
        .find(|transform| transform.name == "Legacy Backup")
        .unwrap();
    assert_eq!(imported.stable_ref, "transform:legacy-backup");
    assert_eq!(imported.shortcut.as_deref(), Some("Alt+L"));
    assert_eq!(imported.revision, 3);
    assert_eq!(imported.steps[0].failure_policy, "skip");
    assert!(!destination
        .export_backup_json()
        .unwrap()
        .contains("\"pipelines\""));
}

#[test]
fn backup_roundtrip_preserves_bin_clip_order() {
    let source = setup_test_db();
    let first = source
        .save_clip("text", Some("First"), None, None, "backup-order-1", "App")
        .unwrap();
    let second = source
        .save_clip("text", Some("Second"), None, None, "backup-order-2", "App")
        .unwrap();
    let bin = source
        .create_bin("Ordered", "Folder", "default", None)
        .unwrap();
    source.assign_to_bin(first.id, Some(bin.id)).unwrap();
    source.assign_to_bin(second.id, Some(bin.id)).unwrap();
    source
        .reorder_bin_clips(bin.id, vec![first.id, second.id])
        .unwrap();

    let destination = setup_test_db();
    destination
        .import_backup_json(&source.export_backup_json().unwrap())
        .unwrap();
    let restored_bin = destination
        .get_bins()
        .unwrap()
        .into_iter()
        .find(|candidate| candidate.name == "Ordered")
        .unwrap();
    let restored = destination.get_clips(Some(restored_bin.id), false).unwrap();
    assert_eq!(
        restored
            .iter()
            .map(|clip| clip.text_content.as_deref().unwrap_or_default())
            .collect::<Vec<_>>(),
        vec!["First", "Second"]
    );
}

#[test]
fn test_backup_export_is_not_limited_to_visible_history() {
    let db = setup_test_db();
    for index in 0..501 {
        db.save_clip(
            "text",
            Some(&format!("Backup item {index}")),
            None,
            None,
            &format!("backup-limit-{index}"),
            "App",
        )
        .unwrap();
    }

    let json = db.export_backup_json().unwrap();
    let payload: BackupPayload = serde_json::from_str(&json).unwrap();
    assert_eq!(payload.version, BACKUP_SCHEMA_VERSION);
    assert_eq!(payload.clips.len(), 501);
    assert_eq!(db.get_clips(None, false).unwrap().len(), 501);
}

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
    let clip = source
        .save_clip(
            "text",
            Some("Idempotent archive clip"),
            None,
            None,
            "idempotent-archive-clip",
            "Tests",
        )
        .unwrap();
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

#[test]
fn clip_exports_match_their_documented_json_and_csv_contracts() {
    let db = setup_test_db();
    let active = db
        .save_clip(
            "text",
            Some("=SUM(A1:A2), \"quoted\""),
            Some("<b>preserved in JSON</b>"),
            None,
            "clip-export-active",
            "Editor, Inc.",
        )
        .unwrap();
    db.toggle_pin(active.id).unwrap();
    db.update_clip_name(active.id, Some("Formula 📊")).unwrap();
    let trashed = db
        .save_clip(
            "text",
            Some("must not be exported"),
            None,
            None,
            "clip-export-trashed",
            "Tests",
        )
        .unwrap();
    db.delete_clip(trashed.id).unwrap();

    let json = db.export_clips_json().unwrap();
    let clips: Vec<ClipItem> = serde_json::from_str(&json).unwrap();
    assert_eq!(clips.len(), 1);
    assert_eq!(clips[0].content_hash, "clip-export-active");
    assert_eq!(
        clips[0].html_content.as_deref(),
        Some("<b>preserved in JSON</b>")
    );
    assert!(clips[0].is_pinned);
    assert_eq!(clips[0].name.as_deref(), Some("Formula 📊"));
    assert!(!json.contains("must not be exported"));

    let csv = db.export_clips_csv().unwrap();
    let mut lines = csv.lines();
    assert_eq!(
        lines.next(),
        Some("id,content_type,source,is_pinned,created_at,name,text_content")
    );
    let row = lines.next().unwrap();
    assert!(row.contains("\"Editor, Inc.\""));
    assert!(row.contains("\"'=SUM(A1:A2), \"\"quoted\"\"\""));
    assert!(row.contains(",true,"));
    assert!(row.contains("\"Formula 📊\""));
    assert!(lines.next().is_none());

    let json_target = setup_test_db();
    let json_preview = json_target.inspect_clips_json(&json).unwrap();
    assert_eq!(json_preview.imported_count, 1);
    assert!(json_target.get_all_clips_for_backup().unwrap().is_empty());
    let first_json_import = json_target.import_clips_json(&json).unwrap();
    assert_eq!(first_json_import.scanned_count, 1);
    assert_eq!(first_json_import.imported_count, 1);
    assert_eq!(first_json_import.duplicate_count, 0);
    let second_json_import = json_target.import_clips_json(&json).unwrap();
    assert_eq!(second_json_import.imported_count, 0);
    assert_eq!(second_json_import.duplicate_count, 1);
    let imported_json_clip = json_target.get_all_clips_for_backup().unwrap().remove(0);
    assert_eq!(
        imported_json_clip.html_content.as_deref(),
        Some("<b>preserved in JSON</b>")
    );
    assert!(imported_json_clip.is_pinned);
    assert_eq!(imported_json_clip.name.as_deref(), Some("Formula 📊"));

    let csv_target = setup_test_db();
    let csv_preview = csv_target.inspect_clips_csv(&csv).unwrap();
    assert_eq!(csv_preview.imported_count, 1);
    assert!(csv_target.get_clips(None, false).unwrap().is_empty());
    let first_csv_import = csv_target.import_clips_csv(&csv).unwrap();
    assert_eq!(first_csv_import.imported_count, 1);
    assert_eq!(first_csv_import.duplicate_count, 0);
    let second_csv_import = csv_target.import_clips_csv(&csv).unwrap();
    assert_eq!(second_csv_import.imported_count, 0);
    assert_eq!(second_csv_import.duplicate_count, 1);
    let imported_csv_clip = csv_target.get_clips(None, false).unwrap().remove(0);
    assert_eq!(
        imported_csv_clip.text_content.as_deref(),
        Some("=SUM(A1:A2), \"quoted\"")
    );
    assert_eq!(imported_csv_clip.source, "Editor, Inc.");
    assert_eq!(imported_csv_clip.name.as_deref(), Some("Formula 📊"));

    let invalid_target = setup_test_db();
    let invalid_csv = format!("{csv}\n\"broken\",\"row\"");
    assert!(invalid_target.import_clips_csv(&invalid_csv).is_err());
    assert!(invalid_target.get_clips(None, false).unwrap().is_empty());
}

#[test]
fn clip_json_import_round_trips_stored_images() {
    let source = setup_test_db();
    source
        .save_clip(
            "image",
            Some("recognized text"),
            None,
            Some(crate::resource_limits::TEST_PNG_DATA_URL),
            "clip-image-export-hash",
            "Screenshot",
        )
        .unwrap();
    let json = source.export_clips_json().unwrap();

    let target = setup_test_db();
    let report = target.import_clips_json(&json).unwrap();
    assert_eq!(report.imported_count, 1);
    let imported = target.get_all_clips_for_backup().unwrap().remove(0);
    assert_eq!(imported.content_type, "image");
    assert_eq!(imported.text_content.as_deref(), Some("recognized text"));
    assert_eq!(
        imported.image_base64.as_deref(),
        Some(crate::resource_limits::TEST_PNG_DATA_URL)
    );
    assert_eq!(imported.content_hash, "clip-image-export-hash");
}

#[test]
fn raster_image_boundaries_reject_active_content_without_mutation() {
    let malicious = "data:image/png;base64,PHN2ZyBvbmxvYWQ9ImFsZXJ0KDEpIj48L3N2Zz4=";
    let direct = setup_test_db();
    assert!(direct
        .save_clip(
            "image",
            None,
            None,
            Some(malicious),
            "malicious-direct-image",
            "Tests",
        )
        .is_err());
    assert!(direct.get_all_clips_for_backup().unwrap().is_empty());

    let source = setup_test_db();
    source
        .save_clip(
            "image",
            None,
            None,
            Some(crate::resource_limits::TEST_PNG_DATA_URL),
            "malicious-import-image",
            "Tests",
        )
        .unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(&source.export_clips_json().unwrap()).unwrap();
    payload[0]["image_base64"] = malicious.into();
    let payload = serde_json::to_string(&payload).unwrap();
    let target = setup_test_db();
    assert!(target.inspect_clips_json(&payload).is_err());
    assert!(target.import_clips_json(&payload).is_err());
    assert!(target.get_all_clips_for_backup().unwrap().is_empty());

    let legacy = source.get_all_clips_for_backup().unwrap().remove(0);
    source
        .conn
        .lock()
        .execute(
            "UPDATE clips SET image_base64 = ?1 WHERE id = ?2",
            params![malicious, legacy.id],
        )
        .unwrap();
    assert_eq!(source.get_clip_image(legacy.id).unwrap(), None);
    assert_eq!(
        source
            .get_all_clips_for_backup()
            .unwrap()
            .remove(0)
            .image_base64
            .as_deref(),
        Some(malicious)
    );
}

#[test]
fn insights_summary_is_strictly_read_only() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "text",
            Some("Read-only insight"),
            None,
            None,
            "insights-read-only",
            "",
        )
        .unwrap();
    let changes_before = db.conn.lock().total_changes();
    let before = db.get_clip_by_id(clip.id).unwrap();
    let summary = db.get_analytics_summary().unwrap();
    let after = db.get_clip_by_id(clip.id).unwrap();

    assert_eq!(summary.total_clips, 1);
    assert_eq!(db.conn.lock().total_changes(), changes_before);
    assert_eq!(after.source, before.source);
    assert_eq!(after.content_hash, before.content_hash);
}

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

    let mut archive: serde_json::Value =
        serde_json::from_str(&source.export_backup_json().unwrap()).unwrap();
    archive["bins"][0]["created_at"] = serde_json::json!("2026-08-16T18:45:00-05:00");
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
    migrate_analysis_classification_timestamps(&conn).unwrap();
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

#[test]
fn insights_groups_daily_activity_by_the_requested_local_calendar() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "text",
            Some("Boundary clip"),
            None,
            None,
            "boundary-clip",
            "Tests",
        )
        .unwrap();
    db.conn
        .lock()
        .execute(
            "UPDATE clips SET created_at = '2026-08-17T00:15:00Z' WHERE id = ?1",
            [clip.id],
        )
        .unwrap();

    let conn = db.conn.lock();
    let west =
        DbState::get_daily_activity_for_calendar(&conn, "2026-08-17T00:30:00Z", "-05:00").unwrap();
    assert_eq!(west[0].date, "2026-08-16");
    assert_eq!(west[0].count, 1);

    let east =
        DbState::get_daily_activity_for_calendar(&conn, "2026-08-17T00:30:00Z", "+05:30").unwrap();
    assert_eq!(east[0].date, "2026-08-17");
    assert_eq!(east[0].count, 1);
}

#[test]
fn backup_roundtrip_preserves_completed_ocr_lifecycle_state() {
    let source = setup_test_db();
    let clip = source
        .save_clip(
            "image",
            None,
            None,
            Some(crate::resource_limits::TEST_PNG_DATA_URL),
            "ocr-backup-hash",
            "Screenshot",
        )
        .unwrap();
    assert!(source
        .complete_ocr_attempt_with_extractor(
            clip.id,
            "ocr-backup-hash",
            Some("Recovered words"),
            OcrExtractorProvenance::identified(
                "vision-test-v1",
                "extractor:test-vision",
                "Test Vision OCR",
            ),
            None,
        )
        .unwrap());

    let backup = source.export_backup_json().unwrap();
    let destination = setup_test_db();
    assert_eq!(destination.import_backup_json(&backup).unwrap(), 1);

    let status = destination.get_ocr_backfill_status().unwrap();
    assert_eq!(status.total_images, 1);
    assert_eq!(status.completed_count, 1);
    assert_eq!(status.eligible_count, 0);

    let restored_payload: BackupPayload =
        serde_json::from_str(&destination.export_backup_json().unwrap()).unwrap();
    assert_eq!(restored_payload.ocr_metadata.len(), 1);
    assert_eq!(restored_payload.ocr_metadata[0].status, "complete");
    assert_eq!(
        restored_payload.ocr_metadata[0].engine_version.as_deref(),
        Some("vision-test-v1")
    );
    assert_eq!(
        restored_payload.ocr_metadata[0].extractor_ref.as_deref(),
        Some("extractor:test-vision")
    );
    assert_eq!(
        restored_payload.ocr_metadata[0].extractor_name.as_deref(),
        Some("Test Vision OCR")
    );
}

#[test]
fn backup_import_rejects_unknown_schema_without_mutating_data() {
    let source = setup_test_db();
    source
        .save_clip(
            "text",
            Some("future data"),
            None,
            None,
            "future-backup-item",
            "Test",
        )
        .unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(&source.export_backup_json().unwrap()).unwrap();
    payload["version"] = serde_json::json!(BACKUP_SCHEMA_VERSION + 1);

    let destination = setup_test_db();
    let error = destination
        .import_backup_json(&serde_json::to_string(&payload).unwrap())
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("unsupported transfer schema version"));
    assert!(destination.get_clips(None, false).unwrap().is_empty());
}

#[test]
fn backup_import_rolls_back_earlier_writes_when_valid_payload_fails_midway() {
    let source = setup_test_db();
    source
        .create_bin("Imported Bin", "Folder", "default", None)
        .unwrap();
    source
        .create_operation(
            "Imported Operation",
            "uppercase",
            Some("{}"),
            Some("Import Test"),
        )
        .unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(&source.export_backup_json().unwrap()).unwrap();
    let custom_operation = payload["operations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|operation| {
            operation["stable_id"]
                .as_str()
                .is_some_and(|stable_id| stable_id.starts_with("custom:"))
        })
        .unwrap();
    custom_operation["stable_id"] = serde_json::json!("invalid-operation-reference");

    let destination = setup_test_db();
    let existing = destination
        .save_clip(
            "text",
            Some("Destination must survive"),
            None,
            None,
            "backup-rollback-existing",
            "Test",
        )
        .unwrap();
    destination.save_setting("themeMode", "warm").unwrap();
    let bins_before = destination
        .get_bins()
        .unwrap()
        .into_iter()
        .map(|bin| (bin.id, bin.name))
        .collect::<Vec<_>>();

    let error = destination
        .import_backup_json(&serde_json::to_string(&payload).unwrap())
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("custom operation in transfer file is missing a stable reference"));
    assert_eq!(
        destination
            .get_clip_by_id(existing.id)
            .unwrap()
            .text_content
            .as_deref(),
        Some("Destination must survive")
    );
    assert_eq!(
        destination.get_setting("themeMode").unwrap().as_deref(),
        Some("warm")
    );
    assert_eq!(
        destination
            .get_bins()
            .unwrap()
            .into_iter()
            .map(|bin| (bin.id, bin.name))
            .collect::<Vec<_>>(),
        bins_before
    );
    assert!(!destination
        .get_operations()
        .unwrap()
        .iter()
        .any(|operation| operation.name == "Imported Operation"));
}
