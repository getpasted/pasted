use super::json_error;
use pasted_lib::db::DbState;
use rusqlite::{Connection, Result};
use std::path::PathBuf;

mod reset_preview;

pub(crate) fn run(args: &[String], db_path: PathBuf, conn: Connection) -> Result<()> {
    drop(conn);
    let db = DbState::new(db_path.clone())?;
    let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
    let json = args.iter().any(|argument| argument == "--json");
    let dry_run = args.iter().any(|argument| argument == "--dry-run");
    match subcommand {
        "list" | "ls" => {
            let mut values = db.get_all_settings()?;
            values.retain(|key, _| pasted_lib::settings_contract::is_cli_readable(key));
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
            if !pasted_lib::settings_contract::is_cli_readable(key) {
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
            if !pasted_lib::settings_contract::is_cli_readable(key) {
                eprintln!("That setting is internal and cannot be changed through the CLI.");
                std::process::exit(2);
            }
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
        "reset" => {
            let page = args.get(3).map(String::as_str).unwrap_or_else(|| {
                eprintln!("Usage: pasted settings reset <page> [--dry-run] [--stdin] [--json]");
                std::process::exit(2);
            });
            if page == "security" {
                let preview = pasted_lib::settings_service::preview_dedicated_page_reset(&db, page)
                    .map_err(|error| super::cli_input_error(error.to_string()))?;
                if !dry_run {
                    super::require_app_lock_passphrase(&db, args)?;
                    pasted_lib::app_lock::reset_policy(&db).map_err(super::cli_input_error)?;
                    let _ = db.log_activity("settings_changed", "Reset security preferences");
                }
                print_reset(
                    page,
                    json,
                    dry_run,
                    serde_json::json!({
                        "changeCount": preview.changes.len(),
                        "changes": preview.changes,
                        "credentialsPreserved": true,
                        "idleMinutes": pasted_lib::app_lock::DEFAULT_IDLE_MINUTES,
                        "lockOnSleep": true,
                        "lockOnRestart": true,
                        "captureWhileLocked": true,
                        "systemAuthEnabled": false,
                        "appleWatchEnabled": false
                    }),
                );
                return Ok(());
            }
            if page == "analysis" {
                let changes = reset_preview::analysis_changes(&db)?;
                if !dry_run {
                    db.restore_default_content_classifiers()?;
                    db.restore_default_content_extractors()?;
                    db.restore_default_content_types()?;
                    db.restore_default_content_type_groups()?;
                }
                print_reset(
                    page,
                    json,
                    dry_run,
                    serde_json::json!({ "changeCount": changes.len(), "changes": changes, "customDefinitionsPreserved": true }),
                );
                return Ok(());
            }
            if page == "intelligence" {
                let detected =
                    pasted_lib::intelligence_connections::detect_intelligence_connections();
                let identities = detected
                    .into_iter()
                    .map(|candidate| {
                        let endpoint = if candidate.provider_kind == "cli" {
                            candidate.executable_path
                        } else {
                            candidate.default_endpoint.map(str::to_string)
                        };
                        (candidate.provider_kind.to_string(), endpoint)
                    })
                    .collect::<Vec<_>>();
                let before = db.get_intelligence_connections()?;
                let changes = reset_preview::intelligence_changes(&before, &identities);
                if !dry_run {
                    db.reset_intelligence_connection_policy(&identities)?;
                }
                print_reset(
                    page,
                    json,
                    dry_run,
                    serde_json::json!({ "changeCount": changes.len(), "changes": changes, "connectionDetailsPreserved": true }),
                );
                return Ok(());
            }
            let outcome = if dry_run {
                pasted_lib::settings_service::preview_page_reset(&db, page)
            } else {
                pasted_lib::settings_service::reset_page(&db, page)
            }
            .map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))?;
            print_reset(
                page,
                json,
                dry_run,
                serde_json::json!({ "changeCount": outcome.changes.len(), "changes": outcome.changes }),
            );
        }
        _ => {
            eprintln!("Usage: pasted settings list|get|set|reset [arguments] [--json]");
            std::process::exit(2);
        }
    }
    Ok(())
}

fn print_reset(page: &str, json: bool, dry_run: bool, details: serde_json::Value) {
    if json {
        println!(
            "{}",
            serde_json::json!({ "page": page, "reset": !dry_run, "dryRun": dry_run, "details": details })
        );
    } else if dry_run {
        println!("Would reset {page} settings.");
    } else {
        println!("Reset {page} settings.");
    }
}
