use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

use crate::db::DbState;

#[tauri::command]
pub async fn export_backup_file(
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<String>, String> {
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
    if let Some(state) = client_state_json.as_deref() {
        db.save_setting("backedUpClientState", state)
            .map_err(|error| error.to_string())?;
    }
    let window_flags =
        StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED | StateFlags::FULLSCREEN;
    let _ = app.save_window_state(window_flags);
    let window_state_json = app
        .path()
        .app_config_dir()
        .ok()
        .and_then(|directory| std::fs::read_to_string(directory.join(app.filename())).ok());
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
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
    current_client_state_json: Option<String>,
    backup_path: Option<String>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<crate::db::FullRestoreReport>, String> {
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
    let window_flags =
        StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED | StateFlags::FULLSCREEN;
    let _ = app.save_window_state(window_flags);
    let current_window_state_json = app
        .path()
        .app_config_dir()
        .ok()
        .and_then(|directory| std::fs::read_to_string(directory.join(app.filename())).ok());
    let db = Arc::clone(&db);
    let restore_db = Arc::clone(&db);
    let (report, _client_state, restored_window_state) =
        tauri::async_runtime::spawn_blocking(move || {
            restore_db
                .restore_full_backup(
                    &path,
                    current_client_state_json.as_deref(),
                    current_window_state_json.as_deref(),
                )
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| error.to_string())??;

    if let Some(window_state) = restored_window_state {
        let parsed = serde_json::from_str::<serde_json::Value>(&window_state)
            .map_err(|error| format!("The backup contains invalid window state: {error}"))?;
        let directory = app
            .path()
            .app_config_dir()
            .map_err(|error| error.to_string())?;
        std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        std::fs::write(
            directory.join(app.filename()),
            serde_json::to_vec_pretty(&parsed).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("Could not restore the saved window state: {error}"))?;
    }
    if let Ok(cache_directory) = app.path().app_cache_dir() {
        let _ = std::fs::remove_dir_all(cache_directory);
    }
    let _ = db.log_activity(
        "backup_recovery_completed",
        "Recovered the complete state from a backup",
    );
    if !tauri::is_dev() {
        let restart_handle = app.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(500));
            restart_handle.restart();
        });
    }
    Ok(Some(report))
}

#[tauri::command]
pub fn consume_pending_full_restore_client_state(
    db: State<'_, Arc<DbState>>,
) -> Result<Option<String>, String> {
    db.consume_pending_full_restore_client_state()
        .map_err(|error| error.to_string())
}
