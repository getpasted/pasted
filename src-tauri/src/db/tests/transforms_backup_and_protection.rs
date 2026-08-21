use super::*;
#[test]
fn test_saved_transform_roundtrip_and_delete() {
    let db = setup_test_db();
    let connection = db
        .create_intelligence_connection(
            "Codex CLI",
            "cli",
            Some("/usr/local/bin/codex"),
            None,
            None,
        )
        .unwrap();
    let plan = crate::transformation_intent::TransformationPlan {
        schema_version: crate::transformation_intent::TRANSFORMATION_PLAN_SCHEMA_VERSION,
        intent: "Convert this text to Markdown".to_string(),
        summary: "Convert text to Markdown".to_string(),
        planning_mode: crate::transformation_intent::IntentPlanningMode::Pinned,
        steps: vec![crate::transformation_intent::PlannedTransformationStep {
            name: "Convert to Markdown".to_string(),
            rationale: "Structure requires interpretation".to_string(),
            scope: crate::transformation_intent::StepExecutionScope::WholeInput,
            failure_policy: Default::default(),
            executor: crate::transformation_intent::PlannedExecutor::Semantic {
                instructions: "Return clean Markdown".to_string(),
                output_schema: None,
                model_policy: crate::transformation_intent::ModelPolicy::Balanced,
            },
        }],
    };
    let transform = db
        .create_saved_transform("Markdown", &plan, Some(connection.id.as_str()))
        .unwrap();
    assert!(transform.stable_ref.starts_with("transform:"));
    assert_eq!(
        transform.connection_id.as_deref(),
        Some(connection.id.as_str())
    );
    assert_eq!(transform.plan, plan);
    assert_eq!(db.get_saved_transforms().unwrap().len(), 1);
    assert_eq!(
        db.resolve_saved_transform(&transform.stable_ref)
            .unwrap()
            .unwrap()
            .name,
        "Markdown"
    );
    let mut updated_plan = plan.clone();
    updated_plan.summary = "Convert text to concise Markdown".to_string();
    let updated = db
        .update_saved_transform(
            &transform.stable_ref,
            "Concise Markdown",
            &updated_plan,
            Some(connection.id.as_str()),
        )
        .unwrap();
    assert_eq!(updated.stable_ref, transform.stable_ref);
    assert_eq!(updated.name, "Concise Markdown");
    assert_eq!(updated.revision, transform.revision + 1);
    assert_eq!(updated.plan, updated_plan);
    db.delete_saved_transform(&transform.stable_ref).unwrap();
    assert!(db.get_saved_transforms().unwrap().is_empty());
}

