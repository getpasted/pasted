use super::capture_window_state;
use crate::db::DbState;
use std::{path::PathBuf, sync::Arc, thread, time::Duration};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_window_state::AppHandleExt;

pub(crate) async fn activate(
    session: State<'_, Arc<crate::library_storage::LibrarySession>>,
    current_client_state_json: Option<String>,
    path: PathBuf,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
    feature: crate::features::Feature,
) -> Result<Option<crate::db::FullRestoreReport>, String> {
    crate::features::require(&db, feature)?;
    let current_window_state_json = capture_window_state(&app);
    let db = Arc::clone(&db);
    let restore_db = Arc::clone(&db);
    let session = session.inner().clone();
    let (report, _client_state, restored_window_state) =
        tauri::async_runtime::spawn_blocking(move || {
            session.exclusive(|| {
                crate::features::require(&restore_db, feature)?;
                restore_db
                    .restore_full_backup(
                        &path,
                        current_client_state_json.as_deref(),
                        current_window_state_json.as_deref(),
                    )
                    .map_err(|error| error.to_string())
            })
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
