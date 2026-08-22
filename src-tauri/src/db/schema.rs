use super::*;

pub(super) fn insert_default_bins(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT INTO bins (name, icon, color, smart_rule) VALUES ('Images', '🖼️', '#ec4899', '{\"version\":1,\"conditions\":[{\"type\":\"clip_type\",\"operator\":\"is\",\"value\":\"image\"}],\"match\":\"any\"}')",
        [],
    )?;
    conn.execute(
        "INSERT INTO bins (name, icon, color, smart_rule) VALUES ('Links and Web', 'Link', '#3b82f6', '{\"version\":1,\"conditions\":[{\"type\":\"content_type\",\"operator\":\"is\",\"value\":\"link\"}],\"match\":\"any\"}')",
        [],
    )?;
    conn.execute(
        "INSERT INTO bins (name, icon, color, smart_rule) VALUES ('Code Snippets', 'Code', '#10b981', '{\"version\":1,\"conditions\":[{\"type\":\"content_type\",\"operator\":\"is\",\"value\":\"code\"}],\"match\":\"any\"}')",
        [],
    )?;
    Ok(())
}

pub(super) fn table_exists(conn: &Connection, name: &str) -> Result<bool> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        params![name],
        |row| row.get(0),
    )
}

pub(super) fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let names = statement.query_map([], |row| row.get::<_, String>(1))?;
    for name in names {
        if name? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<()> {
    if !column_exists(conn, table, column)? {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
            [],
        )?;
    }
    Ok(())
}

fn migrate_app_exclusion_hotkey_setting(conn: &Connection) -> Result<()> {
    if !table_exists(conn, "settings")? {
        return Ok(());
    }
    let stored: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'blacklistApps'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let Some(stored) = stored else {
        return Ok(());
    };
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&stored) else {
        return Ok(());
    };
    let Some(entries) = value.as_array_mut() else {
        return Ok(());
    };
    let mut changed = false;
    for entry in entries {
        let Some(rule) = entry.as_object_mut() else {
            continue;
        };
        let legacy = rule.remove("ignoreShortcuts");
        if let Some(legacy) = legacy {
            rule.entry("ignoreHotkeys").or_insert(legacy);
            changed = true;
        }
    }
    if changed {
        let serialized = serde_json::to_string(&value)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        conn.execute(
            "UPDATE settings SET value = ?1 WHERE key = 'blacklistApps'",
            params![serialized],
        )?;
    }
    Ok(())
}

pub(super) struct NamedMigration {
    pub(super) key: &'static str,
    pub(super) apply: fn(&Connection) -> Result<()>,
}

pub(super) fn run_named_migrations(conn: &Connection, migrations: &[NamedMigration]) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            key TEXT PRIMARY KEY,
            applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    for migration in migrations {
        let applied: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE key = ?1)",
            [migration.key],
            |row| row.get(0),
        )?;
        if applied {
            continue;
        }
        let transaction = conn.unchecked_transaction()?;
        (migration.apply)(&transaction)?;
        transaction.execute(
            "INSERT INTO schema_migrations (key) VALUES (?1)",
            [migration.key],
        )?;
        transaction.commit()?;
    }
    Ok(())
}

fn migrate_transform_activity_terminology(conn: &Connection) -> Result<()> {
    conn.execute(
        "UPDATE activity_logs
         SET event_type = replace(event_type, 'recipe_', 'transform_'),
             description = replace(replace(description, 'Recipes', 'Transforms'), 'Recipe', 'Transform')
         WHERE event_type LIKE '%recipe%' OR description LIKE '%Recipe%'",
        [],
    )?;
    Ok(())
}

fn backfill_current_transformation(conn: &Connection) -> Result<()> {
    conn.execute(
        "UPDATE clips SET current_transformation_id = (
            SELECT id FROM clip_transformations
            WHERE clip_id = clips.id
            ORDER BY created_at DESC, rowid DESC LIMIT 1
         )
         WHERE current_transformation_id IS NULL
           AND EXISTS (SELECT 1 FROM clip_transformations WHERE clip_id = clips.id)",
        [],
    )?;
    Ok(())
}

fn migrate_legacy_container_schema(conn: &Connection) -> Result<()> {
    // Pre-release databases used "board" for the same concept now consistently named "bin".
    if table_exists(conn, "boards")? && !table_exists(conn, "bins")? {
        conn.execute("ALTER TABLE boards RENAME TO bins", [])?;
    }
    if table_exists(conn, "clips")?
        && column_exists(conn, "clips", "board_id")?
        && !column_exists(conn, "clips", "bin_id")?
    {
        conn.execute("ALTER TABLE clips RENAME COLUMN board_id TO bin_id", [])?;
    }
    if table_exists(conn, "bins")?
        && column_exists(conn, "bins", "board_type")?
        && !column_exists(conn, "bins", "bin_type")?
    {
        conn.execute("ALTER TABLE bins RENAME COLUMN board_type TO bin_type", [])?;
    }
    if table_exists(conn, "clip_boards")? && !table_exists(conn, "clip_bins")? {
        conn.execute("ALTER TABLE clip_boards RENAME TO clip_bins", [])?;
    }
    if table_exists(conn, "clip_bins")?
        && column_exists(conn, "clip_bins", "board_id")?
        && !column_exists(conn, "clip_bins", "bin_id")?
    {
        conn.execute("ALTER TABLE clip_bins RENAME COLUMN board_id TO bin_id", [])?;
    }

    // SQLite cannot rename indexes; replace any pre-release names after their tables move.
    conn.execute("DROP INDEX IF EXISTS idx_clips_board_created", [])?;
    conn.execute("DROP INDEX IF EXISTS idx_clip_boards_board_id", [])?;
    conn.execute("DROP INDEX IF EXISTS idx_clip_boards_clip_id", [])?;
    Ok(())
}

