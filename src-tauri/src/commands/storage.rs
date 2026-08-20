use std::sync::Arc;

use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::db::DbState;
use crate::library_storage::{self, LibraryLocationInfo};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryMoveReport {
    location: LibraryLocationInfo,
    recovery_path: String,
}

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
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<LibraryMoveReport>, String> {
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
    let current_path = db.database_path();
    let target = library_storage::validate_destination_directory(&directory, &current_path)?;
    if target == current_path {
        let app_data = app
            .path()
            .app_data_dir()
            .map_err(|error| error.to_string())?;
        return Ok(Some(LibraryMoveReport {
            location: library_storage::location_info(&app_data, &current_path),
            recovery_path: current_path.to_string_lossy().into_owned(),
        }));
    }

    let db_for_move = Arc::clone(&db);
    let target_for_move = target.clone();
    let previous = tauri::async_runtime::spawn_blocking(move || {
        db_for_move.relocate_database(target_for_move)
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| format!("Could not move the Pasted library: {error}"))?;

    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    if let Err(error) = library_storage::persist_location(&app_data, &target) {
        db.switch_to_database(previous.clone())
            .map_err(|rollback| {
                format!("{error} The previous library also could not be reopened: {rollback}")
            })?;
        return Err(error);
    }
    let _ = db.log_activity("library_moved", "Moved the Pasted library");
    Ok(Some(LibraryMoveReport {
        location: library_storage::location_info(&app_data, &target),
        recovery_path: previous.to_string_lossy().into_owned(),
    }))
}

#[tauri::command]
pub async fn restore_default_library_location(
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<LibraryMoveReport, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let target = library_storage::default_database_path(&app_data);
    let current = db.database_path();
    if current == target {
        return Ok(LibraryMoveReport {
            location: library_storage::location_info(&app_data, &current),
            recovery_path: current.to_string_lossy().into_owned(),
        });
    }

    let archived_default = library_storage::archive_existing_database(&target)?;
    let db_for_move = Arc::clone(&db);
    let target_for_move = target.clone();
    let move_result = tauri::async_runtime::spawn_blocking(move || {
        db_for_move.relocate_database(target_for_move)
    })
    .await
    .map_err(|error| error.to_string())?;
    let previous = match move_result {
        Ok(previous) => previous,
        Err(error) => {
            if let Some(archived) = archived_default.as_deref() {
                library_storage::restore_archived_database(archived, &target);
            }
            return Err(format!(
                "Could not restore the default library location: {error}"
            ));
        }
    };

    if let Err(error) = library_storage::persist_location(&app_data, &target) {
        db.switch_to_database(previous.clone())
            .map_err(|rollback| {
                format!("{error} The custom library also could not be reopened: {rollback}")
            })?;
        library_storage::remove_database_files(&target);
        if let Some(archived) = archived_default.as_deref() {
            library_storage::restore_archived_database(archived, &target);
        }
        return Err(error);
    }
    let _ = db.log_activity(
        "library_moved",
        "Restored the default Pasted library location",
    );
    Ok(LibraryMoveReport {
        location: library_storage::location_info(&app_data, &target),
        recovery_path: previous.to_string_lossy().into_owned(),
    })
}
