use super::{argument_value, json_error};
use pasted_lib::db::DbState;
use rusqlite::{Connection, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn run(args: &[String], db_path: PathBuf, conn: Connection) -> Result<()> {
    drop(conn);
    let db = DbState::new(db_path.clone())?;
    let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
    match subcommand {
        "list" => {
            let limit = if args.iter().any(|argument| argument == "--all") {
                i64::MAX
            } else {
                argument_value(args, "--limit")
                    .and_then(|value| value.parse::<i64>().ok())
                    .unwrap_or(100)
                    .clamp(1, 100_000)
            };
            let offset = argument_value(args, "--offset")
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(0)
                .max(0);
            let logs = db.get_activity_logs_filtered(
                Some(limit),
                Some(offset),
                argument_value(args, "--category").as_deref(),
                argument_value(args, "--severity").as_deref(),
                argument_value(args, "--event").as_deref(),
            )?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&logs).map_err(json_error)?
                );
            } else {
                for log in logs {
                    println!(
                        "{}\t{}\t{}\t{}",
                        log.created_at, log.severity_text, log.event_type, log.description
                    );
                }
            }
        }
        "export" => {
            let path = args
                .get(3)
                .filter(|argument| !argument.starts_with("--"))
                .map(PathBuf::from);
            let format = argument_value(args, "--format").unwrap_or_else(|| {
                path.as_ref()
                    .and_then(|value| value.extension())
                    .and_then(|value| value.to_str())
                    .unwrap_or("json")
                    .to_ascii_lowercase()
            });
            let contents = match format.as_str() {
                "json" => db.export_activity_json()?,
                "csv" => db.export_activity_csv()?,
                _ => {
                    eprintln!("Activity export format must be json or csv.");
                    std::process::exit(2);
                }
            };
            if let Some(path) = path {
                fs::write(&path, contents)
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
                if args.iter().any(|argument| argument == "--json") {
                    println!("{}", serde_json::json!({ "format": format, "path": path }));
                } else {
                    println!("Exported Activity to {}.", path.display());
                }
            } else {
                print!("{contents}");
            }
        }
        "import" => {
            let Some(path) = args.get(3).filter(|argument| !argument.starts_with("--")) else {
                eprintln!("Usage: pasted activity import <path> [--format json|csv] [--json]");
                std::process::exit(2);
            };
            let format = argument_value(args, "--format").unwrap_or_else(|| {
                Path::new(path)
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or("json")
                    .to_ascii_lowercase()
            });
            let metadata = fs::metadata(path)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            if metadata.len() > pasted_lib::resource_limits::MAX_ACTIVITY_IMPORT_BYTES as u64 {
                eprintln!("Activity imports must be 32 MB or smaller.");
                std::process::exit(2);
            }
            let contents = fs::read_to_string(path)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            let report = match format.as_str() {
                "json" => db.import_activity_json(&contents)?,
                "csv" => db.import_activity_csv(&contents)?,
                _ => {
                    eprintln!("Activity import format must be json or csv.");
                    std::process::exit(2);
                }
            };
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).map_err(json_error)?
                );
            } else {
                println!(
                "Imported {} of {} Activity entries; {} duplicates were skipped and {} entries are retained.",
                report.imported_count,
                report.scanned_count,
                report.duplicate_count,
                report.retained_count,
            );
            }
        }
        "clear" => {
            if !args.iter().any(|argument| argument == "--yes") {
                eprintln!("Clearing Activity is permanent. Re-run with --yes to continue.");
                std::process::exit(2);
            }
            db.clear_activity_logs()?;
            if args.iter().any(|argument| argument == "--json") {
                println!("{}", serde_json::json!({ "cleared": true }));
            } else {
                println!("Cleared Activity.");
            }
        }
        _ => {
            eprintln!("Usage: pasted activity list [--limit N|--all] [--offset N] [--category VALUE] [--severity VALUE] [--event NAME] [--json] | export [path] [--format json|csv] | import <activity.json> [--json] | clear --yes [--json]");
            std::process::exit(2);
        }
    }
    Ok(())
}