fn migrate_clip_source_schema(conn: &Connection) -> Result<()> {
    if !table_exists(conn, "clips")? {
        return Ok(());
    }
    let has_legacy_column = column_exists(conn, "clips", "source_app")?;
    let has_source_column = column_exists(conn, "clips", "source")?;
    if has_legacy_column && has_source_column {
        return Err(rusqlite::Error::InvalidQuery);
    }
    if !has_legacy_column {
        return Ok(());
    }

    // The FTS table is a derived cache whose schema cannot be altered in place.
    // Remove it and its writers inside the same transaction as the canonical
    // column rename; the normal startup path recreates and rebuilds it below.
    let transaction = conn.unchecked_transaction()?;
    transaction.execute_batch(
        "DROP TRIGGER IF EXISTS clips_ai;
         DROP TRIGGER IF EXISTS clips_ad;
         DROP TRIGGER IF EXISTS clips_au;
         DROP TABLE IF EXISTS clips_fts;
         ALTER TABLE clips RENAME COLUMN source_app TO source;",
    )?;
    if table_exists(&transaction, "bins")? && column_exists(&transaction, "bins", "smart_rule")? {
        transaction.execute(
            "UPDATE bins
             SET smart_rule = replace(smart_rule, '\"source_app\"', '\"source\"')
             WHERE smart_rule LIKE '%\"source_app\"%'",
            [],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

fn migrate_multi_type_classifications(conn: &Connection) -> Result<()> {
    if !table_exists(conn, "clip_analysis_classifications")?
        || column_exists(conn, "clip_analysis_classifications", "start_offset")?
    {
        return Ok(());
    }
    let reference_column =
        if column_exists(conn, "clip_analysis_classifications", "classifier_ref")? {
            "classifier_ref"
        } else if column_exists(conn, "clip_analysis_classifications", "detector_ref")? {
            "detector_ref"
        } else {
            return Err(rusqlite::Error::InvalidParameterName(
                "Legacy classifications have no participant reference".into(),
            ));
        };
    let transaction = conn.unchecked_transaction()?;
    transaction.execute_batch(&format!(
        "DROP TABLE IF EXISTS clip_analysis_classifications_multi;
         CREATE TABLE clip_analysis_classifications_multi (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
            content_type TEXT NOT NULL,
            classifier_ref TEXT NOT NULL,
            source_representation TEXT NOT NULL
                CHECK (source_representation IN ('original_text', 'searchable_text')),
            input_hash TEXT NOT NULL,
            start_offset INTEGER,
            end_offset INTEGER,
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
            CHECK (
                (start_offset IS NULL AND end_offset IS NULL)
                OR (start_offset >= 0 AND end_offset > start_offset)
            )
         );
         INSERT INTO clip_analysis_classifications_multi
            (clip_id, content_type, classifier_ref, source_representation, input_hash,
             start_offset, end_offset, updated_at)
         SELECT clip_id, content_type, {reference_column}, source_representation, input_hash,
                NULL, NULL, updated_at
         FROM clip_analysis_classifications;
         DROP TABLE clip_analysis_classifications;
         ALTER TABLE clip_analysis_classifications_multi
            RENAME TO clip_analysis_classifications;"
    ))?;
    transaction.commit()
}

pub(super) fn migrate_legacy_semantic_clip_types(conn: &Connection) -> Result<()> {
    let transaction = conn.unchecked_transaction()?;
    transaction.execute(
        "INSERT INTO clip_analysis_classifications
            (clip_id, content_type, classifier_ref, source_representation, input_hash,
             start_offset, end_offset)
         SELECT clips.id, clips.content_type,
                COALESCE(
                    (SELECT classifiers.stable_ref
                     FROM content_classifiers AS classifiers
                     WHERE classifiers.content_type = clips.content_type
                       AND classifiers.is_deleted = 0
                     ORDER BY classifiers.priority, classifiers.id
                     LIMIT 1),
                    'legacy:' || clips.content_type
                ),
                'original_text', clips.content_hash, NULL, NULL
         FROM clips
         WHERE clips.content_type NOT IN ('text', 'image', 'file')
           AND TRIM(clips.content_type) != ''
           AND NOT EXISTS (
                SELECT 1 FROM clip_analysis_classifications AS existing
                WHERE existing.clip_id = clips.id
                  AND existing.input_hash = clips.content_hash
                  AND existing.content_type = clips.content_type
           )",
        [],
    )?;
    transaction.execute(
        "UPDATE clips
         SET content_type = 'text'
         WHERE content_type NOT IN ('text', 'image', 'file')
           AND TRIM(content_type) != ''",
        [],
    )?;
    transaction.commit()
}

pub(super) fn retire_structural_content_type_entries(conn: &Connection) -> Result<()> {
    if table_exists(conn, "bins")? && column_exists(conn, "bins", "smart_rule")? {
        let rules = {
            let mut statement =
                conn.prepare("SELECT id, smart_rule FROM bins WHERE smart_rule IS NOT NULL")?;
            let rules = statement
                .query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>>>()?;
            rules
        };
        for (id, rule_json) in rules {
            let Ok(mut rule) = serde_json::from_str::<serde_json::Value>(&rule_json) else {
                continue;
            };
            let mut changed = false;
            let mut migrate_condition = |condition: &mut serde_json::Value| {
                let is_legacy_structural = condition["type"].as_str() == Some("content_type")
                    && condition["value"]
                        .as_str()
                        .is_some_and(crate::content_types::is_structural_clip_type_id);
                if is_legacy_structural {
                    condition["type"] = serde_json::Value::String("clip_type".into());
                    changed = true;
                }
            };
            if let Some(conditions) = rule["conditions"].as_array_mut() {
                for condition in conditions {
                    migrate_condition(condition);
                }
            } else {
                migrate_condition(&mut rule);
            }
            if changed {
                conn.execute(
                    "UPDATE bins SET smart_rule = ?1 WHERE id = ?2",
                    params![
                        serde_json::to_string(&rule).map_err(|error| {
                            rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                        })?,
                        id
                    ],
                )?;
            }
        }
    }
    conn.execute(
        "DELETE FROM content_types
         WHERE id IN ('text', 'image', 'file')
           AND NOT EXISTS (
                SELECT 1 FROM content_classifiers
                WHERE content_classifiers.content_type = content_types.id
                  AND content_classifiers.is_deleted = 0
           )",
        [],
    )?;
    Ok(())
}

fn migrate_analysis_terminology_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "DROP TRIGGER IF EXISTS library_items_detector_insert;
         DROP TRIGGER IF EXISTS library_items_detector_update;
         DROP TRIGGER IF EXISTS library_items_detector_delete;",
    )?;
    let has_legacy_classifiers = table_exists(conn, "content_detectors")?;
    let has_classifiers = table_exists(conn, "content_classifiers")?;
    if has_legacy_classifiers && !has_classifiers {
        conn.execute(
            "ALTER TABLE content_detectors RENAME TO content_classifiers",
            [],
        )?;
    } else if has_legacy_classifiers {
        let transaction = conn.unchecked_transaction()?;
        transaction.execute_batch(
            "INSERT OR IGNORE INTO content_classifiers
                (stable_ref, name, content_type, description, patterns_json, validator,
                 enabled, priority, is_builtin, is_deleted, created_at, updated_at)
             SELECT stable_ref, name, content_type, description, patterns_json, validator,
                    enabled, priority, is_builtin, is_deleted, created_at, updated_at
             FROM content_detectors;
             DROP TABLE content_detectors;",
        )?;
        transaction.commit()?;
    }
    conn.execute("DROP INDEX IF EXISTS idx_content_detectors_order", [])?;

    if table_exists(conn, "settings")? {
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value)
             SELECT 'enableContentClassification', value
             FROM settings WHERE key = 'enableContentDetection'",
            [],
        )?;
        conn.execute(
            "DELETE FROM settings WHERE key = 'enableContentDetection'",
            [],
        )?;
    }
    if table_exists(conn, "schema_migrations")? {
        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (key)
             SELECT 'contentClassifierRegistryV1'
             FROM schema_migrations WHERE key = 'contentDetectorRegistryV1'",
            [],
        )?;
        conn.execute(
            "DELETE FROM schema_migrations WHERE key = 'contentDetectorRegistryV1'",
            [],
        )?;
    }
    Ok(())
}

