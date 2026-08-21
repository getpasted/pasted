use super::*;
#[test]
fn test_clip_pinning_and_notes() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "text",
            Some("Pasted Pin Test"),
            None,
            None,
            "hash2",
            "VSCode",
        )
        .unwrap();

    // Pin clip
    let is_pinned = db.toggle_pin(clip.id).unwrap();
    assert!(is_pinned);

    // Add note
    db.update_clip_note(clip.id, Some("Important note"))
        .unwrap();

    let clips = db.get_clips(None, false).unwrap();
    assert!(clips[0].is_pinned);
    assert_eq!(clips[0].note.as_deref(), Some("Important note"));
}

#[test]
fn test_bins_crud() {
    let db = setup_test_db();
    let initial_count = db.get_bins().unwrap().len();

    let bin = db.create_bin("Work", "💼", "#3b82f6", None).unwrap();
    assert!(bin.id > 0);
    db.update_bin_hotkey(bin.id, Some("Alt+W")).unwrap();
    assert_eq!(
        db.get_bin_hotkeys().unwrap(),
        vec![(bin.id, "Work".into(), "Alt+W".into())]
    );

    let bins = db.get_bins().unwrap();
    assert_eq!(bins.len(), initial_count + 1);

    db.delete_bin(bin.id, "keep", None).unwrap();
    let bins_after = db.get_bins().unwrap();
    assert_eq!(bins_after.len(), initial_count);
}

#[test]
fn deleting_a_bin_can_keep_move_or_trash_its_clips() {
    let db = setup_test_db();

    let keep_bin = db.create_bin("Keep", "📁", "default", None).unwrap();
    let kept = db
        .save_clip("text", Some("kept"), None, None, "keep_hash", "App")
        .unwrap();
    db.assign_to_bin(kept.id, Some(keep_bin.id)).unwrap();
    db.delete_bin(keep_bin.id, "keep", None).unwrap();
    assert_eq!(db.get_clip_by_id(kept.id).unwrap().bin_id, None);

    let source_bin = db.create_bin("Source", "📁", "default", None).unwrap();
    let destination_bin = db.create_bin("Destination", "📁", "default", None).unwrap();
    let moved = db
        .save_clip("text", Some("moved"), None, None, "move_hash", "App")
        .unwrap();
    db.assign_to_bin(moved.id, Some(source_bin.id)).unwrap();
    db.delete_bin(source_bin.id, "move", Some(destination_bin.id))
        .unwrap();
    assert_eq!(
        db.get_clip_by_id(moved.id).unwrap().bin_id,
        Some(destination_bin.id)
    );

    let trash_bin = db.create_bin("Trash", "📁", "default", None).unwrap();
    let trashed = db
        .save_clip("text", Some("trashed"), None, None, "trash_hash", "App")
        .unwrap();
    let protected = db
        .save_clip(
            "text",
            Some("protected"),
            None,
            None,
            "protected_hash",
            "App",
        )
        .unwrap();
    db.assign_to_bin(trashed.id, Some(trash_bin.id)).unwrap();
    db.assign_to_bin(protected.id, Some(trash_bin.id)).unwrap();
    db.toggle_protected(protected.id).unwrap();
    db.delete_bin(trash_bin.id, "trash", None).unwrap();

    assert!(db
        .get_trashed_clips()
        .unwrap()
        .iter()
        .any(|clip| clip.id == trashed.id));
    let protected_after = db.get_clip_by_id(protected.id).unwrap();
    assert!(protected_after.is_protected);
    assert!(!protected_after.is_trashed);
    assert_eq!(protected_after.bin_id, None);
}

#[test]
fn deleting_a_bin_rejects_invalid_move_destinations_atomically() {
    let db = setup_test_db();
    let source_bin = db.create_bin("Source", "📁", "default", None).unwrap();
    let clip = db
        .save_clip("text", Some("clip"), None, None, "clip_hash", "App")
        .unwrap();
    db.assign_to_bin(clip.id, Some(source_bin.id)).unwrap();

    assert!(db.delete_bin(source_bin.id, "move", None).is_err());
    assert!(db
        .get_bins()
        .unwrap()
        .iter()
        .any(|bin| bin.id == source_bin.id));
    assert_eq!(
        db.get_clip_by_id(clip.id).unwrap().bin_id,
        Some(source_bin.id)
    );
}