#[test]
fn test_transform_preview_applies_atomically_with_revision_and_provenance() {
    let db = setup_test_db();
    let clip = db
        .save_clip("text", Some("hello"), None, None, "transform-clip", "Test")
        .unwrap();
    let plan = crate::transformation_intent::TransformationPlan {
        schema_version: crate::transformation_intent::TRANSFORMATION_PLAN_SCHEMA_VERSION,
        intent: "Uppercase".to_string(),
        summary: "Uppercase text".to_string(),
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
    let transform = db.create_saved_transform("Uppercase", &plan, None).unwrap();
    let provenance = db
        .apply_transform_output_to_clip(TransformClipApplication {
            clip_id: clip.id,
            transform_ref: &transform.stable_ref,
            expected_input: "hello",
            output: "HELLO",
            connection_id: None,
            duration_ms: 12,
            bin_move: None,
        })
        .unwrap();
    assert_eq!(provenance.transform_name, "Uppercase");
    assert_eq!(provenance.duration_ms, 12);
    assert_eq!(
        db.get_clip_versions(clip.id).unwrap()[0].text_content,
        "hello"
    );
    assert_eq!(
        db.get_clip_transformation_provenance(clip.id)
            .unwrap()
            .unwrap()
            .transform_ref,
        transform.stable_ref
    );
    let current = db
        .get_clips(None, false)
        .unwrap()
        .into_iter()
        .find(|item| item.id == clip.id)
        .unwrap();
    assert_eq!(current.text_content.as_deref(), Some("HELLO"));

    let stale = db.apply_transform_output_to_clip(TransformClipApplication {
        clip_id: clip.id,
        transform_ref: &transform.stable_ref,
        expected_input: "hello",
        output: "ANOTHER RESULT",
        connection_id: None,
        duration_ms: 5,
        bin_move: None,
    });
    assert!(stale
        .unwrap_err()
        .to_string()
        .contains("changed after this preview"));
    assert_eq!(db.get_clip_versions(clip.id).unwrap().len(), 1);
}

#[test]
fn manually_built_transform_applies_with_revision_and_stable_provenance() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "text",
            Some("hello"),
            None,
            None,
            "manual-transform-clip",
            "Test",
        )
        .unwrap();
    let pipeline = db
        .create_pipeline(
            "Uppercase Locally",
            &[PipelineStepInput {
                operation_ref: "builtin:uppercase".to_string(),
                config_json: None,
                failure_policy: "stop".to_string(),
            }],
            None,
        )
        .unwrap();
    assert!(db.get_intent_transforms().unwrap().is_empty());

    let definitions = db.get_transform_definitions().unwrap();
    assert_eq!(
        definitions
            .iter()
            .filter(|item| item.stable_ref == pipeline.stable_ref)
            .count(),
        1,
        "canonical definitions must not duplicate manual Transforms"
    );
    let definition = definitions
        .iter()
        .find(|item| item.stable_ref == pipeline.stable_ref)
        .unwrap();
    assert_eq!(definition.authoring_kind, TransformAuthoringKind::Manual);
    assert_eq!(definition.execution_character, "replayable");

    let provenance = db
        .apply_transform_output_to_clip(TransformClipApplication {
            clip_id: clip.id,
            transform_ref: &pipeline.stable_ref,
            expected_input: "hello",
            output: "HELLO",
            connection_id: None,
            duration_ms: 4,
            bin_move: None,
        })
        .unwrap();
    assert_eq!(provenance.transform_ref, pipeline.stable_ref);
    assert_eq!(
        db.get_clip_versions(clip.id).unwrap()[0].text_content,
        "hello"
    );
    assert_eq!(
        db.get_clip_transformation_provenance(clip.id)
            .unwrap()
            .unwrap()
            .transform_ref,
        pipeline.stable_ref
    );
    let stored: (Option<String>, Option<String>) = db
        .conn
        .lock()
        .query_row(
            "SELECT transform_id, transform_ref FROM clip_transformations WHERE clip_id = ?1",
            params![clip.id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        stored,
        (
            Some(
                pipeline
                    .stable_ref
                    .trim_start_matches("transform:")
                    .to_string()
            ),
            Some(pipeline.stable_ref.clone())
        )
    );

    db.delete_pipeline(&pipeline.stable_ref).unwrap();
    assert_eq!(
        db.get_clip_transformation_provenance(clip.id)
            .unwrap()
            .unwrap()
            .transform_ref,
        pipeline.stable_ref
    );
}

#[test]
fn transformation_provenance_migration_backfills_stable_refs() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "text",
            Some("hello"),
            None,
            None,
            "provenance-migration-clip",
            "Test",
        )
        .unwrap();
    let plan = crate::transformation_intent::TransformationPlan {
        schema_version: crate::transformation_intent::TRANSFORMATION_PLAN_SCHEMA_VERSION,
        intent: "Uppercase".to_string(),
        summary: "Uppercase text".to_string(),
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
    let transform = db.create_saved_transform("Uppercase", &plan, None).unwrap();
    db.apply_transform_output_to_clip(TransformClipApplication {
        clip_id: clip.id,
        transform_ref: &transform.stable_ref,
        expected_input: "hello",
        output: "HELLO",
        connection_id: None,
        duration_ms: 1,
        bin_move: None,
    })
    .unwrap();
    let path = db.path.lock().clone();
    {
        let conn = db.conn.lock();
        conn.execute("DROP INDEX idx_clip_transformations_ref", [])
            .unwrap();
        conn.execute(
            "ALTER TABLE clip_transformations DROP COLUMN transform_ref",
            [],
        )
        .unwrap();
    }
    drop(db);

    let migrated = DbState::new(path).unwrap();
    assert_eq!(
        migrated
            .get_clip_transformation_provenance(clip.id)
            .unwrap()
            .unwrap()
            .transform_ref,
        transform.stable_ref
    );
}