pub(super) fn migrate_pipelines_to_saved_transforms(conn: &Connection) -> Result<()> {
    if !table_exists(conn, "pipelines")? {
        return Ok(());
    }
    let transaction = conn.unchecked_transaction()?;
    transaction.execute_batch(
        "CREATE TEMP TABLE pipeline_transform_map (
            pipeline_id TEXT PRIMARY KEY,
            transform_id TEXT NOT NULL UNIQUE
        );",
    )?;
    if !column_exists(&transaction, "pipelines", "shortcut")? {
        transaction.execute("ALTER TABLE pipelines ADD COLUMN shortcut TEXT", [])?;
    }
    if !column_exists(&transaction, "pipelines", "revision")? {
        transaction.execute(
            "ALTER TABLE pipelines ADD COLUMN revision INTEGER NOT NULL DEFAULT 1",
            [],
        )?;
    }
    if !column_exists(&transaction, "pipelines", "created_at")? {
        transaction.execute("ALTER TABLE pipelines ADD COLUMN created_at DATETIME", [])?;
        transaction.execute(
            "UPDATE pipelines SET created_at = CURRENT_TIMESTAMP WHERE created_at IS NULL",
            [],
        )?;
    }
    if !column_exists(&transaction, "pipelines", "updated_at")? {
        transaction.execute("ALTER TABLE pipelines ADD COLUMN updated_at DATETIME", [])?;
        transaction.execute(
            "UPDATE pipelines SET updated_at = COALESCE(created_at, CURRENT_TIMESTAMP)
             WHERE updated_at IS NULL",
            [],
        )?;
    }
    if !column_exists(&transaction, "pipeline_steps", "config_json")? {
        transaction.execute("ALTER TABLE pipeline_steps ADD COLUMN config_json TEXT", [])?;
    }
    if !column_exists(&transaction, "pipeline_steps", "failure_policy")? {
        transaction.execute(
            "ALTER TABLE pipeline_steps ADD COLUMN failure_policy TEXT NOT NULL DEFAULT 'stop'",
            [],
        )?;
    }
    let orphaned_step: Option<String> = transaction
        .query_row(
            "SELECT pipeline_id FROM pipeline_steps WHERE NOT EXISTS (
                SELECT 1 FROM pipelines WHERE pipelines.id = pipeline_steps.pipeline_id
             ) LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(reference) = orphaned_step {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "Cannot migrate Pipeline steps: {reference} does not identify a legacy Pipeline"
        )));
    }
    let pipeline_rows = {
        let mut statement = transaction.prepare(
            "SELECT id, name, shortcut, COALESCE(revision, 1),
                    COALESCE(created_at, CURRENT_TIMESTAMP),
                    COALESCE(updated_at, created_at, CURRENT_TIMESTAMP)
             FROM pipelines ORDER BY row_id ASC",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })?
            .collect::<Result<Vec<_>>>()?;
        rows
    };
    for (pipeline_id, name, shortcut, revision, created_at, updated_at) in pipeline_rows {
        let steps = {
            let mut statement = transaction.prepare(
                "SELECT operation_ref, config_json, failure_policy
                 FROM pipeline_steps WHERE pipeline_id = ?1 ORDER BY position ASC",
            )?;
            let rows = statement
                .query_map(params![pipeline_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })?
                .collect::<Result<Vec<_>>>()?;
            rows
        };
        let plan = crate::transformation_intent::TransformationPlan {
            schema_version: crate::transformation_intent::TRANSFORMATION_PLAN_SCHEMA_VERSION,
            intent: format!("Run {name}"),
            summary: name.clone(),
            planning_mode: crate::transformation_intent::IntentPlanningMode::Pinned,
            steps: steps
                .into_iter()
                .map(|(operation_ref, config_json, failure_policy)| {
                    let failure_policy = match failure_policy.as_str() {
                        "stop" => crate::transformation_intent::StepFailurePolicy::Stop,
                        "skip" => crate::transformation_intent::StepFailurePolicy::Skip,
                        value => {
                            return Err(rusqlite::Error::InvalidParameterName(format!(
                                "invalid legacy Pipeline failure policy: {value}"
                            )))
                        }
                    };
                    Ok(crate::transformation_intent::PlannedTransformationStep {
                        name: operation_ref
                            .strip_prefix("builtin:")
                            .or_else(|| operation_ref.strip_prefix("custom:"))
                            .unwrap_or(&operation_ref)
                            .replace('_', " "),
                        rationale: "Manually configured Operation".to_string(),
                        scope: crate::transformation_intent::StepExecutionScope::WholeInput,
                        failure_policy,
                        executor: crate::transformation_intent::PlannedExecutor::Deterministic {
                            operation_ref,
                            config_json,
                        },
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        };
        plan.validate()
            .map_err(rusqlite::Error::InvalidParameterName)?;
        let plan_json = serde_json::to_string(&plan)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let collision: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM saved_transforms WHERE id = ?1)",
            params![pipeline_id],
            |row| row.get(0),
        )?;
        let transform_id = if collision {
            transaction.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))?
        } else {
            pipeline_id.clone()
        };
        transaction.execute(
            "INSERT INTO saved_transforms
                (id, name, plan_json, connection_id, shortcut, authoring_kind, revision, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, ?4, 'manual', ?5, ?6, ?7)",
            params![
                transform_id,
                name,
                plan_json,
                shortcut,
                revision,
                created_at,
                updated_at
            ],
        )?;
        transaction.execute(
            "INSERT INTO pipeline_transform_map (pipeline_id, transform_id) VALUES (?1, ?2)",
            params![pipeline_id, transform_id],
        )?;
    }

    let unmapped_reference = |table: &str, reference: &str| {
        rusqlite::Error::InvalidParameterName(format!(
            "Cannot migrate {table}: {reference} does not identify a legacy Pipeline"
        ))
    };

    if column_exists(&transaction, "bins", "default_pipeline_id")? {
        let invalid: Option<String> = transaction
            .query_row(
                "SELECT default_pipeline_id FROM bins
                 WHERE default_pipeline_id IS NOT NULL AND NOT EXISTS (
                    SELECT 1 FROM pipeline_transform_map
                    WHERE pipeline_id = replace(bins.default_pipeline_id, 'pipeline:', '')
                 ) LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(reference) = invalid {
            return Err(unmapped_reference("Bins", &reference));
        }
        transaction.execute(
            "UPDATE bins SET default_transform_id = (
                SELECT transform_id FROM pipeline_transform_map
                WHERE pipeline_id = replace(bins.default_pipeline_id, 'pipeline:', '')
             ) WHERE default_pipeline_id IS NOT NULL
               AND EXISTS (
                SELECT 1 FROM pipeline_transform_map
                WHERE pipeline_id = replace(bins.default_pipeline_id, 'pipeline:', '')
             )",
            [],
        )?;
    }
    let invalid_provenance: Option<String> = transaction
        .query_row(
            "SELECT transform_ref FROM clip_transformations
             WHERE transform_ref LIKE 'pipeline:%' AND NOT EXISTS (
                SELECT 1 FROM pipeline_transform_map
                WHERE pipeline_id = replace(clip_transformations.transform_ref, 'pipeline:', '')
             ) LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(reference) = invalid_provenance {
        return Err(unmapped_reference("clip provenance", &reference));
    }
    transaction.execute(
        "UPDATE clip_transformations SET
            transform_id = (
                SELECT transform_id FROM pipeline_transform_map
                WHERE pipeline_id = replace(clip_transformations.transform_ref, 'pipeline:', '')
            ),
            transform_ref = 'transform:' || (
                SELECT transform_id FROM pipeline_transform_map
                WHERE pipeline_id = replace(clip_transformations.transform_ref, 'pipeline:', '')
            )
         WHERE transform_ref LIKE 'pipeline:%'",
        [],
    )?;
    let invalid_execution: Option<String> = transaction
        .query_row(
            "SELECT target_ref FROM transformation_executions
             WHERE target_kind = 'pipeline' AND NOT EXISTS (
                SELECT 1 FROM pipeline_transform_map
                WHERE pipeline_id = replace(transformation_executions.target_ref, 'pipeline:', '')
             ) LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(reference) = invalid_execution {
        return Err(unmapped_reference("execution history", &reference));
    }
    transaction.execute(
        "UPDATE transformation_executions
         SET target_kind = 'transform', target_ref = 'transform:' || (
            SELECT transform_id FROM pipeline_transform_map
            WHERE pipeline_id = replace(transformation_executions.target_ref, 'pipeline:', '')
         ) WHERE target_kind = 'pipeline' AND EXISTS (
            SELECT 1 FROM pipeline_transform_map
            WHERE pipeline_id = replace(transformation_executions.target_ref, 'pipeline:', '')
         )",
        [],
    )?;
    let invalid_last_used: Option<String> = transaction
        .query_row(
            "SELECT value FROM settings
             WHERE key = 'lastExecutedPipelineRef' AND NOT EXISTS (
                SELECT 1 FROM pipeline_transform_map
                WHERE pipeline_id = replace(settings.value, 'pipeline:', '')
             ) LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(reference) = invalid_last_used {
        return Err(unmapped_reference("last-used setting", &reference));
    }
    transaction.execute(
        "UPDATE settings SET value = 'transform:' || (
            SELECT transform_id FROM pipeline_transform_map
            WHERE pipeline_id = replace(settings.value, 'pipeline:', '')
         ) WHERE key = 'lastExecutedPipelineRef' AND EXISTS (
            SELECT 1 FROM pipeline_transform_map
            WHERE pipeline_id = replace(settings.value, 'pipeline:', '')
         )",
        [],
    )?;
    transaction.execute(
        "INSERT INTO settings (key, value)
         SELECT 'lastExecutedTransformRef', value FROM settings
         WHERE key = 'lastExecutedPipelineRef'
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [],
    )?;
    transaction.execute(
        "DELETE FROM settings WHERE key = 'lastExecutedPipelineRef'",
        [],
    )?;

    if table_exists(&transaction, "automations")?
        && column_exists(&transaction, "automations", "pipeline_id")?
    {
        let invalid_automation: Option<String> = transaction
            .query_row(
                "SELECT pipeline_id FROM automations WHERE NOT EXISTS (
                    SELECT 1 FROM pipeline_transform_map
                    WHERE pipeline_id = automations.pipeline_id
                 ) LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(reference) = invalid_automation {
            return Err(unmapped_reference("Automations", &reference));
        }
        let orphaned_condition: Option<String> = transaction
            .query_row(
                "SELECT automation_id FROM automation_conditions WHERE NOT EXISTS (
                    SELECT 1 FROM automations
                    WHERE automations.id = automation_conditions.automation_id
                 ) LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(reference) = orphaned_condition {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "Cannot migrate Automation conditions: {reference} does not identify an Automation"
            )));
        }
        transaction.execute_batch(
            "ALTER TABLE automation_conditions RENAME TO automation_conditions_pipeline_legacy;
             ALTER TABLE automations RENAME TO automations_pipeline_legacy;
             CREATE TABLE automations (
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
             INSERT INTO automations
                (row_id, id, name, trigger_kind, transform_id, enabled, trusted,
                 priority, action_json, created_at, updated_at)
             SELECT legacy.row_id, legacy.id, legacy.name, legacy.trigger_kind,
                    mapping.transform_id, legacy.enabled, legacy.trusted,
                    legacy.priority, legacy.action_json, legacy.created_at, legacy.updated_at
             FROM automations_pipeline_legacy AS legacy
             JOIN pipeline_transform_map AS mapping ON mapping.pipeline_id = legacy.pipeline_id;
             CREATE TABLE automation_conditions (
                id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                automation_id TEXT NOT NULL REFERENCES automations(id) ON DELETE CASCADE,
                position INTEGER NOT NULL CHECK (position >= 0),
                condition_kind TEXT NOT NULL,
                config_json TEXT NOT NULL CHECK (json_valid(config_json)),
                UNIQUE (automation_id, position)
             );
             INSERT INTO automation_conditions
                (id, automation_id, position, condition_kind, config_json)
             SELECT conditions.id, conditions.automation_id, conditions.position,
                    conditions.condition_kind, conditions.config_json
             FROM automation_conditions_pipeline_legacy AS conditions
             JOIN automations ON automations.id = conditions.automation_id;
             DROP TABLE automation_conditions_pipeline_legacy;
             DROP TABLE automations_pipeline_legacy;",
        )?;
    }
    transaction.execute_batch(
        "DROP TRIGGER IF EXISTS library_items_pipeline_insert;
         DROP TRIGGER IF EXISTS library_items_pipeline_update;
         DROP TRIGGER IF EXISTS library_items_pipeline_delete;
         DROP TRIGGER IF EXISTS custom_operation_delete_guard;
         DROP TABLE pipeline_steps;
         DROP TABLE pipelines;
         DROP TABLE pipeline_transform_map;",
    )?;
    transaction.commit()?;
    Ok(())
}
impl DbState {
    pub(super) fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock();

