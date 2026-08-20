use super::super::{get_app_config_dir, read_library_archive};
use super::json_error;
use pasted_lib::db::DbState;
use rusqlite::{Connection, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn run_transfer(args: &[String], db_path: PathBuf, conn: Connection) -> Result<()> {
    drop(conn);
    let db = DbState::new(db_path.clone())?;
    let subcommand = args.get(2).map(String::as_str).unwrap_or("inspect");
    let Some(path) = args.get(3).filter(|argument| !argument.starts_with("--")) else {
        eprintln!("Usage: pasted transfer export|inspect|import <path.json> [--json]");
        std::process::exit(2);
    };
    match subcommand {
        "export" => {
            let contents = db.export_backup_json()?;
            let inspection = DbState::inspect_library_archive_json(&contents)?;
            fs::write(path, contents)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "path": path,
                        "inspection": inspection,
                    }))
                    .map_err(json_error)?
                );
            } else {
                println!("Exported history and organization data to {path}.");
            }
        }
        "inspect" => {
            let inspection =
                DbState::inspect_library_archive_json(&read_library_archive(Path::new(path))?)?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&inspection).map_err(json_error)?
                );
            } else {
                println!(
                "Transfer file v{}: {} clips, {} Bins, {} Transforms, {} Operations, {} Classifiers, and {} Content Types.",
                inspection.schema_version,
                inspection.clip_count,
                inspection.bin_count,
                inspection.transform_count,
                inspection.operation_count,
                inspection.classifier_count,
                inspection.content_type_count,
            );
            }
        }
        "import" => {
            let contents = read_library_archive(Path::new(path))?;
            let inspection = DbState::inspect_library_archive_json(&contents)?;
            let imported_count = db.import_backup_json(&contents)?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "inspection": inspection,
                        "processedClipCount": imported_count,
                    }))
                    .map_err(json_error)?
                );
            } else {
                println!(
                "Imported history and organization data. Processed {imported_count} clips after preflight."
            );
            }
        }
        _ => {
            eprintln!("Usage: pasted transfer export|inspect|import <path.json> [--json]");
            std::process::exit(2);
        }
    }
    Ok(())
}

pub(crate) fn run_backup(args: &[String], db_path: PathBuf, conn: Connection) -> Result<()> {
    drop(conn);
    let db = DbState::new(db_path.clone())?;
    let subcommand = args.get(2).map(String::as_str).unwrap_or("create");
    let Some(path) = args.get(3).filter(|argument| !argument.starts_with("--")) else {
        eprintln!(
            "Usage: pasted backup create|inspect|restore <path.pastedbackup> [--yes] [--json]"
        );
        std::process::exit(2);
    };
    let window_state = fs::read_to_string(get_app_config_dir().join(".window-state.json")).ok();
    match subcommand {
        "create" => {
            let report = db.create_full_backup(Path::new(path), None, window_state.as_deref())?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).map_err(json_error)?
                );
            } else {
                println!("Created full backup at {}.", report.path);
            }
        }
        "inspect" => {
            let inspection = db.inspect_full_backup(Path::new(path))?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&inspection).map_err(json_error)?
                );
            } else {
                println!(
                    "Full Backup v{} · {} bytes · created {}",
                    inspection.format_version, inspection.size_bytes, inspection.created_at
                );
            }
        }
        "restore" => {
            if !args.iter().any(|argument| argument == "--yes") {
                eprintln!(
                    "Full restore replaces the current state. Quit Pasted, then re-run with --yes."
                );
                std::process::exit(2);
            }
            let (report, _, restored_window_state) =
                db.restore_full_backup(Path::new(path), None, window_state.as_deref())?;
            if let Some(state) = restored_window_state {
                fs::create_dir_all(get_app_config_dir())
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
                fs::write(get_app_config_dir().join(".window-state.json"), state)
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            }
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).map_err(json_error)?
                );
            } else {
                println!(
                    "Restored the full backup. The previous state remains recoverable at {}.",
                    report.recovery_path
                );
            }
        }
        _ => {
            eprintln!(
                "Usage: pasted backup create|inspect|restore <path.pastedbackup> [--yes] [--json]"
            );
            std::process::exit(2);
        }
    }
    Ok(())
}
