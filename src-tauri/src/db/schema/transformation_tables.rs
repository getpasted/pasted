use super::*;

impl DbState {
    pub(super) fn init_transformation_tables(&self, conn: &Connection) -> Result<()> {
        // The app has not shipped, so keep the domain and storage vocabulary
        // aligned. These renames preserve development data without carrying a
        // second set of compatibility APIs through the codebase.
        let has_legacy_transforms = table_exists(conn, "transformation_recipes")?;
        let has_saved_transforms = table_exists(conn, "saved_transforms")?;
        if has_legacy_transforms && !has_saved_transforms {
            conn.execute(
                "ALTER TABLE transformation_recipes RENAME TO saved_transforms",
                [],
            )?;
        } else if has_legacy_transforms && has_saved_transforms {
            // A hot-reloaded frontend can call the new API before the Rust
            // process restarts, leaving both pre-release tables behind. Merge
            // them instead of treating the new-but-empty table as authoritative.
            conn.execute(
                "INSERT OR IGNORE INTO saved_transforms
                    (row_id, id, name, plan_json, connection_id, revision, created_at, updated_at)
                 SELECT row_id, id, name, plan_json, connection_id, revision, created_at, updated_at
                 FROM transformation_recipes",
                [],
            )?;
        }
        if column_exists(conn, "clip_transformations", "recipe_id")?
            && !column_exists(conn, "clip_transformations", "transform_id")?
        {
            conn.execute(
                "ALTER TABLE clip_transformations RENAME COLUMN recipe_id TO transform_id",
                [],
            )?;
            conn.execute(
                "ALTER TABLE clip_transformations RENAME COLUMN recipe_name TO transform_name",
                [],
            )?;
            conn.execute(
                "ALTER TABLE clip_transformations RENAME COLUMN recipe_revision TO transform_revision",
                [],
            )?;
        }
        let has_legacy_bin_transform = column_exists(conn, "bins", "default_recipe_id")?;
        let has_current_bin_transform = column_exists(conn, "bins", "default_transform_id")?;
        if has_legacy_bin_transform && !has_current_bin_transform {
            conn.execute(
                "ALTER TABLE bins RENAME COLUMN default_recipe_id TO default_transform_id",
                [],
            )?;
        } else if has_legacy_bin_transform && has_current_bin_transform {
            conn.execute(
                "UPDATE bins SET default_transform_id = default_recipe_id
                 WHERE default_transform_id IS NULL AND default_recipe_id IS NOT NULL",
                [],
            )?;
            conn.execute("ALTER TABLE bins DROP COLUMN default_recipe_id", [])?;
        }

        if has_legacy_transforms && has_saved_transforms {
            let provenance_sql: String = conn.query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'clip_transformations'",
                [],
                |row| row.get(0),
            )?;
            if provenance_sql.contains("transformation_recipes") {
                conn.execute_batch(
                    "CREATE TABLE clip_transformations_migrated (
                        id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                        clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                        transform_id TEXT REFERENCES saved_transforms(id) ON DELETE SET NULL,
                        transform_ref TEXT,
                        transform_name TEXT NOT NULL,
                        transform_revision INTEGER NOT NULL,
                        connection_id TEXT REFERENCES intelligence_connections(id) ON DELETE SET NULL,
                        duration_ms INTEGER NOT NULL DEFAULT 0 CHECK (duration_ms >= 0),
                        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                    );
                    INSERT INTO clip_transformations_migrated
                        (id, clip_id, transform_id, transform_ref, transform_name, transform_revision,
                         connection_id, duration_ms, created_at)
                    SELECT id, clip_id, transform_id,
                           CASE WHEN transform_id IS NOT NULL THEN 'transform:' || transform_id END,
                           transform_name, transform_revision,
                           connection_id, duration_ms, created_at
                    FROM clip_transformations;
                    DROP TABLE clip_transformations;
                    ALTER TABLE clip_transformations_migrated RENAME TO clip_transformations;",
                )?;
            }
            conn.execute("DROP TABLE transformation_recipes", [])?;
        }

        let execution_ledger_exists = table_exists(conn, "transformation_executions")?;
        let legacy_execution_has_destination = execution_ledger_exists
            && column_exists(conn, "transformation_executions", "destination_kind")?;
        let legacy_execution_has_completed = execution_ledger_exists
            && column_exists(conn, "transformation_executions", "completed_at")?;
        let rebuild_execution_ledger = if execution_ledger_exists {
            let table_sql: String = conn.query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'transformation_executions'",
                [],
                |row| row.get(0),
            )?;
            !table_sql.contains("'transform'")
                || !table_sql.contains("'queued'")
                || !table_sql.contains("'cancelled'")
        } else {
            false
        };
        if rebuild_execution_ledger {
            conn.execute(
                "ALTER TABLE transformation_executions RENAME TO transformation_executions_legacy",
                [],
            )?;
        }

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS custom_operations (
                row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                id TEXT NOT NULL UNIQUE DEFAULT (lower(hex(randomblob(16)))),
                name TEXT NOT NULL,
                executor_kind TEXT NOT NULL CHECK (
                    executor_kind IN ('builtin', 'regex', 'cli', 'shell', 'http', 'ai')
                ),
                config_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(config_json)),
                category TEXT NOT NULL DEFAULT 'Custom Operations',
                enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
                trusted INTEGER NOT NULL DEFAULT 0 CHECK (trusted IN (0, 1)),
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS saved_transforms (
                row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                id TEXT NOT NULL UNIQUE DEFAULT (lower(hex(randomblob(16)))),
                name TEXT NOT NULL,
                plan_json TEXT NOT NULL CHECK (json_valid(plan_json)),
                connection_id TEXT REFERENCES intelligence_connections(id) ON DELETE SET NULL,
                shortcut TEXT,
                authoring_kind TEXT NOT NULL DEFAULT 'intent' CHECK (authoring_kind IN ('intent', 'manual')),
                revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
            );

            CREATE TABLE IF NOT EXISTS clip_transformations (
                id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                transform_id TEXT REFERENCES saved_transforms(id) ON DELETE SET NULL,
                transform_ref TEXT,
                transform_name TEXT NOT NULL,
                transform_revision INTEGER NOT NULL,
                connection_id TEXT REFERENCES intelligence_connections(id) ON DELETE SET NULL,
                duration_ms INTEGER NOT NULL DEFAULT 0 CHECK (duration_ms >= 0),
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
            );
            CREATE INDEX IF NOT EXISTS idx_clip_transformations_clip
                ON clip_transformations(clip_id, created_at DESC);

            CREATE TABLE IF NOT EXISTS automations (
                row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                id TEXT NOT NULL UNIQUE DEFAULT (lower(hex(randomblob(16)))),
                name TEXT NOT NULL,
                trigger_kind TEXT NOT NULL CHECK (trigger_kind IN ('capture', 'copy', 'paste')),
                transform_id TEXT NOT NULL REFERENCES saved_transforms(id) ON DELETE RESTRICT,
                enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
                trusted INTEGER NOT NULL DEFAULT 0 CHECK (trusted IN (0, 1)),
                priority INTEGER NOT NULL DEFAULT 0,
                action_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(action_json)),
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS automation_conditions (
                id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                automation_id TEXT NOT NULL REFERENCES automations(id) ON DELETE CASCADE,
                position INTEGER NOT NULL CHECK (position >= 0),
                condition_kind TEXT NOT NULL,
                config_json TEXT NOT NULL CHECK (json_valid(config_json)),
                UNIQUE (automation_id, position)
            );

            CREATE TABLE IF NOT EXISTS transformation_executions (
                id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                target_kind TEXT NOT NULL CHECK (target_kind IN ('operation', 'pipeline', 'transform')),
                target_ref TEXT NOT NULL,
                target_revision INTEGER,
                source_clip_id INTEGER REFERENCES clips(id) ON DELETE SET NULL,
                trigger_kind TEXT NOT NULL CHECK (
                    trigger_kind IN ('manual', 'shortcut', 'bin', 'automation', 'cli')
                ),
                destination_kind TEXT NOT NULL DEFAULT 'preview' CHECK (
                    destination_kind IN ('preview', 'replace', 'copy', 'paste', 'route')
                ),
                started_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                completed_at TEXT,
                duration_ms INTEGER CHECK (duration_ms IS NULL OR duration_ms >= 0),
                status TEXT NOT NULL DEFAULT 'queued' CHECK (
                    status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')
                ),
                error_summary TEXT,
                input_hash TEXT NOT NULL,
                output_hash TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_transformation_executions_started
                ON transformation_executions(started_at DESC);
            CREATE INDEX IF NOT EXISTS idx_transformation_executions_target
                ON transformation_executions(target_kind, target_ref, started_at DESC);

            CREATE TABLE IF NOT EXISTS intelligence_connections (
                row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                id TEXT NOT NULL UNIQUE DEFAULT (lower(hex(randomblob(16)))),
                name TEXT NOT NULL,
                provider_kind TEXT NOT NULL CHECK (
                    provider_kind IN ('openai_compatible', 'anthropic', 'gemini', 'ollama', 'lm_studio', 'cli')
                ),
                endpoint TEXT,
                model TEXT,
                credential_ref TEXT,
                enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
                priority INTEGER NOT NULL DEFAULT 0 CHECK (priority >= 0),
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_intelligence_connections_enabled
                ON intelligence_connections(enabled, provider_kind);

            ",
        )?;

        if !column_exists(conn, "saved_transforms", "shortcut")? {
            conn.execute("ALTER TABLE saved_transforms ADD COLUMN shortcut TEXT", [])?;
        }
        if !column_exists(conn, "saved_transforms", "authoring_kind")? {
            conn.execute(
                "ALTER TABLE saved_transforms ADD COLUMN authoring_kind TEXT NOT NULL DEFAULT 'intent'",
                [],
            )?;
        }

        if !column_exists(conn, "clip_transformations", "transform_ref")? {
            conn.execute(
                "ALTER TABLE clip_transformations ADD COLUMN transform_ref TEXT",
                [],
            )?;
        }
        conn.execute(
            "UPDATE clip_transformations
             SET transform_ref = 'transform:' || transform_id
             WHERE transform_ref IS NULL AND transform_id IS NOT NULL",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clip_transformations_ref
             ON clip_transformations(transform_ref, created_at DESC)",
            [],
        )?;

        if rebuild_execution_ledger {
            let destination_expression = if legacy_execution_has_destination {
                "destination_kind"
            } else {
                "'preview'"
            };
            let completed_expression = if legacy_execution_has_completed {
                "completed_at"
            } else {
                "CASE WHEN status = 'running' THEN NULL ELSE started_at END"
            };
            conn.execute(
                &format!(
                    "INSERT INTO transformation_executions
                    (id, target_kind, target_ref, target_revision, source_clip_id,
                     trigger_kind, destination_kind, started_at, completed_at,
                     duration_ms, status, error_summary, input_hash, output_hash)
                 SELECT id, target_kind, target_ref, target_revision, source_clip_id,
                        trigger_kind, {destination_expression}, started_at,
                        {completed_expression},
                        duration_ms, status, error_summary, input_hash, output_hash
                 FROM transformation_executions_legacy"
                ),
                [],
            )?;
            conn.execute("DROP TABLE transformation_executions_legacy", [])?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_transformation_executions_started
                 ON transformation_executions(started_at DESC)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_transformation_executions_target
                 ON transformation_executions(target_kind, target_ref, started_at DESC)",
                [],
            )?;
        }

        if !column_exists(conn, "bins", "default_transform_id")? {
            conn.execute("ALTER TABLE bins ADD COLUMN default_transform_id TEXT", [])?;
        }
        if !column_exists(conn, "intelligence_connections", "priority")? {
            conn.execute(
                "ALTER TABLE intelligence_connections ADD COLUMN priority INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }
        if !column_exists(conn, "transformation_executions", "destination_kind")? {
            conn.execute(
                "ALTER TABLE transformation_executions ADD COLUMN destination_kind TEXT NOT NULL DEFAULT 'preview'",
                [],
            )?;
        }
        if !column_exists(conn, "transformation_executions", "completed_at")? {
            conn.execute(
                "ALTER TABLE transformation_executions ADD COLUMN completed_at DATETIME",
                [],
            )?;
        }
        run_registered_migrations(conn)?;

        Ok(())
    }
}