        // High-performance SQLite configuration
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        let _ = conn.pragma_update(None, "synchronous", "NORMAL");
        let _ = conn.pragma_update(None, "temp_store", "MEMORY");
        let _ = conn.pragma_update(None, "wal_autocheckpoint", "500");
        let _ = conn.pragma_update(None, "auto_vacuum", "INCREMENTAL");
        let _ = conn.pragma_update(None, "optimize", "");

        migrate_legacy_container_schema(&conn)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS clips (
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
                created_at DATETIME DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
            )",
            [],
        )?;
        crate::file_reference_health::create_file_reference_health_table(&conn)?;
        // High-speed composite indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_pinned_created ON clips (is_pinned, created_at DESC)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_bin_created ON clips (bin_id, created_at DESC)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_hash ON clips (content_hash)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS bins (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                icon TEXT DEFAULT 'Folder',
                color TEXT DEFAULT 'default',
                smart_rule TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // Every additive migration distinguishes an existing column from a real
        // SQLite failure. Never discard ALTER TABLE errors during startup.
        add_column_if_missing(&conn, "clips", "note", "TEXT")?;
        add_column_if_missing(&conn, "clips", "name", "TEXT")?;
        add_column_if_missing(&conn, "clips", "is_trashed", "INTEGER DEFAULT 0")?;
        add_column_if_missing(&conn, "clips", "trashed_at", "DATETIME")?;
        add_column_if_missing(&conn, "clips", "is_protected", "INTEGER DEFAULT 0")?;
        add_column_if_missing(&conn, "clips", "is_concealed", "INTEGER NOT NULL DEFAULT 0")?;
        add_column_if_missing(&conn, "clips", "is_revealed", "INTEGER NOT NULL DEFAULT 0")?;
        add_column_if_missing(&conn, "clips", "shortcut", "TEXT")?;
        add_column_if_missing(&conn, "clips", "image_path", "TEXT")?;
        add_column_if_missing(&conn, "clips", "pin_order", "INTEGER DEFAULT 0")?;
        add_column_if_missing(&conn, "clips", "current_transformation_id", "TEXT")?;
        add_column_if_missing(
            &conn,
            "clips",
            "ocr_status",
            "TEXT NOT NULL DEFAULT 'not_applicable'",
        )?;
        add_column_if_missing(&conn, "clips", "ocr_input_hash", "TEXT")?;
        add_column_if_missing(&conn, "clips", "ocr_engine_version", "TEXT")?;
        add_column_if_missing(&conn, "clips", "ocr_extractor_ref", "TEXT")?;
        add_column_if_missing(&conn, "clips", "ocr_extractor_name", "TEXT")?;
        add_column_if_missing(&conn, "clips", "ocr_attempted_at", "DATETIME")?;
        add_column_if_missing(&conn, "clips", "ocr_error", "TEXT")?;
        conn.execute(
            "UPDATE clips
             SET ocr_status = CASE
                    WHEN content_type = 'image' AND COALESCE(text_content, '') != '' THEN 'complete'
                    WHEN content_type = 'image' THEN 'never'
                    ELSE 'not_applicable'
                 END,
                 ocr_input_hash = CASE WHEN content_type = 'image' THEN content_hash ELSE NULL END,
                 ocr_engine_version = CASE
                    WHEN content_type = 'image' AND COALESCE(text_content, '') != '' THEN COALESCE(ocr_engine_version, 'legacy')
                    ELSE ocr_engine_version
                 END
             WHERE content_type = 'image' AND ocr_input_hash IS NULL",
            [],
        )?;
        conn.execute(
            "UPDATE clips
             SET ocr_extractor_ref = CASE ocr_engine_version
                    WHEN 'macos-vision-v1' THEN 'extractor:apple-vision-ocr'
                    ELSE ocr_extractor_ref
                 END,
                 ocr_extractor_name = CASE ocr_engine_version
                    WHEN 'macos-vision-v1' THEN 'Apple Vision OCR'
                    WHEN 'legacy' THEN 'Legacy OCR'
                    ELSE ocr_extractor_name
                 END
             WHERE ocr_status = 'complete' AND ocr_extractor_name IS NULL",
            [],
        )?;
        conn.execute(
            "UPDATE clips SET ocr_status = 'never', ocr_error = NULL
             WHERE content_type = 'image' AND ocr_status IN ('queued', 'running')",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_ocr_backfill
             ON clips (content_type, ocr_status, is_trashed, id)",
            [],
        )?;
        add_column_if_missing(&conn, "bins", "smart_rule", "TEXT")?;
        add_column_if_missing(&conn, "bins", "bin_type", "TEXT DEFAULT 'category'")?;
        add_column_if_missing(&conn, "bins", "shortcut", "TEXT")?;
        add_column_if_missing(&conn, "bins", "protect_clips", "INTEGER NOT NULL DEFAULT 0")?;
        add_column_if_missing(&conn, "bins", "conceal_clips", "INTEGER NOT NULL DEFAULT 0")?;

        migrate_clip_source_schema(&conn)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS clip_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                text_content TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clip_versions_clip_id ON clip_versions(clip_id, created_at DESC)",
            [],
        )?;
        add_column_if_missing(&conn, "clip_versions", "context_json", "TEXT")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS clip_analysis_classifications (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                content_type TEXT NOT NULL,
                classifier_ref TEXT NOT NULL,
                source_representation TEXT NOT NULL
                    CHECK (source_representation IN ('original_text', 'searchable_text')),
                input_hash TEXT NOT NULL,
                start_offset INTEGER,
                end_offset INTEGER,
                updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                CHECK (
                    (start_offset IS NULL AND end_offset IS NULL)
                    OR (start_offset >= 0 AND end_offset > start_offset)
                )
            )",
            [],
        )?;
        migrate_multi_type_classifications(&conn)?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clip_analysis_classification_type
             ON clip_analysis_classifications(content_type, clip_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clip_analysis_classification_clip
             ON clip_analysis_classifications(clip_id, input_hash, classifier_ref, start_offset)",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS clip_analysis_results (
                clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                participant_ref TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                input_hash TEXT NOT NULL,
                format_version INTEGER NOT NULL CHECK(format_version > 0),
                result_json TEXT NOT NULL,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (clip_id, participant_ref)
            )",
            [],
        )?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS clip_extraction_attempts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                run_id TEXT NOT NULL,
                participant_ref TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                priority INTEGER NOT NULL,
                result_json TEXT NOT NULL,
                run_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_clip_extraction_attempts_history
                ON clip_extraction_attempts (clip_id, run_at DESC, id DESC, priority, participant_ref);",
        )?;
        conn.execute(
            "INSERT INTO clip_extraction_attempts
                (clip_id, run_id, participant_ref, content_hash, priority, result_json, run_at)
             SELECT results.clip_id,
                    'migrated-' || results.clip_id,
                    results.participant_ref,
                    results.content_hash,
                    CAST(json_extract(results.result_json, '$.priority') AS INTEGER),
                    results.result_json,
                    COALESCE(
                        strftime('%Y-%m-%dT%H:%M:%SZ', results.updated_at),
                        strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                    )
             FROM clip_analysis_results AS results
             WHERE results.participant_ref LIKE 'extractor:%'
               AND NOT EXISTS (
                    SELECT 1 FROM clip_extraction_attempts AS attempts
                    WHERE attempts.clip_id = results.clip_id
               )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS clip_searchable_text (
                clip_id INTEGER PRIMARY KEY REFERENCES clips(id) ON DELETE CASCADE,
                extractor_ref TEXT NOT NULL,
                extractor_name TEXT NOT NULL,
                engine TEXT NOT NULL,
                input_hash TEXT NOT NULL,
                searchable_text TEXT NOT NULL,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_trashed ON clips (is_trashed, created_at DESC)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_protected ON clips (is_protected, created_at DESC)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_named_created ON clips (created_at DESC)
             WHERE name IS NOT NULL AND TRIM(name) != ''",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_shortcut ON clips (shortcut)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_active_timeline ON clips (is_trashed, is_pinned DESC, created_at DESC)",
            [],
        );

        search_indexes::ensure_search_indexes(&conn);

        conn.execute(
            "CREATE TABLE IF NOT EXISTS clip_bins (
                clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                bin_id INTEGER NOT NULL REFERENCES bins(id) ON DELETE CASCADE,
                PRIMARY KEY (clip_id, bin_id)
            )",
            [],
        )?;
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clip_bins_bin_id ON clip_bins (bin_id)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clip_bins_clip_id ON clip_bins (clip_id)",
            [],
        );

        // One shared contract protects clips from every cleanup and destructive path.
        // Smart-rule matches are intentionally excluded: only durable manual membership
        // can apply inherited protection.
        conn.execute_batch(
            "DROP VIEW IF EXISTS effective_clip_protection;
             CREATE VIEW effective_clip_protection AS
             SELECT clips.id AS clip_id,
                    CASE WHEN COALESCE(clips.is_protected, 0) = 1
                              OR NULLIF(TRIM(clips.shortcut), '') IS NOT NULL
                              OR EXISTS (
                                  SELECT 1 FROM bins
                                  WHERE COALESCE(bins.protect_clips, 0) = 1
                                    AND (bins.id = clips.bin_id OR EXISTS (
                                        SELECT 1 FROM clip_bins
                                        WHERE clip_bins.clip_id = clips.id
                                          AND clip_bins.bin_id = bins.id
                                    ))
                              )
                         THEN 1 ELSE 0 END AS is_protected,
                    (SELECT GROUP_CONCAT(protecting.id)
                     FROM bins AS protecting
                     WHERE COALESCE(protecting.protect_clips, 0) = 1
                       AND (protecting.id = clips.bin_id OR EXISTS (
                           SELECT 1 FROM clip_bins
                           WHERE clip_bins.clip_id = clips.id
                             AND clip_bins.bin_id = protecting.id
                       ))) AS protecting_bin_ids
             FROM clips;",
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS bin_clip_order (
                bin_id INTEGER NOT NULL REFERENCES bins(id) ON DELETE CASCADE,
                clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                position INTEGER NOT NULL CHECK(position >= 0),
                PRIMARY KEY (bin_id, clip_id),
                UNIQUE (bin_id, position)
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_bin_clip_order_position
             ON bin_clip_order (bin_id, position)",
            [],
        )?;

        let _ = conn.execute(
            "INSERT OR IGNORE INTO clip_bins (clip_id, bin_id)
             SELECT id, bin_id FROM clips WHERE bin_id IS NOT NULL",
            [],
        );

        // Trash is deliberately outside the organizational hierarchy. Clean up
        // legacy rows so restored clips never silently reappear in an old Bin.
        let _ = conn.execute(
            "DELETE FROM clip_bins
             WHERE clip_id IN (SELECT id FROM clips WHERE is_trashed = 1)
               AND bin_id IN (
                   SELECT id FROM bins WHERE COALESCE(bin_type, 'category') != 'tag'
               )",
            [],
        );
        let _ = conn.execute("UPDATE clips SET bin_id = NULL WHERE is_trashed = 1", []);

        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS activity_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                description TEXT NOT NULL,
                created_at DATETIME DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                observed_at DATETIME DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                severity_text TEXT NOT NULL DEFAULT 'info',
                category TEXT NOT NULL DEFAULT 'general',
                outcome TEXT NOT NULL DEFAULT 'unknown',
                attributes_json TEXT NOT NULL DEFAULT '{}'
            )",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE activity_logs ADD COLUMN observed_at DATETIME",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE activity_logs ADD COLUMN severity_text TEXT NOT NULL DEFAULT 'info'",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE activity_logs ADD COLUMN category TEXT NOT NULL DEFAULT 'general'",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE activity_logs ADD COLUMN outcome TEXT NOT NULL DEFAULT 'unknown'",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE activity_logs ADD COLUMN attributes_json TEXT NOT NULL DEFAULT '{}'",
            [],
        );
        let _ = conn.execute(
            "UPDATE activity_logs SET observed_at = created_at WHERE observed_at IS NULL",
            [],
        );
        let _ = conn.execute(
            "UPDATE activity_logs
             SET severity_text = CASE
                    WHEN event_type LIKE '%failed%' OR event_type LIKE '%error%' THEN 'error'
                    WHEN event_type LIKE '%ignored%' OR event_type LIKE '%skipped%'
                      OR event_type LIKE '%cancelled%' OR event_type LIKE '%auto_paused%' THEN 'warn'
                    ELSE severity_text
                 END,
                 category = CASE
                    WHEN event_type LIKE 'clip_%' OR event_type LIKE 'clips_%'
                      OR event_type LIKE 'trash_%' OR event_type LIKE 'note_%' THEN 'clip'
                    WHEN event_type LIKE 'recording_%' OR event_type LIKE 'clipboard_%' THEN 'capture'
                    WHEN event_type LIKE 'bin_%' OR event_type LIKE 'type_%'
                      OR event_type LIKE 'classifier_%' OR event_type LIKE 'content_%' THEN 'organization'
                    WHEN event_type LIKE 'transform%' OR event_type LIKE 'operation_%'
                      OR event_type LIKE 'intelligence_%' THEN 'transformation'
                    WHEN event_type LIKE 'setting_%' OR event_type = 'settings_changed' THEN 'settings'
                    WHEN event_type LIKE 'queue_%' OR event_type LIKE 'hud_%' THEN 'workflow'
                    WHEN event_type LIKE 'app_%' OR event_type LIKE 'library_%'
                      OR event_type LIKE 'backup_%' OR event_type LIKE 'external_%' THEN 'system'
                    ELSE category
                 END,
                 outcome = CASE
                    WHEN event_type LIKE '%failed%' OR event_type LIKE '%error%' THEN 'failure'
                    WHEN event_type LIKE '%succeeded%' OR event_type LIKE '%_completed' THEN 'success'
                    ELSE outcome
                 END",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_logs_created ON activity_logs (created_at DESC)",
            [],
        );

        self.init_transformation_tables(&conn)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;
        migrate_pipelines_to_saved_transforms(&conn)?;
        migrate_analysis_terminology_schema(&conn)?;

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
        configure_content_type_schema(&conn)?;
        if !column_exists(&conn, "content_extractors", "model_path")? {
            conn.execute(
                "ALTER TABLE content_extractors ADD COLUMN model_path TEXT",
                [],
            )?;
        }
        if !column_exists(&conn, "content_extractors", "executable_path")? {
            conn.execute(
                "ALTER TABLE content_extractors ADD COLUMN executable_path TEXT",
                [],
            )?;
        }
        if !column_exists(&conn, "content_extractors", "revision")? {
            conn.execute(
                "ALTER TABLE content_extractors ADD COLUMN revision INTEGER NOT NULL DEFAULT 1",
                [],
            )?;
        }
        if !column_exists(&conn, "content_extractors", "shipped_revision")? {
            conn.execute(
                "ALTER TABLE content_extractors ADD COLUMN shipped_revision INTEGER",
                [],
            )?;
        }
        if !column_exists(&conn, "content_extractors", "shipped_definition_json")? {
            conn.execute(
                "ALTER TABLE content_extractors ADD COLUMN shipped_definition_json TEXT",
                [],
            )?;
        }
        if !column_exists(&conn, "content_extractors", "recipe_json")? {
            conn.execute(
                "ALTER TABLE content_extractors ADD COLUMN recipe_json TEXT",
                [],
            )?;
        }
        if !column_exists(&conn, "content_extractors", "shipped_recipe_json")? {
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
        create_effective_view(&conn)?;
        migrate_legacy_semantic_clip_types(&conn)?;
        retire_structural_content_type_entries(&conn)?;
        for preset in crate::content_extraction::EXTRACTOR_PRESETS {
            conn.execute(
                "INSERT OR IGNORE INTO content_extractors
                    (stable_ref, name, description, engine, executable_path, model_path,
                     input_contract, output_contract, enabled, priority, revision,
                     shipped_revision, shipped_definition_json, is_builtin)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, 1, ?10, ?11, 1)",
                params![
                    preset.stable_ref,
                    preset.name,
                    preset.description,
                    preset.engine,
                    preset.executable_path,
                    preset.model_path,
                    preset.input_contract,
                    preset.output_contract,
                    preset.priority,
                    preset.revision,
                    serde_json::to_string(&preset.definition()).map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                    })?
                ],
            )?;
            conn.execute(
                "UPDATE content_extractors
                 SET shipped_revision = COALESCE(shipped_revision, ?1),
                     shipped_definition_json = COALESCE(shipped_definition_json, ?2)
                 WHERE stable_ref = ?3 AND is_builtin = 1",
                params![
                    preset.revision,
                    serde_json::to_string(&preset.definition()).map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                    })?,
                    preset.stable_ref,
                ],
            )?;
            let shipped = conn.query_row(
                "SELECT shipped_revision, shipped_definition_json,
                        name, description, engine, executable_path, model_path,
                        input_contract, output_contract, enabled, priority
                 FROM content_extractors WHERE stable_ref = ?1 AND is_builtin = 1",
                params![preset.stable_ref],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        crate::content_extraction::ExtractorDefinitionInput {
                            name: row.get(2)?,
                            description: row.get(3)?,
                            engine: row.get(4)?,
                            executable_path: row.get(5)?,
                            model_path: row.get(6)?,
                            input_contract: row.get(7)?,
                            output_contract: row.get(8)?,
                            enabled: row.get(9)?,
                            priority: row.get(10)?,
                        },
                    ))
                },
            )?;
            if shipped.0 < preset.revision {
                let previous = serde_json::from_str::<
                    crate::content_extraction::ExtractorDefinitionInput,
                >(&shipped.1)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
                let next = preset.definition();
                let effective = crate::content_extraction::merge_shipped_definition(
                    &shipped.2, &previous, &next,
                );
                conn.execute(
                    "UPDATE content_extractors
                     SET name = ?1, description = ?2, engine = ?3, executable_path = ?4,
                         model_path = ?5, input_contract = ?6, output_contract = ?7,
                         enabled = ?8, priority = ?9, revision = revision + 1,
                         shipped_revision = ?10, shipped_definition_json = ?11,
                         updated_at = CURRENT_TIMESTAMP
                     WHERE stable_ref = ?12 AND is_builtin = 1",
                    params![
                        effective.name,
                        effective.description,
                        effective.engine,
                        effective.executable_path,
                        effective.model_path,
                        effective.input_contract,
                        effective.output_contract,
                        effective.enabled,
                        effective.priority,
                        preset.revision,
                        serde_json::to_string(&next).map_err(|error| {
                            rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                        })?,
                        preset.stable_ref,
                    ],
                )?;
            }
            let recipe = preset.recipe();
            crate::extractor_recipe::validate_recipe(&recipe).map_err(|error| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    error,
                )))
            })?;
            let recipe_json = serde_json::to_string(&recipe)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            let (current_recipe, previous_shipped_recipe) = conn.query_row(
                "SELECT recipe_json, shipped_recipe_json
                 FROM content_extractors WHERE stable_ref = ?1 AND is_builtin = 1",
                params![preset.stable_ref],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                    ))
                },
            )?;
            let effective_recipe = match (current_recipe, previous_shipped_recipe) {
                (Some(current), Some(previous)) => {
                    let matches_previous = match (
                        serde_json::from_str::<crate::extractor_recipe::ExtractorRecipe>(&current),
                        serde_json::from_str::<crate::extractor_recipe::ExtractorRecipe>(&previous),
                    ) {
                        (Ok(current), Ok(previous)) => current == previous,
                        _ => current == previous,
                    };
                    if matches_previous {
                        recipe_json.clone()
                    } else {
                        current
                    }
                }
                (Some(current), None) => current,
                _ => recipe_json.clone(),
            };
            let effective_recipe =
                serde_json::from_str::<crate::extractor_recipe::ExtractorRecipe>(&effective_recipe)
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            let effective_recipe = crate::content_extraction::migrate_builtin_recipe_compatibility(
                preset.stable_ref,
                &effective_recipe,
                shipped.2.model_path.as_deref(),
            );
            crate::extractor_recipe::validate_recipe(&effective_recipe).map_err(|error| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    error,
                )))
            })?;
            let effective_recipe = serde_json::to_string(&effective_recipe)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            conn.execute(
                "UPDATE content_extractors
                 SET recipe_json = ?1, shipped_recipe_json = ?2
                 WHERE stable_ref = ?3 AND is_builtin = 1",
                params![effective_recipe, recipe_json, preset.stable_ref],
            )?;
        }
        {
            let legacy = {
                let mut statement = conn.prepare(
                    "SELECT id, name, description, engine, executable_path, model_path,
                            input_contract, output_contract, enabled, priority, revision
                     FROM content_extractors WHERE recipe_json IS NULL",
                )?;
                let rows = statement
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, i64>(10)?,
                            crate::content_extraction::ExtractorDefinitionInput {
                                name: row.get(1)?,
                                description: row.get(2)?,
                                engine: row.get(3)?,
                                executable_path: row.get(4)?,
                                model_path: row.get(5)?,
                                input_contract: row.get(6)?,
                                output_contract: row.get(7)?,
                                enabled: row.get(8)?,
                                priority: row.get(9)?,
                            },
                        ))
                    })?
                    .collect::<Result<Vec<_>>>()?;
                rows
            };
            for (id, revision, definition) in legacy {
                let recipe = crate::content_extraction::recipe_for_legacy_definition(&definition);
                crate::extractor_recipe::validate_recipe(&recipe).map_err(|error| {
                    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        error,
                    )))
                })?;
                let recipe_json = serde_json::to_string(&recipe)
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
                let recipe_hash = recipe.hash().map_err(|error| {
                    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(error)))
                })?;
                conn.execute(
                    "UPDATE content_extractors SET recipe_json = ?1 WHERE id = ?2",
                    params![recipe_json, id],
                )?;
                conn.execute(
                    "INSERT OR IGNORE INTO extractor_recipe_revisions
                        (extractor_id, revision, recipe_json, recipe_hash)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![id, revision, recipe_json, recipe_hash],
                )?;
            }
        }
        {
            let recipes = {
                let mut statement = conn.prepare(
                    "SELECT id, revision, recipe_json FROM content_extractors
                     WHERE recipe_json IS NOT NULL",
                )?;
                let rows = statement
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    })?
                    .collect::<Result<Vec<_>>>()?;
                rows
            };
            for (id, revision, recipe_json) in recipes {
                let recipe =
                    serde_json::from_str::<crate::extractor_recipe::ExtractorRecipe>(&recipe_json)
                        .map_err(|error| {
                            rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                        })?;
                let recipe_hash = recipe.hash().map_err(|error| {
                    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(error)))
                })?;
                conn.execute(
                    "INSERT OR IGNORE INTO extractor_recipe_revisions
                        (extractor_id, revision, recipe_json, recipe_hash)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![id, revision, recipe_json, recipe_hash],
                )?;
            }
        }
        let legacy_type_ids = {
            let mut statement = conn.prepare(
                "SELECT content_type FROM content_classifiers
                 UNION SELECT content_type FROM clips
                 ORDER BY content_type",
            )?;
            let ids = statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>>>()?;
            ids
        };
        for id in legacy_type_ids {
            if crate::content_types::is_structural_clip_type_id(&id) {
                continue;
            }
            conn.execute(
                "INSERT OR IGNORE INTO content_types
                    (id, label, icon, group_name, is_builtin, is_archived)
                 VALUES (?1, ?2, 'FileText', 'custom', 0, 0)",
                params![id, crate::content_types::fallback_label(&id)],
            )?;
        }
        let classifier_migration_applied: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE key = 'contentClassifierRegistryV1')",
            [],
            |row| row.get(0),
        )?;
        if !classifier_migration_applied {
            for (setting_key, stable_ref) in [
                ("detectColors", "color"),
                ("detectLinks", "url"),
                ("detectCode", "code"),
            ] {
                let disabled: bool = conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM settings WHERE key = ?1 AND value = 'false')",
                    params![setting_key],
                    |row| row.get(0),
                )?;
                if disabled {
                    conn.execute(
                        "UPDATE content_classifiers SET enabled = 0 WHERE stable_ref = ?1",
                        params![stable_ref],
                    )?;
                }
            }
            conn.execute(
                "INSERT INTO schema_migrations (key) VALUES ('contentClassifierRegistryV1')",
                [],
            )?;
        }
        Self::init_library_items(&conn)?;
        migrate_canonical_timestamps(&conn)?;
        migrate_analysis_classification_timestamps(&conn)?;

        // Insert default bins if empty
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM bins", [], |r| r.get(0))
            .unwrap_or(0);
        if count == 0 {
            insert_default_bins(&conn)?;
        }

        Ok(())
    }

    fn init_library_items(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "DROP TRIGGER IF EXISTS library_items_extractor_insert;
            DROP TRIGGER IF EXISTS library_items_extractor_update;
            DROP TRIGGER IF EXISTS library_items_extractor_delete;
            DROP TRIGGER IF EXISTS library_items_classifier_insert;
            DROP TRIGGER IF EXISTS library_items_classifier_update;
            DROP TRIGGER IF EXISTS library_items_classifier_delete;
            DROP TRIGGER IF EXISTS library_items_content_type_update;
            DROP TRIGGER IF EXISTS library_items_content_group_update;
            DROP TRIGGER IF EXISTS library_items_operation_insert;
            DROP TRIGGER IF EXISTS library_items_operation_update;
            DROP TRIGGER IF EXISTS library_items_operation_delete;
            DROP TRIGGER IF EXISTS library_items_pipeline_insert;
            DROP TRIGGER IF EXISTS library_items_pipeline_update;
            DROP TRIGGER IF EXISTS library_items_pipeline_delete;
            DROP TRIGGER IF EXISTS library_items_transform_insert;
            DROP TRIGGER IF EXISTS library_items_transform_update;
            DROP TRIGGER IF EXISTS library_items_transform_delete;
            DROP TABLE IF EXISTS library_items;
            CREATE TABLE library_items (
                stable_ref TEXT PRIMARY KEY,
                kind TEXT NOT NULL CHECK (kind IN ('capture', 'inspector', 'extractor', 'classifier', 'suggestion', 'operation', 'transform')),
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                group_label TEXT,
                icon TEXT NOT NULL DEFAULT 'FileText',
                enabled INTEGER CHECK (enabled IS NULL OR enabled IN (0, 1)),
                is_builtin INTEGER NOT NULL DEFAULT 0 CHECK (is_builtin IN (0, 1)),
                is_archived INTEGER NOT NULL DEFAULT 0 CHECK (is_archived IN (0, 1)),
                sort_order INTEGER,
                revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
                input_contract TEXT NOT NULL DEFAULT 'text',
                output_contract TEXT NOT NULL DEFAULT 'preserve_type',
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_library_items_kind_order
                ON library_items(kind, is_archived, sort_order, name);

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            VALUES ('capture:clip-type-v1', 'capture', 'Clip Type',
                    'Assigns exactly one structural Text, Image, or Files type from the captured clipboard representation.',
                    'Capture', 'Shapes', NULL, 1, 0, 0, 1,
                    'clipboard_representation', 'clip_type', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            VALUES ('capture:source-attribution-v1', 'capture', 'Source Attribution',
                    'Records the application associated with a clipboard capture and resolves its icon when shown.',
                    'Capture', 'AppWindow', NULL, 1, 0, 10, 1,
                    'clipboard_event', 'source_attribution', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            VALUES ('inspector:structure-v1', 'inspector', 'Structure',
                    'Measures stable clip structure without retaining clipboard contents.',
                    'Content Analysis', 'ScanSearch', NULL, 1, 0, 0, 1,
                    'clip', 'structural_metadata', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            VALUES ('inspector:file-format-v1', 'inspector', 'File Format',
                    'Identifies referenced file formats from bounded byte signatures.',
                    'Content Analysis', 'FileType2', NULL, 1, 0, 10, 1,
                    'file_references', 'file_formats', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            VALUES ('inspector:media-metadata-v1', 'inspector', 'Media Metadata',
                    'Reads bounded audio and video metadata locally.',
                    'Content Analysis', 'FileAudio', NULL, 1, 0, 20, 1,
                    'file_references', 'media_metadata', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            VALUES ('suggestion:smart-actions-v1', 'suggestion', 'Smart Actions',
                    'Suggests saved Transforms from content-free analysis signals.',
                    'Content Analysis', 'Lightbulb', NULL, 1, 0, 0, 1,
                    'analyzable_text+structural_metadata', 'suggestions',
                    CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            SELECT stable_ref, 'extractor', name, description, 'Content Analysis',
                   'ScanText', enabled, is_builtin, is_deleted, priority, 1,
                   input_contract, output_contract, created_at, updated_at
            FROM content_extractors
            WHERE 1 = 1
            ON CONFLICT(stable_ref) DO UPDATE SET
                name=excluded.name, description=excluded.description,
                enabled=excluded.enabled, is_builtin=excluded.is_builtin,
                is_archived=excluded.is_archived, sort_order=excluded.sort_order,
                input_contract=excluded.input_contract,
                output_contract=excluded.output_contract, updated_at=excluded.updated_at;

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            SELECT classifiers.stable_ref, 'classifier', classifiers.name, classifiers.description,
                   groups.label, types.icon, classifiers.enabled, classifiers.is_builtin,
                   classifiers.is_deleted, classifiers.priority, 1, 'text',
                   'set_type:' || classifiers.content_type, classifiers.created_at, classifiers.updated_at
            FROM content_classifiers AS classifiers
            LEFT JOIN content_types AS types ON types.id = classifiers.content_type
            LEFT JOIN content_type_groups AS groups ON groups.id = types.group_name
            WHERE 1 = 1
            ON CONFLICT(stable_ref) DO UPDATE SET
                name=excluded.name, description=excluded.description,
                group_label=excluded.group_label, icon=excluded.icon,
                enabled=excluded.enabled, is_builtin=excluded.is_builtin,
                is_archived=excluded.is_archived, sort_order=excluded.sort_order,
                output_contract=excluded.output_contract, updated_at=excluded.updated_at;

            INSERT INTO library_items
                (stable_ref, kind, name, group_label, icon, enabled, is_builtin,
                 is_archived, sort_order, revision, input_contract, output_contract,
                 created_at, updated_at)
            SELECT 'custom:' || id, 'operation', name, category, 'Wrench', enabled, 0,
                   0, row_id, 1, 'text', 'preserve_type', created_at, updated_at
            FROM custom_operations
            WHERE 1 = 1
            ON CONFLICT(stable_ref) DO UPDATE SET
                name=excluded.name, group_label=excluded.group_label,
                enabled=excluded.enabled, sort_order=excluded.sort_order,
                updated_at=excluded.updated_at;

            INSERT INTO library_items
                (stable_ref, kind, name, group_label, icon, enabled, is_builtin,
                 is_archived, sort_order, revision, input_contract, output_contract,
                 created_at, updated_at)
            SELECT 'transform:' || id, 'transform', name,
                   CASE authoring_kind WHEN 'manual' THEN 'Local Transforms' ELSE 'Transforms' END,
                   'Workflow', NULL, 0,
                   0, row_id, revision, 'text', 'preserve_type', created_at, updated_at
            FROM saved_transforms
            WHERE 1 = 1
            ON CONFLICT(stable_ref) DO UPDATE SET
                name=excluded.name, sort_order=excluded.sort_order,
                revision=excluded.revision, updated_at=excluded.updated_at;

            DROP TRIGGER IF EXISTS library_items_extractor_insert;
            DROP TRIGGER IF EXISTS library_items_extractor_update;
            DROP TRIGGER IF EXISTS library_items_extractor_delete;
            DROP TRIGGER IF EXISTS library_items_classifier_insert;
            DROP TRIGGER IF EXISTS library_items_classifier_update;
            DROP TRIGGER IF EXISTS library_items_classifier_delete;
            DROP TRIGGER IF EXISTS library_items_content_type_update;
            DROP TRIGGER IF EXISTS library_items_content_group_update;
            DROP TRIGGER IF EXISTS library_items_operation_insert;
            DROP TRIGGER IF EXISTS library_items_operation_update;
            DROP TRIGGER IF EXISTS library_items_operation_delete;
            DROP TRIGGER IF EXISTS library_items_pipeline_insert;
            DROP TRIGGER IF EXISTS library_items_pipeline_update;
            DROP TRIGGER IF EXISTS library_items_pipeline_delete;

            CREATE TRIGGER library_items_extractor_insert AFTER INSERT ON content_extractors BEGIN
              DELETE FROM library_items WHERE stable_ref=NEW.stable_ref;
              INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled, is_builtin,
                 is_archived, sort_order, revision, input_contract, output_contract, created_at, updated_at)
              VALUES (NEW.stable_ref, 'extractor', NEW.name, NEW.description, 'Content Analysis',
                      'ScanText', NEW.enabled, NEW.is_builtin, NEW.is_deleted, NEW.priority,
                      1, NEW.input_contract, NEW.output_contract, NEW.created_at, NEW.updated_at);
            END;
            CREATE TRIGGER library_items_extractor_update AFTER UPDATE ON content_extractors BEGIN
              DELETE FROM library_items WHERE stable_ref=OLD.stable_ref OR stable_ref=NEW.stable_ref;
              INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled, is_builtin,
                 is_archived, sort_order, revision, input_contract, output_contract, created_at, updated_at)
              VALUES (NEW.stable_ref, 'extractor', NEW.name, NEW.description, 'Content Analysis',
                      'ScanText', NEW.enabled, NEW.is_builtin, NEW.is_deleted, NEW.priority,
                      1, NEW.input_contract, NEW.output_contract, NEW.created_at, NEW.updated_at);
            END;
            CREATE TRIGGER library_items_extractor_delete AFTER DELETE ON content_extractors BEGIN
              DELETE FROM library_items WHERE stable_ref=OLD.stable_ref;
            END;
            CREATE TRIGGER library_items_classifier_insert AFTER INSERT ON content_classifiers BEGIN
              DELETE FROM library_items WHERE stable_ref=NEW.stable_ref;
              INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled, is_builtin,
                 is_archived, sort_order, revision, input_contract, output_contract, created_at, updated_at)
              SELECT NEW.stable_ref, 'classifier', NEW.name, NEW.description, groups.label,
                     types.icon, NEW.enabled, NEW.is_builtin, NEW.is_deleted, NEW.priority,
                     1, 'text', 'set_type:' || NEW.content_type, NEW.created_at, NEW.updated_at
              FROM content_types AS types LEFT JOIN content_type_groups AS groups ON groups.id=types.group_name
              WHERE types.id=NEW.content_type;
            END;
            CREATE TRIGGER library_items_classifier_update AFTER UPDATE ON content_classifiers BEGIN
              DELETE FROM library_items WHERE stable_ref=OLD.stable_ref OR stable_ref=NEW.stable_ref;
              INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled, is_builtin,
                 is_archived, sort_order, revision, input_contract, output_contract, created_at, updated_at)
              SELECT NEW.stable_ref, 'classifier', NEW.name, NEW.description, groups.label,
                     types.icon, NEW.enabled, NEW.is_builtin, NEW.is_deleted, NEW.priority,
                     1, 'text', 'set_type:' || NEW.content_type, NEW.created_at, NEW.updated_at
              FROM content_types AS types LEFT JOIN content_type_groups AS groups ON groups.id=types.group_name
              WHERE types.id=NEW.content_type;
            END;
            CREATE TRIGGER library_items_classifier_delete AFTER DELETE ON content_classifiers BEGIN
              DELETE FROM library_items WHERE stable_ref=OLD.stable_ref;
            END;
            CREATE TRIGGER library_items_content_type_update AFTER UPDATE ON content_types BEGIN
              UPDATE library_items SET
                icon=NEW.icon,
                group_label=(SELECT label FROM content_type_groups WHERE id=NEW.group_name),
                output_contract='set_type:'||NEW.id,
                updated_at=CURRENT_TIMESTAMP
              WHERE kind='classifier' AND stable_ref IN (
                SELECT stable_ref FROM content_classifiers WHERE content_type=NEW.id
              );
            END;
            CREATE TRIGGER library_items_content_group_update AFTER UPDATE ON content_type_groups BEGIN
              UPDATE library_items SET group_label=NEW.label,updated_at=CURRENT_TIMESTAMP
              WHERE kind='classifier' AND stable_ref IN (
                SELECT classifiers.stable_ref FROM content_classifiers AS classifiers
                JOIN content_types AS types ON types.id=classifiers.content_type
                WHERE types.group_name=NEW.id
              );
            END;
            CREATE TRIGGER library_items_operation_insert AFTER INSERT ON custom_operations BEGIN
              INSERT OR REPLACE INTO library_items (stable_ref,kind,name,group_label,icon,enabled,is_builtin,is_archived,sort_order,revision,input_contract,output_contract,created_at,updated_at)
              VALUES ('custom:'||NEW.id,'operation',NEW.name,NEW.category,'Wrench',NEW.enabled,0,0,NEW.row_id,1,'text','preserve_type',NEW.created_at,NEW.updated_at);
            END;
            CREATE TRIGGER library_items_operation_update AFTER UPDATE ON custom_operations BEGIN
              UPDATE library_items SET name=NEW.name,group_label=NEW.category,enabled=NEW.enabled,updated_at=NEW.updated_at WHERE stable_ref='custom:'||NEW.id;
            END;
            CREATE TRIGGER library_items_operation_delete AFTER DELETE ON custom_operations BEGIN
              DELETE FROM library_items WHERE stable_ref='custom:'||OLD.id;
            END;
            CREATE TRIGGER library_items_transform_insert AFTER INSERT ON saved_transforms BEGIN
              INSERT OR REPLACE INTO library_items (stable_ref,kind,name,group_label,icon,enabled,is_builtin,is_archived,sort_order,revision,input_contract,output_contract,created_at,updated_at)
              VALUES ('transform:'||NEW.id,'transform',NEW.name,CASE NEW.authoring_kind WHEN 'manual' THEN 'Local Transforms' ELSE 'Transforms' END,'Workflow',NULL,0,0,NEW.row_id,NEW.revision,'text','preserve_type',NEW.created_at,NEW.updated_at);
            END;
            CREATE TRIGGER library_items_transform_update AFTER UPDATE ON saved_transforms BEGIN
              UPDATE library_items SET name=NEW.name,group_label=CASE NEW.authoring_kind WHEN 'manual' THEN 'Local Transforms' ELSE 'Transforms' END,revision=NEW.revision,updated_at=NEW.updated_at WHERE stable_ref='transform:'||NEW.id;
            END;
            CREATE TRIGGER library_items_transform_delete AFTER DELETE ON saved_transforms BEGIN
              DELETE FROM library_items WHERE stable_ref='transform:'||OLD.id;
            END;",
        )?;
        for (index, definition) in crate::operation_registry::BUILTIN_OPERATIONS
            .iter()
            .enumerate()
        {
            conn.execute(
                "INSERT INTO library_items
                    (stable_ref, kind, name, group_label, icon, enabled, is_builtin,
                     is_archived, sort_order, revision, input_contract, output_contract)
                 VALUES (?1, 'operation', ?2, ?3, 'Wrench', 1, 1, 0, ?4, 1, 'text', 'preserve_type')
                 ON CONFLICT(stable_ref) DO UPDATE SET name=excluded.name,
                    group_label=excluded.group_label, sort_order=excluded.sort_order",
                params![
                    format!("builtin:{}", definition.key),
                    definition.name,
                    definition.category_label,
                    index as i64
                ],
            )?;
        }
        Ok(())
    }
    fn init_transformation_tables(&self, conn: &Connection) -> Result<()> {
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
                        created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
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
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
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
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
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
                started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                completed_at DATETIME,
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
        run_named_migrations(
            conn,
            &[
                NamedMigration {
                    key: "appExclusionHotkeysV1",
                    apply: migrate_app_exclusion_hotkey_setting,
                },
                NamedMigration {
                    key: "transformTerminologyV1",
                    apply: migrate_transform_activity_terminology,
                },
                NamedMigration {
                    key: "currentTransformationBackfillV1",
                    apply: backfill_current_transformation,
                },
            ],
        )?;

        Ok(())
    }
}
