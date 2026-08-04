use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::time::Instant;

use crate::db::{DbState, ResolvedCustomOperation};
use crate::filter_engine::apply_filter;
use crate::operation_registry::is_builtin_operation;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionTrigger {
    Manual,
    Shortcut,
    Bin,
    Automation,
    Cli,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionDestination {
    #[default]
    Preview,
    Replace,
    Copy,
    Paste,
    Route,
}

impl ExecutionDestination {
    fn as_str(self) -> &'static str {
        match self {
            Self::Preview => "preview",
            Self::Replace => "replace",
            Self::Copy => "copy",
            Self::Paste => "paste",
            Self::Route => "route",
        }
    }
}

impl ExecutionTrigger {
    fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Shortcut => "shortcut",
            Self::Bin => "bin",
            Self::Automation => "automation",
            Self::Cli => "cli",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ExecutionTarget {
    Operation { operation_ref: String },
    Pipeline { pipeline_ref: String },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionRequest {
    pub input: String,
    pub target: ExecutionTarget,
    pub source_clip_id: Option<i64>,
    pub trigger: ExecutionTrigger,
    #[serde(default)]
    pub destination: ExecutionDestination,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionOutcome {
    pub execution_id: String,
    pub output: String,
}

const LAST_PIPELINE_SETTING: &str = "lastExecutedPipelineRef";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionError {
    pub code: &'static str,
    pub message: String,
    pub step: Option<usize>,
    pub operation_ref: Option<String>,
}

impl ExecutionError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            step: None,
            operation_ref: None,
        }
    }

    fn at_step(mut self, step: usize, operation_ref: &str) -> Self {
        self.step = Some(step);
        self.operation_ref = Some(operation_ref.to_string());
        self
    }

    fn safe_summary(&self) -> String {
        let summary = match (self.step, self.operation_ref.as_deref()) {
            (Some(step), Some(operation_ref)) => format!(
                "{} at pipeline step {} ({}): {}",
                self.code, step, operation_ref, self.message
            ),
            _ => format!("{}: {}", self.code, self.message),
        };
        summary.chars().take(512).collect()
    }
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.safe_summary())
    }
}

fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn database_error(error: impl fmt::Display) -> ExecutionError {
    ExecutionError::new("database_error", error.to_string())
}

fn config_as_text(config_json: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(config_json)
        .ok()
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| value.to_string())
        })
}

fn execute_custom_operation(
    db: &DbState,
    input: &str,
    operation: &ResolvedCustomOperation,
    override_config: Option<&str>,
) -> Result<String, ExecutionError> {
    if !operation.enabled {
        return Err(ExecutionError::new(
            "operation_disabled",
            "Custom operation is disabled",
        ));
    }

    let config = override_config
        .map(str::to_string)
        .or_else(|| config_as_text(&operation.config_json));
    match operation.executor_kind.as_str() {
        "builtin" => {
            let value = serde_json::from_str::<serde_json::Value>(&operation.config_json)
                .map_err(|error| ExecutionError::new("invalid_config", error.to_string()))?;
            let key = value["key"]
                .as_str()
                .ok_or_else(|| ExecutionError::new("invalid_config", "Missing built-in key"))?;
            if !is_builtin_operation(key) {
                return Err(ExecutionError::new(
                    "unknown_operation",
                    format!("Unknown built-in operation: {key}"),
                ));
            }
            let default_config = value.get("legacy_config").and_then(|config| {
                if config.is_null() {
                    None
                } else if let Some(text) = config.as_str() {
                    Some(text.to_string())
                } else {
                    Some(config.to_string())
                }
            });
            apply_filter(input, key, override_config.or(default_config.as_deref()))
                .map_err(|error| ExecutionError::new("operation_failed", error))
        }
        "regex" => apply_filter(input, "regex", config.as_deref())
            .map_err(|error| ExecutionError::new("operation_failed", error)),
        "ai" => {
            if !operation.trusted {
                return Err(ExecutionError::new(
                    "operation_untrusted",
                    "Connected intelligence must be explicitly trusted before it can run",
                ));
            }
            let value = serde_json::from_str::<serde_json::Value>(&operation.config_json)
                .map_err(|error| ExecutionError::new("invalid_config", error.to_string()))?;
            let instructions = value["instructions"].as_str().ok_or_else(|| {
                ExecutionError::new(
                    "invalid_config",
                    "Connected intelligence instructions are missing",
                )
            })?;
            let connection_id = value["connectionId"].as_str();
            crate::intelligence_executor::execute_semantic_operation(
                db,
                input,
                instructions,
                connection_id,
            )
            .map_err(|error| ExecutionError::new(error.code, error.message))
        }
        "cli" | "shell" | "http" => {
            if !operation.trusted {
                Err(ExecutionError::new(
                    "operation_untrusted",
                    "Privileged custom operation must be explicitly trusted before it can run",
                ))
            } else {
                Err(ExecutionError::new(
                    "executor_unavailable",
                    format!(
                        "The {} executor is not available until its sandbox and limits are enabled",
                        operation.executor_kind
                    ),
                ))
            }
        }
        kind => Err(ExecutionError::new(
            "unknown_executor",
            format!("Unknown executor kind: {kind}"),
        )),
    }
}

