use super::super::*;
use super::*;

pub(crate) fn run_transforms(args: Vec<String>, db_path: PathBuf, _conn: Connection) -> Result<()> {
    let db = DbState::new(db_path.clone())?;
    require_feature(&db, Feature::Transformations);
    let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
    match subcommand {
        "list" | "ls" => {
            let transforms = db.get_transform_definitions()?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&transforms).map_err(json_error)?
                );
            } else if transforms.is_empty() {
                println!("No saved Transforms.");
            } else {
                for transform in transforms {
                    let execution_label = match transform.execution_character.as_str() {
                        "replayable" => "local",
                        "interpretive" => "AI-assisted",
                        _ => "mixed",
                    };
                    let step_count = transform
                        .plan
                        .as_ref()
                        .map(|plan| plan.steps.len())
                        .unwrap_or_else(|| transform.steps.len());
                    println!(
                        "{}\t{}\trevision {}\t{} steps\t{}",
                        transform.stable_ref,
                        transform.name,
                        transform.revision,
                        step_count,
                        execution_label
                    );
                }
            }
        }
        "get" => {
            let transform_ref = args.get(3).unwrap_or_else(|| {
                eprintln!("Usage: pasted transform get <transform-ref> [--json]");
                std::process::exit(2);
            });
            let definition = db
                .resolve_transform_definition(transform_ref)?
                .unwrap_or_else(|| {
                    eprintln!("Transform {transform_ref} was not found.");
                    std::process::exit(1);
                });
            print_transform_definition(&definition, args.iter().any(|arg| arg == "--json"))?;
        }
        "plan" => {
            let intent = match argument_value(&args, "--intent") {
                Some(intent) => intent,
                None => read_stdin_bounded(8_000)?,
            };
            let outcome = plan_transform_or_exit(&db, &args, intent);
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&outcome).map_err(json_error)?
                );
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&outcome.plan).map_err(json_error)?
                );
            }
        }
        "test" => {
            let plan_json = argument_value(&args, "--plan-json").unwrap_or_else(|| {
            eprintln!("Usage: pasted transform test --plan-json JSON [--text TEXT | --stdin] [--connection ID] [--json]");
            std::process::exit(2);
        });
            let plan =
                serde_json::from_str::<TransformationPlan>(&plan_json).unwrap_or_else(|error| {
                    eprintln!("Transform plan is invalid: {error}");
                    std::process::exit(2);
                });
            let input = match argument_value(&args, "--text") {
                Some(text) => text,
                None => read_stdin_bounded(pasted_lib::resource_limits::MAX_TRANSFORM_TEXT_BYTES)?,
            };
            if input.is_empty() {
                eprintln!("Provide test input with --text or stdin.");
                std::process::exit(2);
            }
            match pasted_lib::intelligence_executor::execute_plan(
                &db,
                ExecutePlanRequest {
                    plan,
                    input,
                    connection_id: argument_value(&args, "--connection"),
                },
            ) {
                Ok(outcome) => {
                    let provider = outcome
                        .connection_name
                        .as_deref()
                        .unwrap_or("local Operations");
                    let _ = db.log_activity(
                        "transform_tested",
                        &format!(
                            "Tested a Transform with {provider} in {} ms",
                            outcome.duration_ms
                        ),
                    );
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&outcome).map_err(json_error)?
                        );
                    } else {
                        print!("{}", outcome.output);
                    }
                }
                Err(error) => {
                    let _ = db.log_activity(
                        "transform_test_failed",
                        &format!("Transform test failed ({})", error.code),
                    );
                    eprintln!("Transform test failed ({}): {}", error.code, error.message);
                    std::process::exit(1);
                }
            }
        }
        "create" | "new" => {
            let name = argument_value(&args, "--name").unwrap_or_else(|| {
            eprintln!("Usage: pasted transform create --name NAME (--intent TEXT | --plan-json JSON | --steps-json JSON) [options] [--json]");
            std::process::exit(2);
        });
            if name.trim().is_empty() {
                eprintln!("Transform name cannot be empty.");
                std::process::exit(2);
            }
            let plan_json = argument_value(&args, "--plan-json");
            let steps_json = argument_value(&args, "--steps-json");
            let intent = argument_value(&args, "--intent");
            let definition = match (intent, plan_json, steps_json) {
                (Some(intent), None, None) => {
                    let outcome = plan_transform_or_exit(&db, &args, intent);
                    TransformDefinition::from(db.create_saved_transform(
                        &name,
                        &outcome.plan,
                        Some(&outcome.connection_id),
                    )?)
                }
                (None, Some(plan_json), None) => {
                    let plan: TransformationPlan =
                        serde_json::from_str(&plan_json).map_err(json_error)?;
                    TransformDefinition::from(db.create_saved_transform(
                        &name,
                        &plan,
                        argument_value(&args, "--connection").as_deref(),
                    )?)
                }
                (None, None, Some(steps_json)) => {
                    if argument_value(&args, "--hotkey").is_some() {
                        require_feature(&db, Feature::Hotkeys);
                    }
                    let steps: Vec<PipelineStepInput> =
                        serde_json::from_str(&steps_json).map_err(json_error)?;
                    TransformDefinition::from(pasted_lib::manual_transform_service::create(
                        &db,
                        &name,
                        &steps,
                        argument_value(&args, "--hotkey").as_deref(),
                    )?)
                }
                _ => {
                    eprintln!("Provide exactly one of --intent, --plan-json, or --steps-json.");
                    std::process::exit(2);
                }
            };
            print_transform_definition(&definition, args.iter().any(|arg| arg == "--json"))?;
        }
        "update" | "edit" => {
            let transform_ref = args.get(3).unwrap_or_else(|| {
            eprintln!("Usage: pasted transform update <transform-ref> [--name NAME] [--plan-json JSON | --steps-json JSON] [--connection ID | --clear-connection] [--hotkey HOTKEY | --clear-hotkey] [--json]");
            std::process::exit(2);
        });
            let current = db
                .resolve_transform_definition(transform_ref)?
                .unwrap_or_else(|| {
                    eprintln!("Transform {transform_ref} was not found.");
                    std::process::exit(1);
                });
            let name = argument_value(&args, "--name").unwrap_or_else(|| current.name.clone());
            if name.trim().is_empty() {
                eprintln!("Transform name cannot be empty.");
                std::process::exit(2);
            }
            let updated = match current.authoring_kind {
                TransformAuthoringKind::Intent => {
                    if argument_value(&args, "--steps-json").is_some()
                        || argument_value(&args, "--hotkey").is_some()
                        || args.iter().any(|arg| arg == "--clear-hotkey")
                    {
                        eprintln!("Intent-authored Transforms accept --plan-json and connection options; use duplicate/create to change authoring form.");
                        std::process::exit(2);
                    }
                    let plan = match argument_value(&args, "--plan-json") {
                        Some(plan_json) => serde_json::from_str::<TransformationPlan>(&plan_json)
                            .map_err(json_error)?,
                        None => current.plan.clone().expect("saved Transform has a plan"),
                    };
                    let connection_id = if args.iter().any(|arg| arg == "--clear-connection") {
                        None
                    } else {
                        argument_value(&args, "--connection").or(current.connection_id.clone())
                    };
                    TransformDefinition::from(db.update_saved_transform(
                        transform_ref,
                        &name,
                        &plan,
                        connection_id.as_deref(),
                    )?)
                }
                TransformAuthoringKind::Manual => {
                    if argument_value(&args, "--plan-json").is_some()
                        || argument_value(&args, "--connection").is_some()
                        || args.iter().any(|arg| arg == "--clear-connection")
                    {
                        eprintln!("Manually built Transforms accept --steps-json and hotkey options; use duplicate/create to change authoring form.");
                        std::process::exit(2);
                    }
                    let steps = match argument_value(&args, "--steps-json") {
                        Some(steps_json) => {
                            serde_json::from_str::<Vec<PipelineStepInput>>(&steps_json)
                                .map_err(json_error)?
                        }
                        None => current
                            .steps
                            .iter()
                            .map(|step| PipelineStepInput {
                                operation_ref: step.operation_ref.clone(),
                                config_json: step.config_json.clone(),
                                failure_policy: step.failure_policy.clone(),
                            })
                            .collect(),
                    };
                    if argument_value(&args, "--hotkey").is_some()
                        || args.iter().any(|arg| arg == "--clear-hotkey")
                    {
                        require_feature(&db, Feature::Hotkeys);
                    }
                    let hotkey = if args.iter().any(|arg| arg == "--clear-hotkey") {
                        None
                    } else {
                        argument_value(&args, "--hotkey").or(current.shortcut.clone())
                    };
                    TransformDefinition::from(pasted_lib::manual_transform_service::update(
                        &db,
                        transform_ref,
                        &name,
                        &steps,
                        hotkey.as_deref(),
                    )?)
                }
            };
            print_transform_definition(&updated, args.iter().any(|arg| arg == "--json"))?;
        }
        "duplicate" | "copy" => {
            let transform_ref = args.get(3).unwrap_or_else(|| {
                eprintln!(
                    "Usage: pasted transform duplicate <transform-ref> [--name NAME] [--json]"
                );
                std::process::exit(2);
            });
            let duplicate = db.duplicate_transform_definition(
                transform_ref,
                argument_value(&args, "--name").as_deref(),
            )?;
            print_transform_definition(&duplicate, args.iter().any(|arg| arg == "--json"))?;
        }
        "delete" | "remove" => {
            let transform_ref = args.get(3).unwrap_or_else(|| {
                eprintln!("Usage: pasted transform delete <transform-ref> [--json]");
                std::process::exit(2);
            });
            db.delete_transform_definition(transform_ref)?;
            if args.iter().any(|arg| arg == "--json") {
                println!(
                    "{}",
                    serde_json::json!({ "deleted": true, "stableRef": transform_ref })
                );
            } else {
                println!("Deleted Transform {transform_ref}.");
            }
        }
        "run" => {
            let Some(transform_ref) = args.get(3) else {
                eprintln!("Usage: pasted transform run <transform-ref> [--text TEXT | --clip ID | --stdin] [--apply] [--json]");
                std::process::exit(2);
            };
            let clip_id = args
                .iter()
                .position(|arg| arg == "--clip")
                .and_then(|index| args.get(index + 1))
                .and_then(|value| value.parse::<i64>().ok());
            let explicit_text = args
                .iter()
                .position(|arg| arg == "--text")
                .and_then(|index| args.get(index + 1))
                .cloned();
            let input = if let Some(text) = explicit_text {
                text
            } else if let Some(clip_id) = clip_id {
                match db.get_active_clip_text(clip_id)? {
                    Some(text) => text,
                    None => {
                        eprintln!("Clip #{clip_id} has no transformable text.");
                        std::process::exit(2);
                    }
                }
            } else {
                read_stdin_bounded(pasted_lib::resource_limits::MAX_TRANSFORM_TEXT_BYTES)?
            };
            if input.is_empty() {
                eprintln!("Provide input with --text, --clip, or stdin.");
                std::process::exit(2);
            }
            let replace = args
                .iter()
                .any(|arg| arg == "--apply" || arg == "--replace");
            if replace && clip_id.is_none() {
                eprintln!("--apply requires --clip ID so Pasted can create a revision.");
                std::process::exit(2);
            }
            let target = ExecutionTarget::Transform {
                transform_ref: transform_ref.clone(),
            };
            match execute(
                &db,
                ExecutionRequest {
                    input: input.clone(),
                    target,
                    source_clip_id: clip_id,
                    trigger: ExecutionTrigger::Cli,
                    destination: if replace {
                        ExecutionDestination::Replace
                    } else {
                        ExecutionDestination::Preview
                    },
                    client_request_id: None,
                },
            ) {
                Ok(outcome) => {
                    if let Some(clip_id) = clip_id.filter(|_| replace) {
                        if let Err(error) =
                            db.apply_transform_output_to_clip(TransformClipApplication {
                                clip_id,
                                transform_ref,
                                expected_input: &input,
                                output: &outcome.output,
                                connection_id: outcome.connection_id.as_deref(),
                                duration_ms: outcome.duration_ms,
                                bin_move: None,
                            })
                        {
                            eprintln!("Transform ran, but its output was not applied: {error}");
                            std::process::exit(1);
                        }
                    }
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "targetKind": "transform",
                                "targetRef": transform_ref,
                                "executionId": outcome.execution_id,
                                "output": outcome.output,
                                "durationMs": outcome.duration_ms,
                                "appliedClipId": clip_id.filter(|_| replace),
                                "replacedClipId": clip_id.filter(|_| replace),
                            }))
                            .expect("transform output is serializable")
                        );
                    } else {
                        print!("{}", outcome.output);
                    }
                }
                Err(error) => {
                    eprintln!("Transform failed ({}): {}", error.code, error.message);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("Usage: pasted transform list|get|plan|test|create|update|duplicate|delete|run [options] [--json]");
            std::process::exit(2);
        }
    }
    Ok(())
}
