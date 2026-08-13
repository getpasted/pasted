use rusqlite::{params, Connection, OptionalExtension, Result};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use pasted_lib::bin_assignment::assign_clips_to_bin;
use pasted_lib::content_detection::detect_with_detectors;
use pasted_lib::content_detection::DetectorInput;
use pasted_lib::content_types::{ContentTypeGroupInput, ContentTypeInput};
use pasted_lib::db::{
    ClipMutationSummary, DbState, PipelineStepInput, TransformAuthoringKind,
    TransformClipApplication, TransformDefinition,
};
use pasted_lib::external_import::{self, ExternalImportSource};
use pasted_lib::features::{setting_value_is_enabled, Feature};
use pasted_lib::installation_diagnostics::{InstallationDiagnostics, APP_IDENTIFIER};
use pasted_lib::library_storage;
use pasted_lib::third_party_licenses;
use pasted_lib::transformation_intent::TransformationPlan;
use pasted_lib::transformation_service::{
    execute, ExecutionDestination, ExecutionRequest, ExecutionTarget, ExecutionTrigger,
};

fn get_app_data_dir() -> PathBuf {
    if let Some(mut dir) = dirs::data_dir() {
        dir.push(APP_IDENTIFIER);
        if dir.exists() {
            return dir;
        }
    }
    if let Some(mut dir) = dirs::data_dir() {
        dir.push("com.tauri.dev");
        if dir.exists() {
            return dir;
        }
    }
    if let Some(mut dir) = dirs::data_dir() {
        dir.push("tauri-app");
        if dir.exists() {
            return dir;
        }
    }
    let local_dir = PathBuf::from("./pasted_data");
    let _ = fs::create_dir_all(&local_dir);
    local_dir
}

fn get_app_config_dir() -> PathBuf {
    dirs::config_dir()
        .map(|mut dir| {
            dir.push(APP_IDENTIFIER);
            dir
        })
        .unwrap_or_else(get_app_data_dir)
}

fn get_db_path() -> PathBuf {
    let app_data = get_app_data_dir();
    library_storage::resolve_database_path(&app_data)
}