fn execute_operation_ref(
    db: &DbState,
    input: &str,
    operation_ref: &str,
    override_config: Option<&str>,
) -> Result<String, ExecutionError> {
    if let Some(key) = operation_ref.strip_prefix("builtin:") {
        if !is_builtin_operation(key) {
            return Err(ExecutionError::new(
                "unknown_operation",
                format!("Unknown built-in operation: {key}"),
            ));
        }
        return apply_filter(input, key, override_config)
            .map_err(|error| ExecutionError::new("operation_failed", error));
    }

    let operation = db
        .resolve_custom_operation(operation_ref)
        .map_err(database_error)?
        .ok_or_else(|| {
            ExecutionError::new(
                "unknown_operation",
                format!("Unknown operation reference: {operation_ref}"),
            )
        })?;
    execute_custom_operation(db, input, &operation, override_config)
}

pub(crate) fn execute_operation_inline(
    db: &DbState,
    input: &str,
    operation_ref: &str,
    config_json: Option<&str>,
) -> Result<String, ExecutionError> {
    execute_operation_ref(db, input, operation_ref, config_json)
}

fn execute_pipeline_ref(
    db: &DbState,
    input: &str,
    pipeline_ref: &str,
) -> Result<(String, i64), ExecutionError> {
    let pipeline = db
        .resolve_pipeline(pipeline_ref)
        .map_err(database_error)?
        .ok_or_else(|| {
            ExecutionError::new(
                "unknown_pipeline",
                format!("Unknown pipeline reference: {pipeline_ref}"),
            )
        })?;
    let mut current = input.to_string();
    for step in &pipeline.steps {
        match execute_operation_ref(
            db,
            &current,
            &step.operation_ref,
            step.config_json.as_deref(),
        ) {
            Ok(output) => current = output,
            Err(_error) if step.failure_policy == "skip" => continue,
            Err(error) => {
                return Err(error.at_step((step.position + 1) as usize, &step.operation_ref))
            }
        }
    }
    Ok((current, pipeline.revision))
}

pub fn execute(
    db: &DbState,
    request: ExecutionRequest,
) -> Result<ExecutionOutcome, ExecutionError> {
    let started = Instant::now();
    let (target_kind, target_ref) = match &request.target {
        ExecutionTarget::Operation { operation_ref } => ("operation", operation_ref.clone()),
        ExecutionTarget::Pipeline { pipeline_ref } => ("pipeline", pipeline_ref.clone()),
    };

    // Resolve the revision before opening the execution record, but perform the
    // actual work through the same operation path in both direct and pipeline runs.
    let target_revision = match &request.target {
        ExecutionTarget::Pipeline { pipeline_ref } => db
            .resolve_pipeline(pipeline_ref)
            .map_err(database_error)?
            .map(|pipeline| pipeline.revision),
        ExecutionTarget::Operation { .. } => None,
    };
    let execution_id = db
        .begin_transformation_execution(
            target_kind,
            &target_ref,
            target_revision,
            request.source_clip_id,
            request.trigger.as_str(),
            request.destination.as_str(),
            &content_hash(&request.input),
        )
        .map_err(database_error)?;
    db.start_transformation_execution(&execution_id)
        .map_err(database_error)?;

    let result = match &request.target {
        ExecutionTarget::Operation { operation_ref } => {
            execute_operation_ref(db, &request.input, operation_ref, None)
        }
        ExecutionTarget::Pipeline { pipeline_ref } => {
            execute_pipeline_ref(db, &request.input, pipeline_ref).map(|(output, _)| output)
        }
    };
    let duration_ms = started.elapsed().as_millis().min(i64::MAX as u128) as i64;

    match result {
        Ok(output) => {
            db.finish_transformation_execution(
                &execution_id,
                duration_ms,
                Some(&content_hash(&output)),
                None,
            )
            .map_err(database_error)?;
            if target_kind == "pipeline" {
                db.save_setting(LAST_PIPELINE_SETTING, &target_ref)
                    .map_err(database_error)?;
            }
            Ok(ExecutionOutcome {
                execution_id,
                output,
            })
        }
        Err(error) => {
            let summary = error.safe_summary();
            db.finish_transformation_execution(&execution_id, duration_ms, None, Some(&summary))
                .map_err(database_error)?;
            Err(error)
        }
    }
}

