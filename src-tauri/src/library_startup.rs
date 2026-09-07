use crate::library_storage::{self, LibraryStartupStatus};
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;

pub(crate) struct LibraryStartupState {
    app_data: Option<PathBuf>,
    status: Mutex<LibraryStartupStatus>,
    action: Mutex<()>,
}

pub(crate) fn initialize(app: &tauri::AppHandle) -> Option<Arc<crate::db::DbState>> {
    let app_data = app.path().app_data_dir().ok();
    let opened = app_data.as_ref().and_then(|directory| {
        match library_storage::open_library_automatically(directory) {
            Ok(opened) => Some(opened),
            Err(error) => {
                eprintln!("Could not open the saved library: {error}");
                None
            }
        }
    });
    let status = if opened.is_some() {
        LibraryStartupStatus::ready()
    } else {
        app_data
            .as_ref()
            .map(|directory| LibraryStartupStatus::unavailable(directory))
            .unwrap_or(LibraryStartupStatus {
                ready: false,
                path: None,
                recovery_created_at: None,
            })
    };
    app.manage(Arc::new(LibraryStartupState {
        app_data,
        status: Mutex::new(status),
        action: Mutex::new(()),
    }));
    opened.map(|(session, db)| {
        app.manage(Arc::new(session));
        Arc::new(db)
    })
}

#[tauri::command]
pub(crate) fn get_library_startup_status(
    state: tauri::State<'_, Arc<LibraryStartupState>>,
) -> LibraryStartupStatus {
    state.status.lock().clone()
}

#[tauri::command]
pub(crate) async fn retry_library_startup(
    state: tauri::State<'_, Arc<LibraryStartupState>>,
    app: tauri::AppHandle,
) -> Result<LibraryStartupStatus, String> {
    let state = state.inner().clone();
    let status = tauri::async_runtime::spawn_blocking(move || {
        let _action = state.action.try_lock().ok_or("library_recovery_busy")?;
        if state.status.lock().ready {
            return Err("library_already_open".to_string());
        }
        let directory = state
            .app_data
            .as_ref()
            .ok_or("library_location_unavailable")?;
        let next = match library_storage::open_library_automatically(directory) {
            Ok(_) => LibraryStartupStatus::ready(),
            Err(_) => LibraryStartupStatus::unavailable(directory),
        };
        *state.status.lock() = next.clone();
        Ok(next)
    })
    .await
    .map_err(|error| error.to_string())??;
    if status.ready {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(250));
            app.restart();
        });
    }
    Ok(status)
}

#[tauri::command]
pub(crate) fn get_library_recovery_notice(
    state: tauri::State<'_, Arc<LibraryStartupState>>,
) -> Option<library_storage::LibraryRecoveryNotice> {
    state
        .app_data
        .as_deref()
        .and_then(library_storage::visible_recovery_notice)
}

#[tauri::command]
pub(crate) async fn dismiss_library_recovery_notice(
    occurred_at: String,
    preserved_path: PathBuf,
    state: tauri::State<'_, Arc<LibraryStartupState>>,
) -> Result<(), String> {
    let directory = state
        .app_data
        .clone()
        .ok_or("library_location_unavailable")?;
    tauri::async_runtime::spawn_blocking(move || {
        library_storage::dismiss_recovery_notice(&directory, &occurred_at, &preserved_path)
    })
    .await
    .map_err(|error| error.to_string())?
}
