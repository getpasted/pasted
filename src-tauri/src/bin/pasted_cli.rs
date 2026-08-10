use rusqlite::{params, Connection, OptionalExtension, Result};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use pasted_lib::bin_assignment::assign_clips_to_bin;
use pasted_lib::db::{ClipMutationSummary, DbState, TransformClipApplication};
use pasted_lib::features::{setting_value_is_enabled, Feature};
use pasted_lib::installation_diagnostics::{InstallationDiagnostics, APP_IDENTIFIER};
use pasted_lib::intelligence_executor::execute_saved_transform;
use pasted_lib::library_storage;
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

fn get_db_path() -> PathBuf {
    let app_data = get_app_data_dir();
    library_storage::resolve_database_path(&app_data)
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    let db_path = get_db_path();
    let conn = match Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error opening Pasted database at '{:?}': {}", db_path, e);
            std::process::exit(1);
        }
    };

    // Ensure database table schema exists
    let _ = conn.execute(
        "CREATE TABLE IF NOT EXISTS clips (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content_type TEXT NOT NULL,
            text_content TEXT,
            html_content TEXT,
            image_base64 TEXT,
            content_hash TEXT,
            source_app TEXT DEFAULT 'System Clipboard',
            is_pinned INTEGER DEFAULT 0,
            bin_id INTEGER,
            note TEXT,
            is_trashed INTEGER DEFAULT 0,
            trashed_at TEXT,
            created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
        )",
        [],
    );

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
        eprintln!("Pasted CLI is disabled in Settings → Features.");
        std::process::exit(1);
    }

    match command {
        "library" => {
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
                        eprintln!("Usage: pasted library move <folder> [--json]");
                        std::process::exit(2);
                    };
                    let directory = fs::canonicalize(directory).unwrap_or_else(|error| {
                        eprintln!("Could not resolve the library folder: {error}");
                        std::process::exit(2);
                    });
                    let target =
                        library_storage::validate_destination_directory(&directory, &db_path)
                            .unwrap_or_else(|error| {
                                eprintln!("{error}");
                                std::process::exit(2);
                            });
                    if target == db_path {
                        println!("The Pasted library is already in that folder.");
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
                        println!("Moved the Pasted library to {}.", location.path);
                        println!("Previous library retained at {}.", previous.display());
                    }
                }
                "default" => {
                    let target = library_storage::default_database_path(&app_data);
                    if target == db_path {
                        println!("The Pasted library is already in its default location.");
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
                        println!("Restored the default Pasted library location.");
                        println!("Custom library retained at {}.", previous.display());
                    }
                }
                _ => {
                    eprintln!("Usage: pasted library location|move <folder>|default [--json]");
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
                eprintln!("OCR is disabled in Settings → Features.");
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
            let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
            match subcommand {
                "list" | "ls" => {
                    let transforms = db.get_saved_transforms()?;
                    if transforms.is_empty() {
                        println!("No saved Transforms.");
                    } else {
                        for transform in transforms {
                            println!(
                                "{}\t{}\trevision {}\t{} steps",
                                transform.stable_ref,
                                transform.name,
                                transform.revision,
                                transform.plan.steps.len()
                            );
                        }
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
                    match execute_saved_transform(
                        &db,
                        transform_ref,
                        input.clone(),
                        pasted_lib::intelligence_executor::SavedTransformExecutionContext {
                            source_clip_id: clip_id,
                            trigger_kind: "cli",
                            destination_kind: if replace { "replace" } else { "preview" },
                            client_request_id: None,
                        },
                        None,
                    ) {
                        Ok((_name, _execution_id, outcome)) => {
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
                            print!("{}", outcome.output);
                        }
                        Err(error) => {
                            eprintln!("Transform failed ({}): {}", error.code, error.message);
                            std::process::exit(1);
                        }
                    }
                }
                _ => {
                    eprintln!("Unknown transform command: {subcommand}");
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
                "run" => run_advanced_transformation(&args, &db, "operation"),
                _ => {
                    eprintln!("Usage: pasted operation [list [--json] | run <operation-ref> [--text TEXT | --clip ID | --stdin] [--json]]");
                    std::process::exit(2);
                }
            }
        }
        "pipeline" | "pipelines" => {
            let db = DbState::new(db_path.clone())?;
            require_feature(&db, Feature::Transformations);
            let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
            match subcommand {
                "list" | "ls" => {
                    let pipelines = db.get_pipelines()?;
                    if args.iter().any(|argument| argument == "--json") {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&pipelines).map_err(json_error)?
                        );
                    } else {
                        for pipeline in pipelines {
                            println!(
                                "{}\t{}\trevision {}\t{} steps",
                                pipeline.stable_ref,
                                pipeline.name,
                                pipeline.revision,
                                pipeline.steps.len()
                            );
                        }
                    }
                }
                "run" => run_advanced_transformation(&args, &db, "pipeline"),
                _ => {
                    eprintln!("Usage: pasted pipeline [list [--json] | run <pipeline-ref> [--text TEXT | --clip ID | --stdin] [--json]]");
                    std::process::exit(2);
                }
            }
        }
        "bin" | "bins" => {
            let db = DbState::new(db_path.clone())?;
            let bins_setting = db.get_setting(Feature::Bins.setting_key())?;
            if !setting_value_is_enabled(bins_setting.as_deref()) {
                eprintln!("Bins are disabled in Settings → Features.");
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
                            clip.source_app,
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
                _ => {
                    eprintln!("Usage: pasted clip [get <id> | pin|unpin|protect|unprotect|trash|restore <id>... | assign <bin-id|none> <id>...] [--json]");
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

            conn.execute(
                "INSERT INTO clips (content_type, text_content, source_app, created_at) VALUES ('text', ?1, 'CLI Terminal', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
                params![trimmed],
            )?;

            println!("✓ Successfully copied clip to Pasted history!");
        }
        "list" | "ls" => {
            let limit: i64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);
            let mut stmt = conn.prepare(
                "SELECT id, content_type, text_content, source_app, created_at FROM clips WHERE is_trashed = 0 ORDER BY created_at DESC LIMIT ?1"
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
            let query = args.get(2).cloned().unwrap_or_default();
            let pattern = format!("%{}%", query);
            let mut stmt = conn.prepare(
                "SELECT id, content_type, text_content, source_app, created_at FROM clips WHERE is_trashed = 0 AND text_content LIKE ?1 ORDER BY created_at DESC LIMIT 20"
            )?;
            let rows = stmt.query_map(params![pattern], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?;

            for r in rows {
                let (id, c_type, content, source, date) = r?;
                println!("[#{id}] ({c_type} from {source} @ {date}):\n{content}\n---");
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
            println!(
                "  pasted copy <text>       Save text or pipe stdin (cat file.txt | pasted copy)"
            );
            println!("  pasted list [limit]      List N recent clipboard items (default: 10)");
            println!("  pasted search <query>    Search clips for keyword query");
            println!("  pasted diagnostics --json Show installation diagnostics");
            println!("  pasted library location --json Show the active SQLite library");
            println!("  pasted library move <folder> --json Move the library (quit Pasted first)");
            println!("  pasted library default --json Restore the native default location");
            println!("  pasted ocr status --json Show OCR background-work status");
            println!("  pasted ocr scan          Scan existing unprocessed images");
            println!("  pasted transform list    List saved Transforms");
            println!(
                "  pasted transform run <ref> [--text TEXT | --clip ID | --stdin] [--replace]"
            );
            println!("  pasted operation list|run Experimental Operation inspection and execution");
            println!("  pasted pipeline list|run Experimental Pipeline inspection and execution");
            println!("  pasted bin list --json   List Bins and their saved clip order");
            println!("  pasted bin clips <id> --json List clips in persistent Bin order");
            println!("  pasted bin order <id> <clip-id>... Persist a complete Bin order");
            println!("  pasted clip get <id> --json Inspect one clip");
            println!("  pasted clip pin|unpin <id>... [--json]");
            println!("  pasted clip protect|unprotect <id>... [--json]");
            println!("  pasted clip trash|restore <id>... [--json]");
            println!("  pasted clip assign <bin-id|none> <id>... [--json]");
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
        eprintln!("{} is disabled in Settings → Features.", feature.label());
        std::process::exit(1);
    }
}

fn run_advanced_transformation(args: &[String], db: &DbState, target_kind: &str) {
    let Some(target_ref) = args.get(3) else {
        eprintln!(
            "Usage: pasted {target_kind} run <ref> [--text TEXT | --clip ID | --stdin] [--json]"
        );
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
    let target = if target_kind == "operation" {
        ExecutionTarget::Operation {
            operation_ref: target_ref.clone(),
        }
    } else {
        ExecutionTarget::Pipeline {
            pipeline_ref: target_ref.clone(),
        }
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
                        "targetKind": target_kind,
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
            eprintln!("{} failed ({}): {}", target_kind, error.code, error.message);
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
