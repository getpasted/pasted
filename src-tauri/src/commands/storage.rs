use std::sync::Arc;

use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::db::DbState;
use crate::library_storage::{self, LibraryLocationInfo};

use crate::library_storage::{LibraryMoveReport, LibrarySession};

#[tauri::command]
pub fn get_library_location(
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<LibraryLocationInfo, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    Ok(library_storage::location_info(
        &app_data,
        &db.database_path(),
    ))
}

#[tauri::command]
pub async fn get_storage_protection(
    db: State<'_, Arc<DbState>>,
) -> Result<crate::storage_protection::StorageProtectionInfo, String> {
    let database_path = db.database_path();
    tauri::async_runtime::spawn_blocking(move || {
        crate::storage_protection::inspect_cached(&database_path)
    })
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn move_library(
    session: State<'_, Arc<LibrarySession>>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<LibraryMoveReport>, String> {
    crate::features::require(&db, crate::features::Feature::LibraryMove)?;
    let Some(folder) = app
        .dialog()
        .file()
        .set_title("Choose Pasted Library Folder")
        .blocking_pick_folder()
    else {
        return Ok(None);
    };
    let directory = folder
        .into_path()
        .map_err(|error| format!("The selected library location is not a local folder: {error}"))?;
    let db = db.inner().clone();
    let session = session.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.move_library(&session, &directory, false).map(Some)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn restore_default_library_location(
    db: State<'_, Arc<DbState>>,
    session: State<'_, Arc<LibrarySession>>,
) -> Result<LibraryMoveReport, String> {
    crate::features::require(&db, crate::features::Feature::LibraryMove)?;
    let db = db.inner().clone();
    let session = session.inner().clone();
    tauri::async_runtime::spawn_blocking(move || db.move_library(&session, &session.app_data, true))
        .await
        .map_err(|error| error.to_string())?
}
