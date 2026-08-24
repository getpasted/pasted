use rusqlite::{params, Connection, OptionalExtension, Result};

use super::super::DbState;
use super::repository::saved_transform_by_id;
use super::{Pipeline, PipelineStep, PipelineStepInput, SavedTransform};

impl DbState {
    pub fn get_pipelines(&self) -> Result<Vec<Pipeline>> {
        let conn = self.conn.lock();
        let refs = {
            let mut statement = conn.prepare(
                "SELECT id FROM saved_transforms
                 WHERE authoring_kind = 'manual' ORDER BY row_id ASC",
            )?;
            let refs = statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>>>()?;
            refs
        };
        refs.into_iter()
            .map(|stable_id| {
                saved_transform_by_id(&conn, &stable_id)
                    .and_then(Self::manual_transform_as_pipeline)
            })
            .collect()
    }

    fn manual_transform_as_pipeline(transform: SavedTransform) -> Result<Pipeline> {
        if transform.authoring_kind != "manual" {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let steps = transform
            .plan
            .steps
            .iter()
            .enumerate()
            .map(|(position, step)| match &step.executor {
                crate::transformation_intent::PlannedExecutor::Deterministic {
                    operation_ref,
                    config_json,
                } => Ok(PipelineStep {
                    position: position as i64,
                    operation_ref: operation_ref.clone(),
                    config_json: config_json.clone(),
                    failure_policy: match step.failure_policy {
                        crate::transformation_intent::StepFailurePolicy::Stop => "stop",
                        crate::transformation_intent::StepFailurePolicy::Skip => "skip",
                    }
                    .to_string(),
                }),
                crate::transformation_intent::PlannedExecutor::Semantic { .. } => {
                    Err(rusqlite::Error::InvalidParameterName(
                        "Manual Transform contains a semantic step".to_string(),
                    ))
                }
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Pipeline {
            id: transform.id,
            stable_ref: transform.stable_ref,
            name: transform.name,
            shortcut: transform.shortcut,
            revision: transform.revision,
            created_at: transform.created_at,
            updated_at: transform.updated_at,
            steps,
        })
    }

    pub(in crate::db) fn validate_pipeline_steps(
        conn: &Connection,
        steps: &[PipelineStepInput],
    ) -> Result<()> {
        if steps.is_empty() {
            return Err(rusqlite::Error::InvalidParameterName(
                "pipeline requires at least one operation".to_string(),
            ));
        }
        for step in steps {
            if !matches!(step.failure_policy.as_str(), "stop" | "skip") {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "invalid failure policy: {}",
                    step.failure_policy
                )));
            }
            if let Some(config) = &step.config_json {
                serde_json::from_str::<serde_json::Value>(config).map_err(|error| {
                    rusqlite::Error::InvalidParameterName(format!(
                        "invalid step config JSON: {error}"
                    ))
                })?;
            }
            if let Some(key) = step.operation_ref.strip_prefix("builtin:") {
                if !crate::operation_registry::is_builtin_operation(key) {
                    return Err(rusqlite::Error::InvalidParameterName(format!(
                        "unknown operation reference: {}",
                        step.operation_ref
                    )));
                }
            } else if let Some(custom_id) = step.operation_ref.strip_prefix("custom:") {
                let exists: bool = conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM custom_operations WHERE id = ?1)",
                    params![custom_id],
                    |row| row.get(0),
                )?;
                if !exists {
                    return Err(rusqlite::Error::InvalidParameterName(format!(
                        "unknown operation reference: {}",
                        step.operation_ref
                    )));
                }
            } else {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "invalid operation reference: {}",
                    step.operation_ref
                )));
            }
        }
        Ok(())
    }

    pub(in crate::db) fn manual_transform_plan(
        name: &str,
        steps: &[PipelineStepInput],
    ) -> Result<crate::transformation_intent::TransformationPlan> {
        let plan = crate::transformation_intent::TransformationPlan {
            schema_version: crate::transformation_intent::TRANSFORMATION_PLAN_SCHEMA_VERSION,
            intent: format!("Run {}", name.trim()),
            summary: name.trim().to_string(),
            planning_mode: crate::transformation_intent::IntentPlanningMode::Pinned,
            steps: steps
                .iter()
                .map(
                    |step| crate::transformation_intent::PlannedTransformationStep {
                        name: step
                            .operation_ref
                            .strip_prefix("builtin:")
                            .or_else(|| step.operation_ref.strip_prefix("custom:"))
                            .unwrap_or(&step.operation_ref)
                            .replace('_', " "),
                        rationale: "Manually configured Operation".to_string(),
                        scope: crate::transformation_intent::StepExecutionScope::WholeInput,
                        failure_policy: if step.failure_policy == "skip" {
                            crate::transformation_intent::StepFailurePolicy::Skip
                        } else {
                            crate::transformation_intent::StepFailurePolicy::Stop
                        },
                        executor: crate::transformation_intent::PlannedExecutor::Deterministic {
                            operation_ref: step.operation_ref.clone(),
                            config_json: step.config_json.clone(),
                        },
                    },
                )
                .collect(),
        };
        plan.validate()
            .map_err(rusqlite::Error::InvalidParameterName)?;
        Ok(plan)
    }

    pub fn create_pipeline(
        &self,
        name: &str,
        steps: &[PipelineStepInput],
        hotkey: Option<&str>,
    ) -> Result<Pipeline> {
        let conn = self.conn.lock();
        Self::validate_pipeline_steps(&conn, steps)?;
        let plan = Self::manual_transform_plan(name, steps)?;
        let plan_json = serde_json::to_string(&plan).map_err(|error| {
            rusqlite::Error::InvalidParameterName(format!("invalid Transform: {error}"))
        })?;
        conn.execute(
            "INSERT INTO saved_transforms
                (name, plan_json, connection_id, shortcut, authoring_kind,
                 created_at, updated_at)
             VALUES (?1, ?2, NULL, ?3, 'manual',
                     strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                     strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
            params![name.trim(), plan_json, hotkey],
        )?;
        let stable_id: String = conn.query_row(
            "SELECT id FROM saved_transforms WHERE row_id = last_insert_rowid()",
            [],
            |row| row.get(0),
        )?;
        let pipeline =
            Self::manual_transform_as_pipeline(saved_transform_by_id(&conn, &stable_id)?)?;
        drop(conn);
        let _ = self.log_activity(
            "transform_saved",
            &format!("Created Transform \"{}\"", pipeline.name),
        );
        Ok(pipeline)
    }

    pub fn update_pipeline(
        &self,
        pipeline_ref: &str,
        name: &str,
        steps: &[PipelineStepInput],
        hotkey: Option<&str>,
    ) -> Result<Pipeline> {
        let transform_id = pipeline_ref
            .strip_prefix("transform:")
            .or_else(|| pipeline_ref.strip_prefix("pipeline:"))
            .unwrap_or(pipeline_ref);
        let conn = self.conn.lock();
        Self::validate_pipeline_steps(&conn, steps)?;
        let plan_json =
            serde_json::to_string(&Self::manual_transform_plan(name, steps)?).map_err(|error| {
                rusqlite::Error::InvalidParameterName(format!("invalid Transform: {error}"))
            })?;
        let changed = conn.execute(
            "UPDATE saved_transforms
             SET name = ?1, plan_json = ?2, shortcut = ?3, revision = revision + 1,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
             WHERE id = ?4 AND authoring_kind = 'manual'",
            params![name.trim(), plan_json, hotkey, transform_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let pipeline =
            Self::manual_transform_as_pipeline(saved_transform_by_id(&conn, transform_id)?)?;
        drop(conn);
        let _ = self.log_activity(
            "transform_updated",
            &format!("Updated Transform \"{}\"", pipeline.name),
        );
        Ok(pipeline)
    }

    pub fn update_pipeline_hotkey(&self, pipeline_ref: &str, hotkey: Option<&str>) -> Result<()> {
        let pipeline_id = pipeline_ref
            .strip_prefix("transform:")
            .or_else(|| pipeline_ref.strip_prefix("pipeline:"))
            .unwrap_or(pipeline_ref);
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE saved_transforms
             SET shortcut = ?1, revision = revision + 1,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
             WHERE id = ?2 AND authoring_kind = 'manual'",
            params![hotkey, pipeline_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        drop(conn);
        let _ = self.log_activity(
            "transform_updated",
            &format!("Updated Transform transform:{pipeline_id}"),
        );
        Ok(())
    }

    pub fn delete_pipeline(&self, pipeline_ref: &str) -> Result<()> {
        let pipeline_id = pipeline_ref
            .strip_prefix("transform:")
            .or_else(|| pipeline_ref.strip_prefix("pipeline:"))
            .unwrap_or(pipeline_ref);
        let conn = self.conn.lock();
        let name = conn
            .query_row(
                "SELECT name FROM saved_transforms WHERE id = ?1 AND authoring_kind = 'manual'",
                params![pipeline_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let changed = conn.execute(
            "DELETE FROM saved_transforms WHERE id = ?1 AND authoring_kind = 'manual'",
            params![pipeline_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        drop(conn);
        let _ = self.log_activity(
            "transform_deleted",
            &format!(
                "Deleted Transform \"{}\"",
                name.unwrap_or_else(|| pipeline_id.to_string())
            ),
        );
        Ok(())
    }
}
