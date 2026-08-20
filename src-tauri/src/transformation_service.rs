use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};
use std::time::Instant;

use crate::db::{DbState, ResolvedCustomOperation, TransformationExecutionStart};
use crate::filter_engine::apply_filter;
use crate::manual_transform_service::ManualTransformStepInput;
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
    Transform {
        transform_ref: String,
    },
    Operation {
        operation_ref: String,
    },
    #[serde(alias = "pipeline")]
    ManualTransform {
        #[serde(alias = "pipelineRef")]
        transform_ref: String,
    },
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
    #[serde(default)]
    pub client_request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionOutcome {
    pub execution_id: String,
    pub output: String,
    pub connection_id: Option<String>,
    pub connection_name: Option<String>,
    pub duration_ms: i64,
}

const LAST_TRANSFORM_SETTING: &str = "lastExecutedTransformRef";

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
                "{} at manual Transform step {} ({}): {}",
                self.code, step, operation_ref, self.message
            ),
            _ => format!("{}: {}", self.code, self.message),
        };
        summary.chars().take(512).collect()
    }
}

static EXECUTION_CANCELLATIONS: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();

fn cancellation_registry() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    EXECUTION_CANCELLATIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub struct CancellationRegistration {
    request_id: String,
    flag: Arc<AtomicBool>,
}

impl CancellationRegistration {
    pub fn register(request_id: String) -> Self {
        let flag = Arc::new(AtomicBool::new(false));
        cancellation_registry()
            .lock()
            .expect("transformation cancellation registry poisoned")
            .insert(request_id.clone(), Arc::clone(&flag));
        Self { request_id, flag }
    }

    pub fn flag(&self) -> &AtomicBool {
        self.flag.as_ref()
    }
}

impl Drop for CancellationRegistration {
    fn drop(&mut self) {
        let mut registry = cancellation_registry()
            .lock()
            .expect("transformation cancellation registry poisoned");
        if registry
            .get(&self.request_id)
            .is_some_and(|flag| Arc::ptr_eq(flag, &self.flag))
        {
            registry.remove(&self.request_id);
        }
    }
}

pub fn cancel_execution(client_request_id: &str) -> bool {
    let flag = cancellation_registry()
        .lock()
        .expect("transformation cancellation registry poisoned")
        .get(client_request_id)
        .cloned();
    if let Some(flag) = flag {
        flag.store(true, Ordering::Release);
        true
    } else {
        false
    }
}

