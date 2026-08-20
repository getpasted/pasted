use super::super::*;
use super::*;

pub(crate) fn run_classifier(args: Vec<String>, db_path: PathBuf, conn: Connection) -> Result<()> {
    drop(conn);
    let db = DbState::new(db_path.clone())?;
    let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
    match subcommand {
        "list" | "ls" => {
            let classifiers = db.get_content_classifiers()?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&classifiers).map_err(json_error)?
                );
            } else {
                for classifier in classifiers {
                    println!(
                        "{}\t{}\t{}\t{}\t{}",
                        classifier.stable_ref,
                        classifier.priority,
                        if classifier.enabled { "on" } else { "off" },
                        classifier.content_type,
                        classifier.name
                    );
                }
            }
        }
        "get" => {
            let reference = args.get(3).unwrap_or_else(|| {
                eprintln!("Usage: pasted classifier get <ref> [--json]");
                std::process::exit(2);
            });
            let classifier = db.get_content_classifier(reference)?;
            print_classifier(
                &classifier,
                args.iter().any(|argument| argument == "--json"),
            )?;
        }
        "create" | "new" => {
            let input = classifier_input_from_args(&args, None);
            let classifier = db.create_content_classifier(&input)?;
            print_classifier(
                &classifier,
                args.iter().any(|argument| argument == "--json"),
            )?;
        }
        "update" | "edit" => {
            let reference = args.get(3).unwrap_or_else(|| {
            eprintln!("Usage: pasted classifier update <ref> [--name NAME] [--type TYPE] [--regex REGEX] [--priority N] [--enabled|--disabled] [--json]");
            std::process::exit(2);
        });
            let current = db.get_content_classifier(reference)?;
            let input = classifier_input_from_args(&args, Some(&current));
            let classifier = db.update_content_classifier(current.id, &input)?;
            print_classifier(
                &classifier,
                args.iter().any(|argument| argument == "--json"),
            )?;
        }
        "duplicate" | "copy" => {
            let reference = args.get(3).unwrap_or_else(|| {
                eprintln!("Usage: pasted classifier duplicate <ref> [--name NAME] [--json]");
                std::process::exit(2);
            });
            let duplicate = db.duplicate_content_classifier(
                reference,
                argument_value(&args, "--name").as_deref(),
            )?;
            print_classifier(&duplicate, args.iter().any(|argument| argument == "--json"))?;
        }
        "delete" | "remove" => {
            let reference = args.get(3).unwrap_or_else(|| {
                eprintln!("Usage: pasted classifier delete <ref> [--json]");
                std::process::exit(2);
            });
            let classifier = db.get_content_classifier(reference)?;
            db.delete_content_classifier(classifier.id)?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::json!({ "deleted": true, "stableRef": classifier.stable_ref })
                );
            } else {
                println!("Deleted Classifier {}.", classifier.name);
            }
        }
        "run" | "test" => {
            require_feature(&db, Feature::ContentClassification);
            let reference = args.get(3).unwrap_or_else(|| {
            eprintln!("Usage: pasted classifier run <ref> [--text TEXT | --clip ID | --stdin] [--apply] [--json]");
            std::process::exit(2);
        });
            let clip_id =
                argument_value(&args, "--clip").and_then(|value| value.parse::<i64>().ok());
            let explicit_text = argument_value(&args, "--text");
            if clip_id.is_some() && explicit_text.is_some() {
                eprintln!("Provide only one of --text or --clip ID.");
                std::process::exit(2);
            }
            let apply = args.iter().any(|argument| argument == "--apply");
            if apply && clip_id.is_none() {
                eprintln!("--apply requires --clip ID.");
                std::process::exit(2);
            }
            let result = if apply {
                db.apply_content_classifier(clip_id.expect("checked above"), reference)?
            } else {
                let classifier = db.get_content_classifier(reference)?;
                let input = if let Some(text) = explicit_text {
                    text
                } else if let Some(clip_id) = clip_id {
                    match db.get_active_clip_text(clip_id)? {
                        Some(text) if !text.trim().is_empty() => text,
                        _ => {
                            eprintln!("Clip #{clip_id} has no analyzable text.");
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
                pasted_lib::classification_execution::ClassificationApplicationResult::preview(
                    pasted_lib::classification_execution::analyze_classifier(&input, &classifier),
                )
            };
            if args.iter().any(|argument| argument == "--json") {
                println!("{}", serde_json::json!(&result));
            } else if let Some(failure) = result.analysis.failure.as_ref() {
                eprintln!("Classifier failed ({}): {}", failure.code, failure.message);
            } else if result.analysis.matched {
                println!("Matches {}.", result.analysis.content_types.join(", "));
            } else {
                println!("Does not match.");
            }
            if result.analysis.failed() {
                let _ = io::stdout().flush();
                let _ = io::stderr().flush();
                std::process::exit(1);
            }
        }
        "restore-defaults" => {
            db.restore_default_content_classifiers()?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::json!({ "restoredDefaults": true, "kind": "classifiers" })
                );
            } else {
                println!(
                    "Restored shipped classifier defaults; custom classifiers were preserved."
                );
            }
        }
        "rescan" => {
            if !args.iter().any(|argument| argument == "--yes") {
                eprintln!("History rescans can change Content Types, Smart Bin membership, and sensitive-content masking. Re-run with --yes to continue.");
                std::process::exit(2);
            }
            let report = db.rescan_content_classification()?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).map_err(json_error)?
                );
            } else {
                println!(
                    "Rescanned {} text clips; {} changed, {} were unchanged, and {} failed.",
                    report.scanned_count,
                    report.changed_count,
                    report.unchanged_count,
                    report.failed_count
                );
            }
        }
        _ => {
            eprintln!("Usage: pasted classifier list|get|create|update|duplicate|delete|run|restore-defaults|rescan [options] [--json]");
            std::process::exit(2);
        }
    }
    Ok(())
}
