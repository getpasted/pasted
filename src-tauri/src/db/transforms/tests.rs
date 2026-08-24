use super::*;
use crate::db::{tests::setup_test_db, DbState};

mod fixtures;
use fixtures::deterministic_plan;

fn save_text_clip_id(db: &DbState, source_id: &str) -> i64 {
    db.save_clip("text", Some("hello"), None, None, source_id, "Tests")
        .unwrap()
        .id
}

#[test]
fn definition_facade_preserves_intent_and_manual_compatibility() {
    let db = setup_test_db();
    let intent = db
        .create_saved_transform("Uppercase", &deterministic_plan(), None)
        .unwrap();
    let intent_definition = db
        .resolve_transform_definition(&intent.stable_ref)
        .unwrap()
        .unwrap();
    assert_eq!(
        intent_definition.authoring_kind,
        TransformAuthoringKind::Intent
    );
    assert_eq!(intent_definition.plan, Some(deterministic_plan()));

    let intent_copy = db
        .duplicate_transform_definition(&intent.stable_ref, Some("Uppercase Copy"))
        .unwrap();
    assert_eq!(intent_copy.name, "Uppercase Copy");
    assert_eq!(intent_copy.authoring_kind, TransformAuthoringKind::Intent);

    let manual = db
        .create_pipeline(
            "Trim",
            &[PipelineStepInput {
                operation_ref: "builtin:trim".to_string(),
                config_json: None,
                failure_policy: "stop".to_string(),
            }],
            None,
        )
        .unwrap();
    let compatibility_ref = manual.stable_ref.replacen("transform:", "pipeline:", 1);
    let manual_definition = db
        .resolve_transform_definition(&compatibility_ref)
        .unwrap()
        .unwrap();
    assert_eq!(
        manual_definition.authoring_kind,
        TransformAuthoringKind::Manual
    );
    assert_eq!(manual_definition.steps[0].operation_ref, "builtin:trim");

    let manual_copy = db
        .duplicate_transform_definition(&compatibility_ref, None)
        .unwrap();
    assert_eq!(manual_copy.name, "Trim Copy");
    assert_eq!(manual_copy.authoring_kind, TransformAuthoringKind::Manual);

    db.delete_transform_definition(&compatibility_ref).unwrap();
    db.delete_transform_definition(&intent.stable_ref).unwrap();
    assert!(db
        .resolve_transform_definition(&intent.stable_ref)
        .unwrap()
        .is_none());
}

#[test]
fn execution_owner_preserves_transitions_order_and_limit() {
    let db = setup_test_db();
    let clip_id = save_text_clip_id(&db, "execution-test");
    let mut execution_ids = Vec::new();
    for index in 0..27 {
        let execution_id = db
            .begin_transformation_execution(TransformationExecutionStart {
                target_kind: "operation",
                target_ref: "builtin:trim",
                target_revision: None,
                source_clip_id: Some(clip_id),
                trigger_kind: "manual",
                destination_kind: "preview",
                input_hash: &format!("input-{index}"),
            })
            .unwrap();
        execution_ids.push(execution_id);
    }
    db.start_transformation_execution(&execution_ids[26])
        .unwrap();
    db.finish_transformation_execution(&execution_ids[26], 7, Some("output"), None)
        .unwrap();
    db.start_transformation_execution(&execution_ids[25])
        .unwrap();
    db.cancel_transformation_execution(&execution_ids[25], 3)
        .unwrap();

    let executions = db.get_clip_transformation_executions(clip_id).unwrap();
    assert_eq!(executions.len(), 25);
    assert_eq!(executions[0].id, execution_ids[26]);
    assert_eq!(executions[0].status, "succeeded");
    assert_eq!(executions[1].id, execution_ids[25]);
    assert_eq!(executions[1].status, "cancelled");
    assert_eq!(executions[24].id, execution_ids[2]);
}

#[test]
fn application_owner_preserves_atomic_revision_and_provenance() {
    let db = setup_test_db();
    let clip_id = save_text_clip_id(&db, "application-test");
    let transform = db
        .create_saved_transform("Uppercase", &deterministic_plan(), None)
        .unwrap();
    let provenance = db
        .apply_transform_output_to_clip(TransformClipApplication {
            clip_id,
            transform_ref: &transform.stable_ref,
            expected_input: "hello",
            output: "HELLO",
            connection_id: None,
            duration_ms: 8,
            bin_move: None,
        })
        .unwrap();
    assert_eq!(provenance.transform_ref, transform.stable_ref);
    assert_eq!(provenance.duration_ms, 8);
    assert_eq!(
        db.get_clip_versions(clip_id).unwrap()[0].text_content,
        "hello"
    );

    let stale = db.apply_transform_output_to_clip(TransformClipApplication {
        clip_id,
        transform_ref: &transform.stable_ref,
        expected_input: "hello",
        output: "STALE",
        connection_id: None,
        duration_ms: 2,
        bin_move: None,
    });
    assert!(stale
        .unwrap_err()
        .to_string()
        .contains("changed after this preview"));
    assert_eq!(db.get_clip_versions(clip_id).unwrap().len(), 1);
    assert_eq!(
        db.get_clip_transformation_provenance(clip_id)
            .unwrap()
            .unwrap()
            .transform_ref,
        transform.stable_ref
    );
}
