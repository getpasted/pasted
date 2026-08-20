use super::super::*;
use super::*;

pub(crate) fn run_suggestion(args: Vec<String>, db_path: PathBuf, conn: Connection) -> Result<()> {
    drop(conn);
    let db = DbState::new(db_path.clone())?;
    require_feature(&db, Feature::Transformations);
    let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
    let json = args.iter().any(|argument| argument == "--json");
    match subcommand {
        "list" | "ls" => {
            let suggestions =
                vec![pasted_lib::content_suggestions::smart_actions_suggestion_definition()];
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&suggestions).map_err(json_error)?
                );
            } else {
                for suggestion in suggestions {
                    println!(
                        "{}\t{}\t{} → {}\t{}",
                        suggestion.stable_ref,
                        suggestion.priority,
                        suggestion.input_contracts.join(" + "),
                        suggestion.output_contract,
                        suggestion.name
                    );
                }
            }
        }
        "get" => {
            let reference = args.get(3).unwrap_or_else(|| {
                eprintln!("Usage: pasted suggestion get <ref> [--json]");
                std::process::exit(2);
            });
            let suggestion = pasted_lib::content_suggestions::smart_actions_suggestion_definition();
            if reference != &suggestion.stable_ref {
                eprintln!("Suggestion {reference} was not found.");
                std::process::exit(1);
            }
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&suggestion).map_err(json_error)?
                );
            } else {
                println!("{}\t{}", suggestion.stable_ref, suggestion.name);
            }
        }
        "run" | "test" => {
            let clip_id =
                argument_value(&args, "--clip").and_then(|value| value.parse::<i64>().ok());
            let explicit_text = argument_value(&args, "--text");
            if clip_id.is_some() && explicit_text.is_some() {
                eprintln!("Provide only one of --text or --clip ID.");
                std::process::exit(2);
            }
            let result = if let Some(clip_id) = clip_id {
                pasted_lib::suggestion_execution::suggest_clip(&db, clip_id)
            } else {
                let text = explicit_text.unwrap_or_else(|| {
                    read_stdin_bounded(pasted_lib::resource_limits::MAX_CLIP_TEXT_BYTES)
                        .unwrap_or_else(|error| {
                            eprintln!("Could not read suggestion input: {error}");
                            std::process::exit(2);
                        })
                });
                if text.is_empty() {
                    eprintln!("Provide input with --text, --clip, or stdin.");
                    std::process::exit(2);
                }
                pasted_lib::suggestion_execution::suggest_text(&db, &text, Some("Pasted CLI"))
            }
            .map_err(rusqlite::Error::InvalidParameterName)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).map_err(json_error)?
                );
            } else if result.analysis.result.actions.is_empty() {
                println!("No Smart Actions were suggested.");
            } else {
                if !result.analysis.result.signal_labels.is_empty() {
                    println!(
                        "Signals: {}",
                        result.analysis.result.signal_labels.join(", ")
                    );
                }
                for action in result.analysis.result.actions {
                    println!(
                        "{}\trevision {}\t{}",
                        action.transform_ref, action.transform_revision, action.transform_name
                    );
                }
            }
        }
        _ => {
            eprintln!("Usage: pasted suggestion list|get|run [options] [--json]");
            std::process::exit(2);
        }
    }
    Ok(())
}