fn ensure_not_cancelled(cancellation: Option<&AtomicBool>) -> Result<(), ExecutionError> {
    if cancellation.is_some_and(|flag| flag.load(Ordering::Acquire)) {
        Err(ExecutionError::new(
            "execution_cancelled",
            "Transform was cancelled",
        ))
    } else {
        Ok(())
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
    client_request_id: Option<&str>,
    cancellation: Option<&AtomicBool>,
) -> Result<String, ExecutionError> {
    ensure_not_cancelled(cancellation)?;
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
                client_request_id,
                cancellation,
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
    client_request_id: Option<&str>,
    cancellation: Option<&AtomicBool>,
) -> Result<String, ExecutionError> {
    ensure_not_cancelled(cancellation)?;
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
    execute_custom_operation(
        db,
        input,
        &operation,
        override_config,
        client_request_id,
        cancellation,
    )
}

pub(crate) fn execute_operation_inline(
    db: &DbState,
    input: &str,
    operation_ref: &str,
    config_json: Option<&str>,
) -> Result<String, ExecutionError> {
    execute_operation_ref(db, input, operation_ref, config_json, None, None)
}

/// Preview unsaved manual Transform steps through the same Operation executor
/// used by persisted Transforms. This keeps the editor honest without creating
/// a temporary database record or updating last-used Transform state.
pub fn preview_manual_transform_steps(
    db: &DbState,
    input: &str,
    steps: &[ManualTransformStepInput],
    client_request_id: Option<&str>,
    cancellation: Option<&AtomicBool>,
) -> Result<String, ExecutionError> {
    ensure_transform_text_size(input)?;
    if steps.is_empty() {
        return Err(ExecutionError::new(
            "empty_pipeline",
            "A manually built Transform requires at least one Operation",
        ));
    }

    let mut current = input.to_string();
    for (position, step) in steps.iter().enumerate() {
        ensure_not_cancelled(cancellation)?;
        if !matches!(step.failure_policy.as_str(), "stop" | "skip") {
            return Err(ExecutionError::new(
                "invalid_failure_policy",
                format!("Unknown failure policy: {}", step.failure_policy),
            )
            .at_step(position + 1, &step.operation_ref));
        }
        match execute_operation_ref(
            db,
            &current,
            &step.operation_ref,
            step.config_json.as_deref(),
            client_request_id,
            cancellation,
        ) {
            Ok(output) => {
                ensure_transform_text_size(&output)?;
                current = output;
            }
            Err(_error) if step.failure_policy == "skip" => continue,
            Err(error) => return Err(error.at_step(position + 1, &step.operation_ref)),
        }
    }
    Ok(current)
}

pub fn execute(
    db: &DbState,
    request: ExecutionRequest,
) -> Result<ExecutionOutcome, ExecutionError> {
    execute_with_cancellation(db, request, None)
}

pub fn execute_with_cancellation(
    db: &DbState,
    request: ExecutionRequest,
    cancellation: Option<&AtomicBool>,
) -> Result<ExecutionOutcome, ExecutionError> {
    let mut request = request;
    request.target = match request.target {
        ExecutionTarget::ManualTransform { transform_ref } => ExecutionTarget::Transform {
            transform_ref: format!(
                "transform:{}",
                transform_ref
                    .strip_prefix("pipeline:")
                    .or_else(|| transform_ref.strip_prefix("transform:"))
                    .unwrap_or(&transform_ref)
            ),
        },
        ExecutionTarget::Transform { transform_ref } if transform_ref.starts_with("pipeline:") => {
            ExecutionTarget::Transform {
                transform_ref: format!(
                    "transform:{}",
                    transform_ref.trim_start_matches("pipeline:")
                ),
            }
        }
        target => target,
    };
    ensure_transform_text_size(&request.input)?;
    if let Some(clip_id) = request.source_clip_id {
        let clip = db.get_clip_by_id(clip_id).map_err(database_error)?;
        if clip.content_type == "file" {
            return Err(ExecutionError::new(
                "unsupported_clip_type",
                "File clips must be converted with an explicit File Operation before using text Transforms",
            ));
        }
    }
    if let ExecutionTarget::Transform { transform_ref } = &request.target {
        let transform = db
            .resolve_saved_transform(transform_ref)
            .map_err(database_error)?
            .ok_or_else(|| ExecutionError::new("unknown_transform", "Transform was not found"))?;
        let remember_as_last = transform.authoring_kind == "manual";
        let result = crate::intelligence_executor::execute_saved_transform(
            db,
            transform_ref,
            request.input,
            crate::intelligence_executor::SavedTransformExecutionContext {
                source_clip_id: request.source_clip_id,
                trigger_kind: request.trigger.as_str(),
                destination_kind: request.destination.as_str(),
                client_request_id: request.client_request_id.as_deref(),
            },
            cancellation,
        );
        return match result {
            Ok((transform_name, execution_id, outcome)) => {
                if remember_as_last {
                    db.save_setting(LAST_TRANSFORM_SETTING, transform_ref)
                        .map_err(database_error)?;
                }
                let _ = db.log_activity(
                    "transform_executed",
                    &format!(
                        "Ran Transform: {} in {} ms",
                        transform_name, outcome.duration_ms
                    ),
                );
                Ok(ExecutionOutcome {
                    execution_id,
                    output: outcome.output,
                    connection_id: outcome.connection_id,
                    connection_name: outcome.connection_name,
                    duration_ms: outcome.duration_ms,
                })
            }
            Err(error) => {
                if error.code == "execution_cancelled" {
                    let _ = db.log_activity(
                        "transform_execution_cancelled",
                        &format!("Cancelled Transform: {transform_ref}"),
                    );
                } else {
                    let _ = db.log_activity(
                        "transform_execution_failed",
                        &format!("Transform failed: {} ({})", transform_ref, error.code),
                    );
                }
                Err(ExecutionError::new(error.code, error.message))
            }
        };
    }

    let started = Instant::now();
    let (target_kind, target_ref) = match &request.target {
        ExecutionTarget::Transform { .. } => unreachable!("Transforms return above"),
        ExecutionTarget::Operation { operation_ref } => ("operation", operation_ref.clone()),
        ExecutionTarget::ManualTransform { .. } => {
            unreachable!("Manual Transform targets normalize before execution")
        }
    };

    // Resolve the revision before opening the execution record, but perform the
    // actual work through the same operation path in direct and manual Transform runs.
    let target_revision = match &request.target {
        ExecutionTarget::Transform { .. } => unreachable!("Transforms return above"),
        ExecutionTarget::ManualTransform { .. } => {
            unreachable!("Manual Transform targets normalize before execution")
        }
        ExecutionTarget::Operation { .. } => None,
    };
    let execution_id = db
        .begin_transformation_execution(TransformationExecutionStart {
            target_kind,
            target_ref: &target_ref,
            target_revision,
            source_clip_id: request.source_clip_id,
            trigger_kind: request.trigger.as_str(),
            destination_kind: request.destination.as_str(),
            input_hash: &content_hash(&request.input),
        })
        .map_err(database_error)?;
    db.start_transformation_execution(&execution_id)
        .map_err(database_error)?;

    let result = match &request.target {
        ExecutionTarget::Transform { .. } => unreachable!("Transforms return above"),
        ExecutionTarget::Operation { operation_ref } => execute_operation_ref(
            db,
            &request.input,
            operation_ref,
            None,
            request.client_request_id.as_deref(),
            cancellation,
        ),
        ExecutionTarget::ManualTransform { .. } => {
            unreachable!("Manual Transform targets normalize before execution")
        }
    }
    .and_then(|output| {
        ensure_not_cancelled(cancellation)?;
        ensure_transform_text_size(&output)?;
        Ok(output)
    });
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
            let _ = db.log_activity(
                "transformation_execution_succeeded",
                &format!("Ran {target_kind} {target_ref} in {duration_ms} ms"),
            );
            Ok(ExecutionOutcome {
                execution_id,
                output,
                connection_id: None,
                connection_name: None,
                duration_ms,
            })
        }
        Err(error) => {
            let summary = error.safe_summary();
            if error.code == "execution_cancelled" {
                db.cancel_transformation_execution(&execution_id, duration_ms)
                    .map_err(database_error)?;
                let _ = db.log_activity(
                    "transformation_execution_cancelled",
                    &format!("Cancelled {target_kind} {target_ref}"),
                );
            } else {
                db.finish_transformation_execution(
                    &execution_id,
                    duration_ms,
                    None,
                    Some(&summary),
                )
                .map_err(database_error)?;
                let _ = db.log_activity(
                    "transformation_execution_failed",
                    &format!("Failed {target_kind} {target_ref} ({})", error.code),
                );
            }
            Err(error)
        }
    }
}