fn read_library_archive(path: &Path) -> Result<String> {
    let metadata = fs::metadata(path)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    if metadata.len() > pasted_lib::resource_limits::MAX_BACKUP_IMPORT_BYTES as u64 {
        return Err(rusqlite::Error::InvalidParameterName(
            "Transfer file exceeds the 256 MB safety limit".to_string(),
        ));
    }
    fs::read_to_string(path)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    // Legal notices must remain available even when the app database does not
    // exist yet or the optional clipboard-management CLI feature is disabled.
    if matches!(command, "licenses" | "license") {
        let document = third_party_licenses::document();
        if args.iter().any(|argument| argument == "--json") {
            println!(
                "{}",
                serde_json::to_string_pretty(document).map_err(json_error)?
            );
        } else {
            print!("{}", document.notice_text());
        }
        return Ok(());
    }

    let db_path = get_db_path();
    let migration_db = match DbState::new(db_path.clone()) {
        Ok(db) => db,
        Err(error) => {
            eprintln!("Error migrating Pasted database at '{db_path:?}': {error}");
            std::process::exit(1);
        }
    };
    drop(migration_db);
    let conn = match Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error opening Pasted database at '{:?}': {}", db_path, e);
            std::process::exit(1);
        }
    };

    let cli_setting = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [Feature::Cli.setting_key()],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten();
    if !setting_value_is_enabled(cli_setting.as_deref()) {
        eprintln!("Pasted CLI is disabled in Settings → Functionality.");
        std::process::exit(1);
    }

    match command {
        "retention" => {
            drop(conn);
            let db = DbState::new(db_path.clone())?;
            let current_count = db
                .get_setting("keepClipCount")?
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(1000);
            let current_age_days = db
                .get_setting("keepClipAgeDays")?
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(0);
            let count = parse_retention_argument(&args, "--count", "unlimited", 100_000)
                .unwrap_or(current_count);
            let age_days = parse_retention_argument(&args, "--days", "forever", 36_500)
                .unwrap_or(current_age_days);
            let trash_count =
                parse_retention_argument(&args, "--trash-count", "unlimited", 100_000)
                    .unwrap_or(setting_i64(&db, "trashCapacityCount", 500)?);
            let trash_age_days = parse_retention_argument(&args, "--trash-days", "forever", 36_500)
                .unwrap_or(setting_i64(&db, "trashAgeDays", 0)?);
            let activity_count =
                parse_retention_argument(&args, "--log-count", "unlimited", 100_000)
                    .unwrap_or(setting_i64(&db, "activityLogCapacity", 1000)?);
            let activity_age_days =
                parse_retention_argument(&args, "--log-days", "forever", 36_500)
                    .unwrap_or(setting_i64(&db, "activityLogAgeDays", 0)?);
            let history_changed = args
                .iter()
                .any(|argument| argument == "--count" || argument == "--days");
            let trash_changed = args
                .iter()
                .any(|argument| argument == "--trash-count" || argument == "--trash-days");
            let activity_changed = args
                .iter()
                .any(|argument| argument == "--log-count" || argument == "--log-days");
            if history_changed {
                db.configure_clip_retention(count, age_days)?;
            }
            if trash_changed {
                db.configure_trash_retention(trash_count, trash_age_days)?;
            }
            if activity_changed {
                db.configure_activity_retention(activity_count, activity_age_days)?;
            }
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "maximumClips": count,
                        "maximumAgeDays": age_days,
                        "maximumClipsUnlimited": count == 0,
                        "maximumAgeForever": age_days == 0,
                        "trashMaximumClips": trash_count,
                        "trashMaximumAgeDays": trash_age_days,
                        "trashMaximumClipsUnlimited": trash_count == 0,
                        "trashMaximumAgeForever": trash_age_days == 0,
                        "activityMaximumEntries": activity_count,
                        "activityMaximumAgeDays": activity_age_days,
                        "activityMaximumEntriesUnlimited": activity_count == 0,
                        "activityMaximumAgeForever": activity_age_days == 0,
                    }))
                    .map_err(json_error)?
                );
            } else {
                let count_label = if count == 0 {
                    "Unlimited".to_string()
                } else {
                    format!("{count} clips")
                };
                let age_label = if age_days == 0 {
                    "Forever".to_string()
                } else {
                    format!("{age_days} days")
                };
                println!(
                    "History: {count_label}; {age_label}\nTrash: {}; {}\nActivity: {}; {}",
                    retention_count_label(trash_count, "clips"),
                    retention_age_label(trash_age_days),
                    retention_count_label(activity_count, "entries"),
                    retention_age_label(activity_age_days),
                );
            }
        }
        "activity" => {
            drop(conn);
            let db = DbState::new(db_path.clone())?;
            let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
            match subcommand {
                "list" => {
                    let limit = if args.iter().any(|argument| argument == "--all") {
                        i64::MAX
                    } else {
                        argument_value(&args, "--limit")
                            .and_then(|value| value.parse::<i64>().ok())
                            .unwrap_or(100)
                            .clamp(1, 100_000)
                    };
                    let logs = db.get_activity_logs(Some(limit), Some(0))?;
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
                    let format = argument_value(&args, "--format").unwrap_or_else(|| {
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
                        fs::write(&path, contents).map_err(|error| {
                            rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                        })?;
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
                    let Some(path) = args.get(3).filter(|argument| !argument.starts_with("--"))
                    else {
                        eprintln!(
                            "Usage: pasted activity import <path> [--format json|csv] [--json]"
                        );
                        std::process::exit(2);
                    };
                    let format = argument_value(&args, "--format").unwrap_or_else(|| {
                        Path::new(path)
                            .extension()
                            .and_then(|value| value.to_str())
                            .unwrap_or("json")
                            .to_ascii_lowercase()
                    });
                    let metadata = fs::metadata(path).map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                    })?;
                    if metadata.len()
                        > pasted_lib::resource_limits::MAX_ACTIVITY_IMPORT_BYTES as u64
                    {
                        eprintln!("Activity imports must be 32 MB or smaller.");
                        std::process::exit(2);
                    }
                    let contents = fs::read_to_string(path).map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                    })?;
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
                    eprintln!("Usage: pasted activity list [--limit N|--all] [--json] | export [path] [--format json|csv] | import <activity.json> [--json] | clear --yes [--json]");
                    std::process::exit(2);
                }
            }
        }
        "transfer" | "archive" => {
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
                    fs::write(path, contents).map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                    })?;
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
                    let inspection = DbState::inspect_library_archive_json(&read_library_archive(
                        Path::new(path),
                    )?)?;
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&inspection).map_err(json_error)?
                        );
                    } else {
                        println!(
                            "Transfer file v{}: {} clips, {} Bins, {} Transforms, {} Operations, {} detectors, and {} Types.",
                            inspection.schema_version,
                            inspection.clip_count,
                            inspection.bin_count,
                            inspection.transform_count,
                            inspection.operation_count,
                            inspection.detector_count,
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
        }
        "backup" => {
            drop(conn);
            let db = DbState::new(db_path.clone())?;
            let subcommand = args.get(2).map(String::as_str).unwrap_or("create");
            let Some(path) = args.get(3).filter(|argument| !argument.starts_with("--")) else {
                eprintln!("Usage: pasted backup create <path.pastedbackup> | restore <path.pastedbackup> --yes [--json]");
                std::process::exit(2);
            };
            let window_state =
                fs::read_to_string(get_app_config_dir().join(".window-state.json")).ok();
            match subcommand {
                "create" => {
                    let report =
                        db.create_full_backup(Path::new(path), None, window_state.as_deref())?;
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&report).map_err(json_error)?
                        );
                    } else {
                        println!("Created full backup at {}.", report.path);
                    }
                }
                "restore" => {
                    if !args.iter().any(|argument| argument == "--yes") {
                        eprintln!("Full restore replaces the current state. Quit Pasted, then re-run with --yes.");
                        std::process::exit(2);
                    }
                    let (report, _, restored_window_state) =
                        db.restore_full_backup(Path::new(path), None, window_state.as_deref())?;
                    if let Some(state) = restored_window_state {
                        fs::create_dir_all(get_app_config_dir()).map_err(|error| {
                            rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                        })?;
                        fs::write(get_app_config_dir().join(".window-state.json"), state).map_err(
                            |error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)),
                        )?;
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
                    eprintln!("Usage: pasted backup create <path.pastedbackup> | restore <path.pastedbackup> --yes [--json]");
                    std::process::exit(2);
                }
            }
        }
        "registry" => {
            drop(conn);
            let db = DbState::new(db_path.clone())?;
            let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
            let kind = argument_value(&args, "--kind");
            if matches!(subcommand, "enable" | "disable") {
                let kind = kind.ok_or_else(|| {
                    rusqlite::Error::InvalidParameterName(
                        "registry enable/disable requires --kind".to_string(),
                    )
                })?;
                let stable_ref = argument_value(&args, "--ref").ok_or_else(|| {
                    rusqlite::Error::InvalidParameterName(
                        "registry enable/disable requires --ref".to_string(),
                    )
                })?;
                db.set_library_item_enabled(&kind, &stable_ref, subcommand == "enable")?;
                if args.iter().any(|argument| argument == "--json") {
                    println!(
                        "{}",
                        serde_json::json!({
                            "kind": kind,
                            "stableRef": stable_ref,
                            "enabled": subcommand == "enable",
                        })
                    );
                }
                return Ok(());
            }
            if subcommand != "list" && !subcommand.starts_with('-') {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "Unknown registry command: {subcommand}"
                )));
            }
            let items = db.get_library_items(
                kind.as_deref(),
                args.iter().any(|argument| argument == "--all"),
            )?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&items).map_err(json_error)?
                );
            } else {
                for view in items {
                    println!(
                        "{}\t{}\t{}",
                        view.item.kind, view.item.stable_ref, view.item.name
                    );
                }
            }
        }
        "type" | "types" => {
            drop(conn);
            let db = DbState::new(db_path.clone())?;
            let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
            let json = args.iter().any(|argument| argument == "--json");
            match subcommand {
                "group-list" => {
                    let groups = db
                        .get_content_type_groups(args.iter().any(|argument| argument == "--all"))?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&groups).map_err(json_error)?
                        );
                    } else {
                        for group in groups {
                            println!(
                                "{}\t{}\t{}\t{}",
                                group.id,
                                group.sort_order,
                                if group.is_archived {
                                    "archived"
                                } else {
                                    "active"
                                },
                                group.label
                            );
                        }
                    }
                }
                "group-create" => {
                    let id = argument_value(&args, "--id").unwrap_or_else(|| { eprintln!("Usage: pasted type group-create --id ID --name NAME [--order NUMBER] [--json]"); std::process::exit(2); });
                    let label = argument_value(&args, "--name").unwrap_or_else(|| {
                        eprintln!("Group creation requires --name.");
                        std::process::exit(2);
                    });
                    let created = db.create_content_type_group(&ContentTypeGroupInput {
                        id,
                        label,
                        sort_order: argument_value(&args, "--order")
                            .and_then(|value| value.parse().ok())
                            .unwrap_or(100),
                    })?;
                    println!(
                        "{}",
                        if json {
                            serde_json::to_string_pretty(&created).map_err(json_error)?
                        } else {
                            format!("Saved content type group {}: {}", created.id, created.label)
                        }
                    );
                }
                "group-update" => {
                    let id = args.get(3).cloned().unwrap_or_else(|| { eprintln!("Usage: pasted type group-update <id> [--name NAME] [--order NUMBER] [--json]"); std::process::exit(2); });
                    let current = db
                        .get_content_type_groups(true)?
                        .into_iter()
                        .find(|item| item.id == id)
                        .unwrap_or_else(|| {
                            eprintln!("Content type group {id} was not found.");
                            std::process::exit(1);
                        });
                    let updated = db.update_content_type_group(
                        &id,
                        &ContentTypeGroupInput {
                            id: id.clone(),
                            label: argument_value(&args, "--name").unwrap_or(current.label),
                            sort_order: argument_value(&args, "--order")
                                .and_then(|value| value.parse().ok())
                                .unwrap_or(current.sort_order),
                        },
                    )?;
                    println!(
                        "{}",
                        if json {
                            serde_json::to_string_pretty(&updated).map_err(json_error)?
                        } else {
                            format!("Saved content type group {}: {}", updated.id, updated.label)
                        }
                    );
                }
                "group-archive" | "group-restore" => {
                    let id = args.get(3).cloned().unwrap_or_else(|| {
                        eprintln!("Usage: pasted type {subcommand} <id>");
                        std::process::exit(2);
                    });
                    db.set_content_type_group_archived(&id, subcommand == "group-archive")?;
                    println!(
                        "{} content type group {id}.",
                        if subcommand == "group-archive" {
                            "Archived"
                        } else {
                            "Restored"
                        }
                    );
                }
                "group-delete" => {
                    let id = args.get(3).cloned().unwrap_or_else(|| {
                        eprintln!("Usage: pasted type group-delete <id>");
                        std::process::exit(2);
                    });
                    db.delete_content_type_group(&id)?;
                    println!("Deleted content type group {id}.");
                }
                "group-restore-defaults" => {
                    db.restore_default_content_type_groups()?;
                    println!("Restored built-in content type groups.");
                }
                "list" | "ls" => {
                    let types =
                        db.get_content_types(args.iter().any(|argument| argument == "--all"))?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&types).map_err(json_error)?
                        );
                    } else {
                        for item in types {
                            println!(
                                "{}\t{}\t{}\t{}",
                                item.id,
                                item.icon,
                                if item.is_archived {
                                    "archived"
                                } else {
                                    "active"
                                },
                                item.label
                            );
                        }
                    }
                }
                "create" => {
                    let id = argument_value(&args, "--id").unwrap_or_else(|| {
                        eprintln!("Usage: pasted type create --id ID --name NAME [--icon ICON] [--group GROUP] [--json]");
                        std::process::exit(2);
                    });
                    let label = argument_value(&args, "--name").unwrap_or_else(|| {
                        eprintln!("Type creation requires --name.");
                        std::process::exit(2);
                    });
                    let created = db.create_content_type(&ContentTypeInput {
                        id,
                        label,
                        icon: argument_value(&args, "--icon").unwrap_or_else(|| "FileText".into()),
                        group: argument_value(&args, "--group").unwrap_or_else(|| "custom".into()),
                    })?;
                    print_content_type(&created, json)?;
                }
                "update" => {
                    let id = args.get(3).cloned().unwrap_or_else(|| {
                        eprintln!("Usage: pasted type update <id> [--name NAME] [--icon ICON] [--group GROUP] [--json]");
                        std::process::exit(2);
                    });
                    let current = db
                        .get_content_types(true)?
                        .into_iter()
                        .find(|item| item.id == id)
                        .unwrap_or_else(|| {
                            eprintln!("Content type {id} was not found.");
                            std::process::exit(1);
                        });
                    let updated = db.update_content_type(
                        &id,
                        &ContentTypeInput {
                            id: id.clone(),
                            label: argument_value(&args, "--name").unwrap_or(current.label),
                            icon: argument_value(&args, "--icon").unwrap_or(current.icon),
                            group: argument_value(&args, "--group").unwrap_or(current.group),
                        },
                    )?;
                    print_content_type(&updated, json)?;
                }
                "archive" | "restore" => {
                    let id = args.get(3).cloned().unwrap_or_else(|| {
                        eprintln!("Usage: pasted type {subcommand} <id>");
                        std::process::exit(2);
                    });
                    db.set_content_type_archived(&id, subcommand == "archive")?;
                    println!(
                        "{} content type {id}.",
                        if subcommand == "archive" {
                            "Archived"
                        } else {
                            "Restored"
                        }
                    );
                }
                "restore-defaults" => {
                    db.restore_default_content_types()?;
                    db.restore_default_content_type_groups()?;
                    println!("Restored built-in content type names, icons, and groups.");
                }
                _ => {
                    eprintln!("Usage: pasted type list|create|update|archive|restore|restore-defaults [--json]");
                    std::process::exit(2);
                }
            }
        }
        "detector" | "detectors" => {
            drop(conn);
            let db = DbState::new(db_path.clone())?;
            let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
            match subcommand {
                "list" | "ls" => {
                    let detectors = db.get_content_detectors()?;
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&detectors).map_err(json_error)?
                        );
                    } else {
                        for detector in detectors {
                            println!(
                                "{}\t{}\t{}\t{}\t{}",
                                detector.id,
                                detector.priority,
                                if detector.enabled { "on" } else { "off" },
                                detector.content_type,
                                detector.name
                            );
                        }
                    }
                }
                "create" => {
                    let input = detector_input_from_args(&args, None);
                    let detector = db.create_content_detector(&input)?;
                    print_detector(&detector, args.iter().any(|argument| argument == "--json"))?;
                }
                "update" => {
                    let id = args.get(3).and_then(|value| value.parse::<i64>().ok()).unwrap_or_else(|| {
                        eprintln!("Usage: pasted detector update <id> [--name NAME] [--type TYPE] [--regex REGEX] [--priority N] [--disabled] [--json]");
                        std::process::exit(2);
                    });
                    let current = db
                        .get_content_detectors()?
                        .into_iter()
                        .find(|item| item.id == id)
                        .unwrap_or_else(|| {
                            eprintln!("Detector {id} was not found.");
                            std::process::exit(1);
                        });
                    let input = detector_input_from_args(&args, Some(&current));
                    let detector = db.update_content_detector(id, &input)?;
                    print_detector(&detector, args.iter().any(|argument| argument == "--json"))?;
                }
                "delete" => {
                    let id = args
                        .get(3)
                        .and_then(|value| value.parse::<i64>().ok())
                        .unwrap_or_else(|| {
                            eprintln!("Usage: pasted detector delete <id>");
                            std::process::exit(2);
                        });
                    db.delete_content_detector(id)?;
                    println!("Deleted detector {id}.");
                }
                "restore-defaults" => {
                    db.restore_default_content_detectors()?;
                    println!(
                        "Restored shipped detector defaults; custom detectors were preserved."
                    );
                }
                "rescan" => {
                    if !args.iter().any(|argument| argument == "--yes") {
                        eprintln!("History rescans can change Types, Smart Bin membership, and sensitive-content masking. Re-run with --yes to continue.");
                        std::process::exit(2);
                    }
                    let report = db.rescan_content_detection()?;
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&report).map_err(json_error)?
                        );
                    } else {
                        println!(
                            "Rescanned {} text clips; {} changed and {} were unchanged.",
                            report.scanned_count, report.changed_count, report.unchanged_count
                        );
                    }
                }
                _ => {
                    eprintln!("Usage: pasted detector list|create|update|delete|restore-defaults|rescan [--yes] [--json]");
                    std::process::exit(2);
                }
            }
        }
        "import" => {
            let Some(source_name) = args.get(2) else {
                eprintln!(
                    "Usage: pasted import <alfred|pastebot|pasta|paste|copyclip|maccy|flycut> [history-file-or-folder] [--json]"
                );
                std::process::exit(2);
            };
            let source = source_name
                .parse::<ExternalImportSource>()
                .unwrap_or_else(|error| {
                    eprintln!("{error}");
                    std::process::exit(2);
                });
            let path = args
                .get(3)
                .filter(|argument| !argument.starts_with("--"))
                .map(PathBuf::from);
            drop(conn);
            let db = DbState::new(db_path.clone())?;
            let report =
                external_import::import_history(&db, source, path).unwrap_or_else(|error| {
                    eprintln!("Import failed: {error}");
                    std::process::exit(1);
                });
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).map_err(json_error)?
                );
            } else {
                println!(
                    "Imported {} of {} {} clips; {} duplicates and {} unsupported items were skipped.",
                    report.imported_count,
                    report.scanned_count,
                    source.label(),
                    report.duplicate_count,
                    report.skipped_count
                );
                if let Some(capacity) = report.history_capacity_adjusted_to {
                    println!(
                        "Expanded the history limit to {capacity} clips so the imported history is retained."
                    );
                }
            }
        }
        "database" | "library" => {
            let app_data = get_app_data_dir();
            let subcommand = args.get(2).map(String::as_str).unwrap_or("location");
            match subcommand {
                "location" => {
                    let location = library_storage::location_info(&app_data, &db_path);
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&location).map_err(|error| {
                                rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                            })?
                        );
                    } else {
                        println!("{}", location.path);
                    }
                }
                "move" => {
                    let Some(directory) = args.get(3) else {
                        eprintln!("Usage: pasted database move <folder> [--json]");
                        std::process::exit(2);
                    };
                    let directory = fs::canonicalize(directory).unwrap_or_else(|error| {
                        eprintln!("Could not resolve the database folder: {error}");
                        std::process::exit(2);
                    });
                    let target =
                        library_storage::validate_destination_directory(&directory, &db_path)
                            .unwrap_or_else(|error| {
                                eprintln!("{error}");
                                std::process::exit(2);
                            });
                    if target == db_path {
                        println!("The database is already in that folder.");
                        return Ok(());
                    }
                    drop(conn);
                    let db = DbState::new(db_path.clone())?;
                    let previous = db.relocate_database(target.clone())?;
                    if let Err(error) = library_storage::persist_location(&app_data, &target) {
                        let _ = db.switch_to_database(previous.clone());
                        eprintln!("{error}");
                        std::process::exit(1);
                    }
                    let location = library_storage::location_info(&app_data, &target);
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "location": location,
                                "recoveryPath": previous,
                            }))
                            .map_err(|error| {
                                rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                            })?
                        );
                    } else {
                        println!("Moved the database to {}.", location.path);
                        println!("Previous database retained at {}.", previous.display());
                    }
                }
                "default" => {
                    let target = library_storage::default_database_path(&app_data);
                    if target == db_path {
                        println!("The database is already in its default location.");
                        return Ok(());
                    }
                    let archived_default = library_storage::archive_existing_database(&target)
                        .unwrap_or_else(|error| {
                            eprintln!("{error}");
                            std::process::exit(1);
                        });
                    drop(conn);
                    let db = DbState::new(db_path.clone())?;
                    let previous = match db.relocate_database(target.clone()) {
                        Ok(previous) => previous,
                        Err(error) => {
                            if let Some(archived) = archived_default.as_deref() {
                                library_storage::restore_archived_database(archived, &target);
                            }
                            return Err(error);
                        }
                    };
                    if let Err(error) = library_storage::persist_location(&app_data, &target) {
                        let _ = db.switch_to_database(previous.clone());
                        let _ = fs::remove_file(&target);
                        if let Some(archived) = archived_default.as_deref() {
                            library_storage::restore_archived_database(archived, &target);
                        }
                        eprintln!("{error}");
                        std::process::exit(1);
                    }
                    let location = library_storage::location_info(&app_data, &target);
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "location": location,
                                "recoveryPath": previous,
                            }))
                            .map_err(|error| {
                                rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                            })?
                        );
                    } else {
                        println!("Restored the default database location.");
                        println!("Custom database retained at {}.", previous.display());
                    }
                }
                _ => {
                    eprintln!("Usage: pasted database location|move <folder>|default [--json]");
                    std::process::exit(2);
                }
            }
        }
        "diagnostics" | "diagnose" => {
            let executable = env::current_exe().unwrap_or_else(|_| PathBuf::from("pasted"));
            let app_path = executable
                .ancestors()
                .find(|path| path.extension().is_some_and(|extension| extension == "app"))
                .map(PathBuf::from)
                .unwrap_or(executable);
            let data_path = db_path
                .parent()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("./pasted_data"));
            let details = InstallationDiagnostics::collect_with_database(
                app_path,
                data_path,
                db_path.clone(),
            );
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&details).map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                    })?
                );
            } else {
                println!("{}", details.plain_text());
            }
        }
        "ocr" => {
            let db = DbState::new(db_path.clone())?;
            let ocr_setting = db.get_setting(Feature::Ocr.setting_key())?;
            if !setting_value_is_enabled(ocr_setting.as_deref()) {
                eprintln!("OCR is disabled in Settings → Functionality.");
                std::process::exit(1);
            }
            let subcommand = args.get(2).map(String::as_str).unwrap_or("status");
            match subcommand {
                "status" => {
                    let status = db.get_ocr_backfill_status()?;
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::to_string(&status).map_err(|error| {
                                rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                            })?
                        );
                    } else {
                        println!(
                            "{} images · {} waiting · {} running · {} complete · {} no text · {} failed",
                            status.total_images,
                            status.eligible_count,
                            status.running_count,
                            status.completed_count,
                            status.no_text_count,
                            status.failed_count
                        );
                    }
                }
                "scan" => {
                    let mut scanned = 0usize;
                    while let Some(candidate) = db.claim_next_ocr_candidate()? {
                        let Some(bytes) =
                            pasted_lib::ocr::decode_stored_image(&candidate.image_base64)
                        else {
                            db.complete_ocr_attempt(
                                candidate.clip_id,
                                &candidate.content_hash,
                                None,
                                "macos-vision-v1",
                                Some("invalid_image_data"),
                            )?;
                            continue;
                        };
                        let text = pasted_lib::ocr::perform_ocr_on_image_bytes(&bytes);
                        db.complete_ocr_attempt(
                            candidate.clip_id,
                            &candidate.content_hash,
                            text.as_deref(),
                            "macos-vision-v1",
                            None,
                        )?;
                        scanned += 1;
                    }
                    println!(
                        "Scanned {scanned} existing image{}.",
                        if scanned == 1 { "" } else { "s" }
                    );
                }
                _ => {
                    eprintln!("Usage: pasted ocr [status [--json] | scan]");
                    std::process::exit(2);
                }
            }
        }
        "transform" | "transforms" => {
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
                    print_transform_definition(
                        &definition,
                        args.iter().any(|arg| arg == "--json"),
                    )?;
                }
                "create" | "new" => {
                    let name = argument_value(&args, "--name").unwrap_or_else(|| {
                        eprintln!("Usage: pasted transform create --name NAME (--plan-json JSON | --steps-json JSON) [--connection ID | --shortcut HOTKEY] [--json]");
                        std::process::exit(2);
                    });
                    if name.trim().is_empty() {
                        eprintln!("Transform name cannot be empty.");
                        std::process::exit(2);
                    }
                    let plan_json = argument_value(&args, "--plan-json");
                    let steps_json = argument_value(&args, "--steps-json");
                    let definition = match (plan_json, steps_json) {
                        (Some(plan_json), None) => {
                            let plan: TransformationPlan =
                                serde_json::from_str(&plan_json).map_err(json_error)?;
                            TransformDefinition::from(db.create_saved_transform(
                                &name,
                                &plan,
                                argument_value(&args, "--connection").as_deref(),
                            )?)
                        }
                        (None, Some(steps_json)) => {
                            let steps: Vec<PipelineStepInput> =
                                serde_json::from_str(&steps_json).map_err(json_error)?;
                            TransformDefinition::from(db.create_pipeline(
                                &name,
                                &steps,
                                argument_value(&args, "--shortcut").as_deref(),
                            )?)
                        }
                        _ => {
                            eprintln!("Provide exactly one of --plan-json or --steps-json.");
                            std::process::exit(2);
                        }
                    };
                    print_transform_definition(
                        &definition,
                        args.iter().any(|arg| arg == "--json"),
                    )?;
                }
                "update" | "edit" => {
                    let transform_ref = args.get(3).unwrap_or_else(|| {
                        eprintln!("Usage: pasted transform update <transform-ref> [--name NAME] [--plan-json JSON | --steps-json JSON] [--connection ID | --clear-connection] [--shortcut HOTKEY | --clear-shortcut] [--json]");
                        std::process::exit(2);
                    });
                    let current = db
                        .resolve_transform_definition(transform_ref)?
                        .unwrap_or_else(|| {
                            eprintln!("Transform {transform_ref} was not found.");
                            std::process::exit(1);
                        });
                    let name =
                        argument_value(&args, "--name").unwrap_or_else(|| current.name.clone());
                    if name.trim().is_empty() {
                        eprintln!("Transform name cannot be empty.");
                        std::process::exit(2);
                    }
                    let updated = match current.authoring_kind {
                        TransformAuthoringKind::Intent => {
                            if argument_value(&args, "--steps-json").is_some()
                                || argument_value(&args, "--shortcut").is_some()
                                || args.iter().any(|arg| arg == "--clear-shortcut")
                            {
                                eprintln!("Intent-authored Transforms accept --plan-json and connection options; use duplicate/create to change authoring form.");
                                std::process::exit(2);
                            }
                            let plan = match argument_value(&args, "--plan-json") {
                                Some(plan_json) => {
                                    serde_json::from_str::<TransformationPlan>(&plan_json)
                                        .map_err(json_error)?
                                }
                                None => current.plan.clone().expect("saved Transform has a plan"),
                            };
                            let connection_id =
                                if args.iter().any(|arg| arg == "--clear-connection") {
                                    None
                                } else {
                                    argument_value(&args, "--connection")
                                        .or(current.connection_id.clone())
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
                                eprintln!("Manually built Transforms accept --steps-json and shortcut options; use duplicate/create to change authoring form.");
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
                            let shortcut = if args.iter().any(|arg| arg == "--clear-shortcut") {
                                None
                            } else {
                                argument_value(&args, "--shortcut").or(current.shortcut.clone())
                            };
                            TransformDefinition::from(db.update_pipeline(
                                transform_ref,
                                &name,
                                &steps,
                                shortcut.as_deref(),
                            )?)
                        }
                    };
                    print_transform_definition(&updated, args.iter().any(|arg| arg == "--json"))?;
                }
                "duplicate" | "copy" => {
                    let transform_ref = args.get(3).unwrap_or_else(|| {
                        eprintln!("Usage: pasted transform duplicate <transform-ref> [--name NAME] [--json]");
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
                        eprintln!("Usage: pasted transform run <transform-ref> [--text TEXT | --clip ID | --stdin] [--replace]");
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
                    let replace = args.iter().any(|arg| arg == "--replace");
                    if replace && clip_id.is_none() {
                        eprintln!("--replace requires --clip ID so Pasted can create a revision.");
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
                                    eprintln!(
                                        "Transform ran, but the clip was not replaced: {error}"
                                    );
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
                    eprintln!("Usage: pasted transform list|get|create|update|duplicate|delete|run [options] [--json]");
                    std::process::exit(2);
                }
            }
        }
        "operation" | "operations" => {
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
                                operation.stable_id,
                                operation.name,
                                operation.op_type,
                                operation.category
                            );
                        }
                    }
                }
                "run" => run_operation(&args, &db),
                _ => {
                    eprintln!("Usage: pasted operation [list [--json] | run <operation-ref> [--text TEXT | --clip ID | --stdin] [--json]]");
                    std::process::exit(2);
                }
            }
        }
        "bin" | "bins" => {
            let db = DbState::new(db_path.clone())?;
            let bins_setting = db.get_setting(Feature::Bins.setting_key())?;
            if !setting_value_is_enabled(bins_setting.as_deref()) {
                eprintln!("Bins are disabled in Settings → Functionality.");
                std::process::exit(1);
            }
            let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
            match subcommand {
                "list" | "ls" => {
                    let bins = db.get_bins()?;
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&bins).map_err(|error| {
                                rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                            })?
                        );
                    } else {
                        for bin in bins {
                            println!(
                                "{}\t{}\t{} clips",
                                bin.id,
                                bin.name,
                                bin.clip_count.unwrap_or(0)
                            );
                        }
                    }
                }
                "clips" => {
                    let Some(bin_id) = args.get(3).and_then(|value| value.parse::<i64>().ok())
                    else {
                        eprintln!("Usage: pasted bin clips <bin-id> [--json]");
                        std::process::exit(2);
                    };
                    let clips = db.get_clips(None, Some(bin_id), false)?;
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&clips).map_err(|error| {
                                rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                            })?
                        );
                    } else {
                        for (position, clip) in clips.iter().enumerate() {
                            println!(
                                "{}\t{}\t{}",
                                position + 1,
                                clip.id,
                                clip.text_content.as_deref().unwrap_or("")
                            );
                        }
                    }
                }
                "order" => {
                    let Some(bin_id) = args.get(3).and_then(|value| value.parse::<i64>().ok())
                    else {
                        eprintln!("Usage: pasted bin order <bin-id> <clip-id>... [--json]");
                        std::process::exit(2);
                    };
                    let clip_ids = args
                        .iter()
                        .skip(4)
                        .filter(|argument| argument.as_str() != "--json")
                        .map(|value| value.parse::<i64>())
                        .collect::<Result<Vec<_>, _>>()
                        .unwrap_or_else(|_| {
                            eprintln!("Every clip ID must be an integer.");
                            std::process::exit(2);
                        });
                    db.reorder_bin_clips(bin_id, clip_ids.clone())?;
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::json!({ "binId": bin_id, "clipIds": clip_ids })
                        );
                    } else {
                        println!("Reordered {} clips in Bin #{bin_id}.", clip_ids.len());
                    }
                }
                _ => {
                    eprintln!("Usage: pasted bin [list | clips <bin-id> | order <bin-id> <clip-id>...] [--json]");
                    std::process::exit(2);
                }
            }
        }
        "clip" | "clips" => {
            let db = DbState::new(db_path.clone())?;
            let subcommand = args.get(2).map(String::as_str).unwrap_or("help");
            let json = args.iter().any(|argument| argument == "--json");
            match subcommand {
                "export" => {
                    let path = args
                        .get(3)
                        .filter(|argument| !argument.starts_with("--"))
                        .map(PathBuf::from);
                    let format = argument_value(&args, "--format").unwrap_or_else(|| {
                        path.as_ref()
                            .and_then(|value| value.extension())
                            .and_then(|value| value.to_str())
                            .unwrap_or("json")
                            .to_ascii_lowercase()
                    });
                    let contents = match format.as_str() {
                        "json" => db.export_clips_json()?,
                        "csv" => db.export_clips_csv()?,
                        _ => {
                            eprintln!("Clip export format must be json or csv.");
                            std::process::exit(2);
                        }
                    };
                    if let Some(path) = path {
                        fs::write(&path, contents).map_err(|error| {
                            rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                        })?;
                        if json {
                            println!("{}", serde_json::json!({ "format": format, "path": path }));
                        } else {
                            println!("Exported clips in History to {}.", path.display());
                        }
                    } else {
                        print!("{contents}");
                    }
                }
                "import" => {
                    let Some(path) = args.get(3).filter(|argument| !argument.starts_with("--"))
                    else {
                        eprintln!("Usage: pasted clip import <path> [--format json|csv] [--json]");
                        std::process::exit(2);
                    };
                    let format = argument_value(&args, "--format").unwrap_or_else(|| {
                        Path::new(path)
                            .extension()
                            .and_then(|value| value.to_str())
                            .unwrap_or("json")
                            .to_ascii_lowercase()
                    });
                    let contents = read_library_archive(Path::new(path))?;
                    let report = match format.as_str() {
                        "json" => db.import_clips_json(&contents)?,
                        "csv" => db.import_clips_csv(&contents)?,
                        _ => {
                            eprintln!("Clip import format must be json or csv.");
                            std::process::exit(2);
                        }
                    };
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&report).map_err(json_error)?
                        );
                    } else {
                        println!(
                            "Imported {} clips; skipped {} duplicates.",
                            report.imported_count, report.duplicate_count
                        );
                    }
                }
                "get" | "show" => {
                    let Some(clip_id) = args.get(3).and_then(|value| value.parse::<i64>().ok())
                    else {
                        eprintln!("Usage: pasted clip get <clip-id> [--json]");
                        std::process::exit(2);
                    };
                    let clip = db.get_clip_by_id(clip_id)?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&clip).map_err(json_error)?
                        );
                    } else {
                        println!(
                            "#{}\t{}\t{}\t{}",
                            clip.id,
                            clip.content_type,
                            clip.source,
                            clip.text_content.as_deref().unwrap_or("")
                        );
                    }
                }
                "pin" | "unpin" => {
                    require_feature(&db, Feature::Pinning);
                    let ids = parse_clip_ids(&args, 3);
                    let summary = db.batch_pin_clips(ids, subcommand == "pin")?;
                    print_mutation_summary(&summary, json)?;
                }
                "protect" | "unprotect" => {
                    require_feature(&db, Feature::Protection);
                    let ids = parse_clip_ids(&args, 3);
                    let summary = db.batch_protect_clips(ids, subcommand == "protect")?;
                    print_mutation_summary(&summary, json)?;
                }
                "trash" => {
                    require_feature(&db, Feature::Trash);
                    let summary = db.batch_trash_clips(parse_clip_ids(&args, 3))?;
                    print_mutation_summary(&summary, json)?;
                }
                "restore" => {
                    require_feature(&db, Feature::Trash);
                    let ids = parse_clip_ids(&args, 3);
                    let requested_count = ids.len();
                    let mut changed_ids = Vec::new();
                    for id in ids {
                        changed_ids.extend(db.restore_clip(id)?.clip_ids);
                    }
                    let summary = ClipMutationSummary {
                        action: "restore".to_string(),
                        requested_count,
                        changed_count: changed_ids.len(),
                        skipped_count: requested_count.saturating_sub(changed_ids.len()),
                        clip_ids: changed_ids,
                    };
                    print_mutation_summary(&summary, json)?;
                }
                "restore-all" => {
                    require_feature(&db, Feature::Trash);
                    let summary = db.restore_all_trashed_clips()?;
                    print_mutation_summary(&summary, json)?;
                }
                "assign" => {
                    require_feature(&db, Feature::Bins);
                    let Some(destination) = args.get(3) else {
                        eprintln!("Usage: pasted clip assign <bin-id|none> <clip-id>... [--json]");
                        std::process::exit(2);
                    };
                    let bin_id = if matches!(destination.as_str(), "none" | "null" | "-") {
                        None
                    } else {
                        destination.parse::<i64>().ok().or_else(|| {
                            eprintln!("Bin ID must be an integer or 'none'.");
                            std::process::exit(2);
                        })
                    };
                    let outcome = assign_clips_to_bin(&db, parse_clip_ids(&args, 4), bin_id)
                        .map_err(|error| {
                            rusqlite::Error::ToSqlConversionFailure(Box::new(io::Error::other(
                                error,
                            )))
                        })?;
                    print_mutation_summary(&outcome.mutation, json)?;
                }
                "remove-bin" => {
                    require_feature(&db, Feature::Bins);
                    let Some(bin_id) = args.get(3).and_then(|value| value.parse::<i64>().ok())
                    else {
                        eprintln!("Usage: pasted clip remove-bin <bin-id> <clip-id>... [--json]");
                        std::process::exit(2);
                    };
                    let outcome = pasted_lib::bin_assignment::remove_clips_from_bin(
                        &db,
                        parse_clip_ids(&args, 4),
                        bin_id,
                    )
                    .map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(io::Error::other(error)))
                    })?;
                    print_mutation_summary(&outcome.mutation, json)?;
                }
                _ => {
                    eprintln!("Usage: pasted clip [export [path] [--format json|csv] | import <path> [--format json|csv] | get <id> | pin|unpin|protect|unprotect|trash|restore <id>... | restore-all | assign <bin-id|none> <id>... | remove-bin <bin-id> <id>...] [--json]");
                    std::process::exit(2);
                }
            }
        }
        "copy" | "add" => {
            let capture_limit = configured_capture_bytes(&conn);
            let text = if let Some(arg_text) = args.get(2) {
                arg_text.clone()
            } else {
                read_stdin_bounded(capture_limit)?
            };

            let trimmed = text.trim().to_string();
            if trimmed.is_empty() {
                eprintln!("Error: Cannot copy empty content.");
                std::process::exit(1);
            }
            if trimmed.len() > capture_limit {
                eprintln!(
                    "Error: Content exceeds the configured {} MB clip limit.",
                    capture_limit / 1024 / 1024
                );
                std::process::exit(1);
            }

            let detection_db = DbState::new(db_path.clone())?;
            let content_type = if setting_value_is_enabled(
                detection_db
                    .get_setting(Feature::ContentDetection.setting_key())?
                    .as_deref(),
            ) {
                detect_with_detectors(&trimmed, &detection_db.get_content_detectors()?)
            } else {
                "text".to_string()
            };
            drop(detection_db);
            conn.execute(
                "INSERT INTO clips (content_type, text_content, source, created_at) VALUES (?1, ?2, 'CLI Terminal', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
                params![content_type, trimmed],
            )?;
            let id = conn.last_insert_rowid();

            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::json!({ "id": id, "content_type": content_type })
                );
            } else {
                println!("✓ Saved {content_type} clip #{id} to Pasted history.");
            }
        }
        "list" | "ls" => {
            let limit: i64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);
            let mut stmt = conn.prepare(
                "SELECT id, content_type, text_content, source, created_at FROM clips WHERE is_trashed = 0 ORDER BY created_at DESC LIMIT ?1"
            )?;
            let rows = stmt.query_map(params![limit], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?;

            println!(
                "{:<5} | {:<8} | {:<15} | {:<20} | CONTENT",
                "ID", "TYPE", "SOURCE", "DATE"
            );
            println!(
                "{:-<5}-+-{:-<8}-+-{:-<15}-+-{:-<20}-+-{:-<30}",
                "", "", "", "", ""
            );

            for r in rows {
                let (id, c_type, content, source, date) = r?;
                let snippet: String = content
                    .lines()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(40)
                    .collect();
                println!(
                    "{:<5} | {:<8} | {:<15} | {:<20} | {}",
                    id, c_type, source, date, snippet
                );
            }
        }
        "search" | "find" => {
            let option_value = |name: &str| {
                args.iter()
                    .position(|argument| argument == name)
                    .and_then(|index| args.get(index + 1))
                    .cloned()
            };
            let content_type = option_value("--type");
            let source = option_value("--source");
            let json = args.iter().any(|argument| argument == "--json");
            let query = args
                .iter()
                .skip(2)
                .take_while(|argument| !argument.starts_with("--"))
                .cloned()
                .collect::<Vec<_>>()
                .join(" ");
            let pattern = format!("%{}%", query);
            let mut stmt = conn.prepare(
                "SELECT id, content_type, text_content, source, created_at
                 FROM clips
                 WHERE is_trashed = 0
                   AND (?1 = '' OR text_content LIKE ?2)
                   AND (?3 IS NULL OR content_type = ?3)
                   AND (?4 IS NULL OR source = ?4)
                 ORDER BY created_at DESC
                 LIMIT 20",
            )?;
            let rows = stmt
                .query_map(params![query, pattern, content_type, source], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;

            if json {
                let payload = rows
                    .into_iter()
                    .map(|(id, content_type, content, source, created_at)| {
                        serde_json::json!({
                            "id": id,
                            "content_type": content_type,
                            "text_content": content,
                            "source": source,
                            "created_at": created_at,
                        })
                    })
                    .collect::<Vec<_>>();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&payload).map_err(json_error)?
                );
            } else {
                for (id, c_type, content, source, date) in rows {
                    println!("[#{id}] ({c_type} from {source} @ {date}):\n{content}\n---");
                }
            }
        }
        "clear" => {
            drop(conn);
            let db = DbState::new(db_path.clone())?;
            db.purge_unpinned_clips()?;
            println!("✓ Cleared unpinned clipboard history via CLI.");
        }
        "reset" => {
            if !args.iter().any(|argument| argument == "--yes") {
                eprintln!(
                    "Refusing to reset without --yes. Quit Pasted first, and export a backup if you may need this data."
                );
                std::process::exit(2);
            }
            drop(conn);
            let db = DbState::new(db_path.clone())?;
            let report = db.factory_reset()?;
            if let Some(cache_directory) = dirs::cache_dir() {
                let app_cache = cache_directory.join(APP_IDENTIFIER);
                if app_cache.exists() {
                    let _ = fs::remove_dir_all(app_cache);
                }
            }
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                    })?
                );
            } else {
                println!(
                    "Reset Pasted: removed {} clips, {} bins, {} Transforms, {} connections, and {} activity entries.",
                    report.clips_deleted,
                    report.bins_deleted,
                    report.transforms_deleted,
                    report.connections_deleted,
                    report.activity_entries_deleted
                );
            }
        }
        _ => {
            println!("Pasted CLI Tool (v{})", env!("CARGO_PKG_VERSION"));
            println!("Usage:");
            println!("  pasted copy <text> [--json] Detect and save content, or pipe stdin");
            println!("  pasted list [limit]      List N recent clipboard items (default: 10)");
            println!("  pasted search [query] [--type <type>] [--source <app>] [--json]");
            println!("  pasted import <source> [path] --json Import history from Alfred, Pastebot, Pasta, Paste, CopyClip 2, Maccy, or Flycut");
            println!("  pasted diagnostics --json Show installation diagnostics");
            println!("  pasted licenses [--json] Show bundled open-source licenses and notices");
            println!("  pasted retention [--count N|unlimited] [--days N|forever] [--json]");
            println!("                   [--trash-count N|unlimited] [--trash-days N|forever]");
            println!("                   [--log-count N|unlimited] [--log-days N|forever]");
            println!("  pasted activity list [--limit N|--all] [--json]");
            println!("  pasted activity export [path] [--format json|csv]");
            println!("  pasted activity import <path> [--format json|csv] [--json]");
            println!("  pasted activity clear --yes [--json]");
            println!("  pasted transfer export|inspect|import <path.json> [--json]");
            println!("  pasted backup create <path.pastedbackup> [--json]");
            println!("  pasted backup restore <path.pastedbackup> --yes [--json]");
            println!("  pasted clip export [path] [--format json|csv]");
            println!("  pasted clip import <path> [--format json|csv] [--json]");
            println!("  pasted detector list --json List editable content detectors");
            println!("  pasted type list --json List registered content types");
            println!(
                "  pasted registry list [--kind KIND] [--json] List shared processing metadata"
            );
            println!("  pasted registry enable|disable --kind KIND --ref REF");
            println!("  pasted detector create|update|delete Manage content detectors");
            println!("  pasted detector rescan --yes --json Reclassify existing text clips");
            println!("  pasted database location --json Show the active SQLite database");
            println!(
                "  pasted database move <folder> --json Move the database (quit Pasted first)"
            );
            println!("  pasted database default --json Restore the native default location");
            println!("  pasted ocr status --json Show OCR background-work status");
            println!("  pasted ocr scan          Scan existing unprocessed images");
            println!("  pasted transform list [--json] List saved and manually built Transforms");
            println!("  pasted transform get|create|update|duplicate|delete Manage either Transform authoring form");
            println!(
                "  pasted transform run <ref> [--text TEXT | --clip ID | --stdin] [--replace]"
            );
            println!("  pasted operation list|run Experimental Operation inspection and execution");
            println!("  pasted bin list --json   List Bins and their saved clip order");
            println!("  pasted bin clips <id> --json List clips in persistent Bin order");
            println!("  pasted bin order <id> <clip-id>... Persist a complete Bin order");
            println!("  pasted clip get <id> --json Inspect one clip");
            println!("  pasted clip pin|unpin <id>... [--json]");
            println!("  pasted clip protect|unprotect <id>... [--json]");
            println!("  pasted clip trash|restore <id>... [--json]");
            println!("  pasted clip restore-all [--json] Restore every clip from Trash");
            println!("  pasted clip assign <bin-id|none> <id>... [--json] Add to a Bin, or clear all manual Bins");
            println!("  pasted clip remove-bin <bin-id> <id>... [--json]");
            println!("  pasted clear             Clear unpinned clipboard history");
            println!("  pasted reset --yes [--json] Reset all Pasted data and preferences");
        }
    }

    Ok(())
}

