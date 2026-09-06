use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Manager, State};
#[path = "backup_activation.rs"]
pub(crate) mod activation;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

use crate::db::DbState;

#[tauri::command]
pub async fn export_backup_file(
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<String>, String> {
    crate::features::require(&db, crate::features::Feature::Backups)?;
    let suggested_name = format!(
        "Pasted_History_and_Organization_{}.json",
        chrono::Local::now().format("%Y-%m-%d")
    );
    let Some(selected_file) = app
        .dialog()
        .file()
        .set_title("Export History and Organization")
        .set_file_name(suggested_name)
        .add_filter("Pasted JSON Export", &["json"])
        .blocking_save_file()
    else {
        return Ok(None);
    };

    let path = selected_file.into_path().map_err(|error| {
        format!("The selected export location is not a writable file path: {error}")
    })?;
    crate::features::require(&db, crate::features::Feature::Backups)?;
    let json = db.export_backup_json().map_err(|error| error.to_string())?;
    std::fs::write(&path, json)
        .map_err(|error| format!("Could not save the history and organization export: {error}"))?;
    let _ = db.log_activity(
        "data_export_completed",
        "Exported History and Organization as JSON",
    );
    Ok(Some(path.to_string_lossy().into_owned()))
}

#[tauri::command]
pub async fn export_full_backup_file(
    client_state_json: Option<String>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<crate::db::FullBackupReport>, String> {
    crate::features::require(&db, crate::features::Feature::Backups)?;
    let suggested_name = format!(
        "Pasted_Full_Backup_{}.pastedbackup",
        chrono::Local::now().format("%Y-%m-%d")
    );
    let Some(selected_file) = app
        .dialog()
        .file()
        .set_title("Create Full Pasted Backup")
        .set_file_name(suggested_name)
        .add_filter("Pasted Full Backup", &["pastedbackup"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path = selected_file
        .into_path()
        .map_err(|error| format!("The selected backup location is not writable: {error}"))?;
    crate::features::require(&db, crate::features::Feature::Backups)?;
    if let Some(state) = client_state_json.as_deref() {
        db.save_setting("backedUpClientState", state)
            .map_err(|error| error.to_string())?;
    }
    let window_state_json = capture_window_state(&app);
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        crate::features::require(&db, crate::features::Feature::Backups)?;
        db.create_full_backup(
            &path,
            client_state_json.as_deref(),
            window_state_json.as_deref(),
        )
        .inspect(|_| {
            let _ = db.log_activity("backup_created", "Created a complete recovery backup");
        })
        .map(Some)
        .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn restore_full_backup_file(
    session: State<'_, Arc<crate::library_storage::LibrarySession>>,
    current_client_state_json: Option<String>,
    backup_path: Option<String>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<crate::db::FullRestoreReport>, String> {
    crate::features::require(&db, crate::features::Feature::Backups)?;
    let path = if let Some(path) = backup_path {
        PathBuf::from(path)
    } else {
        let Some(selected_file) = app
            .dialog()
            .file()
            .set_title("Restore Full Pasted Backup")
            .add_filter("Pasted Full Backup", &["pastedbackup"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        selected_file
            .into_path()
            .map_err(|error| format!("The selected backup is not accessible: {error}"))?
    };
    activation::activate(
        session,
        current_client_state_json,
        path,
        app,
        db,
        crate::features::Feature::Backups,
    )
    .await
}

#[tauri::command]
pub fn consume_pending_full_restore_client_state(
    db: State<'_, Arc<DbState>>,
) -> Result<Option<String>, String> {
    db.consume_pending_full_restore_client_state()
        .map_err(|error| error.to_string())
}

pub(crate) fn capture_window_state(app: &AppHandle) -> Option<String> {
    let flags =
        StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED | StateFlags::FULLSCREEN;
    let _ = app.save_window_state(flags);
    app.path()
        .app_config_dir()
        .ok()
        .and_then(|directory| std::fs::read_to_string(directory.join(app.filename())).ok())
}