pub fn get_last_pipeline_ref(db: &DbState) -> Result<Option<String>, ExecutionError> {
    db.get_setting(LAST_PIPELINE_SETTING)
        .map_err(database_error)
}

pub fn execute_last_pipeline(
    db: &DbState,
    input: String,
    source_clip_id: Option<i64>,
    trigger: ExecutionTrigger,
) -> Result<ExecutionOutcome, ExecutionError> {
    let pipeline_ref = get_last_pipeline_ref(db)?.ok_or_else(|| {
        ExecutionError::new(
            "no_last_pipeline",
            "No Pipeline has completed successfully yet",
        )
    })?;
    let result = execute(
        db,
        ExecutionRequest {
            input,
            target: ExecutionTarget::Pipeline {
                pipeline_ref: pipeline_ref.clone(),
            },
            source_clip_id,
            trigger,
            destination: ExecutionDestination::Preview,
        },
    );
    if matches!(&result, Err(error) if error.code == "unknown_pipeline") {
        db.delete_setting(LAST_PIPELINE_SETTING)
            .map_err(database_error)?;
    }
    result
}

pub fn execute_shortcut_pipeline(
    db: &DbState,
    input: String,
    pipeline_ref: Option<&str>,
) -> Result<ExecutionOutcome, ExecutionError> {
    match pipeline_ref {
        Some(pipeline_ref) => execute(
            db,
            ExecutionRequest {
                input,
                target: ExecutionTarget::Pipeline {
                    pipeline_ref: pipeline_ref.to_string(),
                },
                source_clip_id: None,
                trigger: ExecutionTrigger::Shortcut,
                destination: ExecutionDestination::Paste,
            },
        ),
        None => execute_last_pipeline(db, input, None, ExecutionTrigger::Shortcut),
    }
}

