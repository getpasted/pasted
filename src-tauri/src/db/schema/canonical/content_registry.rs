use super::*;

pub(super) fn initialize_content_registry(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS content_type_groups (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL,
            sort_order INTEGER NOT NULL DEFAULT 100,
            is_builtin INTEGER NOT NULL DEFAULT 0,
            is_archived INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS content_types (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL,
            icon TEXT NOT NULL,
            group_name TEXT NOT NULL,
            is_builtin INTEGER NOT NULL DEFAULT 0,
            is_archived INTEGER NOT NULL DEFAULT 0,
            conceal_clips INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_content_types_order
            ON content_types (is_archived, is_builtin DESC, group_name, label);
        CREATE TABLE IF NOT EXISTS content_classifiers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            stable_ref TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            content_type TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            patterns_json TEXT NOT NULL,
            validator TEXT,
            enabled INTEGER NOT NULL DEFAULT 1,
            priority INTEGER NOT NULL DEFAULT 100,
            is_builtin INTEGER NOT NULL DEFAULT 0,
            is_deleted INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_content_classifiers_order
            ON content_classifiers (is_deleted, enabled, priority, id);
        CREATE TABLE IF NOT EXISTS content_extractors (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            stable_ref TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            engine TEXT NOT NULL,
            executable_path TEXT,
            model_path TEXT,
            input_contract TEXT NOT NULL,
            output_contract TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            priority INTEGER NOT NULL DEFAULT 100,
            revision INTEGER NOT NULL DEFAULT 1,
            shipped_revision INTEGER,
            shipped_definition_json TEXT,
            recipe_json TEXT,
            shipped_recipe_json TEXT,
            is_builtin INTEGER NOT NULL DEFAULT 0,
            is_deleted INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_content_extractors_order
            ON content_extractors (is_deleted, enabled, priority, id);",
    )?;
    configure_content_type_schema(conn)?;
    if !column_exists(conn, "content_extractors", "model_path")? {
        conn.execute(
            "ALTER TABLE content_extractors ADD COLUMN model_path TEXT",
            [],
        )?;
    }
    if !column_exists(conn, "content_extractors", "executable_path")? {
        conn.execute(
            "ALTER TABLE content_extractors ADD COLUMN executable_path TEXT",
            [],
        )?;
    }
    if !column_exists(conn, "content_extractors", "revision")? {
        conn.execute(
            "ALTER TABLE content_extractors ADD COLUMN revision INTEGER NOT NULL DEFAULT 1",
            [],
        )?;
    }
    if !column_exists(conn, "content_extractors", "shipped_revision")? {
        conn.execute(
            "ALTER TABLE content_extractors ADD COLUMN shipped_revision INTEGER",
            [],
        )?;
    }
    if !column_exists(conn, "content_extractors", "shipped_definition_json")? {
        conn.execute(
            "ALTER TABLE content_extractors ADD COLUMN shipped_definition_json TEXT",
            [],
        )?;
    }
    if !column_exists(conn, "content_extractors", "recipe_json")? {
        conn.execute(
            "ALTER TABLE content_extractors ADD COLUMN recipe_json TEXT",
            [],
        )?;
    }
    if !column_exists(conn, "content_extractors", "shipped_recipe_json")? {
        conn.execute(
            "ALTER TABLE content_extractors ADD COLUMN shipped_recipe_json TEXT",
            [],
        )?;
    }
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS extractor_authoring_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            extractor_id INTEGER NOT NULL,
            source TEXT NOT NULL,
            provider TEXT,
            model TEXT,
            original_prompt TEXT,
            manifest_version INTEGER NOT NULL DEFAULT 1,
            created_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
            updated_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
            FOREIGN KEY (extractor_id) REFERENCES content_extractors(id)
        );
        CREATE INDEX IF NOT EXISTS idx_extractor_authoring_sessions
            ON extractor_authoring_sessions (extractor_id, created_at, id);
        CREATE TABLE IF NOT EXISTS extractor_authoring_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER NOT NULL,
            sequence INTEGER NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            structured_content_json TEXT,
            created_at DATETIME NOT NULL,
            FOREIGN KEY (session_id) REFERENCES extractor_authoring_sessions(id),
            UNIQUE (session_id, sequence)
        );
        CREATE TABLE IF NOT EXISTS extractor_recipe_revisions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            extractor_id INTEGER NOT NULL,
            revision INTEGER NOT NULL,
            recipe_json TEXT NOT NULL,
            recipe_hash TEXT NOT NULL,
            authoring_session_id INTEGER,
            created_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
            FOREIGN KEY (extractor_id) REFERENCES content_extractors(id),
            FOREIGN KEY (authoring_session_id) REFERENCES extractor_authoring_sessions(id),
            UNIQUE (extractor_id, revision)
        );",
    )?;
    for preset in crate::content_types::CONTENT_TYPE_GROUP_PRESETS {
        conn.execute(
            "INSERT OR IGNORE INTO content_type_groups
                (id, label, sort_order, is_builtin, is_archived)
             VALUES (?1, ?2, ?3, 1, 0)",
            params![preset.id, preset.label, preset.sort_order],
        )?;
    }
    for preset in crate::content_types::CONTENT_TYPE_PRESETS {
        conn.execute(
            "INSERT OR IGNORE INTO content_types
                (id, label, icon, group_name, is_builtin, is_archived, conceal_clips)
             VALUES (?1, ?2, ?3, ?4, 1, 0, ?5)",
            params![
                preset.id,
                preset.label,
                preset.icon,
                preset.group,
                preset.conceal_clips()
            ],
        )?;
    }
    for preset in crate::content_classification::CLASSIFIER_PRESETS {
        let patterns_json = serde_json::to_string(&preset.patterns)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        conn.execute(
            "INSERT OR IGNORE INTO content_classifiers
                (stable_ref, name, content_type, description, patterns_json, validator, enabled, priority, is_builtin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, 1)",
            params![preset.stable_ref, preset.name, preset.content_type, preset.description, patterns_json, preset.validator, preset.priority],
        )?;
    }
    create_effective_view(conn)?;
    migrate_legacy_semantic_clip_types(conn)?;
    retire_structural_content_type_entries(conn)?;
    Ok(())
}
