use super::super::json_error;
use pasted_lib::db::DbState;
use rusqlite::{Connection, Result};
use std::path::PathBuf;

pub(crate) fn run(args: &[String], db_path: PathBuf, conn: Connection) -> Result<()> {
    drop(conn);
    let db = DbState::new(db_path.clone())?;
    let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
    let json = args.iter().any(|argument| argument == "--json");
    match subcommand {
        "list" | "ls" => {
            let mut values = db.get_all_settings()?;
            values.remove("pendingFullBackupClientState");
            values.retain(|key, _| !pasted_lib::app_lock::is_private_setting(key));
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&values).map_err(json_error)?
                );
            } else {
                let mut values = values.into_iter().collect::<Vec<_>>();
                values.sort_by(|left, right| left.0.cmp(&right.0));
                for (key, value) in values {
                    println!("{key}\t{value}");
                }
            }
        }
        "get" => {
            let key = args.get(3).unwrap_or_else(|| {
                eprintln!("Usage: pasted settings get <key> [--json]");
                std::process::exit(2);
            });
            if key == "pendingFullBackupClientState"
                || pasted_lib::app_lock::is_private_setting(key)
            {
                eprintln!("That setting is internal and cannot be read through the CLI.");
                std::process::exit(2);
            }
            let value = db.get_setting(key)?;
            if json {
                println!("{}", serde_json::json!({ "key": key, "value": value }));
            } else if let Some(value) = value {
                println!("{value}");
            } else {
                eprintln!("Setting {key} was not found.");
                std::process::exit(1);
            }
        }
        "set" => {
            let key = args.get(3).unwrap_or_else(|| {
                eprintln!("Usage: pasted settings set <key> <value> [--json]");
                std::process::exit(2);
            });
            let value = args.get(4).unwrap_or_else(|| {
                eprintln!("Usage: pasted settings set <key> <value> [--json]");
                std::process::exit(2);
            });
            if let Err(error) =
                pasted_lib::settings_service::update_setting(&db, key.clone(), value.clone())
            {
                if json {
                    eprintln!("{}", serde_json::json!({ "error": error }));
                } else {
                    eprintln!("{error}");
                }
                std::process::exit(2);
            }
            if json {
                println!("{}", serde_json::json!({ "key": key, "value": value }));
            } else {
                println!("Saved {key}.");
            }
        }
        _ => {
            eprintln!("Usage: pasted settings list|get|set [arguments] [--json]");
            std::process::exit(2);
        }
    }
    Ok(())
}
