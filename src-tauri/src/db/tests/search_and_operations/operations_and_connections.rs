use super::super::*;

#[test]
fn test_pipelines_and_operations_crud() {
    let db = setup_test_db();

    // Built-ins are registry-owned and the old seeded snapshot tables are gone.
    assert!(db.get_pipelines().unwrap().is_empty());
    assert_eq!(
        db.get_library_items(Some("operation"), false)
            .unwrap()
            .iter()
            .filter(|item| item.item.is_builtin)
            .count(),
        crate::operation_registry::BUILTIN_OPERATIONS.len()
    );
    {
        let conn = db.conn.lock();
        assert!(!table_exists(&conn, "operations").unwrap());
        assert!(table_exists(&conn, "custom_operations").unwrap());
        assert!(!table_exists(&conn, "pipelines").unwrap());
        assert!(!table_exists(&conn, "pipeline_steps").unwrap());
        let persisted_builtins: i64 = conn
            .query_row("SELECT COUNT(*) FROM custom_operations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(persisted_builtins, 0);
    }

    // Pipeline CRUD
    let pipeline = db
        .create_pipeline(
            "Trim",
            &[PipelineStepInput {
                operation_ref: "builtin:trim".to_string(),
                config_json: None,
                failure_policy: "stop".to_string(),
            }],
            Some("Alt+T"),
        )
        .unwrap();
    assert!(pipeline.id > 0);
    let pipeline_item = db
        .get_library_items(Some("transform"), false)
        .unwrap()
        .into_iter()
        .find(|item| item.item.stable_ref == pipeline.stable_ref)
        .unwrap();
    assert_eq!(pipeline_item.item.input_contract, "text");
    assert!(pipeline_item.capabilities.can_edit);

    let pipelines = db.get_pipelines().unwrap();
    assert_eq!(pipelines[0].name, "Trim");
    assert_eq!(pipelines[0].steps[0].operation_ref, "builtin:trim");

    db.delete_pipeline(&pipeline.stable_ref).unwrap();
    assert!(db.get_pipelines().unwrap().is_empty());
    assert!(db
        .get_library_items(Some("transform"), false)
        .unwrap()
        .is_empty());

    // Operation CRUD
    let op = db
        .create_operation("JSON Prettify", "json_format", None, Some("Format"))
        .unwrap();
    assert!(op.id > 0);
    assert!(db
        .get_library_items(Some("operation"), false)
        .unwrap()
        .iter()
        .any(|item| item.item.stable_ref == op.stable_id && item.capabilities.can_delete));

    db.set_library_item_enabled("operation", &op.stable_id, false)
        .unwrap();
    let disabled = db
        .get_library_items(Some("operation"), false)
        .unwrap()
        .into_iter()
        .find(|item| item.item.stable_ref == op.stable_id)
        .unwrap();
    assert_eq!(disabled.item.enabled, Some(false));
    assert!(
        !db.resolve_custom_operation(&op.stable_id)
            .unwrap()
            .unwrap()
            .enabled
    );
    db.set_library_item_enabled("operation", &op.stable_id, true)
        .unwrap();

    let ops = db.get_operations().unwrap();
    assert!(ops.iter().any(|o| o.name == "JSON Prettify"));
    assert_eq!(db.get_operation(&op.stable_id).unwrap().id, op.id);
    let duplicate = db
        .duplicate_operation(&op.stable_id, Some("JSON Prettify Copy"))
        .unwrap();
    assert_eq!(duplicate.op_type, op.op_type);
    assert_eq!(duplicate.name, "JSON Prettify Copy");
    db.delete_operation(duplicate.id).unwrap();

    db.delete_operation(op.id).unwrap();
    let ops_after = db.get_operations().unwrap();
    assert!(!ops_after.iter().any(|o| o.id == op.id));
    assert!(db
        .get_library_items(Some("operation"), false)
        .unwrap()
        .iter()
        .all(|item| item.item.stable_ref != op.stable_id));
}

#[test]
fn deleting_an_operation_preserves_pipelines_that_depend_on_it() {
    let db = setup_test_db();
    let operation = db
        .create_operation(
            "Reusable cleanup",
            "regex",
            Some(r#"{"pattern":"x","replacement":"y"}"#),
            Some("Custom Operations"),
        )
        .unwrap();
    let pipeline = db
        .create_pipeline(
            "Important Pipeline",
            &[PipelineStepInput {
                operation_ref: operation.stable_id.clone(),
                config_json: None,
                failure_policy: "stop".to_string(),
            }],
            None,
        )
        .unwrap();

    let error = db.delete_operation(operation.id).unwrap_err().to_string();
    assert!(error.contains("Important Pipeline"));
    assert!(db
        .get_operations()
        .unwrap()
        .iter()
        .any(|candidate| candidate.id == operation.id));

    db.delete_pipeline(&pipeline.stable_ref).unwrap();
    db.delete_operation(operation.id).unwrap();
    assert!(!db
        .get_operations()
        .unwrap()
        .iter()
        .any(|candidate| candidate.id == operation.id));
}

#[test]
fn intelligence_connections_store_references_but_not_credentials() {
    let db = setup_test_db();
    let connection = db
        .create_intelligence_connection(
            "Local Ollama",
            "ollama",
            Some("http://127.0.0.1:11434"),
            Some("qwen3"),
            None,
        )
        .unwrap();
    assert_eq!(
        db.get_intelligence_connection(&connection.id).unwrap(),
        connection
    );
    assert_eq!(connection.provider_kind, "ollama");
    assert_eq!(connection.credential_ref, None);

    db.update_intelligence_connection(IntelligenceConnectionUpdate {
        id: &connection.id,
        name: "Local Planner",
        provider_kind: "openai_compatible",
        endpoint: Some("http://127.0.0.1:1234/v1"),
        model: Some("local-model"),
        credential_ref: Some("env:PASTED_AI_API_KEY"),
        enabled: false,
    })
    .unwrap();
    let connections = db.get_intelligence_connections().unwrap();
    assert_eq!(connections.len(), 1);
    assert_eq!(connections[0].name, "Local Planner");
    assert!(!connections[0].enabled);
    assert_eq!(
        connections[0].credential_ref.as_deref(),
        Some("env:PASTED_AI_API_KEY")
    );

    let fallback = db
        .create_intelligence_connection(
            "Fallback Ollama",
            "ollama",
            Some("http://127.0.0.1:11434"),
            None,
            None,
        )
        .unwrap();
    assert!(db
        .reorder_intelligence_connections(std::slice::from_ref(&connection.id))
        .is_err());
    assert!(db
        .reorder_intelligence_connections(&[connection.id.clone(), connection.id.clone()])
        .is_err());
    db.reorder_intelligence_connections(&[fallback.id.clone(), connection.id.clone()])
        .unwrap();
    let reordered = db.get_intelligence_connections().unwrap();
    assert_eq!(reordered[0].id, fallback.id);
    assert_eq!(reordered[0].priority, 0);
    assert_eq!(reordered[1].id, connection.id);
    assert_eq!(reordered[1].priority, 1);

    db.delete_intelligence_connection(&connection.id).unwrap();
    db.delete_intelligence_connection(&fallback.id).unwrap();
    assert!(db.get_intelligence_connections().unwrap().is_empty());
}

#[test]
fn detected_intelligence_candidates_are_disabled_and_idempotent() {
    let db = setup_test_db();
    db.ensure_intelligence_connection_candidate("Codex CLI", "cli", Some("/usr/local/bin/codex"))
        .unwrap();
    db.ensure_intelligence_connection_candidate("Codex CLI", "cli", Some("/usr/local/bin/codex"))
        .unwrap();

    let connections = db.get_intelligence_connections().unwrap();
    assert_eq!(connections.len(), 1);
    assert_eq!(connections[0].name, "Codex CLI");
    assert!(!connections[0].enabled);
    assert_eq!(connections[0].priority, 0);
}

#[test]
fn test_pipeline_roundtrip_update_and_validation_rollback() {
    let db = setup_test_db();
    let created = db
        .create_pipeline(
            "Normalize",
            &[
                PipelineStepInput {
                    operation_ref: "builtin:trim".to_string(),
                    config_json: None,
                    failure_policy: "stop".to_string(),
                },
                PipelineStepInput {
                    operation_ref: "builtin:wrap_tags".to_string(),
                    config_json: Some(r#""strong""#.to_string()),
                    failure_policy: "stop".to_string(),
                },
            ],
            Some("Alt+N"),
        )
        .unwrap();
    assert_eq!(created.revision, 1);
    assert_eq!(created.steps.len(), 2);
    assert_eq!(created.steps[0].position, 0);
    assert_eq!(created.steps[0].operation_ref, "builtin:trim");
    assert_eq!(created.steps[1].position, 1);
    assert_eq!(created.steps[1].config_json.as_deref(), Some(r#""strong""#));
    assert_eq!(
        db.get_pipeline_hotkeys().unwrap(),
        vec![(
            created
                .stable_ref
                .strip_prefix("transform:")
                .unwrap()
                .to_string(),
            "Normalize".into(),
            "Alt+N".into()
        )]
    );

    let updated = db
        .update_pipeline(
            &created.stable_ref,
            "Loud Quote",
            &[
                PipelineStepInput {
                    operation_ref: "builtin:uppercase".to_string(),
                    config_json: None,
                    failure_policy: "stop".to_string(),
                },
                PipelineStepInput {
                    operation_ref: "builtin:quote_text".to_string(),
                    config_json: None,
                    failure_policy: "skip".to_string(),
                },
            ],
            Some("Alt+L"),
        )
        .unwrap();
    assert_eq!(updated.stable_ref, created.stable_ref);
    assert_eq!(updated.id, created.id);
    assert_eq!(updated.revision, 2);
    assert_eq!(updated.name, "Loud Quote");
    assert_eq!(updated.shortcut.as_deref(), Some("Alt+L"));
    assert_eq!(
        updated
            .steps
            .iter()
            .map(|step| (step.position, step.operation_ref.as_str()))
            .collect::<Vec<_>>(),
        vec![(0, "builtin:uppercase"), (1, "builtin:quote_text")]
    );

    let invalid = db.update_pipeline(
        &created.stable_ref,
        "Must Roll Back",
        &[PipelineStepInput {
            operation_ref: "builtin:not-real".to_string(),
            config_json: None,
            failure_policy: "stop".to_string(),
        }],
        None,
    );
    assert!(invalid.is_err());
    let after_failure = db
        .get_pipelines()
        .unwrap()
        .into_iter()
        .find(|pipeline| pipeline.stable_ref == created.stable_ref)
        .unwrap();
    assert_eq!(after_failure.name, "Loud Quote");
    assert_eq!(after_failure.revision, 2);
    assert_eq!(after_failure.steps, updated.steps);

    let too_many_steps = (0..33)
        .map(|_| PipelineStepInput {
            operation_ref: "builtin:trim".to_string(),
            config_json: None,
            failure_policy: "stop".to_string(),
        })
        .collect::<Vec<_>>();
    assert!(db
        .create_pipeline("Too Many Steps", &too_many_steps, None)
        .unwrap_err()
        .to_string()
        .contains("at most 32 steps"));
    assert!(db
        .get_pipelines()
        .unwrap()
        .iter()
        .all(|pipeline| pipeline.name != "Too Many Steps"));
}

#[test]
fn test_pipeline_update_and_delete_report_not_found() {
    let db = setup_test_db();
    let steps = [PipelineStepInput {
        operation_ref: "builtin:trim".to_string(),
        config_json: None,
        failure_policy: "stop".to_string(),
    }];
    assert!(db
        .update_pipeline("pipeline:missing", "Missing", &steps, None)
        .is_err());
    assert!(db.delete_pipeline("pipeline:missing").is_err());
    assert!(db
        .update_pipeline_hotkey("pipeline:missing", Some("Alt+M"))
        .is_err());
}
