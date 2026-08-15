use rusqlite::{params, Connection, OptionalExtension, Result};
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use pasted_lib::bin_assignment::assign_clips_to_bin;
use pasted_lib::content_detection::DetectorInput;
use pasted_lib::content_extraction::{ExtractorDefinitionInput, APPLE_VISION_ENGINE};
use pasted_lib::content_types::{ContentTypeGroupInput, ContentTypeInput};
use pasted_lib::db::{
    ClipMutationSummary, DbState, IntelligenceConnectionUpdate, PipelineStepInput,
    TransformAuthoringKind, TransformClipApplication, TransformDefinition,
};
use pasted_lib::external_import::{self, ExternalImportSource};
use pasted_lib::features::{setting_value_is_enabled, Feature};
use pasted_lib::installation_diagnostics::{InstallationDiagnostics, APP_IDENTIFIER};
use pasted_lib::intelligence_executor::{ExecutePlanRequest, PlanIntentOutcome, PlanIntentRequest};
use pasted_lib::library_storage;
use pasted_lib::third_party_licenses;
use pasted_lib::transformation_intent::{IntentPlanningMode, TransformationPlan};
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
    if let Some(path) = env::var_os("PASTED_CONFIG_DIR") {
        return PathBuf::from(path);
    }
    dirs::config_dir()
        .map(|mut dir| {
            dir.push(APP_IDENTIFIER);
            dir
        })
        .unwrap_or_else(get_app_data_dir)
}

