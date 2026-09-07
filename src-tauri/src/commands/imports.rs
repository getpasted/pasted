use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::db::DbState;
#[path = "import_inspection.rs"]
mod inspection;
use inspection::{inspect_import_file_path, ImportFileInspection};

use super::{refresh_native_app_menu, settings::emit_window_appearance_change};

#[tauri::command]
pub async fn choose_import_file(
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<ImportFileInspection>, String> {
    crate::features::require(&db, crate::features::Feature::Backups)?;
    let Some(selected_file) = app
        .dialog()
        .file()
        .set_title("Choose Data to Import or Recover")
        .add_filter("Pasted Data", &["json", "csv", "pastedbackup"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = selected_file
        .into_path()
        .map_err(|error| format!("The selected file is not accessible: {error}"))?;
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        crate::features::require(&db, crate::features::Feature::Backups)?;
        inspect_import_file_path(path, &db)
    })
    .await
    .map_err(|error| error.to_string())?
    .map(Some)
}

#[tauri::command]
pub async fn import_inspected_file(
    path: String,
    kind: String,
    format: String,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<serde_json::Value, String> {
    crate::features::require(&db, crate::features::Feature::Backups)?;
    let refresh_menu = kind == "organization";
    let db = Arc::clone(&db);
    let worker_db = Arc::clone(&db);
    let report = tauri::async_runtime::spawn_blocking(move || {
        crate::features::require(&worker_db, crate::features::Feature::Backups)?;
        let contents = std::fs::read_to_string(PathBuf::from(path))
            .map_err(|error| format!("The selected file could not be read: {error}"))?;
        let result: Result<serde_json::Value, String> = match (kind.as_str(), format.as_str()) {
            ("clips", "json") => serde_json::to_value(
                worker_db
                    .import_clips_json(&contents)
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string()),
            ("clips", "csv") => serde_json::to_value(
                worker_db
                    .import_clips_csv(&contents)
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string()),
            ("activity", "json") => serde_json::to_value(
                worker_db
                    .import_activity_json(&contents)
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string()),
            ("activity", "csv") => serde_json::to_value(
                worker_db
                    .import_activity_csv(&contents)
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string()),
            ("organization", "json") => {
                let imported = worker_db
                    .import_backup_json(&contents)
                    .map_err(|error| error.to_string())?;
                Ok(serde_json::json!({ "importedCount": imported }))
            }
            _ => Err("The selected import action is not supported.".to_string()),
        };
        result
    })
    .await
    .map_err(|error| error.to_string())??;
    if refresh_menu {
        refresh_native_app_menu(&app, &db);
    }
    Ok(report)
}

#[tauri::command]
pub fn get_external_import_sources() -> Vec<crate::external_import::ExternalImportSourceInfo> {
    crate::external_import::source_infos()
}

#[tauri::command]
pub async fn import_external_history(
    source: String,
    path: Option<String>,
    choose_file: Option<bool>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<crate::external_import::ExternalImportReport>, String> {
    crate::features::require(&db, crate::features::Feature::Backups)?;
    let source = source.parse::<crate::external_import::ExternalImportSource>()?;
    let selected_path =
        if choose_file.unwrap_or(false) {
            let mut picker = app
                .dialog()
                .file()
                .set_title(if source.prefers_folder_selection() {
                    format!("Choose the {} Data Folder", source.label())
                } else {
                    format!("Import {} History", source.label())
                });
            if let Some(directory) = source.suggested_directory() {
                picker = picker.set_directory(directory);
            }
            if source.prefers_folder_selection() {
                let Some(selected_folder) = picker.blocking_pick_folder() else {
                    return Ok(None);
                };
                Some(selected_folder.into_path().map_err(|error| {
                    format!("The selected history folder is not accessible: {error}")
                })?)
            } else {
                let Some(selected_file) = picker
                    .add_filter(
                        "Clipboard History",
                        &["sqlite", "db", "alfdb", "plist", "data"],
                    )
                    .blocking_pick_file()
                else {
                    return Ok(None);
                };
                Some(selected_file.into_path().map_err(|error| {
                    format!("The selected history file is not accessible: {error}")
                })?)
            }
        } else {
            path.map(PathBuf::from)
        };
    let db = Arc::clone(&db);
    let report = tauri::async_runtime::spawn_blocking(move || {
        crate::features::require(&db, crate::features::Feature::Backups)?;
        crate::external_import::import_history(&db, source, selected_path).map(Some)
    })
    .await
    .map_err(|error| error.to_string())??;
    if let Some(capacity) = report
        .as_ref()
        .and_then(|report| report.history_capacity_adjusted_to)
    {
        emit_window_appearance_change(&app, "keepClipCount", &capacity.to_string());
    }
    Ok(report)
}
