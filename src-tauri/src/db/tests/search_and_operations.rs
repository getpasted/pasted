use super::*;
#[test]
fn test_clip_search_and_deletion() {
    let db = setup_test_db();
    let clip1 = db
        .save_clip(
            "text",
            Some("Unique Search Secret"),
            None,
            None,
            "h1",
            "Terminal",
        )
        .unwrap();
    let _clip2 = db
        .save_clip("text", Some("Unrelated text"), None, None, "h2", "Finder")
        .unwrap();
    let classified = db.save_text_clip("person@example.com", "Mail").unwrap();

    // Search by query
    let search_results = search_test_clips(&db, "Secret");
    assert_eq!(search_results.len(), 1);
    assert_eq!(
        search_results[0].text_content.as_deref(),
        Some("Unique Search Secret")
    );
    let type_results = search_test_clips(&db, "email");
    assert_eq!(type_results.len(), 1);
    assert_eq!(type_results[0].id, classified.id);
    assert_eq!(type_results[0].content_types, vec!["email"]);

    // Test distinct apps
    let apps = db.get_distinct_sources().unwrap();
    assert!(apps.contains(&"Terminal".to_string()));
    assert!(apps.contains(&"Finder".to_string()));

    // Delete single clip (moves to trash)
    db.delete_clip(clip1.id).unwrap();
    let after_delete = db.get_clips(None, false).unwrap();
    assert_eq!(after_delete.len(), 2);

    // Verify clip is in Trash
    let trashed = db.get_trashed_clips().unwrap();
    assert_eq!(trashed.len(), 1);
    assert_eq!(trashed[0].id, clip1.id);
    assert_eq!(db.get_total_clip_count().unwrap(), 2);

    // Restore clip
    db.restore_clip(clip1.id).unwrap();
    let after_restore = db.get_clips(None, false).unwrap();
    assert_eq!(after_restore.len(), 3);
}