#[test]
fn transform_bin_drop_revision_restores_content_and_previous_bin_only() {
    let db = setup_test_db();
    let source_bin = db.create_bin("Source", "📥", "#111111", None).unwrap();
    let destination_bin = db.create_bin("Markdown", "📝", "#222222", None).unwrap();
    let tag = db
        .create_bin_with_type("Important", "⭐", "#333333", None, "tag")
        .unwrap();
    let clip = db
        .save_clip("text", Some("hello"), None, None, "compound-undo", "Test")
        .unwrap();
    db.add_clip_to_bin(clip.id, tag.id).unwrap();
    db.assign_to_bin(clip.id, Some(source_bin.id)).unwrap();
    let plan = crate::transformation_intent::TransformationPlan {
        schema_version: crate::transformation_intent::TRANSFORMATION_PLAN_SCHEMA_VERSION,
        intent: "Uppercase".to_string(),
        summary: "Uppercase text".to_string(),
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
    let transform = db.create_saved_transform("Uppercase", &plan, None).unwrap();

    db.assign_to_bin(clip.id, Some(destination_bin.id)).unwrap();
    db.apply_transform_output_to_clip(TransformClipApplication {
        clip_id: clip.id,
        transform_ref: &transform.stable_ref,
        expected_input: "hello",
        output: "HELLO",
        connection_id: None,
        duration_ms: 3,
        bin_move: Some((Some(source_bin.id), destination_bin.id)),
    })
    .unwrap();
    let version = db.get_clip_versions(clip.id).unwrap().remove(0);
    assert_eq!(
        version.action_label.as_deref(),
        Some("Moved to Markdown · Applied Uppercase")
    );
    assert!(version.restores_organization);

    let restored = db.restore_clip_version(clip.id, version.id).unwrap();
    assert_eq!(restored.text_content.as_deref(), Some("hello"));
    assert_eq!(restored.bin_id, Some(source_bin.id));
    assert!(!restored.is_transformed);
    assert!(db
        .get_clip_transformation_provenance(clip.id)
        .unwrap()
        .is_none());
    assert!(restored.bin_ids.unwrap_or_default().contains(&tag.id));

    let inverse = db.get_clip_versions(clip.id).unwrap().remove(0);
    assert_eq!(inverse.text_content, "HELLO");
    assert!(inverse.restores_organization);
    let redone = db.restore_clip_version(clip.id, inverse.id).unwrap();
    assert_eq!(redone.text_content.as_deref(), Some("HELLO"));
    assert_eq!(redone.bin_id, Some(destination_bin.id));
    assert!(redone.is_transformed);
    assert_eq!(
        db.get_clip_transformation_provenance(clip.id)
            .unwrap()
            .unwrap()
            .transform_name,
        "Uppercase"
    );
}

#[test]
fn clip_collection_pages_and_summary_cover_active_and_trashed_clips() {
    let db = setup_test_db();
    let empty = db.get_clip_collection_summary().unwrap();
    assert_eq!(empty.active_count, 0);
    assert_eq!(empty.trash_count, 0);
    assert!(empty.clip_type_counts.is_empty());

    let clips = (0..6)
        .map(|index| {
            db.save_clip(
                if index % 2 == 0 { "text" } else { "link" },
                Some(&format!("clip {index}")),
                None,
                None,
                &format!("paged-clip-{index}"),
                if index < 4 { "Editor" } else { "Browser" },
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    db.toggle_pin(clips[0].id).unwrap();
    db.toggle_protected(clips[1].id).unwrap();
    db.toggle_concealed(clips[3].id).unwrap();
    db.update_clip_note(clips[2].id, Some("Remember this"))
        .unwrap();
    db.delete_clip(clips[5].id).unwrap();
    db.delete_clip(clips[4].id).unwrap();

    let first = db.get_clips_page(None, false, Some(2), Some(0)).unwrap();
    let second = db.get_clips_page(None, false, Some(2), Some(2)).unwrap();
    assert_eq!(first.len(), 2);
    assert_eq!(second.len(), 2);
    assert!(first
        .iter()
        .all(|left| second.iter().all(|right| left.id != right.id)));
    assert_eq!(
        db.get_trashed_clips_page(Some(1), Some(0)).unwrap().len(),
        1
    );
    assert_eq!(
        db.get_trashed_clips_page(Some(1), Some(1)).unwrap().len(),
        1
    );

    let summary = db.get_clip_collection_summary().unwrap();
    assert_eq!(summary.active_count, 4);
    assert_eq!(summary.trash_count, 2);
    assert_eq!(summary.pinned_count, 1);
    assert_eq!(summary.protected_count, 1);
    assert_eq!(summary.concealed_count, 1);
    assert_eq!(summary.noted_count, 1);
    assert_eq!(summary.clip_type_counts.len(), 1);
    assert_eq!(summary.clip_type_counts[0].clip_type, "text");
    assert_eq!(summary.clip_type_counts[0].count, 4);
    assert_eq!(summary.type_counts.len(), 1);
    assert_eq!(summary.type_counts[0].content_type, "link");
    assert_eq!(summary.type_counts[0].count, 2);
    assert_eq!(
        summary
            .source_counts
            .iter()
            .map(|item| item.count)
            .sum::<i64>(),
        4
    );
}

#[test]
fn full_backup_round_trip_covers_every_durable_table_and_interface_state() {
    let db = setup_test_db();
    let active_path = db.database_path();
    let backup_path = active_path.with_extension("pastedbackup");
    let clip = db
        .save_clip(
            "text",
            Some("complete backup marker"),
            None,
            None,
            "full-backup-marker",
            "Tests",
        )
        .unwrap();
    db.update_clip_text(clip.id, "updated backup marker")
        .unwrap();
    db.replace_analysis_classifications(
        clip.id,
        &clip.content_hash,
        &[crate::content_classification::ClassificationMatch {
            classifier_ref: "prose".into(),
            classifier_name: "Prose".into(),
            content_type: "prose".into(),
            priority: 180,
            start_offset: 0,
            end_offset: 10,
        }],
        "original_text",
    )
    .unwrap();
    let file_clip = db
        .save_clip(
            "file",
            Some(r#"["/tmp/backup-audio.wav"]"#),
            None,
            None,
            "full-backup-file-marker",
            "Tests",
        )
        .unwrap();
    let protected_bin = db
        .create_bin("Backup Protection", "🔐", "default", None)
        .unwrap();
    db.update_bin_protection(protected_bin.id, true).unwrap();
    db.assign_to_bin(clip.id, Some(protected_bin.id)).unwrap();
    db.update_clip_hotkey(clip.id, Some("Alt+Shift+9")).unwrap();
    let transcription_extractor = db
        .get_content_extractors()
        .unwrap()
        .into_iter()
        .find(|candidate| {
            candidate.stable_ref == crate::content_extraction::WHISPER_TRANSCRIPTION_REF
        })
        .unwrap();
    assert!(db
        .replace_clip_searchable_text(
            file_clip.id,
            &file_clip.content_hash,
            &transcription_extractor,
            Some("complete transcription backup marker"),
        )
        .unwrap());
    db.record_extraction_observations(
        file_clip.id,
        &file_clip.content_hash,
        &[crate::content_analysis::ExtractionObservation {
            extractor_ref: transcription_extractor.stable_ref.clone(),
            extractor_name: transcription_extractor.name.clone(),
            engine: transcription_extractor.engine.clone(),
            priority: transcription_extractor.priority,
            duplicate_of: None,
            outcome: crate::content_extraction::ExtractionOutcome::Produced {
                text: "complete transcription backup marker".into(),
            },
        }],
    )
    .unwrap();
    db.save_setting("fullBackupSetting", "preserved").unwrap();
    db.log_activity("app_started", "Complete backup test")
        .unwrap();
    db.create_intelligence_connection(
        "Backup Connection",
        "openai_compatible",
        Some("http://127.0.0.1:1234/v1"),
        Some("local-model"),
        Some("keychain:pasted:test"),
    )
    .unwrap();
    let extractor = db.get_content_extractors().unwrap().remove(0);
    db.update_content_extractor(
        extractor.id,
        &crate::content_extraction::ExtractorInput {
            name: "Backup Extractor Marker".into(),
            description: extractor.description,
            enabled: false,
            priority: 77,
        },
    )
    .unwrap();

    let client_state = r#"{"version":1,"localStorage":{"pasted_sidebar_width":"280"}}"#;
    let window_state = r#"{"main":{"width":1200,"height":800}}"#;
    let report = db
        .create_full_backup(&backup_path, Some(client_state), Some(window_state))
        .unwrap();
    assert!(report.size_bytes > 0);
    let inspection = db.inspect_full_backup(&backup_path).unwrap();
    assert_eq!(inspection.format_version, FULL_BACKUP_FORMAT_VERSION);
    assert_eq!(inspection.created_at, report.created_at);
    assert_eq!(inspection.size_bytes, report.size_bytes);

    let table_names = |connection: &Connection| -> Vec<String> {
        let mut statement = connection
            .prepare(
                "SELECT name FROM sqlite_master
                     WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
                     ORDER BY name",
            )
            .unwrap();
        statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap()
    };
    let source_tables = table_names(&db.conn.lock());
    let backup_connection = Connection::open(&backup_path).unwrap();
    let backup_tables = table_names(&backup_connection);
    for table in source_tables {
        assert!(
            backup_tables.contains(&table),
            "full backup omitted durable table {table}"
        );
    }
    assert!(backup_tables.contains(&"pasted_backup_manifest".to_string()));
    drop(backup_connection);

    db.save_setting("fullBackupSetting", "mutated").unwrap();
    db.save_clip(
        "text",
        Some("post-backup marker"),
        None,
        None,
        "post-backup-marker",
        "Tests",
    )
    .unwrap();
    let (restore_report, restored_client_state, restored_window_state) = db
        .restore_full_backup(&backup_path, Some("{}"), Some("{}"))
        .unwrap();

    assert_eq!(restored_client_state.as_deref(), Some(client_state));
    assert_eq!(restored_window_state.as_deref(), Some(window_state));
    assert_eq!(
        db.get_setting("fullBackupSetting").unwrap().as_deref(),
        Some("preserved")
    );
    assert_eq!(db.get_all_clips_for_backup().unwrap().len(), 2);
    let restored_clip = db.get_clip_by_id(clip.id).unwrap();
    assert_eq!(restored_clip.shortcut.as_deref(), Some("Alt+Shift+9"));
    assert!(restored_clip.is_protected);
    assert!(db.get_bin(protected_bin.id).unwrap().protect_clips);
    assert!(!db.get_clip_versions(clip.id).unwrap().is_empty());
    assert_eq!(
        db.get_analysis_classifications(clip.id).unwrap()[0].content_type,
        "prose"
    );
    assert!(db
        .get_activity_logs(None, None)
        .unwrap()
        .iter()
        .any(|entry| entry.event_type == "app_started"));
    assert_eq!(db.get_intelligence_connections().unwrap().len(), 1);
    assert_eq!(
        db.get_clip_searchable_text(file_clip.id)
            .unwrap()
            .unwrap()
            .searchable_text,
        "complete transcription backup marker"
    );
    assert_eq!(
        db.get_extraction_history(file_clip.id, 101, 0)
            .unwrap()
            .len(),
        1
    );
    let restored_extractor = db.get_content_extractor(&extractor.stable_ref).unwrap();
    assert_eq!(restored_extractor.name, "Backup Extractor Marker");
    assert!(!restored_extractor.enabled);
    assert_eq!(restored_extractor.priority, 77);
    assert!(Path::new(&restore_report.recovery_path).is_file());
    assert_eq!(
        db.consume_pending_full_restore_client_state()
            .unwrap()
            .as_deref(),
        Some(client_state)
    );
    assert!(db
        .consume_pending_full_restore_client_state()
        .unwrap()
        .is_none());

    let _ = fs::remove_file(backup_path);
    let _ = fs::remove_file(restore_report.recovery_path);
}

#[test]
fn full_restore_rejects_invalid_embedded_state_before_replacing_library() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!("pasted-invalid-backup-{unique}"));
    fs::create_dir_all(&directory).unwrap();
    let db = DbState::new(directory.join("library.db")).unwrap();
    let backup_path = db.database_path().with_extension("pastedbackup");
    db.save_setting("liveStateMarker", "untouched").unwrap();
    db.create_full_backup(&backup_path, Some("{}"), Some("{}"))
        .unwrap();
    let backup = Connection::open(&backup_path).unwrap();
    backup
        .execute(
            "UPDATE pasted_backup_manifest SET client_state_json = 'not-json'",
            [],
        )
        .unwrap();
    let _ = backup.pragma_update(None, "wal_checkpoint", "TRUNCATE");
    drop(backup);

    assert!(db.inspect_full_backup(&backup_path).is_err());
    assert!(db
        .restore_full_backup(&backup_path, Some("{}"), Some("{}"))
        .is_err());
    assert_eq!(
        db.get_setting("liveStateMarker").unwrap().as_deref(),
        Some("untouched")
    );
    let recovery_count = fs::read_dir(&directory)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("Pasted_Pre_Restore_")
        })
        .count();
    assert_eq!(recovery_count, 0);
    let _ = fs::remove_file(backup_path);
    drop(db);
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn clip_shortcuts_protect_assignments_and_keep_protection_when_cleared() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "text",
            Some("durable shortcut"),
            None,
            None,
            "clip-shortcut-protection",
            "Tests",
        )
        .unwrap();

    db.update_clip_hotkey(clip.id, Some("Alt+Shift+7")).unwrap();
    let assigned = db.get_clip_by_id(clip.id).unwrap();
    assert_eq!(assigned.shortcut.as_deref(), Some("Alt+Shift+7"));
    assert!(assigned.is_protected);
    assert_eq!(assigned.is_explicitly_protected, Some(true));
    assert_eq!(
        db.get_clip_hotkeys().unwrap(),
        vec![(clip.id, "Alt+Shift+7".to_string())]
    );
    assert!(db.batch_protect_clips(vec![clip.id], false).is_err());

    db.update_clip_hotkey(clip.id, None).unwrap();
    let cleared = db.get_clip_by_id(clip.id).unwrap();
    assert_eq!(cleared.shortcut, None);
    assert!(cleared.is_protected);
    assert_eq!(cleared.is_explicitly_protected, Some(true));
    assert!(db.get_clip_hotkeys().unwrap().is_empty());

    db.batch_protect_clips(vec![clip.id], false).unwrap();
    assert!(!db.get_clip_by_id(clip.id).unwrap().is_protected);
}

