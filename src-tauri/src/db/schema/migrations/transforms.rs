use super::super::*;

pub(crate) fn migrate_transform_activity_terminology(conn: &Connection) -> Result<()> {
    conn.execute(
        "UPDATE activity_logs
         SET event_type = replace(event_type, 'recipe_', 'transform_'),
             description = replace(replace(description, 'Recipes', 'Transforms'), 'Recipe', 'Transform')
         WHERE event_type LIKE '%recipe%' OR description LIKE '%Recipe%'",
        [],
    )?;
    Ok(())
}

pub(crate) fn backfill_current_transformation(conn: &Connection) -> Result<()> {
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

pub(crate) fn migrate_pipelines_to_saved_transforms(conn: &Connection) -> Result<()> {
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