fn get_db_path() -> PathBuf {
    if let Some(path) = env::var_os("PASTED_DATABASE_PATH") {
        return PathBuf::from(path);
    }
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
            let revision_count =
                parse_retention_argument(&args, "--revision-count", "unlimited", 10_000)
                    .unwrap_or(setting_i64(&db, "revisionHistoryLimit", 10)?);
            let history_changed = args
                .iter()
                .any(|argument| argument == "--count" || argument == "--days");
            let trash_changed = args
                .iter()
                .any(|argument| argument == "--trash-count" || argument == "--trash-days");
            let activity_changed = args
                .iter()
                .any(|argument| argument == "--log-count" || argument == "--log-days");
            let revisions_changed = args.iter().any(|argument| argument == "--revision-count");
            if history_changed {
                db.configure_clip_retention(count, age_days)?;
            }
            if trash_changed {
                db.configure_trash_retention(trash_count, trash_age_days)?;
            }
            if activity_changed {
                db.configure_activity_retention(activity_count, activity_age_days)?;
            }
            if revisions_changed {
                db.enforce_revision_retention(revision_count)?;
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
                        "revisionsPerClip": revision_count,
                        "revisionsUnlimited": revision_count == 0,
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
                    "History: {count_label}; {age_label}\nTrash: {}; {}\nActivity: {}; {}\nRevisions: {}",
                    retention_count_label(trash_count, "clips"),
                    retention_age_label(trash_age_days),
                    retention_count_label(activity_count, "entries"),
                    retention_age_label(activity_age_days),
                    retention_count_label(revision_count, "per clip"),
                );
            }
        }
        "settings" | "setting" => {
            drop(conn);
            let db = DbState::new(db_path.clone())?;
            let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
            let json = args.iter().any(|argument| argument == "--json");
            match subcommand {
                "list" | "ls" => {
                    let mut values = db.get_all_settings()?;
                    values.remove("pendingFullBackupClientState");
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
                    if key == "pendingFullBackupClientState" {
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
                    if key == "pendingFullBackupClientState" || key.trim().is_empty() {
                        eprintln!("That setting cannot be changed through the CLI.");
                        std::process::exit(2);
                    }
                    if key.len() > 128 || value.len() > 1_048_576 {
                        eprintln!(
                            "Setting keys and values must remain within their safety limits."
                        );
                        std::process::exit(2);
                    }
                    let previous = db.get_setting(key)?;
                    db.save_setting(key, value)?;
                    if let Some(activity) = pasted_lib::settings_activity::describe_setting_change(
                        key,
                        previous.as_deref(),
                        value,
                    ) {
                        let _ = db.log_activity(activity.event_type, &activity.description);
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
        }
        "recording" | "capture" => {
            let subcommand = args.get(2).map(String::as_str).unwrap_or("status");
            let action = match subcommand {
                "status" => pasted_lib::live_app::LiveAppAction::ClipboardStatus,
                "pause" => pasted_lib::live_app::LiveAppAction::ClipboardSetPaused { paused: true },
                "resume" => {
                    pasted_lib::live_app::LiveAppAction::ClipboardSetPaused { paused: false }
                }
                _ => {
                    eprintln!("Usage: pasted recording status|pause|resume [--json]");
                    std::process::exit(2);
                }
            };
            let result = send_live_or_exit(action);
            print_live_result(&result, args.iter().any(|argument| argument == "--json"))?;
        }
        "queue" => {
            let subcommand = args.get(2).map(String::as_str).unwrap_or("status");
            let action = match subcommand {
                "status" => pasted_lib::live_app::LiveAppAction::QueueStatus,
                "start" => pasted_lib::live_app::LiveAppAction::QueueStart,
                "stop" => pasted_lib::live_app::LiveAppAction::QueueStop,
                "add" => pasted_lib::live_app::LiveAppAction::QueueAddClips {
                    clip_ids: parse_clip_ids(&args, 3),
                },
                "remove" => pasted_lib::live_app::LiveAppAction::QueueRemove {
                    index: args
                        .get(3)
                        .and_then(|value| value.parse::<usize>().ok())
                        .unwrap_or_else(|| {
                            eprintln!("Usage: pasted queue remove <zero-based-index> [--json]");
                            std::process::exit(2);
                        }),
                },
                "order" => pasted_lib::live_app::LiveAppAction::QueueReorder {
                    item_ids: args
                        .iter()
                        .skip(3)
                        .filter(|argument| argument.as_str() != "--json")
                        .map(|value| value.parse::<u64>())
                        .collect::<std::result::Result<Vec<_>, _>>()
                        .unwrap_or_else(|_| {
                            eprintln!("Every Queue item ID must be an integer.");
                            std::process::exit(2);
                        }),
                },
                "paste" => pasted_lib::live_app::LiveAppAction::QueuePaste {
                    index: args
                        .get(3)
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(0),
                },
                "paste-all" => pasted_lib::live_app::LiveAppAction::QueuePasteAll,
                _ => {
                    eprintln!("Usage: pasted queue status|start|stop|add|remove|order|paste|paste-all [arguments] [--json]");
                    std::process::exit(2);
                }
            };
            let result = send_live_or_exit(action);
            print_live_result(&result, args.iter().any(|argument| argument == "--json"))?;
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
                    let offset = argument_value(&args, "--offset")
                        .and_then(|value| value.parse::<i64>().ok())
                        .unwrap_or(0)
                        .max(0);
                    let logs = db.get_activity_logs_filtered(
                        Some(limit),
                        Some(offset),
                        argument_value(&args, "--category").as_deref(),
                        argument_value(&args, "--severity").as_deref(),
                        argument_value(&args, "--event").as_deref(),
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
                    eprintln!("Usage: pasted activity list [--limit N|--all] [--offset N] [--category VALUE] [--severity VALUE] [--event NAME] [--json] | export [path] [--format json|csv] | import <activity.json> [--json] | clear --yes [--json]");
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
                eprintln!("Usage: pasted backup create|inspect|restore <path.pastedbackup> [--yes] [--json]");
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
                    eprintln!("Usage: pasted backup create|inspect|restore <path.pastedbackup> [--yes] [--json]");
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
                } else {
                    println!(
                        "{} {} {}.",
                        if subcommand == "enable" {
                            "Enabled"
                        } else {
                            "Disabled"
                        },
                        kind,
                        stable_ref
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
                    if json {
                        println!(
                            "{}",
                            serde_json::json!({ "id": id, "archived": subcommand == "group-archive" })
                        );
                    } else {
                        println!(
                            "{} content type group {id}.",
                            if subcommand == "group-archive" {
                                "Archived"
                            } else {
                                "Restored"
                            }
                        );
                    }
                }
                "group-delete" => {
                    let id = args.get(3).cloned().unwrap_or_else(|| {
                        eprintln!("Usage: pasted type group-delete <id>");
                        std::process::exit(2);
                    });
                    db.delete_content_type_group(&id)?;
                    if json {
                        println!("{}", serde_json::json!({ "id": id, "deleted": true }));
                    } else {
                        println!("Deleted content type group {id}.");
                    }
                }
                "group-restore-defaults" => {
                    db.restore_default_content_type_groups()?;
                    if json {
                        println!(
                            "{}",
                            serde_json::json!({ "restoredDefaults": true, "kind": "contentTypeGroups" })
                        );
                    } else {
                        println!("Restored built-in content type groups.");
                    }
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
                    if json {
                        println!(
                            "{}",
                            serde_json::json!({ "id": id, "archived": subcommand == "archive" })
                        );
                    } else {
                        println!(
                            "{} content type {id}.",
                            if subcommand == "archive" {
                                "Archived"
                            } else {
                                "Restored"
                            }
                        );
                    }
                }
                "restore-defaults" => {
                    db.restore_default_content_types()?;
                    db.restore_default_content_type_groups()?;
                    if json {
                        println!(
                            "{}",
                            serde_json::json!({ "restoredDefaults": true, "kind": "contentTypes" })
                        );
                    } else {
                        println!("Restored built-in content type names, icons, and groups.");
                    }
                }
                _ => {
                    eprintln!("Usage: pasted type list|create|update|archive|restore|restore-defaults [--json]");
                    std::process::exit(2);
                }
            }
        }
        "analyzer" | "analyze" => {
            drop(conn);
            let db = DbState::new(db_path.clone())?;
            let subcommand = args.get(2).map(String::as_str).unwrap_or("run");
            if !matches!(subcommand, "run" | "preview") {
                eprintln!("Usage: pasted analyzer run [--text TEXT | --clip ID | --stdin] [--policy POLICY] [--extract] [--json]");
                std::process::exit(2);
            }
            let clip_id = argument_value(&args, "--clip").map(|value| {
                value.parse::<i64>().unwrap_or_else(|_| {
                    eprintln!("--clip requires a numeric clip ID.");
                    std::process::exit(2);
                })
            });
            let explicit_text = argument_value(&args, "--text");
            if clip_id.is_some() && explicit_text.is_some() {
                eprintln!("Provide only one of --text or --clip ID.");
                std::process::exit(2);
            }
            let policy = argument_value(&args, "--policy")
                .unwrap_or_else(|| "interactive".into())
                .parse::<pasted_lib::analysis_contract::AnalysisPolicy>()
                .unwrap_or_else(|error| {
                    eprintln!("{error}");
                    std::process::exit(2);
                });
            let options = pasted_lib::analysis_execution::AnalyzerOptions {
                policy,
                include_extractor: args.iter().any(|argument| argument == "--extract"),
                include_detectors: true,
                include_enricher: pasted_lib::features::is_enabled(&db, Feature::Transformations),
            };
            let result = if let Some(clip_id) = clip_id {
                pasted_lib::analysis_execution::analyze_clip(&db, clip_id, options)
            } else {
                let text = explicit_text.unwrap_or_else(|| {
                    read_stdin_bounded(pasted_lib::resource_limits::MAX_CLIP_TEXT_BYTES)
                        .unwrap_or_else(|error| {
                            eprintln!("Could not read analysis input: {error}");
                            std::process::exit(2);
                        })
                });
                if text.is_empty() {
                    eprintln!("Provide input with --text, --clip, or stdin.");
                    std::process::exit(2);
                }
                pasted_lib::analysis_execution::analyze_text(
                    &db,
                    &text,
                    Some("Pasted CLI"),
                    options,
                )
            }
            .map_err(rusqlite::Error::InvalidParameterName)?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).map_err(json_error)?
                );
            } else {
                println!("Kind: {}", result.analysis.result.clip_kind);
                println!(
                    "Detected type: {}",
                    result
                        .analysis
                        .result
                        .detected_type
                        .as_deref()
                        .unwrap_or("—")
                );
                println!("Participants: {}", result.analysis.participants.len());
                if let Some(recommendations) = result.analysis.result.recommendations.as_ref() {
                    println!("Smart Actions: {}", recommendations.actions.len());
                }
            }
        }
        "inspector" | "inspectors" => {
            drop(conn);
            let db = DbState::new(db_path.clone())?;
            let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
            match subcommand {
                "list" | "ls" => {
                    let inspectors = pasted_lib::content_inspection::inspector_definitions();
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&inspectors).map_err(json_error)?
                        );
                    } else {
                        for inspector in inspectors {
                            println!(
                                "{}\t{}\t{} → {}\t{}{}",
                                inspector.stable_ref,
                                inspector.priority,
                                inspector.input_contract,
                                inspector.output_contract,
                                inspector.name,
                                if inspector.is_available {
                                    ""
                                } else {
                                    " (unavailable)"
                                }
                            );
                        }
                    }
                }
                "get" => {
                    let reference = args.get(3).unwrap_or_else(|| {
                        eprintln!("Usage: pasted inspector get <ref> [--json]");
                        std::process::exit(2);
                    });
                    let inspector = pasted_lib::content_inspection::inspector_definitions()
                        .into_iter()
                        .find(|inspector| reference == &inspector.stable_ref)
                        .unwrap_or_else(|| {
                            eprintln!("Inspector {reference} was not found.");
                            std::process::exit(1);
                        });
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&inspector).map_err(json_error)?
                        );
                    } else {
                        println!("{}\t{}", inspector.stable_ref, inspector.name);
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
                    let apply = args.iter().any(|argument| argument == "--apply");
                    if apply && clip_id.is_none() {
                        eprintln!("--apply requires --clip ID.");
                        std::process::exit(2);
                    }
                    let result = if let Some(clip_id) = clip_id {
                        pasted_lib::inspection_execution::inspect_clip(&db, clip_id, apply)?
                    } else {
                        let text = explicit_text.unwrap_or_else(|| {
                            read_stdin_bounded(pasted_lib::resource_limits::MAX_CLIP_TEXT_BYTES)
                                .unwrap_or_else(|error| {
                                    eprintln!("Could not read inspection input: {error}");
                                    std::process::exit(2);
                                })
                        });
                        if text.is_empty() {
                            eprintln!("Provide input with --text, --clip, or stdin.");
                            std::process::exit(2);
                        }
                        let analysis = pasted_lib::inspection_execution::inspect_text(
                            &text,
                            Some("Pasted CLI"),
                        )
                        .map_err(|failure| {
                            rusqlite::Error::InvalidParameterName(failure.message)
                        })?;
                        pasted_lib::inspection_execution::ClipInspectionResult {
                            analysis,
                            application: pasted_lib::analysis_contract::ClipApplication::preview(),
                            live_file_observations: None,
                            media_metadata: None,
                        }
                    };
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&result).map_err(json_error)?
                        );
                    } else {
                        let metadata = &result.analysis.result;
                        println!("Origin: {}", metadata.origin.stable_name());
                        println!("Bytes: {}", metadata.byte_count);
                        if let Some(text) = metadata.text.as_ref() {
                            println!(
                                "Characters: {}; words: {}; lines: {}",
                                text.character_count, text.word_count, text.line_count
                            );
                        }
                        if let Some(image) = metadata.image.as_ref() {
                            println!("Dimensions: {} × {}", image.width, image.height);
                        }
                        if let Some(files) = metadata.files.as_ref() {
                            println!(
                                "Items: {}; types: {}",
                                files.item_count,
                                files.extensions.join(", ")
                            );
                        }
                        if let Some(media) = result.media_metadata.as_ref() {
                            println!(
                                "Media: {} file(s); audio streams: {}; video streams: {}; duration: {} ms",
                                media.media_file_count,
                                media.audio_stream_count,
                                media.video_stream_count,
                                media.total_duration_ms
                            );
                            if !media.codecs.is_empty() {
                                println!("Codecs: {}", media.codecs.join(", "));
                            }
                        }
                    }
                }
                _ => {
                    eprintln!("Usage: pasted inspector list|get|run [options] [--json]");
                    std::process::exit(2);
                }
            }
        }
        "enricher" | "enrichers" => {
            drop(conn);
            let db = DbState::new(db_path.clone())?;
            require_feature(&db, Feature::Transformations);
            let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
            let json = args.iter().any(|argument| argument == "--json");
            match subcommand {
                "list" | "ls" => {
                    let enrichers =
                        vec![pasted_lib::content_enrichment::smart_actions_enricher_definition()];
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&enrichers).map_err(json_error)?
                        );
                    } else {
                        for enricher in enrichers {
                            println!(
                                "{}\t{}\t{} → {}\t{}",
                                enricher.stable_ref,
                                enricher.priority,
                                enricher.input_contracts.join(" + "),
                                enricher.output_contract,
                                enricher.name
                            );
                        }
                    }
                }
                "get" => {
                    let reference = args.get(3).unwrap_or_else(|| {
                        eprintln!("Usage: pasted enricher get <ref> [--json]");
                        std::process::exit(2);
                    });
                    let enricher =
                        pasted_lib::content_enrichment::smart_actions_enricher_definition();
                    if reference != &enricher.stable_ref {
                        eprintln!("Enricher {reference} was not found.");
                        std::process::exit(1);
                    }
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&enricher).map_err(json_error)?
                        );
                    } else {
                        println!("{}\t{}", enricher.stable_ref, enricher.name);
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
                        pasted_lib::enrichment_execution::enrich_clip(&db, clip_id)
                    } else {
                        let text = explicit_text.unwrap_or_else(|| {
                            read_stdin_bounded(pasted_lib::resource_limits::MAX_CLIP_TEXT_BYTES)
                                .unwrap_or_else(|error| {
                                    eprintln!("Could not read enrichment input: {error}");
                                    std::process::exit(2);
                                })
                        });
                        if text.is_empty() {
                            eprintln!("Provide input with --text, --clip, or stdin.");
                            std::process::exit(2);
                        }
                        pasted_lib::enrichment_execution::enrich_text(
                            &db,
                            &text,
                            Some("Pasted CLI"),
                        )
                    }
                    .map_err(rusqlite::Error::InvalidParameterName)?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&result).map_err(json_error)?
                        );
                    } else if result.analysis.result.actions.is_empty() {
                        println!("No Smart Actions were recommended.");
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
                                action.transform_ref,
                                action.transform_revision,
                                action.transform_name
                            );
                        }
                    }
                }
                _ => {
                    eprintln!("Usage: pasted enricher list|get|run [options] [--json]");
                    std::process::exit(2);
                }
            }
        }
        "extractor" | "extractors" => {
            drop(conn);
            let db = DbState::new(db_path.clone())?;
            let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
            match subcommand {
                "list" | "ls" => {
                    let extractors = db.get_content_extractors()?;
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&extractors).map_err(json_error)?
                        );
                    } else {
                        for extractor in extractors {
                            println!(
                                "{}\t{}\t{}\t{}\t{} → {}\t{}",
                                extractor.stable_ref,
                                extractor.priority,
                                if extractor.enabled { "on" } else { "off" },
                                if extractor.is_available {
                                    "available"
                                } else {
                                    "unavailable"
                                },
                                extractor.input_contract,
                                extractor.output_contract,
                                extractor.name
                            );
                        }
                    }
                }
                "get" => {
                    let reference = args.get(3).unwrap_or_else(|| {
                        eprintln!("Usage: pasted extractor get <ref> [--json]");
                        std::process::exit(2);
                    });
                    let extractor = db.get_content_extractor(reference)?;
                    print_extractor(&extractor, args.iter().any(|argument| argument == "--json"))?;
                }
                "create" | "new" => {
                    let input = extractor_definition_from_args(&args, None);
                    let extractor = db.create_content_extractor(&input)?;
                    print_extractor(&extractor, args.iter().any(|argument| argument == "--json"))?;
                }
                "update" => {
                    let reference = args.get(3).unwrap_or_else(|| {
                        eprintln!("Usage: pasted extractor update <ref> [--name NAME] [--description TEXT] [--engine ENGINE] [--input CONTRACT] [--output CONTRACT] [--priority N] [--enabled|--disabled] [--json]");
                        std::process::exit(2);
                    });
                    let current = db.get_content_extractor(reference)?;
                    let input = extractor_definition_from_args(&args, Some(&current));
                    let updated = db.update_content_extractor_definition(current.id, &input)?;
                    print_extractor(&updated, args.iter().any(|argument| argument == "--json"))?;
                }
                "duplicate" | "copy" => {
                    let reference = args.get(3).unwrap_or_else(|| {
                        eprintln!("Usage: pasted extractor duplicate <ref> [--name NAME] [--json]");
                        std::process::exit(2);
                    });
                    let duplicate = db.duplicate_content_extractor(
                        reference,
                        argument_value(&args, "--name").as_deref(),
                    )?;
                    print_extractor(&duplicate, args.iter().any(|argument| argument == "--json"))?;
                }
                "delete" | "remove" => {
                    let reference = args.get(3).unwrap_or_else(|| {
                        eprintln!("Usage: pasted extractor delete <ref> [--json]");
                        std::process::exit(2);
                    });
                    let extractor = db.get_content_extractor(reference)?;
                    db.delete_content_extractor(extractor.id)?;
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::json!({ "deleted": true, "stableRef": extractor.stable_ref })
                        );
                    } else {
                        println!("Deleted Extractor {}.", extractor.name);
                    }
                }
                "run" | "test" => {
                    require_feature(&db, Feature::Ocr);
                    let reference = args.get(3).unwrap_or_else(|| {
                        eprintln!("Usage: pasted extractor run <ref> (--clip ID | --file PATH) [--apply] [--json]");
                        std::process::exit(2);
                    });
                    let extractor = db.get_content_extractor(reference)?;
                    let clip_id =
                        argument_value(&args, "--clip").and_then(|value| value.parse::<i64>().ok());
                    let file_path = argument_value(&args, "--file");
                    if clip_id.is_some() == file_path.is_some() {
                        eprintln!("Provide exactly one of --clip ID or --file PATH.");
                        std::process::exit(2);
                    }
                    let apply = args.iter().any(|argument| argument == "--apply");
                    if apply && clip_id.is_none() {
                        eprintln!("--apply requires --clip ID.");
                        std::process::exit(2);
                    }
                    let (image_bytes, content_hash) = if let Some(clip_id) = clip_id {
                        let clip = db.get_clip_by_id(clip_id)?;
                        let bytes = clip
                            .image_base64
                            .as_deref()
                            .and_then(pasted_lib::ocr::decode_stored_image)
                            .unwrap_or_else(|| {
                                eprintln!("Clip #{clip_id} has no extractable image data.");
                                std::process::exit(2);
                            });
                        (bytes, Some(clip.content_hash))
                    } else {
                        (
                            read_file_bounded(
                                Path::new(file_path.as_deref().expect("checked above")),
                                pasted_lib::resource_limits::MAX_ENCODED_IMAGE_BYTES,
                            )?,
                            None,
                        )
                    };
                    let detectors = setting_value_is_enabled(
                        db.get_setting(Feature::ContentDetection.setting_key())?
                            .as_deref(),
                    )
                    .then(|| db.get_content_detectors())
                    .transpose()?;
                    let analysis = pasted_lib::extraction_execution::analyze_image(
                        image_bytes,
                        &extractor,
                        detectors.as_deref(),
                    );
                    let result = if apply {
                        let clip_id = clip_id.expect("validated apply target");
                        let content_hash = content_hash.as_deref().expect("clip input has a hash");
                        pasted_lib::extraction_execution::apply_image_analysis(
                            &db,
                            clip_id,
                            content_hash,
                            &extractor,
                            detectors.is_some(),
                            analysis,
                        )?
                    } else {
                        pasted_lib::extraction_execution::ExtractionApplicationResult::preview(
                            analysis,
                        )
                    };
                    if args.iter().any(|argument| argument == "--json") {
                        println!("{}", serde_json::json!(&result));
                    } else if let Some(failure) = result.analysis.failure.as_ref() {
                        eprintln!("Extractor failed ({}): {}", failure.code, failure.message);
                    } else if let Some(text) = result.analysis.output.as_deref() {
                        print!("{text}");
                    } else {
                        println!("No text extracted.");
                    }
                    if result.analysis.failed() {
                        let _ = io::stdout().flush();
                        let _ = io::stderr().flush();
                        std::process::exit(1);
                    }
                }
                "restore-defaults" => {
                    db.restore_default_content_extractors()?;
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::json!({ "restoredDefaults": true, "kind": "extractors" })
                        );
                    } else {
                        println!("Restored shipped Extractor defaults.");
                    }
                }
                _ => {
                    eprintln!("Usage: pasted extractor list|get|create|update|duplicate|delete|run|restore-defaults [options] [--json]");
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
                                detector.stable_ref,
                                detector.priority,
                                if detector.enabled { "on" } else { "off" },
                                detector.content_type,
                                detector.name
                            );
                        }
                    }
                }
                "get" => {
                    let reference = args.get(3).unwrap_or_else(|| {
                        eprintln!("Usage: pasted detector get <ref> [--json]");
                        std::process::exit(2);
                    });
                    let detector = db.get_content_detector(reference)?;
                    print_detector(&detector, args.iter().any(|argument| argument == "--json"))?;
                }
                "create" | "new" => {
                    let input = detector_input_from_args(&args, None);
                    let detector = db.create_content_detector(&input)?;
                    print_detector(&detector, args.iter().any(|argument| argument == "--json"))?;
                }
                "update" | "edit" => {
                    let reference = args.get(3).unwrap_or_else(|| {
                        eprintln!("Usage: pasted detector update <ref> [--name NAME] [--type TYPE] [--regex REGEX] [--priority N] [--enabled|--disabled] [--json]");
                        std::process::exit(2);
                    });
                    let current = db.get_content_detector(reference)?;
                    let input = detector_input_from_args(&args, Some(&current));
                    let detector = db.update_content_detector(current.id, &input)?;
                    print_detector(&detector, args.iter().any(|argument| argument == "--json"))?;
                }
                "duplicate" | "copy" => {
                    let reference = args.get(3).unwrap_or_else(|| {
                        eprintln!("Usage: pasted detector duplicate <ref> [--name NAME] [--json]");
                        std::process::exit(2);
                    });
                    let duplicate = db.duplicate_content_detector(
                        reference,
                        argument_value(&args, "--name").as_deref(),
                    )?;
                    print_detector(&duplicate, args.iter().any(|argument| argument == "--json"))?;
                }
                "delete" | "remove" => {
                    let reference = args.get(3).unwrap_or_else(|| {
                        eprintln!("Usage: pasted detector delete <ref> [--json]");
                        std::process::exit(2);
                    });
                    let detector = db.get_content_detector(reference)?;
                    db.delete_content_detector(detector.id)?;
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::json!({ "deleted": true, "stableRef": detector.stable_ref })
                        );
                    } else {
                        println!("Deleted Detector {}.", detector.name);
                    }
                }
                "run" | "test" => {
                    require_feature(&db, Feature::ContentDetection);
                    let reference = args.get(3).unwrap_or_else(|| {
                        eprintln!("Usage: pasted detector run <ref> [--text TEXT | --clip ID | --stdin] [--apply] [--json]");
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
                        db.apply_content_detector(clip_id.expect("checked above"), reference)?
                    } else {
                        let detector = db.get_content_detector(reference)?;
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
                            read_stdin_bounded(
                                pasted_lib::resource_limits::MAX_TRANSFORM_TEXT_BYTES,
                            )?
                        };
                        if input.is_empty() {
                            eprintln!("Provide input with --text, --clip, or stdin.");
                            std::process::exit(2);
                        }
                        pasted_lib::detection_execution::DetectionApplicationResult::preview(
                            pasted_lib::detection_execution::analyze_detector(&input, &detector),
                        )
                    };
                    if args.iter().any(|argument| argument == "--json") {
                        println!("{}", serde_json::json!(&result));
                    } else if let Some(failure) = result.analysis.failure.as_ref() {
                        eprintln!("Detector failed ({}): {}", failure.code, failure.message);
                    } else if result.analysis.matched {
                        println!("Matches {}.", result.analysis.classification());
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
                    db.restore_default_content_detectors()?;
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::json!({ "restoredDefaults": true, "kind": "detectors" })
                        );
                    } else {
                        println!(
                            "Restored shipped detector defaults; custom detectors were preserved."
                        );
                    }
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
                            "Rescanned {} text clips; {} changed, {} were unchanged, and {} failed.",
                            report.scanned_count,
                            report.changed_count,
                            report.unchanged_count,
                            report.failed_count
                        );
                    }
                }
                _ => {
                    eprintln!("Usage: pasted detector list|get|create|update|duplicate|delete|run|restore-defaults|rescan [options] [--json]");
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
            if matches!(source_name.as_str(), "sources" | "list") {
                let sources = external_import::source_infos();
                if args.iter().any(|argument| argument == "--json") {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&sources).map_err(json_error)?
                    );
                } else {
                    for source in sources {
                        println!(
                            "{}\t{}\t{}\t{}",
                            source.id,
                            if source.detected {
                                "detected"
                            } else {
                                "not detected"
                            },
                            source.selection_kind,
                            source.label
                        );
                    }
                }
                return Ok(());
            }
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
        "insights" | "analytics" => {
            drop(conn);
            let db = DbState::new(db_path.clone())?;
            let subcommand = args.get(2).map(String::as_str).unwrap_or("summary");
            if subcommand != "summary" {
                eprintln!("Usage: pasted insights summary [--json]");
                std::process::exit(2);
            }
            let summary = db.get_analytics_summary()?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&summary).map_err(json_error)?
                );
            } else {
                println!(
                    "{} clips · {} characters",
                    summary.total_clips, summary.total_chars
                );
                if !summary.top_sources.is_empty() {
                    println!("Top sources:");
                    for source in summary.top_sources {
                        println!("{}\t{}", source.count, source.name);
                    }
                }
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
                "scan" | "retry" => {
                    let retried = if subcommand == "retry" {
                        db.reset_failed_ocr()?
                    } else {
                        0
                    };
                    let clip_id =
                        argument_value(&args, "--clip").and_then(|value| value.parse::<i64>().ok());
                    let scanned = scan_existing_images(&db, clip_id)?;
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::json!({
                                "scannedCount": scanned,
                                "retriedCount": retried,
                                "clipId": clip_id,
                            })
                        );
                    } else {
                        println!(
                            "Scanned {scanned} existing image{}{}.",
                            if scanned == 1 { "" } else { "s" },
                            if retried > 0 {
                                format!(
                                    " after resetting {retried} failed attempt{}",
                                    if retried == 1 { "" } else { "s" }
                                )
                            } else {
                                String::new()
                            }
                        );
                    }
                }
                "cancel" => {
                    let result = send_live_or_exit(pasted_lib::live_app::LiveAppAction::OcrCancel);
                    print_live_result(&result, args.iter().any(|argument| argument == "--json"))?;
                }
                _ => {
                    eprintln!("Usage: pasted ocr status | scan [--clip ID] | retry [--json]");
                    std::process::exit(2);
                }
            }
        }
        "connection" | "connections" => {
            let db = DbState::new(db_path.clone())?;
            require_feature(&db, Feature::Transformations);
            let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
            let json = args.iter().any(|argument| argument == "--json");
            match subcommand {
                "list" | "ls" => {
                    let connections = db.get_intelligence_connections()?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&connections).map_err(json_error)?
                        );
                    } else {
                        for connection in connections {
                            print_connection(&connection, false)?;
                        }
                    }
                }
                "get" => {
                    let id = args.get(3).unwrap_or_else(|| {
                        eprintln!("Usage: pasted connection get <id> [--json]");
                        std::process::exit(2);
                    });
                    print_connection(&db.get_intelligence_connection(id)?, json)?;
                }
                "detect" | "discover" => {
                    let detected =
                        pasted_lib::intelligence_connections::detect_intelligence_connections();
                    for candidate in &detected {
                        let endpoint = if candidate.provider_kind == "cli" {
                            candidate.executable_path.as_deref()
                        } else {
                            candidate.default_endpoint
                        };
                        db.ensure_intelligence_connection_candidate(
                            candidate.name,
                            candidate.provider_kind,
                            endpoint,
                        )?;
                    }
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&detected).map_err(json_error)?
                        );
                    } else if detected.is_empty() {
                        println!("No local intelligence connections detected.");
                    } else {
                        for candidate in detected {
                            println!(
                                "{}\t{}\t{}",
                                candidate.adapter_id, candidate.provider_kind, candidate.name
                            );
                        }
                    }
                }
                "create" | "new" => {
                    let name = argument_value(&args, "--name").unwrap_or_else(|| {
                        eprintln!("Usage: pasted connection create --name NAME --provider KIND [--endpoint VALUE] [--model MODEL] [--credential-ref REF] [--json]");
                        std::process::exit(2);
                    });
                    let provider = argument_value(&args, "--provider").unwrap_or_else(|| {
                        eprintln!("Connection creation requires --provider.");
                        std::process::exit(2);
                    });
                    let credential_ref = argument_value(&args, "--credential-ref");
                    pasted_lib::intelligence_connections::validate_credential_reference(
                        credential_ref.as_deref(),
                    )
                    .unwrap_or_else(|error| {
                        eprintln!("{error}");
                        std::process::exit(2);
                    });
                    let connection = db.create_intelligence_connection(
                        &name,
                        &provider,
                        argument_value(&args, "--endpoint").as_deref(),
                        argument_value(&args, "--model").as_deref(),
                        credential_ref.as_deref(),
                    )?;
                    print_connection(&connection, json)?;
                }
                "update" | "edit" => {
                    let id = args.get(3).unwrap_or_else(|| {
                        eprintln!("Usage: pasted connection update <id> [options] [--json]");
                        std::process::exit(2);
                    });
                    let current = db.get_intelligence_connection(id)?;
                    let credential_ref = optional_argument_update(
                        &args,
                        "--credential-ref",
                        "--clear-credential-ref",
                        current.credential_ref.clone(),
                    );
                    pasted_lib::intelligence_connections::validate_credential_reference(
                        credential_ref.as_deref(),
                    )
                    .unwrap_or_else(|error| {
                        eprintln!("{error}");
                        std::process::exit(2);
                    });
                    let enabled = if args.iter().any(|argument| argument == "--disabled") {
                        false
                    } else if args.iter().any(|argument| argument == "--enabled") {
                        true
                    } else {
                        current.enabled
                    };
                    let name = argument_value(&args, "--name").unwrap_or(current.name);
                    let provider =
                        argument_value(&args, "--provider").unwrap_or(current.provider_kind);
                    let endpoint = optional_argument_update(
                        &args,
                        "--endpoint",
                        "--clear-endpoint",
                        current.endpoint,
                    );
                    let model =
                        optional_argument_update(&args, "--model", "--clear-model", current.model);
                    db.update_intelligence_connection(IntelligenceConnectionUpdate {
                        id,
                        name: &name,
                        provider_kind: &provider,
                        endpoint: endpoint.as_deref(),
                        model: model.as_deref(),
                        credential_ref: credential_ref.as_deref(),
                        enabled,
                    })?;
                    print_connection(&db.get_intelligence_connection(id)?, json)?;
                }
                "delete" | "remove" => {
                    let id = args.get(3).unwrap_or_else(|| {
                        eprintln!("Usage: pasted connection delete <id> [--json]");
                        std::process::exit(2);
                    });
                    let connection = db.get_intelligence_connection(id)?;
                    db.delete_intelligence_connection(id)?;
                    if json {
                        println!("{}", serde_json::json!({ "deleted": true, "id": id }));
                    } else {
                        println!("Deleted Connection {}.", connection.name);
                    }
                }
                "order" => {
                    let ids = args
                        .iter()
                        .skip(3)
                        .filter(|argument| argument.as_str() != "--json")
                        .cloned()
                        .collect::<Vec<_>>();
                    if ids.is_empty() {
                        eprintln!("Usage: pasted connection order <id>... [--json]");
                        std::process::exit(2);
                    }
                    db.reorder_intelligence_connections(&ids)?;
                    if json {
                        println!("{}", serde_json::json!({ "connectionIds": ids }));
                    } else {
                        println!("Reordered {} Connections.", ids.len());
                    }
                }
                _ => {
                    eprintln!("Usage: pasted connection list|get|detect|create|update|delete|order [options] [--json]");
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
                    let plan = serde_json::from_str::<TransformationPlan>(&plan_json)
                        .unwrap_or_else(|error| {
                            eprintln!("Transform plan is invalid: {error}");
                            std::process::exit(2);
                        });
                    let input = match argument_value(&args, "--text") {
                        Some(text) => text,
                        None => read_stdin_bounded(
                            pasted_lib::resource_limits::MAX_TRANSFORM_TEXT_BYTES,
                        )?,
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
                            let steps: Vec<PipelineStepInput> =
                                serde_json::from_str(&steps_json).map_err(json_error)?;
                            TransformDefinition::from(db.create_pipeline(
                                &name,
                                &steps,
                                argument_value(&args, "--shortcut").as_deref(),
                            )?)
                        }
                        _ => {
                            eprintln!(
                                "Provide exactly one of --intent, --plan-json, or --steps-json."
                            );
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
                                    eprintln!(
                                        "Transform ran, but its output was not applied: {error}"
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
                    let updated_config =
                        argument_value(&args, "--config-json").or(current.config.clone());
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
                    let operation = db.duplicate_operation(
                        reference,
                        argument_value(&args, "--name").as_deref(),
                    )?;
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
                "get" => {
                    let bin_id =
                        parse_i64_argument(&args, 3, "Usage: pasted bin get <bin-id> [--json]");
                    let bin = db.get_bin(bin_id)?;
                    let transform_ref = db.get_bin_transform_ref(bin_id)?;
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::json!({ "bin": bin, "transformRef": transform_ref })
                        );
                    } else {
                        print_bin(&bin, false)?;
                    }
                }
                "create" | "new" => {
                    let name = argument_value(&args, "--name").unwrap_or_else(|| {
                        eprintln!("Usage: pasted bin create --name NAME [--icon ICON] [--color COLOR] [--smart-rule-json JSON] [--transform REF] [--json]");
                        std::process::exit(2);
                    });
                    let smart_rule = argument_value(&args, "--smart-rule-json");
                    validate_json_or_exit(smart_rule.as_deref(), "Smart Bin rule");
                    let bin = db.create_bin(
                        &name,
                        argument_value(&args, "--icon").as_deref().unwrap_or("📂"),
                        argument_value(&args, "--color")
                            .as_deref()
                            .unwrap_or("default"),
                        smart_rule.as_deref(),
                    )?;
                    if let Some(transform_ref) = argument_value(&args, "--transform") {
                        db.set_bin_transform_ref(bin.id, Some(&transform_ref))?;
                    }
                    print_bin(
                        &db.get_bin(bin.id)?,
                        args.iter().any(|argument| argument == "--json"),
                    )?;
                }
                "update" | "edit" => {
                    let bin_id = parse_i64_argument(
                        &args,
                        3,
                        "Usage: pasted bin update <bin-id> [options] [--json]",
                    );
                    let current = db.get_bin(bin_id)?;
                    let smart_rule = optional_argument_update(
                        &args,
                        "--smart-rule-json",
                        "--clear-smart-rule",
                        current.smart_rule,
                    );
                    validate_json_or_exit(smart_rule.as_deref(), "Smart Bin rule");
                    db.update_bin(
                        bin_id,
                        argument_value(&args, "--name")
                            .as_deref()
                            .unwrap_or(&current.name),
                        argument_value(&args, "--icon")
                            .as_deref()
                            .unwrap_or(&current.icon),
                        argument_value(&args, "--color")
                            .as_deref()
                            .unwrap_or(&current.color),
                        smart_rule.as_deref(),
                    )?;
                    print_bin(
                        &db.get_bin(bin_id)?,
                        args.iter().any(|argument| argument == "--json"),
                    )?;
                }
                "duplicate" | "copy" => {
                    let bin_id = parse_i64_argument(
                        &args,
                        3,
                        "Usage: pasted bin duplicate <bin-id> [--name NAME] [--json]",
                    );
                    let source = db.get_bin(bin_id)?;
                    let duplicate_name = argument_value(&args, "--name")
                        .unwrap_or_else(|| format!("{} Copy", source.name));
                    let duplicate = db.create_bin(
                        &duplicate_name,
                        &source.icon,
                        &source.color,
                        source.smart_rule.as_deref(),
                    )?;
                    if let Some(transform_ref) = db.get_bin_transform_ref(source.id)? {
                        db.set_bin_transform_ref(duplicate.id, Some(&transform_ref))?;
                    }
                    print_bin(
                        &db.get_bin(duplicate.id)?,
                        args.iter().any(|argument| argument == "--json"),
                    )?;
                }
                "delete" | "remove" => {
                    let bin_id = parse_i64_argument(&args, 3, "Usage: pasted bin delete <bin-id> [--disposition keep|trash|move --move-to BIN] [--json]");
                    let bin = db.get_bin(bin_id)?;
                    let disposition =
                        argument_value(&args, "--disposition").unwrap_or_else(|| "keep".into());
                    let destination = argument_value(&args, "--move-to")
                        .and_then(|value| value.parse::<i64>().ok());
                    db.delete_bin(bin_id, &disposition, destination)?;
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::json!({ "deleted": true, "binId": bin_id, "disposition": disposition, "destinationBinId": destination })
                        );
                    } else {
                        println!("Deleted Bin {}.", bin.name);
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
                "transform" => {
                    let bin_id = parse_i64_argument(
                        &args,
                        3,
                        "Usage: pasted bin transform <bin-id> <transform-ref|none> [--json]",
                    );
                    let value = args.get(4).unwrap_or_else(|| {
                        eprintln!(
                            "Usage: pasted bin transform <bin-id> <transform-ref|none> [--json]"
                        );
                        std::process::exit(2);
                    });
                    let transform_ref = (!matches!(value.as_str(), "none" | "null" | "-"))
                        .then_some(value.as_str());
                    db.set_bin_transform_ref(bin_id, transform_ref)?;
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::json!({ "binId": bin_id, "transformRef": transform_ref })
                        );
                    } else {
                        println!("Updated the default Transform for Bin #{bin_id}.");
                    }
                }
                "shortcut" => {
                    let bin_id = parse_i64_argument(
                        &args,
                        3,
                        "Usage: pasted bin shortcut <bin-id> <shortcut|none> [--json]",
                    );
                    let value = args.get(4).unwrap_or_else(|| {
                        eprintln!("Usage: pasted bin shortcut <bin-id> <shortcut|none> [--json]");
                        std::process::exit(2);
                    });
                    let shortcut = (!matches!(value.as_str(), "none" | "null" | "-"))
                        .then_some(value.as_str());
                    db.update_bin_shortcut(bin_id, shortcut)?;
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::json!({ "binId": bin_id, "shortcut": shortcut })
                        );
                    } else {
                        println!("Updated the shortcut for Bin #{bin_id}.");
                    }
                }
                _ => {
                    eprintln!("Usage: pasted bin list|get|create|update|duplicate|delete|clips|order|transform|shortcut [options] [--json]");
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
                "note" => {
                    let clip_id = parse_i64_argument(&args, 3, "Usage: pasted clip note <clip-id> [--text TEXT | --clear | --stdin] [--json]");
                    let note = if args.iter().any(|argument| argument == "--clear") {
                        None
                    } else {
                        Some(match argument_value(&args, "--text") {
                            Some(note) => note,
                            None => read_stdin_bounded(
                                pasted_lib::resource_limits::MAX_CLIP_NOTE_BYTES,
                            )?,
                        })
                    };
                    db.update_clip_note(clip_id, note.as_deref())?;
                    if json {
                        println!("{}", serde_json::json!({ "clipId": clip_id, "note": note }));
                    } else {
                        println!("Updated note for clip #{clip_id}.");
                    }
                }
                "revisions" | "versions" => {
                    let clip_id = parse_i64_argument(
                        &args,
                        3,
                        "Usage: pasted clip revisions <clip-id> [--limit N] [--offset N] [--json]",
                    );
                    let limit = argument_value(&args, "--limit")
                        .and_then(|value| value.parse::<i64>().ok())
                        .unwrap_or(50)
                        .clamp(1, 1_000);
                    let offset = argument_value(&args, "--offset")
                        .and_then(|value| value.parse::<i64>().ok())
                        .unwrap_or(0)
                        .max(0);
                    let revisions = db.get_clip_versions_page(clip_id, limit, offset)?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&revisions).map_err(json_error)?
                        );
                    } else if revisions.is_empty() {
                        println!("No revisions for clip #{clip_id}.");
                    } else {
                        for revision in revisions {
                            println!(
                                "{}\t{}\t{}",
                                revision.id,
                                revision.created_at,
                                revision.action_label.as_deref().unwrap_or("Revision")
                            );
                        }
                    }
                }
                "restore-revision" | "restore-version" => {
                    let clip_id = parse_i64_argument(
                        &args,
                        3,
                        "Usage: pasted clip restore-revision <clip-id> <revision-id> [--json]",
                    );
                    let revision_id = parse_i64_argument(
                        &args,
                        4,
                        "Usage: pasted clip restore-revision <clip-id> <revision-id> [--json]",
                    );
                    let clip = db.restore_clip_version(clip_id, revision_id)?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&clip).map_err(json_error)?
                        );
                    } else {
                        println!("Restored revision #{revision_id} for clip #{clip_id}.");
                    }
                }
                "provenance" => {
                    let clip_id = parse_i64_argument(
                        &args,
                        3,
                        "Usage: pasted clip provenance <clip-id> [--json]",
                    );
                    let provenance = db.get_clip_transformation_provenance(clip_id)?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&provenance).map_err(json_error)?
                        );
                    } else if let Some(provenance) = provenance {
                        println!(
                            "{}\trevision {}\t{} ms\t{}",
                            provenance.transform_ref,
                            provenance.transform_revision,
                            provenance.duration_ms,
                            provenance.transform_name
                        );
                    } else {
                        println!("Clip #{clip_id} has no Transform provenance.");
                    }
                }
                "copy" | "paste" => {
                    let clip_id = parse_i64_argument(
                        &args,
                        3,
                        "Usage: pasted clip copy|paste <clip-id> [--json]",
                    );
                    let action = if subcommand == "copy" {
                        pasted_lib::live_app::LiveAppAction::CopyClip { clip_id }
                    } else {
                        pasted_lib::live_app::LiveAppAction::PasteClip { clip_id }
                    };
                    let result = send_live_or_exit(action);
                    print_live_result(&result, json)?;
                }
                "pin" | "unpin" => {
                    require_feature(&db, Feature::Pinning);
                    let ids = parse_clip_ids(&args, 3);
                    let summary = db.batch_pin_clips(ids, subcommand == "pin")?;
                    print_mutation_summary(&summary, json)?;
                }
                "order-pinned" => {
                    require_feature(&db, Feature::Pinning);
                    let ids = parse_clip_ids(&args, 3);
                    db.reorder_pinned_clips(ids.clone())?;
                    if json {
                        println!("{}", serde_json::json!({ "clipIds": ids }));
                    } else {
                        println!("Saved the order of {} pinned clips.", ids.len());
                    }
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
                "purge" => {
                    if !args.iter().any(|argument| argument == "--yes") {
                        eprintln!("Permanent deletion cannot be undone. Re-run with --yes.");
                        std::process::exit(2);
                    }
                    let ids = parse_clip_ids(&args, 3);
                    for id in &ids {
                        db.purge_clip_permanently(*id)?;
                    }
                    if json {
                        println!("{}", serde_json::json!({ "purgedClipIds": ids }));
                    } else {
                        println!("Permanently deleted {} requested clips; protected clips were preserved.", ids.len());
                    }
                }
                "empty-trash" => {
                    if !args.iter().any(|argument| argument == "--yes") {
                        eprintln!("Emptying Trash is permanent. Re-run with --yes.");
                        std::process::exit(2);
                    }
                    db.empty_trash()?;
                    if json {
                        println!("{}", serde_json::json!({ "emptied": true }));
                    } else {
                        println!("Emptied Trash; protected clips were preserved.");
                    }
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
                    eprintln!("Usage: pasted clip get|note|revisions|restore-revision|provenance|copy|paste|pin|unpin|order-pinned|protect|unprotect|trash|restore|restore-all|purge|empty-trash|assign|remove-bin|export|import [options] [--json]");
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

            drop(conn);
            let db = DbState::new(db_path.clone())?;
            let clip = db.save_text_clip(&trimmed, "CLI Terminal")?;
            if setting_value_is_enabled(db.get_setting(Feature::Bins.setting_key())?.as_deref())
                && setting_value_is_enabled(
                    db.get_setting(Feature::Transformations.setting_key())?
                        .as_deref(),
                )
            {
                pasted_lib::intelligence_executor::apply_smart_bin_transforms_for_clip(
                    &db,
                    clip.id,
                    &clip.content_type,
                    &trimmed,
                    "CLI Terminal",
                );
            }
            let clip = db.get_clip_by_id(clip.id)?;

            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::json!({ "id": clip.id, "contentType": clip.content_type })
                );
            } else {
                println!("Saved {} clip #{} to History.", clip.content_type, clip.id);
            }
        }
        "list" | "ls" => {
            let limit = argument_value(&args, "--limit")
                .as_ref()
                .or_else(|| args.get(2).filter(|value| !value.starts_with("--")))
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(10)
                .clamp(1, 10_000);
            let offset = argument_value(&args, "--offset")
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(0)
                .max(0);
            let bin_id = argument_value(&args, "--bin").and_then(|value| value.parse::<i64>().ok());
            let pinned = args.iter().any(|argument| argument == "--pinned");
            let trash = args.iter().any(|argument| argument == "--trash");
            if trash && (bin_id.is_some() || pinned) {
                eprintln!("--trash cannot be combined with --bin or --pinned.");
                std::process::exit(2);
            }
            drop(conn);
            let db = DbState::new(db_path.clone())?;
            let clips = if trash {
                db.get_trashed_clips_page(Some(limit), Some(offset))?
            } else {
                db.get_clips_page(None, bin_id, pinned, Some(limit), Some(offset))?
            };
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&clips).map_err(json_error)?
                );
                return Ok(());
            }
            println!(
                "{:<5} | {:<8} | {:<15} | {:<20} | CONTENT",
                "ID", "TYPE", "SOURCE", "DATE"
            );
            println!(
                "{:-<5}-+-{:-<8}-+-{:-<15}-+-{:-<20}-+-{:-<30}",
                "", "", "", "", ""
            );
            for clip in clips {
                let snippet: String = clip
                    .text_content
                    .as_deref()
                    .unwrap_or("")
                    .lines()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(40)
                    .collect();
                println!(
                    "{:<5} | {:<8} | {:<15} | {:<20} | {}",
                    clip.id, clip.content_type, clip.source, clip.created_at, snippet
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
            let trash = args.iter().any(|argument| argument == "--trash");
            let limit = option_value("--limit")
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(20)
                .clamp(1, 10_000);
            let offset = option_value("--offset")
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(0)
                .max(0);
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
                 WHERE is_trashed = ?5
                   AND (?1 = '' OR text_content LIKE ?2)
                   AND (?3 IS NULL OR content_type = ?3)
                   AND (?4 IS NULL OR source = ?4)
                 ORDER BY created_at DESC
                 LIMIT ?6 OFFSET ?7",
            )?;
            let rows = stmt
                .query_map(
                    params![query, pattern, content_type, source, trash, limit, offset],
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                        ))
                    },
                )?
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
            if !args.iter().any(|argument| argument == "--yes") {
                eprintln!("Clearing History is permanent. Re-run with --yes to continue.");
                std::process::exit(2);
            }
            drop(conn);
            let db = DbState::new(db_path.clone())?;
            db.purge_unpinned_clips()?;
            if args.iter().any(|argument| argument == "--json") {
                println!("{}", serde_json::json!({ "cleared": true }));
            } else {
                println!("Cleared unpinned, unprotected History clips.");
            }
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
            println!("  pasted list [--limit N] [--offset N] [--bin ID|--pinned|--trash] [--json]");
            println!("  pasted search [query] [--type TYPE] [--source APP] [--trash] [--limit N] [--offset N] [--json]");
            println!("  pasted import sources [--json] List supported external-history sources");
            println!("  pasted import <source> [path] --json Import history from another clipboard manager");
            println!("  pasted diagnostics --json Show installation diagnostics");
            println!("  pasted insights summary --json Show aggregate clipboard insights");
            println!("  pasted licenses [--json] Show bundled open-source licenses and notices");
            println!("  pasted retention [--count N|unlimited] [--days N|forever] [--json]");
            println!("                   [--trash-count N|unlimited] [--trash-days N|forever]");
            println!("                   [--log-count N|unlimited] [--log-days N|forever]");
            println!("                   [--revision-count N|unlimited]");
            println!("  pasted settings list|get|set [arguments] [--json]");
            println!("  pasted recording status|pause|resume [--json] Control the running app");
            println!("  pasted queue status|start|stop|add|remove|order|paste|paste-all [--json]");
            println!("  pasted activity list [--limit N|--all] [--json]");
            println!("  pasted activity export [path] [--format json|csv]");
            println!("  pasted activity import <path> [--format json|csv] [--json]");
            println!("  pasted activity clear --yes [--json]");
            println!("  pasted transfer export|inspect|import <path.json> [--json]");
            println!("  pasted backup create <path.pastedbackup> [--json]");
            println!("  pasted backup inspect <path.pastedbackup> [--json]");
            println!("  pasted backup restore <path.pastedbackup> --yes [--json]");
            println!("  pasted clip export [path] [--format json|csv]");
            println!("  pasted clip import <path> [--format json|csv] [--json]");
            println!("  pasted analyzer run [--text TEXT | --clip ID | --stdin] [--policy POLICY] [--extract] [--json]");
            println!("  pasted extractor list --json List content Extractors and availability");
            println!("  pasted inspector list --json List content Inspectors");
            println!(
                "  pasted inspector run [--text TEXT | --clip ID | --stdin] [--apply] [--json]"
            );
            println!("  pasted enricher list --json List content Enrichers");
            println!("  pasted enricher run [--text TEXT | --clip ID | --stdin] [--json]");
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
            println!("  pasted ocr scan [--clip ID] [--json] Scan eligible images or one clip");
            println!("  pasted ocr retry|cancel [--json] Retry failures or cancel the running app");
            println!("  pasted transform list [--json] List saved and manually built Transforms");
            println!("  pasted transform test --plan-json JSON [--text TEXT|--stdin] [--json]");
            println!("  pasted transform get|plan|test|create|update|duplicate|delete Manage either Transform authoring form");
            println!("  pasted transform run <ref> [--text TEXT | --clip ID | --stdin] [--apply]");
            println!("  pasted operation list|get|create|update|duplicate|delete|run");
            println!("  pasted connection list|get|detect|create|update|delete|order");
            println!("  pasted bin list|get|create|update|duplicate|delete [--json]");
            println!("  pasted bin clips <id> --json List clips in persistent Bin order");
            println!("  pasted bin order <id> <clip-id>... Persist a complete Bin order");
            println!("  pasted clip get <id> --json Inspect one clip");
            println!(
                "  pasted clip copy|paste <id> [--json] Use the running app and system clipboard"
            );
            println!("  pasted clip pin|unpin <id>... [--json]");
            println!("  pasted clip order-pinned <id>... [--json]");
            println!("  pasted clip protect|unprotect <id>... [--json]");
            println!("  pasted clip trash|restore <id>... [--json]");
            println!("  pasted clip restore-all [--json] Restore every clip from Trash");
            println!("  pasted clip note|revisions|restore-revision|provenance <id> [options]");
            println!("  pasted clip purge <id>... --yes | empty-trash --yes");
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
        .filter(|argument| !matches!(argument.as_str(), "--json" | "--yes"))
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

