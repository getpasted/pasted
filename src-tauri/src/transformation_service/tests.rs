use super::*;
use crate::db::DbState;
use crate::manual_transform_service::ManualTransformStepInput;
use rusqlite::params;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn test_db() -> DbState {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    DbState::new(std::env::temp_dir().join(format!(
        "pasted_execution_test_{}_{:?}.db",
        nanos,
        std::thread::current().id()
    )))
    .unwrap()
}

fn request(target: ExecutionTarget, input: &str) -> ExecutionRequest {
    ExecutionRequest {
        input: input.to_string(),
        target,
        source_clip_id: None,
        trigger: ExecutionTrigger::Manual,
        destination: ExecutionDestination::Preview,
        client_request_id: None,
    }
}

fn pipeline(db: &DbState, name: &str, operation_refs: &[&str]) -> String {
    db.create_pipeline(
        name,
        &operation_refs
            .iter()
            .map(|operation_ref| ManualTransformStepInput {
                operation_ref: (*operation_ref).to_string(),
                config_json: None,
                failure_policy: "stop".to_string(),
            })
            .collect::<Vec<_>>(),
        None,
    )
    .unwrap()
    .stable_ref
}

#[test]
fn direct_and_pipeline_operations_share_the_same_executor() {
    let db = test_db();
    let direct = execute(
        &db,
        request(
            ExecutionTarget::Operation {
                operation_ref: "builtin:uppercase".to_string(),
            },
            "hello",
        ),
    )
    .unwrap();
    assert_eq!(direct.output, "HELLO");

    let manual_transform_ref = pipeline(
        &db,
        "Loud Quote",
        &["builtin:uppercase", "builtin:quote_text"],
    );
    let pipeline = execute(
        &db,
        request(
            ExecutionTarget::ManualTransform {
                transform_ref: manual_transform_ref,
            },
            "hello\nworld",
        ),
    )
    .unwrap();
    assert_eq!(pipeline.output, "> HELLO\n> WORLD");

    let conn = db.conn.lock();
    let succeeded: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM transformation_executions
             WHERE status = 'succeeded' AND output_hash IS NOT NULL
               AND input_hash NOT LIKE '%hello%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(succeeded, 2);
}

#[test]
fn unsaved_pipeline_preview_uses_the_canonical_operation_executor() {
    let db = test_db();
    let steps = vec![
        ManualTransformStepInput {
            operation_ref: "builtin:uppercase".to_string(),
            config_json: None,
            failure_policy: "stop".to_string(),
        },
        ManualTransformStepInput {
            operation_ref: "builtin:quote_text".to_string(),
            config_json: None,
            failure_policy: "stop".to_string(),
        },
    ];

    let output = preview_manual_transform_steps(&db, "hello\nworld", &steps, None, None).unwrap();
    assert_eq!(output, "> HELLO\n> WORLD");

    let error = preview_manual_transform_steps(
        &db,
        "hello",
        &[ManualTransformStepInput {
            operation_ref: "builtin:not_real".to_string(),
            config_json: None,
            failure_policy: "stop".to_string(),
        }],
        None,
        None,
    )
    .unwrap_err();
    assert_eq!(error.code, "unknown_operation");
    assert_eq!(error.step, Some(1));
    assert_eq!(error.operation_ref.as_deref(), Some("builtin:not_real"));
}

#[test]
fn file_clips_cannot_be_mistaken_for_serialized_text_transforms() {
    let db = test_db();
    let paths = serde_json::json!(["/tmp/first.txt", "/tmp/second.txt"]).to_string();
    let clip = db
        .save_clip("file", Some(&paths), None, None, "file_clip", "Finder")
        .unwrap();
    let mut execution = request(
        ExecutionTarget::Operation {
            operation_ref: "builtin:uppercase".to_string(),
        },
        &paths,
    );
    execution.source_clip_id = Some(clip.id);
    let error = execute(&db, execution).unwrap_err();
    assert_eq!(error.code, "unsupported_clip_type");
    assert_eq!(
        db.get_clip_by_id(clip.id).unwrap().text_content,
        Some(paths)
    );
}