fn json_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

fn require_feature(db: &DbState, feature: Feature) {
    let enabled = db.get_setting(feature.setting_key()).ok().flatten();
    if !setting_value_is_enabled(enabled.as_deref()) {
        eprintln!(
            "{} is disabled in Settings → Functionality.",
            feature.label()
        );
        std::process::exit(1);
    }
}

fn run_operation(args: &[String], db: &DbState) {
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

fn parse_clip_ids(args: &[String], start: usize) -> Vec<i64> {
    let ids = args
        .iter()
        .skip(start)
        .filter(|argument| argument.as_str() != "--json")
        .map(|value| value.parse::<i64>())
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|_| {
            eprintln!("Every clip ID must be an integer.");
            std::process::exit(2);
        });
    if ids.is_empty() {
        eprintln!("Provide at least one clip ID.");
        std::process::exit(2);
    }
    ids
}

fn print_mutation_summary(summary: &ClipMutationSummary, json: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(summary).map_err(json_error)?
        );
    } else {
        println!(
            "{}: {} changed, {} skipped.",
            summary.action, summary.changed_count, summary.skipped_count
        );
    }
    Ok(())
}
fn configured_capture_bytes(conn: &Connection) -> usize {
    let configured = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'maxClipSizeMb'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok();
    pasted_lib::resource_limits::configured_clip_capture_bytes(configured.as_deref())
}

