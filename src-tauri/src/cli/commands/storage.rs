use super::super::*;
use super::*;

pub(crate) fn run_import(args: Vec<String>, db_path: PathBuf, conn: Connection) -> Result<()> {
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
    let report = external_import::import_history(&db, source, path).unwrap_or_else(|error| {
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
    Ok(())
}

pub(crate) fn run_database(args: Vec<String>, db_path: PathBuf, conn: Connection) -> Result<()> {
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
        "protection" => {
            let protection = pasted_lib::storage_protection::inspect(&db_path);
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&protection).map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                    })?
                );
            } else {
                println!("{}", protection.summary);
                println!("{}", protection.detail);
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
            let target = library_storage::validate_destination_directory(&directory, &db_path)
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
            eprintln!("Usage: pasted database location|protection|move <folder>|default [--json]");
            std::process::exit(2);
        }
    }
    Ok(())
}

pub(crate) fn run_diagnostics(
    args: Vec<String>,
    db_path: PathBuf,
    _conn: Connection,
) -> Result<()> {
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
    let details =
        InstallationDiagnostics::collect_with_database(app_path, data_path, db_path.clone());
    if args.iter().any(|argument| argument == "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&details)
                .map_err(|error| { rusqlite::Error::ToSqlConversionFailure(Box::new(error)) })?
        );
    } else {
        println!("{}", details.plain_text());
    }
    Ok(())
}

pub(crate) fn run_insights(args: Vec<String>, db_path: PathBuf, conn: Connection) -> Result<()> {
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
        if !summary.clip_types.is_empty() {
            println!("Clip types:");
            for clip_type in summary.clip_types {
                println!("{}\t{}", clip_type.count, clip_type.clip_type);
            }
        }
        if !summary.file_formats.is_empty() {
            println!("File formats:");
            for format in summary.file_formats {
                println!("{}\t{}", format.count, format.file_format);
            }
        }
        if !summary.content_types.is_empty() {
            println!("Content types:");
            for content_type in summary.content_types {
                println!("{}\t{}", content_type.count, content_type.content_type);
            }
        }
        if !summary.daily_activity.is_empty() {
            println!("Daily activity (local time):");
            for day in summary.daily_activity {
                println!("{}\t{}", day.count, day.date);
            }
        }
    }
    Ok(())
}

pub(crate) fn run_ocr(args: Vec<String>, db_path: PathBuf, _conn: Connection) -> Result<()> {
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
    Ok(())
}
