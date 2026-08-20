use super::super::*;
use super::*;

pub(crate) fn run_operations(args: Vec<String>, db_path: PathBuf, _conn: Connection) -> Result<()> {
    let db = DbState::new(db_path.clone())?;
    require_feature(&db, Feature::Transformations);
    let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
    match subcommand {
        "list" | "ls" => {
            let operations = db.get_operations()?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&operations).map_err(json_error)?
                );
            } else {
                for operation in operations {
                    println!(
                        "{}\t{}\t{}\t{}",
                        operation.stable_id, operation.name, operation.op_type, operation.category
                    );
                }
            }
        }
        "get" => {
            let reference = args.get(3).unwrap_or_else(|| {
                eprintln!("Usage: pasted operation get <ref> [--json]");
                std::process::exit(2);
            });
            let operation = db.get_operation(reference)?;
            print_operation(&operation, args.iter().any(|argument| argument == "--json"))?;
        }
        "create" | "new" => {
            let name = argument_value(&args, "--name").unwrap_or_else(|| {
            eprintln!("Usage: pasted operation create --name NAME --type TYPE [--config-json JSON] [--category CATEGORY] [--json]");
            std::process::exit(2);
        });
            let op_type = argument_value(&args, "--type").unwrap_or_else(|| {
                eprintln!("Operation creation requires --type.");
                std::process::exit(2);
            });
            if !matches!(op_type.as_str(), "regex" | "ai") {
                eprintln!("New Operations must use type regex or ai.");
                std::process::exit(2);
            }
            let config = argument_value(&args, "--config-json");
            validate_json_or_exit(config.as_deref(), "Operation configuration");
            let operation = db.create_operation(
                &name,
                &op_type,
                config.as_deref(),
                argument_value(&args, "--category").as_deref(),
            )?;
            print_operation(&operation, args.iter().any(|argument| argument == "--json"))?;
        }
        "update" | "edit" => {
            let reference = args.get(3).unwrap_or_else(|| {
            eprintln!("Usage: pasted operation update <ref> [--name NAME] [--type TYPE] [--config-json JSON] [--category CATEGORY] [--json]");
            std::process::exit(2);
        });
            let current = db.get_operation(reference)?;
            if current.id < 0 {
                eprintln!("Built-in Operations cannot be updated; duplicate one first.");
                std::process::exit(2);
            }
            let updated_type =
                argument_value(&args, "--type").unwrap_or_else(|| current.op_type.clone());
            if !matches!(updated_type.as_str(), "regex" | "ai") {
                eprintln!("Custom Operations must use type regex or ai.");
                std::process::exit(2);
            }
            let updated_config = argument_value(&args, "--config-json").or(current.config.clone());
            validate_json_or_exit(updated_config.as_deref(), "Operation configuration");
            db.update_operation(
                current.id,
                argument_value(&args, "--name")
                    .as_deref()
                    .unwrap_or(&current.name),
                &updated_type,
                updated_config.as_deref(),
                argument_value(&args, "--category")
                    .as_deref()
                    .or(Some(&current.category)),
            )?;
            let updated = db.get_operation(&current.stable_id)?;
            print_operation(&updated, args.iter().any(|argument| argument == "--json"))?;
        }
        "duplicate" | "copy" => {
            let reference = args.get(3).unwrap_or_else(|| {
                eprintln!("Usage: pasted operation duplicate <ref> [--name NAME] [--json]");
                std::process::exit(2);
            });
            let operation =
                db.duplicate_operation(reference, argument_value(&args, "--name").as_deref())?;
            print_operation(&operation, args.iter().any(|argument| argument == "--json"))?;
        }
        "delete" | "remove" => {
            let reference = args.get(3).unwrap_or_else(|| {
                eprintln!("Usage: pasted operation delete <ref> [--json]");
                std::process::exit(2);
            });
            let operation = db.get_operation(reference)?;
            if operation.id < 0 {
                eprintln!("Built-in Operations cannot be deleted.");
                std::process::exit(2);
            }
            db.delete_operation(operation.id)?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::json!({ "deleted": true, "stableRef": operation.stable_id })
                );
            } else {
                println!("Deleted Operation {}.", operation.name);
            }
        }
        "run" | "test" => run_operation(&args, &db),
        _ => {
            eprintln!("Usage: pasted operation list|get|create|update|duplicate|delete|run [options] [--json]");
            std::process::exit(2);
        }
    }
    Ok(())
}