#[test]
fn protecting_bin_blocks_unprotect_after_clip_hotkey_is_removed() {
    let db = setup_test_db();
    let bin = db
        .create_bin("Protected Bin", "🛡️", "default", None)
        .unwrap();
    let clip = db
        .save_clip(
            "text",
            Some("hotkey and bin protection"),
            None,
            None,
            "hotkey-bin-protection-precedence",
            "Tests",
        )
        .unwrap();

    db.update_bin_protection(bin.id, true).unwrap();
    db.update_clip_hotkey(clip.id, Some("Alt+Shift+8")).unwrap();
    db.assign_to_bin(clip.id, Some(bin.id)).unwrap();
    db.update_clip_hotkey(clip.id, None).unwrap();

    let protected = db.get_clip_by_id(clip.id).unwrap();
    assert!(protected.is_protected);
    assert_eq!(protected.is_explicitly_protected, Some(true));
    assert_eq!(protected.protecting_bin_ids, vec![bin.id]);
    assert!(db.batch_protect_clips(vec![clip.id], false).is_err());
    assert!(db
        .get_clip_by_id(clip.id)
        .unwrap()
        .is_explicitly_protected
        .unwrap());

    db.assign_to_bin(clip.id, None).unwrap();
    db.batch_protect_clips(vec![clip.id], false).unwrap();
    assert!(!db.get_clip_by_id(clip.id).unwrap().is_protected);
}