#[test]
fn test_legacy_container_schema_migrates_to_bins() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db_path = std::env::temp_dir().join(format!("pasted_legacy_schema_{nanos}.db"));
    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE clips (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    content_type TEXT NOT NULL,
                    text_content TEXT,
                    html_content TEXT,
                    image_base64 TEXT,
                    content_hash TEXT UNIQUE NOT NULL,
                    source TEXT DEFAULT 'Unknown',
                    is_pinned INTEGER DEFAULT 0,
                    board_id INTEGER,
                    note TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE boards (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    icon TEXT DEFAULT 'Folder',
                    color TEXT DEFAULT '#3b82f6',
                    smart_rule TEXT,
                    board_type TEXT DEFAULT 'category',
                    shortcut TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE clip_boards (
                    clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                    board_id INTEGER NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
                    PRIMARY KEY (clip_id, board_id)
                );
                INSERT INTO boards (id, name) VALUES (41, 'Migrated Bin');
                INSERT INTO clips (
                    id, content_type, text_content, content_hash, source, board_id
                ) VALUES (73, 'text', 'Legacy assignment', 'legacy-hash', 'Test', 41);
                INSERT INTO clip_boards (clip_id, board_id) VALUES (73, 41);",
        )
        .unwrap();
    }

    let db = DbState::new(db_path).unwrap();
    let bins = db.get_bins().unwrap();
    let clips = db.get_clips(None, false).unwrap();
    assert_eq!(bins.len(), 1);
    assert_eq!(bins[0].name, "Migrated Bin");
    assert_eq!(clips.len(), 1);
    assert_eq!(clips[0].bin_id, Some(41));
    assert_eq!(clips[0].bin_ids.as_deref(), Some(&[41][..]));

    let conn = db.conn.lock();
    assert!(!table_exists(&conn, "boards").unwrap());
    assert!(!table_exists(&conn, "clip_boards").unwrap());
    assert!(column_exists(&conn, "clips", "bin_id").unwrap());
    assert!(column_exists(&conn, "bins", "bin_type").unwrap());
    assert!(column_exists(&conn, "clip_bins", "bin_id").unwrap());
}