fn argument_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|argument| argument == flag)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn parse_retention_argument(
    args: &[String],
    flag: &str,
    unlimited_label: &str,
    maximum: i64,
) -> Option<i64> {
    let value = argument_value(args, flag)?;
    if value.eq_ignore_ascii_case(unlimited_label) {
        return Some(0);
    }
    match value.parse::<i64>() {
        Ok(value) if (0..=maximum).contains(&value) => Some(value),
        _ => {
            eprintln!("{flag} must be {unlimited_label} or a number from 0 to {maximum}.");
            std::process::exit(2);
        }
    }
}

fn setting_i64(db: &DbState, key: &str, fallback: i64) -> Result<i64> {
    Ok(db
        .get_setting(key)?
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(fallback))
}

fn retention_count_label(value: i64, unit: &str) -> String {
    if value == 0 {
        "Unlimited".to_string()
    } else {
        format!("{value} {unit}")
    }
}

fn retention_age_label(value: i64) -> String {
    if value == 0 {
        "Forever".to_string()
    } else {
        format!("{value} days")
    }
}

fn argument_values(args: &[String], flag: &str) -> Vec<String> {
    args.iter()
        .enumerate()
        .filter_map(|(index, argument)| {
            (argument == flag)
                .then(|| args.get(index + 1).cloned())
                .flatten()
        })
        .collect()
}

