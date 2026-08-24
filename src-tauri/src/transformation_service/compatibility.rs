use crate::db::DbState;
use crate::filter_engine::apply_filter;

use super::contracts::{
    database_error, ExecutionDestination, ExecutionError, ExecutionOutcome, ExecutionRequest,
    ExecutionTarget, ExecutionTrigger,
};
use super::orchestration::{execute, LAST_TRANSFORM_SETTING};

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
