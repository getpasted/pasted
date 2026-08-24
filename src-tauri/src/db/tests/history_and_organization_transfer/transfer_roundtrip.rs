use super::super::*;

#[test]
fn history_and_organization_export_import() {
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
    let trashed = save_plain_test_clip(&db, "text", "In Trash", "HashBK2", "Notes");
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
fn legacy_pipeline_transfers_import_as_manual_transforms() {
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
fn transfer_roundtrip_preserves_bin_clip_order() {
    let source = setup_test_db();
    let first = save_plain_test_clip(&source, "text", "First", "backup-order-1", "App");
    let second = save_plain_test_clip(&source, "text", "Second", "backup-order-2", "App");
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
fn transfer_export_is_not_limited_to_visible_history() {
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