#[test]
fn manual_bin_protection_is_inherited_without_mutating_clips() {
    let db = setup_test_db();
    let bin = db
        .create_bin("Protected Bin", "🛡️", "default", None)
        .unwrap();
    let clip = db
        .save_clip(
            "text",
            Some("inherited"),
            None,
            None,
            "bin-inherited-protection",
            "Tests",
        )
        .unwrap();
    db.update_bin_protection(bin.id, true).unwrap();
    db.assign_to_bin(clip.id, Some(bin.id)).unwrap();

    let protected = db.get_clip_by_id(clip.id).unwrap();
    assert!(protected.is_protected);
    assert_eq!(protected.is_explicitly_protected, Some(false));
    assert_eq!(protected.protecting_bin_ids, vec![bin.id]);
    let raw: i32 = db
        .conn
        .lock()
        .query_row(
            "SELECT is_protected FROM clips WHERE id = ?1",
            params![clip.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(raw, 0, "inherited protection must not mutate the clip flag");

    let trash = db.batch_trash_clips(vec![clip.id]).unwrap();
    assert_eq!(trash.changed_count, 0);
    db.purge_clip_permanently(clip.id).unwrap();
    assert!(db.get_clip_by_id(clip.id).is_ok());
    db.clear_history().unwrap();
    assert!(db.get_clip_by_id(clip.id).is_ok());

    db.assign_to_bin(clip.id, None).unwrap();
    assert!(!db.get_clip_by_id(clip.id).unwrap().is_protected);
    assert_eq!(
        db.batch_trash_clips(vec![clip.id]).unwrap().changed_count,
        1
    );
}

#[test]
fn smart_bins_cannot_apply_inherited_clip_policies() {
    let db = setup_test_db();
    let rule = serde_json::json!({
        "version": 1,
        "conditions": [{"type": "clip_type", "operator": "is", "value": "text"}],
        "match": "all"
    })
    .to_string();
    let bin = db
        .create_bin("Smart", "🧠", "default", Some(&rule))
        .unwrap();
    assert!(db.update_bin_protection(bin.id, true).is_err());
    assert!(!db.get_bin(bin.id).unwrap().protect_clips);
    assert!(db.update_bin_concealment(bin.id, true).is_err());
    assert!(!db.get_bin(bin.id).unwrap().conceal_clips);
}

#[test]
fn legacy_databases_migrate_clip_shortcuts_and_bin_protection() {
    let path = std::env::temp_dir().join(format!(
        "pasted-shortcut-protection-migration-{}.db",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE clips (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    content_type TEXT NOT NULL,
                    text_content TEXT,
                    html_content TEXT,
                    image_base64 TEXT,
                    content_hash TEXT UNIQUE NOT NULL,
                    source TEXT DEFAULT 'Unknown',
                    is_pinned INTEGER DEFAULT 0,
                    bin_id INTEGER,
                    note TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                 );
                 CREATE TABLE bins (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    icon TEXT DEFAULT 'Folder',
                    color TEXT DEFAULT 'default',
                    smart_rule TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                 );",
        )
        .unwrap();
    drop(connection);

    let db = DbState::new(path.clone()).unwrap();
    assert!(column_exists(&db.conn.lock(), "clips", "shortcut").unwrap());
    assert!(column_exists(&db.conn.lock(), "clips", "is_concealed").unwrap());
    assert!(column_exists(&db.conn.lock(), "clips", "is_revealed").unwrap());
    assert!(column_exists(&db.conn.lock(), "bins", "protect_clips").unwrap());
    assert!(column_exists(&db.conn.lock(), "bins", "conceal_clips").unwrap());
    assert!(column_exists(&db.conn.lock(), "content_types", "conceal_clips").unwrap());
    let view_exists: bool = db
        .conn
        .lock()
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'view' AND name = 'effective_clip_protection')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(view_exists);
    drop(db);
    let _ = fs::remove_file(path);
}