fn parse_i64_argument(args: &[String], index: usize, usage: &str) -> i64 {
    args.get(index)
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or_else(|| {
            eprintln!("{usage}");
            std::process::exit(2);
        })
}

fn optional_argument_update(
    args: &[String],
    value_flag: &str,
    clear_flag: &str,
    current: Option<String>,
) -> Option<String> {
    if args.iter().any(|argument| argument == clear_flag) {
        None
    } else {
        argument_value(args, value_flag).or(current)
    }
}

fn validate_json_or_exit(value: Option<&str>, label: &str) {
    if let Some(value) = value {
        if let Err(error) = serde_json::from_str::<serde_json::Value>(value) {
            eprintln!("{label} must be valid JSON: {error}");
            std::process::exit(2);
        }
    }
}

fn read_file_bounded(path: &Path, maximum_bytes: usize) -> Result<Vec<u8>> {
    let metadata = fs::metadata(path).map_err(|_| rusqlite::Error::InvalidPath(path.into()))?;
    if !metadata.is_file() || metadata.len() > maximum_bytes as u64 {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "Input file must be a regular file no larger than {} MB",
            maximum_bytes / 1024 / 1024
        )));
    }
    let bytes = fs::read(path).map_err(|_| rusqlite::Error::InvalidPath(path.into()))?;
    if bytes.len() > maximum_bytes {
        return Err(rusqlite::Error::InvalidParameterName(
            "Input file exceeded the extraction safety limit".into(),
        ));
    }
    Ok(bytes)
}

