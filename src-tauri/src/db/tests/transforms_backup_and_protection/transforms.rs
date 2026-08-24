use super::super::*;

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
    let clip = save_plain_test_clip(&db, "text", "hello", "transform-clip", "Test");
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
    let clip = save_plain_test_clip(&db, "text", "hello", "manual-transform-clip", "Test");
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
    let clip = save_plain_test_clip(&db, "text", "hello", "provenance-migration-clip", "Test");
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
    let clip = save_plain_test_clip(&db, "text", "hello", "compound-undo", "Test");
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