#[test]
fn untrusted_clip_and_metadata_text_cannot_become_sql() {
    let db = setup_test_db();
    let hostile = "'); DROP TABLE clips; DELETE FROM bins; -- \" * OR 1=1";
    let hostile_transform = "AI output: '; UPDATE clips SET is_protected = 0; --";
    let hostile_rule = serde_json::json!({
        "type": "contains",
        "value": hostile,
    })
    .to_string();

    let clip = db
        .save_clip("text", Some(hostile), None, None, "hostile-hash", hostile)
        .unwrap();
    db.update_clip_text(clip.id, hostile_transform).unwrap();
    db.update_clip_note(clip.id, Some(hostile)).unwrap();
    let bin = db
        .create_bin(hostile, hostile, hostile, Some(&hostile_rule))
        .unwrap();

    // Search input is also untrusted. It may use FTS syntax internally, but it must
    // remain a bound value and must never alter the surrounding SQL statement.
    let _ = search_test_clips(&db, hostile);

    let conn = db.conn.lock();
    let clip_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM clips", [], |row| row.get(0))
        .unwrap();
    let stored: (String, String, String) = conn
        .query_row(
            "SELECT text_content, source, note FROM clips WHERE id = ?1",
            params![clip.id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    let stored_bin_name: String = conn
        .query_row(
            "SELECT name FROM bins WHERE id = ?1",
            params![bin.id],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(clip_count, 1);
    assert_eq!(
        stored,
        (hostile_transform.into(), hostile.into(), hostile.into())
    );
    assert_eq!(stored_bin_name, hostile);
}

#[test]
fn oversized_note_updates_are_rejected_without_mutating_the_clip() {
    let db = setup_test_db();
    let clip = db
        .save_clip("text", Some("original"), None, None, "bounded", "Tests")
        .unwrap();
    db.update_clip_note(clip.id, Some("original note")).unwrap();
    let oversized = "x".repeat(crate::resource_limits::MAX_CLIP_NOTE_BYTES + 1);

    assert!(db.update_clip_note(clip.id, Some(&oversized)).is_err());
    let stored = db
        .get_clips(None, false)
        .unwrap()
        .into_iter()
        .find(|item| item.id == clip.id)
        .unwrap();
    assert_eq!(stored.note.as_deref(), Some("original note"));
}

#[test]
fn clip_names_are_bounded_searchable_counted_and_feature_gated() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "text",
            Some("ordinary body"),
            None,
            None,
            "named-clip",
            "Tests",
        )
        .unwrap();

    let named = db
        .update_clip_name(clip.id, Some("  📌 Deploy token  "))
        .unwrap();
    assert_eq!(named.name.as_deref(), Some("📌 Deploy token"));
    assert_eq!(db.get_clip_collection_summary().unwrap().named_count, 1);
    assert_eq!(search_test_clips(&db, "deploy")[0].id, clip.id);
    assert_eq!(search_test_clips(&db, "is:named")[0].id, clip.id);
    assert_eq!(search_test_clips(&db, "has:name")[0].id, clip.id);

    db.save_setting("enableNaming", "false").unwrap();
    assert!(search_test_clips(&db, "deploy").is_empty());
    assert!(search_test_clips(&db, "is:named").is_empty());
    assert_eq!(
        db.get_clip_by_id(clip.id).unwrap().name.as_deref(),
        Some("📌 Deploy token")
    );

    db.save_setting("enableNaming", "true").unwrap();
    let oversized = "x".repeat(clip_names::MAX_CLIP_NAME_CHARS + 1);
    assert!(db.update_clip_name(clip.id, Some(&oversized)).is_err());
    assert!(db.update_clip_name(clip.id, Some("line\nbreak")).is_err());
    assert_eq!(
        db.get_clip_by_id(clip.id).unwrap().name.as_deref(),
        Some("📌 Deploy token")
    );

    db.update_clip_name(clip.id, Some("   ")).unwrap();
    assert_eq!(db.get_clip_collection_summary().unwrap().named_count, 0);
    assert!(db.get_clip_by_id(clip.id).unwrap().name.is_none());

    db.delete_clip(clip.id).unwrap();
    assert!(db.update_clip_name(clip.id, Some("Nope")).is_err());
}

#[test]
fn test_trash_and_activity_logging() {
    let db = setup_test_db();
    let clip = db
        .save_clip("text", Some("Trash Me"), None, None, "thash1", "Notes")
        .unwrap();

    // Trash clip
    db.delete_clip(clip.id).unwrap();
    let trashed = db.get_trashed_clips().unwrap();
    assert_eq!(trashed.len(), 1);

    // Empty trash
    db.empty_trash().unwrap();
    assert_eq!(db.get_trashed_clips().unwrap().len(), 0);

    // Check activity logs
    let logs = db.get_activity_logs(None, None).unwrap();
    assert!(logs.len() >= 2); // clip_trashed, trash_emptied
    assert_eq!(logs[0].event_type, "trash_emptied");

    // Clear logs
    db.clear_activity_logs().unwrap();
    assert_eq!(db.get_activity_logs(None, None).unwrap().len(), 0);
}

#[test]
fn test_trashed_clips_are_read_only_and_leave_category_bins() {
    let db = setup_test_db();
    let category = db
        .create_bin("Projects", "Folder", "#3b82f6", None)
        .unwrap();
    let tag = db
        .create_bin_with_type("Keep", "Tag", "#f59e0b", None, "tag")
        .unwrap();
    let clip = db
        .save_clip(
            "text",
            Some("Original searchable text"),
            None,
            None,
            "trash-policy-hash",
            "Tests",
        )
        .unwrap();

    db.update_clip_note(clip.id, Some("Original searchable note"))
        .unwrap();
    db.assign_to_bin(clip.id, Some(category.id)).unwrap();
    db.add_clip_to_bin(clip.id, tag.id).unwrap();
    db.delete_clip(clip.id).unwrap();

    let trashed = db.get_trashed_clips().unwrap();
    assert_eq!(trashed.len(), 1);
    assert_eq!(trashed[0].bin_id, None);
    assert_eq!(trashed[0].note.as_deref(), Some("Original searchable note"));
    let category_after_trash = db
        .get_bins()
        .unwrap()
        .into_iter()
        .find(|bin| bin.id == category.id)
        .unwrap();
    assert_eq!(category_after_trash.clip_count, Some(0));
    {
        let conn = db.conn.lock();
        let category_links: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM clip_bins WHERE clip_id = ?1 AND bin_id = ?2",
                params![clip.id, category.id],
                |row| row.get(0),
            )
            .unwrap();
        let tag_links: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM clip_bins WHERE clip_id = ?1 AND bin_id = ?2",
                params![clip.id, tag.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(category_links, 0);
        assert_eq!(tag_links, 1);
    }

    db.assign_to_bin(clip.id, Some(category.id)).unwrap();
    db.update_clip_note(clip.id, Some("Should be ignored"))
        .unwrap();
    db.update_clip_text(clip.id, "Should also be ignored")
        .unwrap();
    let unchanged = db.get_trashed_clips().unwrap();
    assert_eq!(unchanged[0].bin_id, None);
    assert_eq!(
        unchanged[0].note.as_deref(),
        Some("Original searchable note")
    );
    assert_eq!(
        unchanged[0].text_content.as_deref(),
        Some("Original searchable text")
    );

    db.restore_clip(clip.id).unwrap();
    let restored = db.get_clips(None, false).unwrap();
    assert_eq!(restored[0].bin_id, None);
    assert!(restored[0].bin_ids.as_ref().unwrap().contains(&tag.id));
    db.assign_to_bin(clip.id, Some(category.id)).unwrap();
    db.update_clip_note(clip.id, Some("Editable after restore"))
        .unwrap();
    let edited = db.get_clips(Some(category.id), false).unwrap();
    assert_eq!(edited[0].note.as_deref(), Some("Editable after restore"));
}

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

#[test]
fn test_wal_mode_and_indexing() {
    let db = setup_test_db();
    let conn = db.conn.lock();

    // Verify WAL mode is configured
    let mode: String = conn
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    assert!(
        mode.to_lowercase() == "wal" || mode.to_lowercase() == "memory",
        "journal_mode should be wal or memory (test db), got: {}",
        mode
    );

    // Verify indexes exist
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='index'")
        .unwrap();
    let index_names: Vec<String> = stmt
        .query_map([], |r| r.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    assert!(index_names.contains(&"idx_clips_pinned_created".to_string()));
    assert!(index_names.contains(&"idx_clips_bin_created".to_string()));
    assert!(index_names.contains(&"idx_clips_hash".to_string()));
    assert!(index_names.contains(&"idx_clips_active_timeline".to_string()));
}

#[test]
fn test_fts5_search_indexing() {
    let db = setup_test_db();

    let clip1 = db
        .save_clip(
            "text",
            Some("Supercalifragilisticexpialidocious secret token"),
            None,
            None,
            "HashFTS1",
            "IntelliJ",
        )
        .unwrap();
    let _clip2 = db
        .save_clip(
            "text",
            Some("Unrelated standard content text"),
            None,
            None,
            "HashFTS2",
            "Safari",
        )
        .unwrap();

    let search_res = search_test_clips(&db, "Supercalifragilisticexpialidocious");
    assert_eq!(search_res.len(), 1);
    assert_eq!(search_res[0].id, clip1.id);

    db.update_clip_name(clip1.id, Some("Celestial Archive"))
        .unwrap();
    let name_search = search_test_clips(&db, "celestial");
    assert_eq!(name_search.len(), 1);
    assert_eq!(name_search[0].id, clip1.id);

    let status = db.get_search_index_status().unwrap();
    assert_eq!(status.indexes.len(), 2);
    assert!(status.indexes.iter().all(|index| index.healthy));
    {
        let conn = db.conn.lock();
        conn.execute("INSERT INTO clips_fts(clips_fts) VALUES('delete-all')", [])
            .unwrap();
    }
    assert!(!db.get_search_index_status().unwrap().indexes[0].healthy);
    assert!(db
        .rebuild_search_index("all")
        .unwrap()
        .indexes
        .iter()
        .all(|index| index.healthy));

    db.delete_clip(clip1.id).unwrap();
    let search_after_delete = search_test_clips(&db, "Supercalifragilisticexpialidocious");
    assert_eq!(search_after_delete.len(), 0);
}

#[test]
fn file_extraction_is_hash_safe_searchable_and_non_destructive() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "file",
            Some(r#"["/tmp/interview.wav"]"#),
            None,
            None,
            "file-transcription-hash",
            "Tests",
        )
        .unwrap();
    let extractor = db
        .get_content_extractors()
        .unwrap()
        .into_iter()
        .find(|extractor| {
            extractor.stable_ref == crate::content_extraction::WHISPER_TRANSCRIPTION_REF
        })
        .unwrap();

    assert!(!db
        .replace_clip_searchable_text(
            clip.id,
            "stale-hash",
            &extractor,
            Some("quasar transcript marker"),
        )
        .unwrap());
    assert!(db
        .replace_clip_searchable_text(
            clip.id,
            &clip.content_hash,
            &extractor,
            Some("quasar transcript marker"),
        )
        .unwrap());
    let stored = db.get_clip_searchable_text(clip.id).unwrap().unwrap();
    assert_eq!(stored.searchable_text, "quasar transcript marker");
    assert_eq!(stored.extractor_ref, extractor.stable_ref);
    assert_eq!(
        db.get_clip_by_id(clip.id).unwrap().text_content.as_deref(),
        Some(r#"["/tmp/interview.wav"]"#)
    );
    let matches = search_test_clips(&db, "quasar");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].id, clip.id);
    assert_eq!(
        db.search_clips(&ClipSearchRequest {
            query: "quasar marker".into(),
            limit: 10,
            ..Default::default()
        })
        .unwrap()
        .items[0]
            .id,
        clip.id
    );
    assert_eq!(
        db.search_clips(&ClipSearchRequest {
            query: "quasar missing".into(),
            limit: 10,
            ..Default::default()
        })
        .unwrap()
        .total_count,
        0
    );

    assert!(db
        .replace_clip_searchable_text(clip.id, &clip.content_hash, &extractor, None)
        .unwrap());
    assert!(db.get_clip_searchable_text(clip.id).unwrap().is_none());
    assert!(search_test_clips(&db, "quasar").is_empty());

    assert!(db
        .replace_clip_searchable_text(
            clip.id,
            &clip.content_hash,
            &extractor,
            Some("stale quasar marker"),
        )
        .unwrap());
    db.conn
        .lock()
        .execute(
            "UPDATE clips SET content_hash = 'changed-file-hash' WHERE id = ?1",
            params![clip.id],
        )
        .unwrap();
    assert!(db.get_clip_searchable_text(clip.id).unwrap().is_none());
    assert!(search_test_clips(&db, "quasar").is_empty());
}