#[test]
fn saved_transforms_use_the_same_execution_contract_and_ledger() {
    let db = test_db();
    let clip = db
        .save_clip(
            "text",
            Some("hello"),
            None,
            None,
            "unified-transform",
            "Test",
        )
        .unwrap();
    let plan = crate::transformation_intent::TransformationPlan {
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
    let transform = db.create_saved_transform("Uppercase", &plan, None).unwrap();

    let outcome = execute(
        &db,
        ExecutionRequest {
            input: "hello".to_string(),
            target: ExecutionTarget::Transform {
                transform_ref: transform.stable_ref.clone(),
            },
            source_clip_id: Some(clip.id),
            trigger: ExecutionTrigger::Manual,
            destination: ExecutionDestination::Preview,
            client_request_id: None,
        },
    )
    .unwrap();

    assert_eq!(outcome.output, "HELLO");
    assert_eq!(outcome.connection_id, None);
    assert!(!outcome.execution_id.is_empty());
    let executions = db.get_clip_transformation_executions(clip.id).unwrap();
    assert_eq!(executions.len(), 1);
    assert_eq!(executions[0].id, outcome.execution_id);
    assert_eq!(executions[0].target_kind, "transform");
    assert_eq!(executions[0].status, "succeeded");
}

#[test]
fn transform_target_accepts_pipeline_compatibility_references() {
    let db = test_db();
    let pipeline = db
        .create_pipeline(
            "Uppercase Locally",
            &[ManualTransformStepInput {
                operation_ref: "builtin:uppercase".to_string(),
                config_json: None,
                failure_policy: "stop".to_string(),
            }],
            None,
        )
        .unwrap();

    let outcome = execute(
        &db,
        request(
            ExecutionTarget::Transform {
                transform_ref: pipeline.stable_ref.clone(),
            },
            "hello",
        ),
    )
    .unwrap();
    assert_eq!(outcome.output, "HELLO");

    let conn = db.conn.lock();
    let stored: (String, String) = conn
        .query_row(
            "SELECT target_kind, target_ref FROM transformation_executions WHERE id = ?1",
            params![outcome.execution_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(stored, ("transform".to_string(), pipeline.stable_ref));
}

#[test]
fn pipeline_errors_identify_the_step_and_operation() {
    let db = test_db();
    let manual_transform_ref = pipeline(&db, "Broken", &["builtin:uppercase", "builtin:trim"]);
    {
        let conn = db.conn.lock();
        conn.execute(
            "UPDATE saved_transforms
             SET plan_json = replace(plan_json, 'builtin:trim', 'builtin:missing')
             WHERE id = ?1",
            params![manual_transform_ref.trim_start_matches("transform:")],
        )
        .unwrap();
    }

    let error = execute(
        &db,
        request(
            ExecutionTarget::ManualTransform {
                transform_ref: manual_transform_ref,
            },
            "hello",
        ),
    )
    .unwrap_err();
    assert_eq!(error.code, "invalid_plan");
    assert!(error.message.contains("step 2"));
    assert!(error.message.contains("builtin:missing") || error.message.contains("missing"));
}

#[test]
fn cancelled_execution_is_recorded_and_does_not_produce_output() {
    let db = test_db();
    let cancellation = AtomicBool::new(true);
    let error = execute_with_cancellation(
        &db,
        request(
            ExecutionTarget::Operation {
                operation_ref: "builtin:uppercase".to_string(),
            },
            "hello",
        ),
        Some(&cancellation),
    )
    .unwrap_err();

    assert_eq!(error.code, "execution_cancelled");
    let conn = db.conn.lock();
    let (status, output_hash): (String, Option<String>) = conn
        .query_row(
            "SELECT status, output_hash FROM transformation_executions LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "cancelled");
    assert_eq!(output_hash, None);
}

#[test]
fn cancellation_registration_targets_only_the_current_request() {
    let first = CancellationRegistration::register("same-request".to_string());
    let second = CancellationRegistration::register("same-request".to_string());
    drop(first);

    assert!(cancel_execution("same-request"));
    assert!(second.flag().load(Ordering::Acquire));
    drop(second);
    assert!(!cancel_execution("same-request"));
}

#[test]
fn privileged_operations_require_trust_and_never_use_the_legacy_bridge() {
    let db = test_db();
    let operation_id = {
        let conn = db.conn.lock();
        conn.execute(
            "INSERT INTO custom_operations
                (name, executor_kind, config_json, enabled, trusted)
             VALUES ('Imported shell', 'shell', '\"cat\"', 1, 0)",
            [],
        )
        .unwrap();
        conn.query_row(
            "SELECT id FROM custom_operations WHERE row_id = last_insert_rowid()",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap()
    };
    let error = execute(
        &db,
        request(
            ExecutionTarget::Operation {
                operation_ref: format!("custom:{operation_id}"),
            },
            "hello",
        ),
    )
    .unwrap_err();
    assert_eq!(error.code, "operation_untrusted");
    assert!(execute_legacy_preview("hello", "shell_script", Some("cat")).is_err());
}

#[test]
fn last_pipeline_changes_only_after_success() {
    let db = test_db();
    execute(
        &db,
        request(
            ExecutionTarget::Operation {
                operation_ref: "builtin:uppercase".to_string(),
            },
            "hello",
        ),
    )
    .unwrap();
    assert_eq!(get_last_manual_transform_ref(&db).unwrap(), None);

    let successful = pipeline(&db, "Successful", &["builtin:uppercase"]);
    execute_shortcut_manual_transform(&db, "hello".to_string(), Some(&successful)).unwrap();
    assert_eq!(
        get_last_manual_transform_ref(&db).unwrap().as_deref(),
        Some(successful.as_str())
    );

    let failing = pipeline(&db, "Failing", &["builtin:trim"]);
    {
        let conn = db.conn.lock();
        conn.execute(
            "UPDATE saved_transforms
             SET plan_json = replace(plan_json, 'builtin:trim', 'builtin:missing')
             WHERE id = ?1",
            params![failing.trim_start_matches("transform:")],
        )
        .unwrap();
    }
    let error =
        execute_shortcut_manual_transform(&db, "hello".to_string(), Some(&failing)).unwrap_err();
    assert_eq!(error.code, "invalid_plan");
    assert_eq!(
        get_last_manual_transform_ref(&db).unwrap().as_deref(),
        Some(successful.as_str())
    );
}

#[test]
fn missing_and_deleted_last_pipeline_are_explicit() {
    let db = test_db();
    let missing = execute_shortcut_manual_transform(&db, "hello".to_string(), None).unwrap_err();
    assert_eq!(missing.code, "no_last_pipeline");

    let manual_transform_ref = pipeline(&db, "Temporary", &["builtin:uppercase"]);
    execute_shortcut_manual_transform(&db, "hello".to_string(), Some(&manual_transform_ref))
        .unwrap();
    db.delete_pipeline(&manual_transform_ref).unwrap();

    let deleted = execute_shortcut_manual_transform(&db, "hello".to_string(), None).unwrap_err();
    assert_eq!(deleted.code, "unknown_transform");
    assert_eq!(get_last_manual_transform_ref(&db).unwrap(), None);
    let cleared = execute_shortcut_manual_transform(&db, "hello".to_string(), None).unwrap_err();
    assert_eq!(cleared.code, "no_last_pipeline");
}

#[test]
fn shortcut_helper_pastes_named_or_last_pipeline_with_same_result() {
    let db = test_db();
    let manual_transform_ref = pipeline(&db, "Normalize", &["builtin:trim", "builtin:uppercase"]);
    let named = execute_shortcut_manual_transform(
        &db,
        "  hello  ".to_string(),
        Some(&manual_transform_ref),
    )
    .unwrap();
    let last = execute_shortcut_manual_transform(&db, "  hello  ".to_string(), None).unwrap();
    assert_eq!(named.output, "HELLO");
    assert_eq!(last.output, named.output);

    let conn = db.conn.lock();
    let shortcut_runs: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM transformation_executions
             WHERE trigger_kind = 'shortcut' AND target_ref = ?1 AND status = 'succeeded'",
            params![manual_transform_ref],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(shortcut_runs, 2);
}
