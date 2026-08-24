use std::sync::atomic::AtomicBool;

use crate::db::{DbState, ResolvedCustomOperation};
use crate::filter_engine::apply_filter;
use crate::operation_registry::is_builtin_operation;

use super::cancellation::ensure_not_cancelled;
use super::contracts::{database_error, ExecutionError};

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

pub(crate) fn execute_operation_ref(
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