#[test]
fn authoritative_search_combines_axes_pagination_trash_extraction_and_feature_gates() {
    let db = setup_test_db();
    assert!(db
        .search_clips(&ClipSearchRequest {
            limit: MAX_CLIP_SEARCH_PAGE_SIZE + 1,
            ..Default::default()
        })
        .is_err());
    let matching = db
        .save_clip(
            "file",
            Some(r#"["/tmp/report.pdf"]"#),
            None,
            None,
            "authoritative-search-match",
            "Finder",
        )
        .unwrap();
    let other = db
        .save_clip(
            "text",
            Some("ordinary shared marker"),
            None,
            None,
            "authoritative-search-other",
            "Terminal",
        )
        .unwrap();
    let extractor = db
        .get_content_extractors()
        .unwrap()
        .into_iter()
        .find(|extractor| {
            extractor.stable_ref == crate::content_extraction::WHISPER_TRANSCRIPTION_REF
        })
        .unwrap();
    db.replace_clip_searchable_text(
        matching.id,
        &matching.content_hash,
        &extractor,
        Some("extracted shared marker"),
    )
    .unwrap();
    {
        let conn = db.conn.lock();
        conn.execute(
            "INSERT INTO clip_analysis_classifications
                    (clip_id, content_type, classifier_ref, source_representation, input_hash,
                     start_offset, end_offset)
                 VALUES (?1, 'document', 'test:document', 'searchable_text', ?2, 0, 9)",
            params![matching.id, matching.content_hash],
        )
        .unwrap();
        conn.execute(
            "UPDATE clip_analysis_results
                 SET content_hash = ?3, input_hash = ?3, format_version = ?4,
                     result_json = '{\"formats\":[{\"format\":\"pdf\"}]}'
                 WHERE clip_id = ?1 AND participant_ref = ?2",
            params![
                matching.id,
                crate::content_inspection::FILE_FORMAT_INSPECTOR_REF,
                matching.content_hash,
                crate::analysis_contract::ANALYSIS_CONTRACT_VERSION,
            ],
        )
        .unwrap();
    }

    let combined = db
        .search_clips(&ClipSearchRequest {
            query: "extracted clip:fi content:doc format:pd source:find".into(),
            limit: 10,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(combined.schema_version, 1);
    assert_eq!(combined.total_count, 1);
    assert_eq!(combined.items[0].id, matching.id);
    assert_eq!(combined.items[0].content_types, vec!["document"]);
    assert_eq!(combined.items[0].file_formats, vec!["pdf"]);
    assert_eq!(combined.items[0].html_content, None);
    assert_eq!(combined.items[0].image_base64, None);
    assert_eq!(
        db.search_clips(&ClipSearchRequest {
            sources: vec!["find".into()],
            limit: 10,
            ..Default::default()
        })
        .unwrap()
        .total_count,
        1,
        "explicit Search filters use partial matching"
    );

    let first_page = db
        .search_clips(&ClipSearchRequest {
            query: "shared marker".into(),
            limit: 1,
            ..Default::default()
        })
        .unwrap();
    let second_page = db
        .search_clips(&ClipSearchRequest {
            query: "shared marker".into(),
            limit: 1,
            offset: 1,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(first_page.total_count, 2);
    assert_eq!(second_page.total_count, 2);
    assert_ne!(first_page.items[0].id, second_page.items[0].id);
    assert!(first_page
        .items
        .iter()
        .chain(&second_page.items)
        .any(|clip| clip.id == other.id));

    db.delete_clip(matching.id).unwrap();
    let trashed = db
        .search_clips(&ClipSearchRequest {
            query: "extracted is:trashed".into(),
            limit: 10,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(trashed.total_count, 1);
    assert_eq!(trashed.items[0].id, matching.id);
    assert!(trashed.items[0].is_trashed);

    for (setting, filter) in [
        ("enableClipTypes", "clip:file"),
        ("enableTypes", "content:document"),
        ("enableFileFormats", "format:pdf"),
        ("enableSources", "source:finder"),
    ] {
        db.save_setting(setting, "false").unwrap();
        assert_eq!(
            db.search_clips(&ClipSearchRequest {
                query: format!("{filter} is:trashed"),
                limit: 10,
                ..Default::default()
            })
            .unwrap()
            .total_count,
            0,
            "{setting} must suspend its Search filter"
        );
        db.save_setting(setting, "true").unwrap();
    }
    assert_eq!(
        db.search_clips(&ClipSearchRequest {
            query: "format:pd is:trashed".into(),
            limit: 10,
            ..Default::default()
        })
        .unwrap()
        .total_count,
        1,
        "collection-axis filters use case-insensitive partial matching"
    );
}

#[test]
fn test_startup_rebuilds_fts_before_clip_updates() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "text",
            Some("Recoverable noted clip"),
            None,
            None,
            "HashFTSRecovery",
            "Notes",
        )
        .unwrap();
    db.update_clip_note(clip.id, Some("Keep this note"))
        .unwrap();

    {
        let conn = db.conn.lock();
        conn.execute("INSERT INTO clips_fts(clips_fts) VALUES('delete-all')", [])
            .unwrap();
    }

    db.init_tables().unwrap();
    let search_results = search_test_clips(&db, "Recoverable");
    assert_eq!(search_results.len(), 1);
    assert_eq!(search_results[0].id, clip.id);

    assert!(db.toggle_pin(clip.id).unwrap());
    db.update_clip_note(clip.id, Some("Updated note")).unwrap();
    db.delete_clip(clip.id).unwrap();
    assert!(db.get_clips(None, false).unwrap().is_empty());
}

#[test]
fn test_unified_taxonomy_and_tags() {
    let db = setup_test_db();
    let tag = db
        .create_bin_with_type("CodeSnippet", "Tag", "#06b6d4", None, "tag")
        .unwrap();
    assert_eq!(tag.bin_type, "tag");

    let bins = db.get_bins().unwrap();
    assert!(bins.iter().any(|b| b.id == tag.id && b.bin_type == "tag"));
}

#[test]
fn test_pin_reordering() {
    let db = setup_test_db();
    let clip1 = db
        .save_clip("text", Some("First Pinned"), None, None, "HashP1", "App")
        .unwrap();
    let clip2 = db
        .save_clip("text", Some("Second Pinned"), None, None, "HashP2", "App")
        .unwrap();
    db.toggle_pin(clip1.id).unwrap();
    db.toggle_pin(clip2.id).unwrap();

    let newly_pinned_first = db.get_clips(None, true).unwrap();
    assert_eq!(newly_pinned_first[0].id, clip2.id);
    assert_eq!(newly_pinned_first[1].id, clip1.id);

    assert!(db.reorder_pinned_clips(vec![clip1.id]).is_err());
    assert!(db.reorder_pinned_clips(vec![clip1.id, clip1.id]).is_err());
    db.reorder_pinned_clips(vec![clip1.id, clip2.id]).unwrap();
    let clips = db.get_clips(None, true).unwrap();
    assert_eq!(clips[0].id, clip1.id);
    assert_eq!(clips[1].id, clip2.id);
}

#[test]
fn bin_clip_order_is_persistent_validated_and_independent_per_bin() {
    let db = setup_test_db();
    let first = db
        .save_clip("text", Some("First"), None, None, "bin-order-1", "App")
        .unwrap();
    let second = db
        .save_clip("text", Some("Second"), None, None, "bin-order-2", "App")
        .unwrap();
    let manual = db
        .create_bin("Manual Order", "Folder", "default", None)
        .unwrap();
    let smart = db
        .create_bin(
            "Smart Order",
            "Sparkles",
            "default",
            Some(r#"{"type":"clip_type","value":"text"}"#),
        )
        .unwrap();

    db.assign_to_bin(first.id, Some(manual.id)).unwrap();
    db.assign_to_bin(second.id, Some(manual.id)).unwrap();
    db.reorder_bin_clips(manual.id, vec![first.id, second.id])
        .unwrap();
    db.reorder_bin_clips(smart.id, vec![second.id, first.id])
        .unwrap();

    let manual_clips = db.get_clips(Some(manual.id), false).unwrap();
    let smart_clips = db.get_clips(Some(smart.id), false).unwrap();
    assert_eq!(
        manual_clips.iter().map(|clip| clip.id).collect::<Vec<_>>(),
        vec![first.id, second.id]
    );
    assert_eq!(
        smart_clips.iter().map(|clip| clip.id).collect::<Vec<_>>(),
        vec![second.id, first.id]
    );

    let bins = db.get_bins().unwrap();
    assert_eq!(
        bins.iter()
            .find(|bin| bin.id == manual.id)
            .unwrap()
            .clip_order,
        vec![first.id, second.id]
    );
    assert_eq!(
        bins.iter()
            .find(|bin| bin.id == smart.id)
            .unwrap()
            .clip_order,
        vec![second.id, first.id]
    );

    assert!(db.reorder_bin_clips(manual.id, vec![first.id]).is_err());
    assert!(db
        .reorder_bin_clips(manual.id, vec![first.id, first.id])
        .is_err());
    assert_eq!(
        db.get_bins()
            .unwrap()
            .iter()
            .find(|bin| bin.id == manual.id)
            .unwrap()
            .clip_order,
        vec![first.id, second.id]
    );
}
