use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::db::{DbState, FullBackupInspection, LibraryArchiveInspection};

use super::{emit_window_appearance_change, refresh_native_app_menu};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFileInspection {
    path: String,
    name: String,
    kind: String,
    format: String,
    size_bytes: u64,
    report: Option<serde_json::Value>,
    library: Option<LibraryArchiveInspection>,
    backup: Option<FullBackupInspection>,
}

#[tauri::command]
pub async fn choose_import_file(
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<ImportFileInspection>, String> {
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
    tauri::async_runtime::spawn_blocking(move || inspect_import_file_path(path, &db))
        .await
        .map_err(|error| error.to_string())?
        .map(Some)
}

fn inspect_import_file_path(path: PathBuf, db: &DbState) -> Result<ImportFileInspection, String> {
    let metadata = std::fs::metadata(&path)
        .map_err(|error| format!("The selected file is not accessible: {error}"))?;
    if !metadata.is_file() {
        return Err("The selected item is not a file.".to_string());
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Selected file")
        .to_string();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let base = |kind: &str, format: &str| ImportFileInspection {
        path: path.to_string_lossy().into_owned(),
        name: name.clone(),
        kind: kind.to_string(),
        format: format.to_string(),
        size_bytes: metadata.len(),
        report: None,
        library: None,
        backup: None,
    };

    if extension == "pastedbackup" {
        let inspection = db
            .inspect_full_backup(&path)
            .map_err(|error| format!("The backup is not valid: {error}"))?;
        return Ok(ImportFileInspection {
            backup: Some(inspection),
            ..base("backup", "backup")
        });
    }
    if !matches!(extension.as_str(), "json" | "csv") {
        return Err("Choose a JSON, CSV, or Pasted Backup file.".to_string());
    }
    let contents = std::fs::read_to_string(&path)
        .map_err(|error| format!("The selected file could not be read: {error}"))?;
    if extension == "csv" {
        let header = contents.lines().next().unwrap_or_default();
        if header.starts_with("timestamp,observed_timestamp,event_name,") {
            let report = db
                .inspect_activity_csv(&contents)
                .map_err(|error| format!("The Activity CSV is not valid: {error}"))?;
            return Ok(ImportFileInspection {
                report: Some(serde_json::to_value(report).map_err(|error| error.to_string())?),
                ..base("activity", "csv")
            });
        }
        if header.starts_with("id,content_type,source,") {
            let report = db
                .inspect_clips_csv(&contents)
                .map_err(|error| format!("The Clips CSV is not valid: {error}"))?;
            return Ok(ImportFileInspection {
                report: Some(serde_json::to_value(report).map_err(|error| error.to_string())?),
                ..base("clips", "csv")
            });
        }
        return Err("The CSV does not match a supported Clips or Activity export.".to_string());
    }

    let parsed: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|error| format!("The selected file is not valid JSON: {error}"))?;
    if parsed.is_array() {
        let report = db
            .inspect_clips_json(&contents)
            .map_err(|error| format!("The Clips JSON is not valid: {error}"))?;
        return Ok(ImportFileInspection {
            report: Some(serde_json::to_value(report).map_err(|error| error.to_string())?),
            ..base("clips", "json")
        });
    }
    let object = parsed
        .as_object()
        .ok_or_else(|| "The JSON does not match a supported export.".to_string())?;
    if object
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .is_some()
        && object
            .get("schemaVersion")
            .and_then(serde_json::Value::as_u64)
            .is_some()
    {
        let report = db
            .inspect_activity_json(&contents)
            .map_err(|error| format!("The Activity JSON is not valid: {error}"))?;
        return Ok(ImportFileInspection {
            report: Some(serde_json::to_value(report).map_err(|error| error.to_string())?),
            ..base("activity", "json")
        });
    }
    if object
        .get("clips")
        .and_then(serde_json::Value::as_array)
        .is_some()
        && object
            .get("bins")
            .and_then(serde_json::Value::as_array)
            .is_some()
        && object
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .is_some()
    {
        let inspection = DbState::inspect_library_archive_json(&contents)
            .map_err(|error| format!("The History and Organization JSON is not valid: {error}"))?;
        return Ok(ImportFileInspection {
            library: Some(inspection),
            ..base("organization", "json")
        });
    }
    Err("The JSON does not match a supported export.".to_string())
}

#[tauri::command]
pub async fn import_inspected_file(
    path: String,
    kind: String,
    format: String,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<serde_json::Value, String> {
    let refresh_menu = kind == "organization";
    let db = Arc::clone(&db);
    let worker_db = Arc::clone(&db);
    let report = tauri::async_runtime::spawn_blocking(move || {
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