fn extractor_definition_from_args(
    args: &[String],
    current: Option<&pasted_lib::content_extraction::Extractor>,
) -> ExtractorDefinitionInput {
    ExtractorDefinitionInput {
        name: argument_value(args, "--name").unwrap_or_else(|| {
            current
                .map(|item| item.name.clone())
                .unwrap_or_else(|| "Custom Extractor".into())
        }),
        description: argument_value(args, "--description").unwrap_or_else(|| {
            current
                .map(|item| item.description.clone())
                .unwrap_or_else(|| "Extracts searchable text from images.".into())
        }),
        engine: argument_value(args, "--engine").unwrap_or_else(|| {
            current
                .map(|item| item.engine.clone())
                .unwrap_or_else(|| APPLE_VISION_ENGINE.into())
        }),
        input_contract: argument_value(args, "--input").unwrap_or_else(|| {
            current
                .map(|item| item.input_contract.clone())
                .unwrap_or_else(|| "image".into())
        }),
        output_contract: argument_value(args, "--output").unwrap_or_else(|| {
            current
                .map(|item| item.output_contract.clone())
                .unwrap_or_else(|| "searchable_text".into())
        }),
        enabled: if args.iter().any(|argument| argument == "--disabled") {
            false
        } else if args.iter().any(|argument| argument == "--enabled") {
            true
        } else {
            current.map(|item| item.enabled).unwrap_or(true)
        },
        priority: argument_value(args, "--priority")
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or_else(|| current.map(|item| item.priority).unwrap_or(100)),
    }
}