pub fn execute_legacy_preview(
    input: &str,
    filter_type: &str,
    config: Option<&str>,
) -> Result<String, String> {
    if matches!(
        filter_type,
        "shell_script" | "cli" | "shell" | "http" | "ai"
    ) {
        return Err(
            "Privileged operations must be saved, enabled, and trusted before execution"
                .to_string(),
        );
    }
    if filter_type == "pipeline" {
        let steps = serde_json::from_str::<Vec<serde_json::Value>>(config.unwrap_or("[]"))
            .map_err(|error| format!("Invalid pipeline configuration: {error}"))?;
        if steps.iter().any(|step| {
            matches!(
                step["filter_type"].as_str(),
                Some("shell_script" | "cli" | "shell" | "http" | "ai")
            )
        }) {
            return Err(
                "Privileged pipeline steps must resolve through saved, trusted operations"
                    .to_string(),
            );
        }
    }
    apply_filter(input, filter_type, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
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
        }
    }

    fn pipeline(db: &DbState, name: &str, operation_refs: &[&str]) -> String {
        let conn = db.conn.lock();
        conn.execute("INSERT INTO pipelines (name) VALUES (?1)", params![name])
            .unwrap();
        let pipeline_id: String = conn
            .query_row(
                "SELECT id FROM pipelines WHERE row_id = last_insert_rowid()",
                [],
                |row| row.get(0),
            )
            .unwrap();
        for (position, operation_ref) in operation_refs.iter().enumerate() {
            conn.execute(
                "INSERT INTO pipeline_steps (pipeline_id, position, operation_ref)
                 VALUES (?1, ?2, ?3)",
                params![pipeline_id, position as i64, operation_ref],
            )
            .unwrap();
        }
        format!("pipeline:{pipeline_id}")
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

        let pipeline_id = {
            let conn = db.conn.lock();
            conn.execute("INSERT INTO pipelines (name) VALUES ('Loud Quote')", [])
                .unwrap();
            let pipeline_id: String = conn
                .query_row(
                    "SELECT id FROM pipelines WHERE row_id = last_insert_rowid()",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            conn.execute(
                "INSERT INTO pipeline_steps (pipeline_id, position, operation_ref)
                 VALUES (?1, 0, 'builtin:uppercase'), (?1, 1, 'builtin:quote_text')",
                params![pipeline_id],
            )
            .unwrap();
            pipeline_id
        };
        let pipeline = execute(
            &db,
            request(
                ExecutionTarget::Pipeline {
                    pipeline_ref: format!("pipeline:{pipeline_id}"),
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
    fn pipeline_errors_identify_the_step_and_operation() {
        let db = test_db();
        let pipeline_id = {
            let conn = db.conn.lock();
            conn.execute("INSERT INTO pipelines (name) VALUES ('Broken')", [])
                .unwrap();
            let pipeline_id: String = conn
                .query_row(
                    "SELECT id FROM pipelines WHERE row_id = last_insert_rowid()",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            conn.execute(
                "INSERT INTO pipeline_steps (pipeline_id, position, operation_ref)
                 VALUES (?1, 0, 'builtin:uppercase'), (?1, 1, 'builtin:missing')",
                params![pipeline_id],
            )
            .unwrap();
            pipeline_id
        };

        let error = execute(
            &db,
            request(
                ExecutionTarget::Pipeline {
                    pipeline_ref: format!("pipeline:{pipeline_id}"),
                },
                "hello",
            ),
        )
        .unwrap_err();
        assert_eq!(error.code, "unknown_operation");
        assert_eq!(error.step, Some(2));
        assert_eq!(error.operation_ref.as_deref(), Some("builtin:missing"));
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
        assert_eq!(get_last_pipeline_ref(&db).unwrap(), None);

        let successful = pipeline(&db, "Successful", &["builtin:uppercase"]);
        execute_shortcut_pipeline(&db, "hello".to_string(), Some(&successful)).unwrap();
        assert_eq!(
            get_last_pipeline_ref(&db).unwrap().as_deref(),
            Some(successful.as_str())
        );

        let failing = pipeline(&db, "Failing", &["builtin:missing"]);
        let error =
            execute_shortcut_pipeline(&db, "hello".to_string(), Some(&failing)).unwrap_err();
        assert_eq!(error.code, "unknown_operation");
        assert_eq!(
            get_last_pipeline_ref(&db).unwrap().as_deref(),
            Some(successful.as_str())
        );
    }

    #[test]
    fn missing_and_deleted_last_pipeline_are_explicit() {
        let db = test_db();
        let missing = execute_shortcut_pipeline(&db, "hello".to_string(), None).unwrap_err();
        assert_eq!(missing.code, "no_last_pipeline");

        let pipeline_ref = pipeline(&db, "Temporary", &["builtin:uppercase"]);
        execute_shortcut_pipeline(&db, "hello".to_string(), Some(&pipeline_ref)).unwrap();
        db.delete_pipeline(&pipeline_ref).unwrap();

        let deleted = execute_shortcut_pipeline(&db, "hello".to_string(), None).unwrap_err();
        assert_eq!(deleted.code, "unknown_pipeline");
        assert_eq!(get_last_pipeline_ref(&db).unwrap(), None);
        let cleared = execute_shortcut_pipeline(&db, "hello".to_string(), None).unwrap_err();
        assert_eq!(cleared.code, "no_last_pipeline");
    }

    #[test]
    fn shortcut_helper_pastes_named_or_last_pipeline_with_same_result() {
        let db = test_db();
        let pipeline_ref = pipeline(&db, "Normalize", &["builtin:trim", "builtin:uppercase"]);
        let named =
            execute_shortcut_pipeline(&db, "  hello  ".to_string(), Some(&pipeline_ref)).unwrap();
        let last = execute_shortcut_pipeline(&db, "  hello  ".to_string(), None).unwrap();
        assert_eq!(named.output, "HELLO");
        assert_eq!(last.output, named.output);

        let conn = db.conn.lock();
        let shortcut_runs: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM transformation_executions
                 WHERE trigger_kind = 'shortcut' AND target_ref = ?1 AND status = 'succeeded'",
                params![pipeline_ref],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(shortcut_runs, 2);
    }
}
