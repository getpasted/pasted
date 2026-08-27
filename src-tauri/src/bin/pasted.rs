use rusqlite::{Connection, OptionalExtension, Result};
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use pasted_lib::bin_assignment::assign_clips_to_bin;
use pasted_lib::content_classification::ClassifierInput;
use pasted_lib::content_extraction::{ExtractorDefinitionInput, CUSTOM_COMMAND_ENGINE};
use pasted_lib::content_types::{ContentTypeGroupInput, ContentTypeInput};
use pasted_lib::db::{
    open_pasted_database, ClipMutationSummary, DbState, IntelligenceConnectionUpdate,
    PipelineStepInput, TransformAuthoringKind, TransformClipApplication, TransformDefinition,
};
use pasted_lib::external_import::{self, ExternalImportSource};
use pasted_lib::extractor_recipe::{
    ExtractorAuthoringManifest, ExtractorAuthoringSource, ExtractorRecipe,
    ExtractorRecipeDefinitionInput, EXTRACTOR_AUTHORING_VERSION,
};
use pasted_lib::features::{setting_value_is_enabled, Feature};
use pasted_lib::installation_diagnostics::{InstallationDiagnostics, APP_IDENTIFIER};
use pasted_lib::intelligence_executor::{
    ExecutePlanRequest, PlanIntentOutcome, PlanIntentRequest, ProposeExtractorRecipeRequest,
};
use pasted_lib::library_storage;

#[path = "../cli/help.rs"]
mod help;
use pasted_lib::third_party_licenses;
use pasted_lib::transformation_intent::{IntentPlanningMode, TransformationPlan};
use pasted_lib::transformation_service::{
    execute, ExecutionDestination, ExecutionRequest, ExecutionTarget, ExecutionTrigger,
};

use cli_commands::json_error;

#[path = "../cli/commands/mod.rs"]
mod cli_commands;

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
    if let Some(code) = pasted_lib::content_extraction::run_bundled_extractor_helper(&args) {
        std::process::exit(code);
    }
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
    if matches!(command, "update" | "updates") {
        return cli_commands::updates::run(&args);
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
    let conn = match open_pasted_database(&db_path) {
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

    let app_lock_feature_setting = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [Feature::AppLock.setting_key()],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten();
    if command != "app-lock"
        && setting_value_is_enabled(app_lock_feature_setting.as_deref())
        && conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                [pasted_lib::app_lock::ENABLED_SETTING],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .ok()
            .flatten()
            .as_deref()
            == Some("true")
    {
        let supplied = env::var("PASTED_APP_LOCK_PASSPHRASE").unwrap_or_default();
        drop(conn);
        let db = DbState::new(db_path.clone())?;
        if supplied.is_empty() || !pasted_lib::app_lock::verify(&db, &supplied).unwrap_or(false) {
            eprintln!("Pasted is locked. Set PASTED_APP_LOCK_PASSPHRASE for this command, or run `pasted app-lock unlock`.");
            std::process::exit(1);
        }
        drop(db);
        let conn = open_pasted_database(&db_path)?;
        return run_command(command, &args, db_path, conn);
    }

    run_command(command, &args, db_path, conn)
}

fn run_command(command: &str, args: &[String], db_path: PathBuf, conn: Connection) -> Result<()> {
    let args = args.to_vec();
    match command {
        "app-lock" => cli_commands::app_lock::run(&args, db_path, conn)?,
        "retention" => cli_commands::retention::run(&args, db_path, conn)?,
        "settings" | "setting" => cli_commands::settings::run(&args, db_path, conn)?,
        "private-browsing" | "private-browser" => {
            cli_commands::private_browsing::run(&args, db_path, conn)?
        }
        "recording" | "capture" => cli_commands::live_app::run_recording(&args)?,
        "queue" => cli_commands::live_app::run_queue(&args)?,
        "activity" => cli_commands::activity::run(&args, db_path, conn)?,
        "transfer" | "archive" => cli_commands::portability::run_transfer(&args, db_path, conn)?,
        "backup" => cli_commands::portability::run_backup(&args, db_path, conn)?,
        "registry" => cli_commands::registry::run_registry(args, db_path, conn)?,
        "type" | "types" => cli_commands::registry::run_types(args, db_path, conn)?,
        "analyzer" | "analyze" => cli_commands::analyzer::run_analyzer(args, db_path, conn)?,
        "inspector" | "inspectors" => cli_commands::inspectors::run_inspector(args, db_path, conn)?,
        "suggestion" | "suggestions" => {
            cli_commands::suggestions::run_suggestion(args, db_path, conn)?
        }
        "extractor" | "extractors" => cli_commands::extractors::run_extractor(args, db_path, conn)?,
        "classifier" | "classifiers" => {
            cli_commands::classifiers::run_classifier(args, db_path, conn)?
        }
        "import" => cli_commands::storage::run_import(args, db_path, conn)?,
        "database" | "library" => cli_commands::storage::run_database(args, db_path, conn)?,
        "diagnostics" | "diagnose" => cli_commands::storage::run_diagnostics(args, db_path, conn)?,
        "insights" | "analytics" => cli_commands::storage::run_insights(args, db_path, conn)?,
        "ocr" => cli_commands::storage::run_ocr(args, db_path, conn)?,
        "connection" | "connections" => {
            cli_commands::connections::run_connections(args, db_path, conn)?
        }
        "transform" | "transforms" => {
            cli_commands::transforms::run_transforms(args, db_path, conn)?
        }
        "operation" | "operations" => {
            cli_commands::operations::run_operations(args, db_path, conn)?
        }
        "bin" | "bins" => cli_commands::bins::run_bins(args, db_path, conn)?,
        "clip" | "clips" => cli_commands::clips::run_clips(args, db_path, conn)?,
        "copy" | "add" => cli_commands::history::run_copy(args, db_path, conn)?,
        "list" | "ls" => cli_commands::history::run_list(args, db_path, conn)?,
        "search" | "find" => cli_commands::history::run_search(args, db_path, conn)?,
        "search-history" | "searches" => cli_commands::search_history::run(&args, db_path, conn)?,
        "clear" => cli_commands::maintenance::run_clear(args, db_path, conn)?,
        "reset" => cli_commands::maintenance::run_reset(args, db_path, conn)?,
        _ => help::print(),
    }

    Ok(())
}
