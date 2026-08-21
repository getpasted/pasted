use rusqlite::{params, Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};

use super::{
    ensure_resource_size, ClipRevisionContext, ClipRevisionOrganization, DbState,
    ResolvedCustomOperation,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Pipeline {
    pub id: i64,
    pub stable_ref: String,
    pub name: String,
    #[serde(rename = "hotkey", alias = "shortcut")]
    pub shortcut: Option<String>,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
    pub steps: Vec<PipelineStep>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SavedTransform {
    pub id: i64,
    pub stable_ref: String,
    pub name: String,
    pub plan: crate::transformation_intent::TransformationPlan,
    pub connection_id: Option<String>,
    #[serde(default)]
    #[serde(rename = "hotkey", alias = "shortcut")]
    pub shortcut: Option<String>,
    #[serde(default = "default_transform_authoring_kind")]
    pub authoring_kind: String,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
}

fn default_transform_authoring_kind() -> String {
    "intent".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransformAuthoringKind {
    Intent,
    Manual,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransformDefinition {
    pub id: i64,
    pub stable_ref: String,
    pub name: String,
    pub authoring_kind: TransformAuthoringKind,
    pub execution_character: String,
    pub connection_id: Option<String>,
    #[serde(rename = "hotkey", alias = "shortcut")]
    pub shortcut: Option<String>,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
    pub plan: Option<crate::transformation_intent::TransformationPlan>,
    pub steps: Vec<PipelineStep>,
}

impl From<SavedTransform> for TransformDefinition {
    fn from(transform: SavedTransform) -> Self {
        let execution_character = match transform.plan.execution_character() {
            crate::transformation_intent::ExecutionCharacter::Replayable => "replayable",
            crate::transformation_intent::ExecutionCharacter::Interpretive => "interpretive",
            crate::transformation_intent::ExecutionCharacter::Mixed => "mixed",
        }
        .to_string();
        let is_manual = transform.authoring_kind == "manual";
        let manual_steps = if is_manual {
            transform
                .plan
                .steps
                .iter()
                .enumerate()
                .filter_map(|(position, step)| match &step.executor {
                    crate::transformation_intent::PlannedExecutor::Deterministic {
                        operation_ref,
                        config_json,
                    } => Some(PipelineStep {
                        position: position as i64,
                        operation_ref: operation_ref.clone(),
                        config_json: config_json.clone(),
                        failure_policy: match step.failure_policy {
                            crate::transformation_intent::StepFailurePolicy::Stop => "stop",
                            crate::transformation_intent::StepFailurePolicy::Skip => "skip",
                        }
                        .to_string(),
                    }),
                    crate::transformation_intent::PlannedExecutor::Semantic { .. } => None,
                })
                .collect()
        } else {
            Vec::new()
        };
        Self {
            id: transform.id,
            stable_ref: transform.stable_ref,
            name: transform.name,
            authoring_kind: if is_manual {
                TransformAuthoringKind::Manual
            } else {
                TransformAuthoringKind::Intent
            },
            execution_character,
            connection_id: transform.connection_id,
            shortcut: transform.shortcut,
            revision: transform.revision,
            created_at: transform.created_at,
            updated_at: transform.updated_at,
            plan: (!is_manual).then_some(transform.plan),
            steps: manual_steps,
        }
    }
}

impl From<Pipeline> for TransformDefinition {
    fn from(pipeline: Pipeline) -> Self {
        Self {
            id: pipeline.id,
            stable_ref: pipeline.stable_ref,
            name: pipeline.name,
            authoring_kind: TransformAuthoringKind::Manual,
            execution_character: "replayable".to_string(),
            connection_id: None,
            shortcut: pipeline.shortcut,
            revision: pipeline.revision,
            created_at: pipeline.created_at,
            updated_at: pipeline.updated_at,
            plan: None,
            steps: pipeline.steps,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClipTransformationProvenance {
    pub transform_ref: String,
    pub transform_name: String,
    pub transform_revision: i64,
    pub connection_id: Option<String>,
    pub duration_ms: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransformationExecution {
    pub id: String,
    pub target_kind: String,
    pub target_ref: String,
    pub target_revision: Option<i64>,
    pub source_clip_id: Option<i64>,
    pub trigger_kind: String,
    pub destination_kind: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub status: String,
    pub error_summary: Option<String>,
}

pub struct TransformationExecutionStart<'a> {
    pub target_kind: &'a str,
    pub target_ref: &'a str,
    pub target_revision: Option<i64>,
    pub source_clip_id: Option<i64>,
    pub trigger_kind: &'a str,
    pub destination_kind: &'a str,
    pub input_hash: &'a str,
}

pub struct TransformClipApplication<'a> {
    pub clip_id: i64,
    pub transform_ref: &'a str,
    pub expected_input: &'a str,
    pub output: &'a str,
    pub connection_id: Option<&'a str>,
    pub duration_ms: i64,
    pub bin_move: Option<(Option<i64>, i64)>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStepInput {
    pub operation_ref: String,
    pub config_json: Option<String>,
    #[serde(default = "default_pipeline_failure_policy")]
    pub failure_policy: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStep {
    pub position: i64,
    pub operation_ref: String,
    pub config_json: Option<String>,
    pub failure_policy: String,
}

fn default_pipeline_failure_policy() -> String {
    "stop".to_string()
}

impl DbState {
    fn canonical_executor_kind(operation_type: &str) -> &str {
        match operation_type {
            "shell_script" => "shell",
            "regex" | "cli" | "shell" | "http" | "ai" => operation_type,
            _ => "cli",
        }
    }

    pub(super) fn operation_storage_fields(
        op_type: &str,
        config: Option<&str>,
    ) -> (String, String) {
        if crate::operation_registry::is_builtin_operation(op_type) {
            (
                "builtin".to_string(),
                serde_json::json!({
                    "key": op_type,
                    "legacy_config": config.map(|value| Self::normalize_json_config(Some(value))),
                })
                .to_string(),
            )
        } else {
            (
                Self::canonical_executor_kind(op_type).to_string(),
                Self::normalize_json_config(config),
            )
        }
    }

    pub(super) fn legacy_operation_fields(
        executor_kind: &str,
        config_json: &str,
    ) -> (String, Option<String>) {
        if executor_kind == "builtin" {
            let value = serde_json::from_str::<serde_json::Value>(config_json).unwrap_or_default();
            let operation_type = value["key"].as_str().unwrap_or("unknown").to_string();
            let config = value.get("legacy_config").and_then(|config| {
                if config.is_null() {
                    None
                } else if let Some(text) = config.as_str() {
                    Some(text.to_string())
                } else {
                    Some(config.to_string())
                }
            });
            (operation_type, config)
        } else {
            let operation_type = if executor_kind == "shell" {
                "shell_script"
            } else {
                executor_kind
            };
            let value = serde_json::from_str::<serde_json::Value>(config_json).ok();
            let config = value.map(|value| {
                value
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| value.to_string())
            });
            (operation_type.to_string(), config)
        }
    }

    pub fn resolve_custom_operation(
        &self,
        operation_ref: &str,
    ) -> Result<Option<ResolvedCustomOperation>> {
        let Some(operation_id) = operation_ref.strip_prefix("custom:") else {
            return Ok(None);
        };
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT executor_kind, config_json, enabled, trusted
             FROM custom_operations WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![operation_id])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        Ok(Some(ResolvedCustomOperation {
            executor_kind: row.get(0)?,
            config_json: row.get(1)?,
            enabled: row.get::<_, i64>(2)? != 0,
            trusted: row.get::<_, i64>(3)? != 0,
        }))
    }

    pub fn begin_transformation_execution(
        &self,
        request: TransformationExecutionStart<'_>,
    ) -> Result<String> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO transformation_executions
                (target_kind, target_ref, target_revision, source_clip_id,
                 trigger_kind, destination_kind, input_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                request.target_kind,
                request.target_ref,
                request.target_revision,
                request.source_clip_id,
                request.trigger_kind,
                request.destination_kind,
                request.input_hash
            ],
        )?;
        conn.query_row(
            "SELECT id FROM transformation_executions WHERE rowid = last_insert_rowid()",
            [],
            |row| row.get(0),
        )
    }

    pub fn finish_transformation_execution(
        &self,
        execution_id: &str,
        duration_ms: i64,
        output_hash: Option<&str>,
        error_summary: Option<&str>,
    ) -> Result<()> {
        let status = if error_summary.is_some() {
            "failed"
        } else {
            "succeeded"
        };
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE transformation_executions
             SET duration_ms = ?1, status = ?2, output_hash = ?3, error_summary = ?4,
                 completed_at = CURRENT_TIMESTAMP
             WHERE id = ?5",
            params![
                duration_ms,
                status,
                output_hash,
                error_summary,
                execution_id
            ],
        )?;
        Ok(())
    }

    pub fn cancel_transformation_execution(
        &self,
        execution_id: &str,
        duration_ms: i64,
    ) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE transformation_executions
             SET duration_ms = ?1, status = 'cancelled', output_hash = NULL,
                 error_summary = NULL, completed_at = CURRENT_TIMESTAMP
             WHERE id = ?2",
            params![duration_ms, execution_id],
        )?;
        Ok(())
    }

    pub fn start_transformation_execution(&self, execution_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE transformation_executions SET status = 'running'
             WHERE id = ?1 AND status = 'queued'",
            params![execution_id],
        )?;
        Ok(())
    }

    pub fn get_clip_transformation_executions(
        &self,
        clip_id: i64,
    ) -> Result<Vec<TransformationExecution>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, target_kind, target_ref, target_revision, source_clip_id,
                    trigger_kind, destination_kind, started_at, completed_at,
                    duration_ms, status, error_summary
             FROM transformation_executions
             WHERE source_clip_id = ?1
             ORDER BY started_at DESC, rowid DESC
             LIMIT 25",
        )?;
        let rows = statement.query_map(params![clip_id], |row| {
            Ok(TransformationExecution {
                id: row.get(0)?,
                target_kind: row.get(1)?,
                target_ref: row.get(2)?,
                target_revision: row.get(3)?,
                source_clip_id: row.get(4)?,
                trigger_kind: row.get(5)?,
                destination_kind: row.get(6)?,
                started_at: row.get(7)?,
                completed_at: row.get(8)?,
                duration_ms: row.get(9)?,
                status: row.get(10)?,
                error_summary: row.get(11)?,
            })
        })?;
        rows.collect()
    }

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
                Self::saved_transform_by_id(&conn, &stable_id)
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

    fn saved_transform_by_id(conn: &Connection, transform_id: &str) -> Result<SavedTransform> {
        conn.query_row(
            "SELECT row_id, id, name, plan_json, connection_id, shortcut, authoring_kind, revision, created_at, updated_at
             FROM saved_transforms WHERE id = ?1",
            params![transform_id],
            |row| {
                let stable_id: String = row.get(1)?;
                let plan_json: String = row.get(3)?;
                let plan = serde_json::from_str(&plan_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
                Ok(SavedTransform {
                    id: row.get(0)?,
                    stable_ref: format!("transform:{stable_id}"),
                    name: row.get(2)?,
                    plan,
                    connection_id: row.get(4)?,
                    shortcut: row.get(5)?,
                    authoring_kind: row.get(6)?,
                    revision: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )
    }

    pub fn get_saved_transforms(&self) -> Result<Vec<SavedTransform>> {
        let conn = self.conn.lock();
        let ids = {
            let mut statement = conn
                .prepare("SELECT id FROM saved_transforms ORDER BY updated_at DESC, row_id DESC")?;
            let ids = statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>>>()?;
            ids
        };
        ids.into_iter()
            .map(|id| Self::saved_transform_by_id(&conn, &id))
            .collect()
    }

    pub fn get_intent_transforms(&self) -> Result<Vec<SavedTransform>> {
        Ok(self
            .get_saved_transforms()?
            .into_iter()
            .filter(|transform| transform.authoring_kind == "intent")
            .collect())
    }

    pub fn get_transform_definitions(&self) -> Result<Vec<TransformDefinition>> {
        let mut definitions = self
            .get_saved_transforms()?
            .into_iter()
            .map(TransformDefinition::from)
            .collect::<Vec<_>>();
        definitions.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.name.cmp(&right.name))
        });
        Ok(definitions)
    }

    pub fn resolve_transform_definition(
        &self,
        transform_ref: &str,
    ) -> Result<Option<TransformDefinition>> {
        if transform_ref.starts_with("pipeline:") {
            return self
                .resolve_saved_transform(transform_ref.trim_start_matches("pipeline:"))
                .map(|transform| transform.map(TransformDefinition::from));
        }
        self.resolve_saved_transform(transform_ref)
            .map(|transform| transform.map(TransformDefinition::from))
    }

    pub fn duplicate_transform_definition(
        &self,
        transform_ref: &str,
        name: Option<&str>,
    ) -> Result<TransformDefinition> {
        let definition = self
            .resolve_transform_definition(transform_ref)?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let duplicate_name = name
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("{} Copy", definition.name));
        match definition.authoring_kind {
            TransformAuthoringKind::Intent => {
                let plan = definition.plan.ok_or_else(|| {
                    rusqlite::Error::InvalidParameterName(
                        "Saved Transform has no execution plan".to_string(),
                    )
                })?;
                self.create_saved_transform(
                    &duplicate_name,
                    &plan,
                    definition.connection_id.as_deref(),
                )
                .map(TransformDefinition::from)
            }
            TransformAuthoringKind::Manual => {
                let steps = definition
                    .steps
                    .into_iter()
                    .map(|step| PipelineStepInput {
                        operation_ref: step.operation_ref,
                        config_json: step.config_json,
                        failure_policy: step.failure_policy,
                    })
                    .collect::<Vec<_>>();
                self.create_pipeline(&duplicate_name, &steps, None)
                    .map(TransformDefinition::from)
            }
        }
    }

    pub fn delete_transform_definition(&self, transform_ref: &str) -> Result<()> {
        if transform_ref.starts_with("pipeline:") {
            self.delete_pipeline(transform_ref)
        } else {
            self.delete_saved_transform(transform_ref)
        }
    }

    pub fn resolve_saved_transform(&self, transform_ref: &str) -> Result<Option<SavedTransform>> {
        let transform_id = transform_ref
            .strip_prefix("transform:")
            .unwrap_or(transform_ref);
        let conn = self.conn.lock();
        match Self::saved_transform_by_id(&conn, transform_id) {
            Ok(transform) => Ok(Some(transform)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn create_saved_transform(
        &self,
        name: &str,
        plan: &crate::transformation_intent::TransformationPlan,
        connection_id: Option<&str>,
    ) -> Result<SavedTransform> {
        plan.validate()
            .map_err(rusqlite::Error::InvalidParameterName)?;
        let plan_json = serde_json::to_string(plan).map_err(|error| {
            rusqlite::Error::InvalidParameterName(format!("invalid Transform: {error}"))
        })?;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO saved_transforms (name, plan_json, connection_id, authoring_kind)
             VALUES (?1, ?2, ?3, 'intent')",
            params![name.trim(), plan_json, connection_id],
        )?;
        let row_id = conn.last_insert_rowid();
        let stable_id: String = conn.query_row(
            "SELECT id FROM saved_transforms WHERE row_id = ?1",
            params![row_id],
            |row| row.get(0),
        )?;
        Self::saved_transform_by_id(&conn, &stable_id)
    }

    pub fn update_saved_transform(
        &self,
        transform_ref: &str,
        name: &str,
        plan: &crate::transformation_intent::TransformationPlan,
        connection_id: Option<&str>,
    ) -> Result<SavedTransform> {
        plan.validate()
            .map_err(rusqlite::Error::InvalidParameterName)?;
        let plan_json = serde_json::to_string(plan).map_err(|error| {
            rusqlite::Error::InvalidParameterName(format!("invalid Transform: {error}"))
        })?;
        let transform_id = transform_ref
            .strip_prefix("transform:")
            .unwrap_or(transform_ref);
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE saved_transforms
             SET name = ?1,
                 plan_json = ?2,
                 connection_id = ?3,
                 revision = revision + 1,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?4",
            params![name.trim(), plan_json, connection_id, transform_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Self::saved_transform_by_id(&conn, transform_id)
    }

    pub fn delete_saved_transform(&self, transform_ref: &str) -> Result<()> {
        let transform_id = transform_ref
            .strip_prefix("transform:")
            .unwrap_or(transform_ref);
        let conn = self.conn.lock();
        let changed = conn.execute(
            "DELETE FROM saved_transforms WHERE id = ?1",
            params![transform_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    pub fn apply_transform_output_to_clip(
        &self,
        request: TransformClipApplication<'_>,
    ) -> Result<ClipTransformationProvenance> {
        let TransformClipApplication {
            clip_id,
            transform_ref,
            expected_input,
            output,
            connection_id,
            duration_ms,
            bin_move,
        } = request;
        ensure_resource_size(
            expected_input,
            crate::resource_limits::MAX_TRANSFORM_TEXT_BYTES,
            "Transform input",
        )?;
        ensure_resource_size(
            output,
            crate::resource_limits::MAX_TRANSFORM_TEXT_BYTES,
            "Transform output",
        )?;
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let transform_id = transform_ref
            .strip_prefix("transform:")
            .or_else(|| transform_ref.strip_prefix("pipeline:"))
            .unwrap_or(transform_ref)
            .to_string();
        let (transform_name, transform_revision): (String, i64) = tx.query_row(
            "SELECT name, revision FROM saved_transforms WHERE id = ?1",
            params![transform_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let canonical_transform_ref = format!("transform:{transform_id}");
        let transform_id = Some(transform_id);
        let (current_text, is_trashed, current_transformation_id): (
            Option<String>,
            i32,
            Option<String>,
        ) = tx.query_row(
            "SELECT text_content, COALESCE(is_trashed, 0), current_transformation_id FROM clips WHERE id = ?1",
            params![clip_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        if is_trashed != 0 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Restore this clip before transforming it".to_string(),
            ));
        }
        if current_text.as_deref() != Some(expected_input) {
            return Err(rusqlite::Error::InvalidParameterName(
                "The clip changed after this preview was generated; preview it again".to_string(),
            ));
        }
        if expected_input == output {
            return Err(rusqlite::Error::InvalidParameterName(
                "The Transform did not change the clip".to_string(),
            ));
        }
        let (action_label, organization) =
            if let Some((previous_bin_id, destination_bin_id)) = bin_move {
                let destination_name = tx
                    .query_row(
                        "SELECT name FROM bins WHERE id = ?1",
                        params![destination_bin_id],
                        |row| row.get::<_, String>(0),
                    )
                    .unwrap_or_else(|_| format!("Bin #{destination_bin_id}"));
                (
                    format!("Moved to {destination_name} · Applied {transform_name}"),
                    Some(ClipRevisionOrganization {
                        category_bin_id: previous_bin_id,
                    }),
                )
            } else {
                (format!("Applied {transform_name}"), None)
            };
        if Self::revision_history_enabled_internal(&tx) {
            let context_json = serde_json::to_string(&ClipRevisionContext {
                schema_version: 1,
                action_kind: if organization.is_some() {
                    "transform_bin_drop".to_string()
                } else {
                    "transform".to_string()
                },
                action_label,
                organization,
                current_transformation_id,
            })
            .map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))?;
            tx.execute(
                "INSERT INTO clip_versions (clip_id, text_content, context_json) VALUES (?1, ?2, ?3)",
                params![clip_id, expected_input, context_json],
            )?;
            Self::prune_clip_versions_internal(&tx, clip_id)?;
        }
        let transformation_id: String =
            tx.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))?;
        tx.execute(
            "INSERT INTO clip_transformations
                (id, clip_id, transform_id, transform_ref, transform_name, transform_revision, connection_id, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                transformation_id,
                clip_id,
                transform_id,
                canonical_transform_ref,
                transform_name,
                transform_revision,
                connection_id,
                duration_ms.max(0)
            ],
        )?;
        tx.execute(
            "UPDATE clips SET text_content = ?1, current_transformation_id = ?2 WHERE id = ?3",
            params![output, transformation_id, clip_id],
        )?;
        let created_at: String = tx.query_row(
            "SELECT created_at FROM clip_transformations WHERE rowid = last_insert_rowid()",
            [],
            |row| row.get(0),
        )?;
        tx.commit()?;
        Ok(ClipTransformationProvenance {
            transform_ref: canonical_transform_ref,
            transform_name,
            transform_revision,
            connection_id: connection_id.map(str::to_string),
            duration_ms: duration_ms.max(0),
            created_at,
        })
    }

    pub fn get_clip_transformation_provenance(
        &self,
        clip_id: i64,
    ) -> Result<Option<ClipTransformationProvenance>> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT transformation.transform_ref, transformation.transform_id,
                    transformation.transform_name,
                    transformation.transform_revision, transformation.connection_id,
                    transformation.duration_ms, transformation.created_at
             FROM clips
             JOIN clip_transformations transformation
               ON transformation.id = clips.current_transformation_id
             WHERE clips.id = ?1",
            params![clip_id],
            |row| {
                let transform_ref: Option<String> = row.get(0)?;
                let transform_id: Option<String> = row.get(1)?;
                Ok(ClipTransformationProvenance {
                    transform_ref: transform_ref
                        .or_else(|| transform_id.map(|id| format!("transform:{id}")))
                        .unwrap_or_else(|| "transform:deleted".to_string()),
                    transform_name: row.get(2)?,
                    transform_revision: row.get(3)?,
                    connection_id: row.get(4)?,
                    duration_ms: row.get(5)?,
                    created_at: row.get(6)?,
                })
            },
        );
        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub(super) fn validate_pipeline_steps(
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

    pub(super) fn manual_transform_plan(
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
                (name, plan_json, connection_id, shortcut, authoring_kind)
             VALUES (?1, ?2, NULL, ?3, 'manual')",
            params![name.trim(), plan_json, hotkey],
        )?;
        let stable_id: String = conn.query_row(
            "SELECT id FROM saved_transforms WHERE row_id = last_insert_rowid()",
            [],
            |row| row.get(0),
        )?;
        let pipeline =
            Self::manual_transform_as_pipeline(Self::saved_transform_by_id(&conn, &stable_id)?)?;
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
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?4 AND authoring_kind = 'manual'",
            params![name.trim(), plan_json, hotkey, transform_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let pipeline =
            Self::manual_transform_as_pipeline(Self::saved_transform_by_id(&conn, transform_id)?)?;
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
             SET shortcut = ?1, revision = revision + 1, updated_at = CURRENT_TIMESTAMP
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