fn print_extractor(
    extractor: &pasted_lib::content_extraction::Extractor,
    json: bool,
) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(extractor).map_err(json_error)?
        );
    } else {
        println!(
            "{}\t{}\t{} → {}\t{}",
            extractor.stable_ref,
            extractor.engine,
            extractor.input_contract,
            extractor.output_contract,
            extractor.name
        );
    }
    Ok(())
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
            "{}\t{}\t{}\t{}",
            detector.stable_ref, detector.priority, detector.content_type, detector.name
        );
    }
    Ok(())
}

fn print_operation(operation: &pasted_lib::db::Operation, json: bool) -> Result<()> {
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

fn print_connection(connection: &pasted_lib::db::IntelligenceConnection, json: bool) -> Result<()> {
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

fn print_bin(bin: &pasted_lib::db::Bin, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(bin).map_err(json_error)?);
    } else {
        println!(
            "#{}\t{}\t{}\t{} clips",
            bin.id,
            bin.icon,
            bin.name,
            bin.clip_count.unwrap_or(0)
        );
    }
    Ok(())
}

fn send_live_or_exit(action: pasted_lib::live_app::LiveAppAction) -> serde_json::Value {
    pasted_lib::live_app::send(action).unwrap_or_else(|error| {
        eprintln!("Live-app command failed: {error}");
        std::process::exit(1);
    })
}