fn print_transform_definition(definition: &TransformDefinition, json: bool) -> Result<()> {
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

fn detector_input_from_args(
    args: &[String],
    current: Option<&pasted_lib::content_detection::Detector>,
) -> DetectorInput {
    let patterns = argument_values(args, "--regex");
    DetectorInput {
        name: argument_value(args, "--name").unwrap_or_else(|| {
            current
                .map(|item| item.name.clone())
                .unwrap_or_else(|| "Custom Detector".into())
        }),
        content_type: argument_value(args, "--type").unwrap_or_else(|| {
            current
                .map(|item| item.content_type.clone())
                .unwrap_or_else(|| "text".into())
        }),
        description: argument_value(args, "--description").unwrap_or_else(|| {
            current
                .map(|item| item.description.clone())
                .unwrap_or_default()
        }),
        patterns: if patterns.is_empty() {
            current
                .map(|item| item.patterns.clone())
                .unwrap_or_else(|| vec!["^.+$".into()])
        } else {
            patterns
        },
        validator: argument_value(args, "--validator")
            .map(|value| (value != "none").then_some(value))
            .unwrap_or_else(|| current.and_then(|item| item.validator.clone())),
        enabled: if args.iter().any(|argument| argument == "--disabled") {
            false
        } else if args.iter().any(|argument| argument == "--enabled") {
            true
        } else {
            current.map(|item| item.enabled).unwrap_or(true)
        },
        priority: argument_value(args, "--priority")
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| current.map(|item| item.priority).unwrap_or(200)),
    }
}

fn print_detector(detector: &pasted_lib::content_detection::Detector, json: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(detector).map_err(json_error)?
        );
    } else {
        println!(
            "Saved detector #{}: {} ({})",
            detector.id, detector.name, detector.content_type
        );
    }
    Ok(())
}

fn print_content_type(
    content_type: &pasted_lib::content_types::ContentTypeDefinition,
    json: bool,
) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(content_type).map_err(json_error)?
        );
    } else {
        println!(
            "Saved content type {}: {}",
            content_type.id, content_type.label
        );
    }
    Ok(())
}

fn read_stdin_bounded(maximum: usize) -> Result<String> {
    let mut buffer = String::new();
    io::stdin()
        .take((maximum + 1) as u64)
        .read_to_string(&mut buffer)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    if buffer.len() > maximum {
        return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "stdin exceeds Pasted's {} MB safety limit",
                    maximum / 1024 / 1024
                ),
            ),
        )));
    }
    Ok(buffer)
}