fn ensure_transform_text_size(value: &str) -> Result<(), ExecutionError> {
    if value.len() <= crate::resource_limits::MAX_TRANSFORM_TEXT_BYTES {
        Ok(())
    } else {
        Err(ExecutionError::new(
            "transform_text_too_large",
            "Transform input or output exceeds Pasted's 8 MB safety limit",
        ))
    }
}

pub fn get_last_manual_transform_ref(db: &DbState) -> Result<Option<String>, ExecutionError> {
    db.get_setting(LAST_TRANSFORM_SETTING)
        .map_err(database_error)
}

pub fn execute_last_manual_transform(
    db: &DbState,
    input: String,
    source_clip_id: Option<i64>,
    trigger: ExecutionTrigger,
) -> Result<ExecutionOutcome, ExecutionError> {
    let manual_transform_ref = get_last_manual_transform_ref(db)?.ok_or_else(|| {
        ExecutionError::new(
            "no_last_pipeline",
            "No manually built Transform has completed successfully yet",
        )
    })?;
    let result = execute(
        db,
        ExecutionRequest {
            input,
            target: ExecutionTarget::Transform {
                transform_ref: manual_transform_ref.clone(),
            },
            source_clip_id,
            trigger,
            destination: ExecutionDestination::Preview,
            client_request_id: None,
        },
    );
    if matches!(&result, Err(error) if error.code == "unknown_transform") {
        db.delete_setting(LAST_TRANSFORM_SETTING)
            .map_err(database_error)?;
    }
    result
}

