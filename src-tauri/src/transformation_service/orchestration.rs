use sha2::{Digest, Sha256};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use crate::db::{DbState, TransformationExecutionStart};
use crate::manual_transform_service::ManualTransformStepInput;

use super::cancellation::ensure_not_cancelled;
use super::contracts::{
    database_error, ExecutionError, ExecutionOutcome, ExecutionRequest, ExecutionTarget,
};
use super::operations::execute_operation_ref;

pub(crate) const LAST_TRANSFORM_SETTING: &str = "lastExecutedTransformRef";

fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    crate::hashing::finalize_sha256_hex(hasher)
}

pub(crate) fn ensure_transform_text_size(value: &str) -> Result<(), ExecutionError> {
    if value.len() <= crate::resource_limits::MAX_TRANSFORM_TEXT_BYTES {
        Ok(())
    } else {
        Err(ExecutionError::new(
            "transform_text_too_large",
            "Transform input or output exceeds Pasted's 8 MB safety limit",
        ))
    }
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
        let transform_ref = transform_ref.clone();
        return execute_saved_transform(db, request, &transform_ref, cancellation);
    }

    execute_direct_operation(db, request, cancellation)
}

fn execute_saved_transform(
    db: &DbState,
    request: ExecutionRequest,
    transform_ref: &str,
    cancellation: Option<&AtomicBool>,
) -> Result<ExecutionOutcome, ExecutionError> {
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
    match result {
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
    }
}

fn execute_direct_operation(
    db: &DbState,
    request: ExecutionRequest,
    cancellation: Option<&AtomicBool>,
) -> Result<ExecutionOutcome, ExecutionError> {
    let started = Instant::now();
    let ExecutionTarget::Operation { operation_ref } = &request.target else {
        unreachable!("Transform targets return before direct Operation execution")
    };
    let target_ref = operation_ref.clone();
    let execution_id = db
        .begin_transformation_execution(TransformationExecutionStart {
            target_kind: "operation",
            target_ref: &target_ref,
            target_revision: None,
            source_clip_id: request.source_clip_id,
            trigger_kind: request.trigger.as_str(),
            destination_kind: request.destination.as_str(),
            input_hash: &content_hash(&request.input),
        })
        .map_err(database_error)?;
    db.start_transformation_execution(&execution_id)
        .map_err(database_error)?;

    let result = execute_operation_ref(
        db,
        &request.input,
        operation_ref,
        None,
        request.client_request_id.as_deref(),
        cancellation,
    )
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
                &format!("Ran operation {target_ref} in {duration_ms} ms"),
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
                    &format!("Cancelled operation {target_ref}"),
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
                    &format!("Failed operation {target_ref} ({})", error.code),
                );
            }
            Err(error)
        }
    }
}
