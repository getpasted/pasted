use std::{sync::Arc, thread, time::Duration};

use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::{DbState, FactoryResetReport};
use crate::sequential_paste::SequentialQueueState;

#[tauri::command]
pub fn factory_reset_app(
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
    session: State<'_, Arc<crate::library_storage::LibrarySession>>,
) -> Result<FactoryResetReport, String> {
    let report = crate::library_storage::factory_reset_library(&db, &session)?;

    if let Some(queue) = app.try_state::<Arc<SequentialQueueState>>() {
        queue.clear_queue();
        let _ = app.emit("sequential-updated", queue.get_status());
    }

    // Cached previews are derived from library state and must not survive a reset.
    if let Ok(cache_directory) = app.path().app_cache_dir() {
        let _ = std::fs::remove_dir_all(cache_directory);
    }

    // A packaged app can restart its own executable. During `tauri dev`, that same
    // exit tears down the supervising CLI and Vite server, so the frontend reloads
    // its webview in place instead after it has cleared browser-side caches.
    if !tauri::is_dev() {
        let restart_handle = app.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(250));
            restart_handle.restart();
        });
    }

    Ok(report)
}