#[test]
fn partial_pre_release_transform_migration_merges_saved_data() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db_path = std::env::temp_dir().join(format!("pasted_transform_terms_{nanos}.db"));
    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
                r#"CREATE TABLE bins (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    icon TEXT DEFAULT 'Folder',
                    color TEXT DEFAULT '#3b82f6',
                    smart_rule TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    default_recipe_id TEXT,
                    default_transform_id TEXT
                );
                CREATE TABLE transformation_recipes (
                    row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    id TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    plan_json TEXT NOT NULL,
                    connection_id TEXT,
                    revision INTEGER NOT NULL DEFAULT 1,
                    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE intelligence_connections (
                    row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    id TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    provider_kind TEXT NOT NULL,
                    endpoint TEXT,
                    model TEXT,
                    credential_ref TEXT,
                    enabled INTEGER NOT NULL DEFAULT 1,
                    priority INTEGER NOT NULL DEFAULT 0,
                    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE clip_transformations (
                    id TEXT PRIMARY KEY,
                    clip_id INTEGER NOT NULL,
                    transform_id TEXT REFERENCES transformation_recipes(id) ON DELETE SET NULL,
                    transform_name TEXT NOT NULL,
                    transform_revision INTEGER NOT NULL,
                    connection_id TEXT,
                    duration_ms INTEGER NOT NULL DEFAULT 0,
                    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE saved_transforms (
                    row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    id TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    plan_json TEXT NOT NULL,
                    connection_id TEXT,
                    revision INTEGER NOT NULL DEFAULT 1,
                    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE transformation_executions (
                    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                    target_kind TEXT NOT NULL CHECK (target_kind IN ('operation', 'pipeline')),
                    target_ref TEXT NOT NULL,
                    target_revision INTEGER,
                    source_clip_id INTEGER,
                    trigger_kind TEXT NOT NULL,
                    started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    duration_ms INTEGER,
                    status TEXT NOT NULL DEFAULT 'running',
                    error_summary TEXT,
                    input_hash TEXT NOT NULL,
                    output_hash TEXT
                );
                INSERT INTO transformation_recipes (id, name, plan_json)
                VALUES ('legacy-transform', 'Legacy Markdown',
                    '{"schema_version":1,"intent":"Markdown","summary":"Markdown","planning_mode":"pinned","steps":[]}');
                INSERT INTO bins (name, default_recipe_id)
                VALUES ('Legacy Bin', 'legacy-transform');"#,
            )
            .unwrap();
    }

    let db = DbState::new(db_path).unwrap();
    let transforms = db.get_saved_transforms().unwrap();
    assert_eq!(transforms.len(), 1);
    assert_eq!(transforms[0].stable_ref, "transform:legacy-transform");
    let legacy_bin_id = db
        .get_bins()
        .unwrap()
        .into_iter()
        .find(|bin| bin.name == "Legacy Bin")
        .unwrap()
        .id;
    assert_eq!(
        db.get_bin_transform_ref(legacy_bin_id).unwrap().as_deref(),
        Some("transform:legacy-transform")
    );

    let execution_id = db
        .begin_transformation_execution(TransformationExecutionStart {
            target_kind: "transform",
            target_ref: "transform:legacy-transform",
            target_revision: Some(1),
            source_clip_id: None,
            trigger_kind: "manual",
            destination_kind: "preview",
            input_hash: "input-hash",
        })
        .unwrap();
    db.finish_transformation_execution(&execution_id, 4, Some("output-hash"), None)
        .unwrap();
    let conn = db.conn.lock();
    assert!(!table_exists(&conn, "transformation_recipes").unwrap());
    assert!(column_exists(&conn, "bins", "default_transform_id").unwrap());
    assert!(!column_exists(&conn, "bins", "default_recipe_id").unwrap());
    assert!(column_exists(&conn, "clip_transformations", "transform_id").unwrap());
}

#[test]
fn legacy_pipelines_migrate_atomically_to_canonical_transforms() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db_path = std::env::temp_dir().join(format!("pasted_pipeline_merge_{nanos}.db"));
    let (bin_id, clip_id) = {
        let db = DbState::new(db_path.clone()).unwrap();
        let bin_id = db.get_bins().unwrap()[0].id;
        let clip_id = db
            .save_clip(
                "text",
                Some("migrate me"),
                None,
                None,
                "pipeline-migration-clip",
                "Test",
            )
            .unwrap()
            .id;
        (bin_id, clip_id)
    };
    {
        let conn = Connection::open(&db_path).unwrap();
        conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
        conn.execute_batch(
                r#"ALTER TABLE bins ADD COLUMN default_pipeline_id TEXT;
                DROP TABLE transformation_executions;
                CREATE TABLE transformation_executions (
                    id TEXT PRIMARY KEY,
                    target_kind TEXT NOT NULL CHECK (target_kind IN ('operation', 'pipeline')),
                    target_ref TEXT NOT NULL,
                    target_revision INTEGER,
                    source_clip_id INTEGER,
                    trigger_kind TEXT NOT NULL,
                    started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    duration_ms INTEGER,
                    status TEXT NOT NULL DEFAULT 'running',
                    error_summary TEXT,
                    input_hash TEXT NOT NULL,
                    output_hash TEXT
                );
                CREATE TABLE pipelines (
                    row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    id TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    shortcut TEXT,
                    revision INTEGER NOT NULL DEFAULT 1,
                    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE pipeline_steps (
                    row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    pipeline_id TEXT NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
                    position INTEGER NOT NULL,
                    operation_ref TEXT NOT NULL,
                    config_json TEXT,
                    failure_policy TEXT NOT NULL DEFAULT 'stop'
                );
                INSERT INTO saved_transforms
                    (id, name, plan_json, authoring_kind)
                VALUES ('shared-id', 'Existing Intent',
                    '{"schema_version":1,"intent":"Keep","summary":"Keep","planning_mode":"pinned","steps":[{"name":"Trim","rationale":"Keep","scope":"whole_input","failure_policy":"stop","executor":{"kind":"deterministic","operation_ref":"builtin:trim","config_json":null}}]}',
                    'intent');
                INSERT INTO pipelines
                    (id, name, shortcut, revision, created_at, updated_at)
                VALUES ('shared-id', 'Legacy Manual', 'Alt+M', 4,
                    '2026-01-01 00:00:00', '2026-01-02 00:00:00');
                INSERT INTO pipeline_steps
                    (pipeline_id, position, operation_ref, failure_policy)
                VALUES ('shared-id', 0, 'builtin:uppercase', 'skip');
                UPDATE bins SET default_pipeline_id = 'pipeline:shared-id' WHERE id = 1;
                INSERT INTO clip_transformations
                    (id, clip_id, transform_ref, transform_name, transform_revision, duration_ms)
                VALUES ('legacy-provenance', 1, 'pipeline:shared-id', 'Legacy Manual', 4, 3);
                INSERT INTO transformation_executions
                    (id, target_kind, target_ref, target_revision, trigger_kind, input_hash)
                VALUES ('legacy-execution', 'pipeline', 'pipeline:shared-id', 4, 'manual', 'hash');
                INSERT INTO settings (key, value)
                VALUES ('lastExecutedPipelineRef', 'pipeline:shared-id');
                DROP TABLE automation_conditions;
                DROP TABLE automations;
                CREATE TABLE automations (
                    row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    id TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    trigger_kind TEXT NOT NULL,
                    pipeline_id TEXT NOT NULL REFERENCES pipelines(id) ON DELETE RESTRICT,
                    enabled INTEGER NOT NULL DEFAULT 0,
                    trusted INTEGER NOT NULL DEFAULT 0,
                    priority INTEGER NOT NULL DEFAULT 0,
                    action_json TEXT NOT NULL DEFAULT '{}',
                    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE automation_conditions (
                    id TEXT PRIMARY KEY,
                    automation_id TEXT NOT NULL REFERENCES automations(id) ON DELETE CASCADE,
                    position INTEGER NOT NULL,
                    condition_kind TEXT NOT NULL,
                    config_json TEXT NOT NULL
                );
                INSERT INTO automations
                    (id, name, trigger_kind, pipeline_id, enabled, trusted)
                VALUES ('legacy-automation', 'Legacy Automation', 'capture', 'shared-id', 1, 1);
                INSERT INTO automation_conditions
                    (id, automation_id, position, condition_kind, config_json)
                VALUES ('legacy-condition', 'legacy-automation', 0, 'content_type', '{}');"#,
            )
            .unwrap();
        conn.execute(
            "UPDATE bins SET default_pipeline_id = 'pipeline:shared-id' WHERE id = ?1",
            params![bin_id],
        )
        .unwrap();
        conn.execute(
            "UPDATE clip_transformations SET clip_id = ?1 WHERE id = 'legacy-provenance'",
            params![clip_id],
        )
        .unwrap();
        conn.execute(
            "UPDATE clips SET current_transformation_id = 'legacy-provenance' WHERE id = ?1",
            params![clip_id],
        )
        .unwrap();
    }

    let db = DbState::new(db_path).unwrap();
    let transforms = db.get_saved_transforms().unwrap();
    let migrated = transforms
        .iter()
        .find(|transform| transform.name == "Legacy Manual")
        .unwrap();
    assert_ne!(migrated.stable_ref, "transform:shared-id");
    assert_eq!(migrated.authoring_kind, "manual");
    assert_eq!(migrated.shortcut.as_deref(), Some("Alt+M"));
    assert_eq!(migrated.revision, 4);
    assert_eq!(
        migrated.plan.steps[0].failure_policy,
        crate::transformation_intent::StepFailurePolicy::Skip
    );
    assert_eq!(
        db.get_bin_transform_ref(bin_id).unwrap().as_deref(),
        Some(migrated.stable_ref.as_str())
    );
    assert_eq!(
        db.get_clip_transformation_provenance(clip_id)
            .unwrap()
            .unwrap()
            .transform_ref,
        migrated.stable_ref
    );
    assert_eq!(
        db.get_setting("lastExecutedTransformRef")
            .unwrap()
            .as_deref(),
        Some(migrated.stable_ref.as_str())
    );
    assert_eq!(db.get_setting("lastExecutedPipelineRef").unwrap(), None);
    let conn = db.conn.lock();
    let execution: (String, String) = conn
        .query_row(
            "SELECT target_kind, target_ref FROM transformation_executions
                 WHERE id = 'legacy-execution'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        execution,
        ("transform".to_string(), migrated.stable_ref.clone())
    );
    assert_eq!(
        conn.query_row(
            "SELECT transform_id FROM clip_transformations
                 WHERE id = 'legacy-provenance'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap(),
        migrated.stable_ref.trim_start_matches("transform:")
    );
    let automation_transform: String = conn
        .query_row(
            "SELECT transform_id FROM automations WHERE id = 'legacy-automation'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        automation_transform,
        migrated.stable_ref.trim_start_matches("transform:")
    );
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM automation_conditions
                 WHERE automation_id = 'legacy-automation'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap(),
        1
    );
    assert!(!table_exists(&conn, "pipelines").unwrap());
    assert!(!table_exists(&conn, "pipeline_steps").unwrap());
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap(),
        0,
        "the migrated database must retain foreign-key integrity"
    );
}

#[test]
fn legacy_pipeline_migration_rolls_back_on_an_orphaned_reference() {
    let db = setup_test_db();
    let conn = db.conn.lock();
    conn.execute_batch(
        "CREATE TABLE pipelines (
                row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                id TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL
             );
             CREATE TABLE pipeline_steps (
                row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                pipeline_id TEXT NOT NULL,
                position INTEGER NOT NULL,
                operation_ref TEXT NOT NULL
             );
             INSERT INTO pipelines (id, name) VALUES ('valid-pipeline', 'Keep Me');
             INSERT INTO pipeline_steps (pipeline_id, position, operation_ref)
             VALUES ('valid-pipeline', 0, 'builtin:trim');
             INSERT INTO settings (key, value)
             VALUES ('lastExecutedPipelineRef', 'pipeline:missing-pipeline');",
    )
    .unwrap();

    let error = migrate_pipelines_to_saved_transforms(&conn)
        .unwrap_err()
        .to_string();
    assert!(error.contains("last-used setting"));
    assert!(table_exists(&conn, "pipelines").unwrap());
    assert!(table_exists(&conn, "pipeline_steps").unwrap());
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM pipelines", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM saved_transforms WHERE authoring_kind = 'manual'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap(),
        0
    );
    assert_eq!(
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'lastExecutedPipelineRef'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap(),
        "pipeline:missing-pipeline"
    );
    assert!(!column_exists(&conn, "pipelines", "shortcut").unwrap());
    assert!(!column_exists(&conn, "pipeline_steps", "failure_policy").unwrap());

    conn.execute(
        "DELETE FROM settings WHERE key = 'lastExecutedPipelineRef'",
        [],
    )
    .unwrap();
    migrate_pipelines_to_saved_transforms(&conn).unwrap();
    assert!(!table_exists(&conn, "pipelines").unwrap());
    assert!(!table_exists(&conn, "pipeline_steps").unwrap());
    let migrated: (String, String) = conn
        .query_row(
            "SELECT name, authoring_kind FROM saved_transforms
                 WHERE id = 'valid-pipeline'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(migrated, ("Keep Me".to_string(), "manual".to_string()));
}

#[test]
fn test_settings_storage() {
    let db = setup_test_db();
    db.save_setting("hudHotkey", "CmdOrCtrl+Shift+V").unwrap();
    let val = db.get_setting("hudHotkey").unwrap();
    assert_eq!(val.as_deref(), Some("CmdOrCtrl+Shift+V"));
}