pub fn execute_shortcut_manual_transform(
    db: &DbState,
    input: String,
    manual_transform_ref: Option<&str>,
) -> Result<ExecutionOutcome, ExecutionError> {
    match manual_transform_ref {
        Some(manual_transform_ref) => execute(
            db,
            ExecutionRequest {
                input,
                target: ExecutionTarget::Transform {
                    transform_ref: manual_transform_ref.to_string(),
                },
                source_clip_id: None,
                trigger: ExecutionTrigger::Shortcut,
                destination: ExecutionDestination::Paste,
                client_request_id: None,
            },
        ),
        None => execute_last_manual_transform(db, input, None, ExecutionTrigger::Shortcut),
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
            client_request_id: None,
        }
    }

    fn pipeline(db: &DbState, name: &str, operation_refs: &[&str]) -> String {
        db.create_pipeline(
            name,
            &operation_refs
                .iter()
                .map(|operation_ref| ManualTransformStepInput {
                    operation_ref: (*operation_ref).to_string(),
                    config_json: None,
                    failure_policy: "stop".to_string(),
                })
                .collect::<Vec<_>>(),
            None,
        )
        .unwrap()
        .stable_ref
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

        let manual_transform_ref = pipeline(
            &db,
            "Loud Quote",
            &["builtin:uppercase", "builtin:quote_text"],
        );
        let pipeline = execute(
            &db,
            request(
                ExecutionTarget::ManualTransform {
                    transform_ref: manual_transform_ref,
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
    fn unsaved_pipeline_preview_uses_the_canonical_operation_executor() {
        let db = test_db();
        let steps = vec![
            ManualTransformStepInput {
                operation_ref: "builtin:uppercase".to_string(),
                config_json: None,
                failure_policy: "stop".to_string(),
            },
            ManualTransformStepInput {
                operation_ref: "builtin:quote_text".to_string(),
                config_json: None,
                failure_policy: "stop".to_string(),
            },
        ];

        let output =
            preview_manual_transform_steps(&db, "hello\nworld", &steps, None, None).unwrap();
        assert_eq!(output, "> HELLO\n> WORLD");

        let error = preview_manual_transform_steps(
            &db,
            "hello",
            &[ManualTransformStepInput {
                operation_ref: "builtin:not_real".to_string(),
                config_json: None,
                failure_policy: "stop".to_string(),
            }],
            None,
            None,
        )
        .unwrap_err();
        assert_eq!(error.code, "unknown_operation");
        assert_eq!(error.step, Some(1));
        assert_eq!(error.operation_ref.as_deref(), Some("builtin:not_real"));
    }

    #[test]
    fn file_clips_cannot_be_mistaken_for_serialized_text_transforms() {
        let db = test_db();
        let paths = serde_json::json!(["/tmp/first.txt", "/tmp/second.txt"]).to_string();
        let clip = db
            .save_clip("file", Some(&paths), None, None, "file_clip", "Finder")
            .unwrap();
        let mut execution = request(
            ExecutionTarget::Operation {
                operation_ref: "builtin:uppercase".to_string(),
            },
            &paths,
        );
        execution.source_clip_id = Some(clip.id);
        let error = execute(&db, execution).unwrap_err();
        assert_eq!(error.code, "unsupported_clip_type");
        assert_eq!(
            db.get_clip_by_id(clip.id).unwrap().text_content,
            Some(paths)
        );
    }

    #[test]
    fn saved_transforms_use_the_same_execution_contract_and_ledger() {
        let db = test_db();
        let clip = db
            .save_clip(
                "text",
                Some("hello"),
                None,
                None,
                "unified-transform",
                "Test",
            )
            .unwrap();
        let plan = crate::transformation_intent::TransformationPlan {
            schema_version: crate::transformation_intent::TRANSFORMATION_PLAN_SCHEMA_VERSION,
            intent: "Uppercase".to_string(),
            summary: "Uppercase".to_string(),
            planning_mode: crate::transformation_intent::IntentPlanningMode::Pinned,
            steps: vec![crate::transformation_intent::PlannedTransformationStep {
                name: "Uppercase".to_string(),
                rationale: "Replayable".to_string(),
                scope: crate::transformation_intent::StepExecutionScope::WholeInput,
                failure_policy: Default::default(),
                executor: crate::transformation_intent::PlannedExecutor::Deterministic {
                    operation_ref: "builtin:uppercase".to_string(),
                    config_json: None,
                },
            }],
        };
        let transform = db.create_saved_transform("Uppercase", &plan, None).unwrap();

        let outcome = execute(
            &db,
            ExecutionRequest {
                input: "hello".to_string(),
                target: ExecutionTarget::Transform {
                    transform_ref: transform.stable_ref.clone(),
                },
                source_clip_id: Some(clip.id),
                trigger: ExecutionTrigger::Manual,
                destination: ExecutionDestination::Preview,
                client_request_id: None,
            },
        )
        .unwrap();

        assert_eq!(outcome.output, "HELLO");
        assert_eq!(outcome.connection_id, None);
        assert!(!outcome.execution_id.is_empty());
        let executions = db.get_clip_transformation_executions(clip.id).unwrap();
        assert_eq!(executions.len(), 1);
        assert_eq!(executions[0].id, outcome.execution_id);
        assert_eq!(executions[0].target_kind, "transform");
        assert_eq!(executions[0].status, "succeeded");
    }

    #[test]
    fn transform_target_accepts_pipeline_compatibility_references() {
        let db = test_db();
        let pipeline = db
            .create_pipeline(
                "Uppercase Locally",
                &[ManualTransformStepInput {
                    operation_ref: "builtin:uppercase".to_string(),
                    config_json: None,
                    failure_policy: "stop".to_string(),
                }],
                None,
            )
            .unwrap();

        let outcome = execute(
            &db,
            request(
                ExecutionTarget::Transform {
                    transform_ref: pipeline.stable_ref.clone(),
                },
                "hello",
            ),
        )
        .unwrap();
        assert_eq!(outcome.output, "HELLO");

        let conn = db.conn.lock();
        let stored: (String, String) = conn
            .query_row(
                "SELECT target_kind, target_ref FROM transformation_executions WHERE id = ?1",
                params![outcome.execution_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(stored, ("transform".to_string(), pipeline.stable_ref));
    }

    #[test]
    fn pipeline_errors_identify_the_step_and_operation() {
        let db = test_db();
        let manual_transform_ref = pipeline(&db, "Broken", &["builtin:uppercase", "builtin:trim"]);
        {
            let conn = db.conn.lock();
            conn.execute(
                "UPDATE saved_transforms
                 SET plan_json = replace(plan_json, 'builtin:trim', 'builtin:missing')
                 WHERE id = ?1",
                params![manual_transform_ref.trim_start_matches("transform:")],
            )
            .unwrap();
        }

        let error = execute(
            &db,
            request(
                ExecutionTarget::ManualTransform {
                    transform_ref: manual_transform_ref,
                },
                "hello",
            ),
        )
        .unwrap_err();
        assert_eq!(error.code, "invalid_plan");
        assert!(error.message.contains("step 2"));
        assert!(error.message.contains("builtin:missing") || error.message.contains("missing"));
    }

    #[test]
    fn cancelled_execution_is_recorded_and_does_not_produce_output() {
        let db = test_db();
        let cancellation = AtomicBool::new(true);
        let error = execute_with_cancellation(
            &db,
            request(
                ExecutionTarget::Operation {
                    operation_ref: "builtin:uppercase".to_string(),
                },
                "hello",
            ),
            Some(&cancellation),
        )
        .unwrap_err();

        assert_eq!(error.code, "execution_cancelled");
        let conn = db.conn.lock();
        let (status, output_hash): (String, Option<String>) = conn
            .query_row(
                "SELECT status, output_hash FROM transformation_executions LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, "cancelled");
        assert_eq!(output_hash, None);
    }

    #[test]
    fn cancellation_registration_targets_only_the_current_request() {
        let first = CancellationRegistration::register("same-request".to_string());
        let second = CancellationRegistration::register("same-request".to_string());
        drop(first);

        assert!(cancel_execution("same-request"));
        assert!(second.flag().load(Ordering::Acquire));
        drop(second);
        assert!(!cancel_execution("same-request"));
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
        assert_eq!(get_last_manual_transform_ref(&db).unwrap(), None);

        let successful = pipeline(&db, "Successful", &["builtin:uppercase"]);
        execute_shortcut_manual_transform(&db, "hello".to_string(), Some(&successful)).unwrap();
        assert_eq!(
            get_last_manual_transform_ref(&db).unwrap().as_deref(),
            Some(successful.as_str())
        );

        let failing = pipeline(&db, "Failing", &["builtin:trim"]);
        {
            let conn = db.conn.lock();
            conn.execute(
                "UPDATE saved_transforms
                 SET plan_json = replace(plan_json, 'builtin:trim', 'builtin:missing')
                 WHERE id = ?1",
                params![failing.trim_start_matches("transform:")],
            )
            .unwrap();
        }
        let error = execute_shortcut_manual_transform(&db, "hello".to_string(), Some(&failing))
            .unwrap_err();
        assert_eq!(error.code, "invalid_plan");
        assert_eq!(
            get_last_manual_transform_ref(&db).unwrap().as_deref(),
            Some(successful.as_str())
        );
    }

    #[test]
    fn missing_and_deleted_last_pipeline_are_explicit() {
        let db = test_db();
        let missing =
            execute_shortcut_manual_transform(&db, "hello".to_string(), None).unwrap_err();
        assert_eq!(missing.code, "no_last_pipeline");

        let manual_transform_ref = pipeline(&db, "Temporary", &["builtin:uppercase"]);
        execute_shortcut_manual_transform(&db, "hello".to_string(), Some(&manual_transform_ref))
            .unwrap();
        db.delete_pipeline(&manual_transform_ref).unwrap();

        let deleted =
            execute_shortcut_manual_transform(&db, "hello".to_string(), None).unwrap_err();
        assert_eq!(deleted.code, "unknown_transform");
        assert_eq!(get_last_manual_transform_ref(&db).unwrap(), None);
        let cleared =
            execute_shortcut_manual_transform(&db, "hello".to_string(), None).unwrap_err();
        assert_eq!(cleared.code, "no_last_pipeline");
    }

    #[test]
    fn shortcut_helper_pastes_named_or_last_pipeline_with_same_result() {
        let db = test_db();
        let manual_transform_ref =
            pipeline(&db, "Normalize", &["builtin:trim", "builtin:uppercase"]);
        let named = execute_shortcut_manual_transform(
            &db,
            "  hello  ".to_string(),
            Some(&manual_transform_ref),
        )
        .unwrap();
        let last = execute_shortcut_manual_transform(&db, "  hello  ".to_string(), None).unwrap();
        assert_eq!(named.output, "HELLO");
        assert_eq!(last.output, named.output);

        let conn = db.conn.lock();
        let shortcut_runs: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM transformation_executions
                 WHERE trigger_kind = 'shortcut' AND target_ref = ?1 AND status = 'succeeded'",
                params![manual_transform_ref],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(shortcut_runs, 2);
    }
}