fn print_live_result(result: &serde_json::Value, json: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(result).map_err(json_error)?
        );
    } else if let Some(paused) = result.get("paused").and_then(serde_json::Value::as_bool) {
        println!(
            "Clipboard recording is {}.",
            if paused { "paused" } else { "active" }
        );
    } else if let Some(total) = result
        .get("total_count")
        .and_then(serde_json::Value::as_u64)
    {
        println!(
            "Queue contains {total} item{}.",
            if total == 1 { "" } else { "s" }
        );
    } else if let Some(status) = result.get("status") {
        let total = status
            .get("total_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        println!(
            "Queue command completed; {total} item{} remain.",
            if total == 1 { "" } else { "s" }
        );
    } else {
        println!("Command completed.");
    }
    Ok(())
}

fn scan_existing_images(db: &DbState, clip_id: Option<i64>) -> Result<usize> {
    let extractor = db.active_image_text_extractor()?.unwrap_or_else(|| {
        eprintln!("No available image text Extractor is enabled.");
        std::process::exit(1);
    });
    let detectors = setting_value_is_enabled(
        db.get_setting(Feature::ContentDetection.setting_key())?
            .as_deref(),
    )
    .then(|| db.get_content_detectors())
    .transpose()?;
    let mut pending = Vec::new();
    if let Some(clip_id) = clip_id {
        let clip = db.get_clip_by_id(clip_id)?;
        let image_base64 = clip.image_base64.clone().unwrap_or_else(|| {
            eprintln!("Clip #{clip_id} has no image data.");
            std::process::exit(2);
        });
        if !db.force_ocr_running(clip_id, &clip.content_hash)? {
            eprintln!("Clip #{clip_id} is not an active image clip.");
            std::process::exit(2);
        }
        pending.push(pasted_lib::db::OcrCandidate {
            clip_id,
            content_hash: clip.content_hash,
            image_base64,
        });
    }

    let mut scanned = 0usize;
    loop {
        let candidate = if !pending.is_empty() {
            Some(pending.remove(0))
        } else if clip_id.is_none() {
            db.claim_next_ocr_candidate()?
        } else {
            None
        };
        let Some(candidate) = candidate else {
            break;
        };
        let Some(bytes) = pasted_lib::ocr::decode_stored_image(&candidate.image_base64) else {
            db.complete_or_reset_ocr_attempt_with_extractor(
                candidate.clip_id,
                &candidate.content_hash,
                None,
                pasted_lib::db::OcrExtractorProvenance::identified(
                    &extractor.engine,
                    &extractor.stable_ref,
                    &extractor.name,
                ),
                Some("invalid_image_data"),
            )?;
            scanned += 1;
            continue;
        };
        let analysis = pasted_lib::extraction_execution::analyze_image(
            bytes,
            &extractor,
            detectors.as_deref(),
        );
        pasted_lib::extraction_execution::persist_claimed_image_analysis(
            db,
            candidate.clip_id,
            &candidate.content_hash,
            &extractor,
            detectors.is_some(),
            analysis,
        )?;
        scanned += 1;
    }
    Ok(scanned)
}

fn plan_transform_or_exit(db: &DbState, args: &[String], intent: String) -> PlanIntentOutcome {
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
