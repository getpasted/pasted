use super::super::*;
use super::common::{argument_value, read_stdin_bounded};

pub(crate) fn run_operation(args: &[String], db: &DbState) {
    let Some(target_ref) = args.get(3) else {
        eprintln!("Usage: pasted operation run <ref> [--text TEXT | --clip ID | --stdin] [--json]");
        std::process::exit(2);
    };
    let clip_id = args
        .iter()
        .position(|argument| argument == "--clip")
        .and_then(|index| args.get(index + 1))
        .and_then(|value| value.parse::<i64>().ok());
    let explicit_text = args
        .iter()
        .position(|argument| argument == "--text")
        .and_then(|index| args.get(index + 1))
        .cloned();
    let input = if let Some(text) = explicit_text {
        text
    } else if let Some(clip_id) = clip_id {
        db.get_active_clip_text(clip_id)
            .unwrap_or_else(|error| {
                eprintln!("Could not read clip #{clip_id}: {error}");
                std::process::exit(1);
            })
            .unwrap_or_else(|| {
                eprintln!("Clip #{clip_id} has no transformable text.");
                std::process::exit(2);
            })
    } else {
        read_stdin_bounded(pasted_lib::resource_limits::MAX_TRANSFORM_TEXT_BYTES).unwrap_or_else(
            |error| {
                eprintln!("Could not read input: {error}");
                std::process::exit(1);
            },
        )
    };
    if input.is_empty() {
        eprintln!("Provide input with --text, --clip, or stdin.");
        std::process::exit(2);
    }
    let target = ExecutionTarget::Operation {
        operation_ref: target_ref.clone(),
    };
    match execute(
        db,
        ExecutionRequest {
            input,
            target,
            source_clip_id: clip_id,
            trigger: ExecutionTrigger::Cli,
            destination: ExecutionDestination::Preview,
            client_request_id: None,
        },
    ) {
        Ok(outcome) => {
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "targetKind": "operation",
                        "targetRef": target_ref,
                        "executionId": outcome.execution_id,
                        "output": outcome.output,
                        "durationMs": outcome.duration_ms,
                    }))
                    .expect("advanced transformation output is serializable")
                );
            } else {
                print!("{}", outcome.output);
            }
        }
        Err(error) => {
            eprintln!("Operation failed ({}): {}", error.code, error.message);
            std::process::exit(1);
        }
    }
}

pub(crate) fn print_transform_definition(
    definition: &TransformDefinition,
    json: bool,
) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(definition).map_err(json_error)?
        );
    } else {
        let step_count = definition
            .plan
            .as_ref()
            .map(|plan| plan.steps.len())
            .unwrap_or_else(|| definition.steps.len());
        println!(
            "{}\t{}\trevision {}\t{} steps\t{}",
            definition.stable_ref,
            definition.name,
            definition.revision,
            step_count,
            definition.execution_character
        );
    }
    Ok(())
}

pub(crate) fn print_operation(operation: &pasted_lib::db::Operation, json: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(operation).map_err(json_error)?
        );
    } else {
        println!(
            "{}\t{}\t{}\t{}",
            operation.stable_id, operation.op_type, operation.category, operation.name
        );
    }
    Ok(())
}

pub(crate) fn print_connection(
    connection: &pasted_lib::db::IntelligenceConnection,
    json: bool,
) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(connection).map_err(json_error)?
        );
    } else {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            connection.id,
            connection.priority,
            if connection.enabled { "on" } else { "off" },
            connection.provider_kind,
            connection.name
        );
    }
    Ok(())
}

pub(crate) fn plan_transform_or_exit(
    db: &DbState,
    args: &[String],
    intent: String,
) -> PlanIntentOutcome {
    let planning_mode = match argument_value(args, "--mode").as_deref() {
        None | Some("pinned") => IntentPlanningMode::Pinned,
        Some("adaptive") => IntentPlanningMode::Adaptive,
        Some(_) => {
            eprintln!("--mode must be pinned or adaptive.");
            std::process::exit(2);
        }
    };
    let outcome = pasted_lib::intelligence_executor::plan_intent(
        db,
        PlanIntentRequest {
            intent,
            sample_input: argument_value(args, "--sample"),
            planning_mode,
            connection_id: argument_value(args, "--connection"),
        },
    )
    .unwrap_or_else(|error| {
        let _ = db.log_activity(
            "transform_draft_failed",
            &format!("Transform draft failed ({})", error.code),
        );
        eprintln!(
            "Transform planning failed ({}): {}",
            error.code, error.message
        );
        std::process::exit(1);
    });
    let _ = db.log_activity(
        "transform_drafted",
        &format!(
            "Drafted a {}-step Transform with {} in {} ms",
            outcome.plan.steps.len(),
            outcome.connection_name,
            outcome.duration_ms
        ),
    );
    outcome
}
