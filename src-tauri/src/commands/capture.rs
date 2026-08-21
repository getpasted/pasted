use std::sync::atomic::Ordering;
use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::clipboard_monitor::ClipboardMonitorState;
use crate::db::DbState;

#[tauri::command]
pub fn toggle_clipboard_pause(
    monitor_state: State<'_, Arc<ClipboardMonitorState>>,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<bool, String> {
    let current = monitor_state.is_manually_paused.load(Ordering::Relaxed);
    let new_value = !current;
    monitor_state
        .is_manually_paused
        .store(new_value, Ordering::Relaxed);

    if new_value {
        let _ = db.log_activity(
            "recording_manually_paused",
            "Clipboard recording manually paused",
        );
    } else {
        let _ = db.log_activity(
            "recording_manually_resumed",
            "Clipboard recording manually resumed",
        );
    }

    let effective = monitor_state.is_paused();
    crate::app_events::emit_clipboard_pause_changed(&app, effective, None);
    Ok(effective)
}

#[tauri::command]
pub fn is_clipboard_paused(
    monitor_state: State<'_, Arc<ClipboardMonitorState>>,
) -> Result<bool, String> {
    Ok(monitor_state.is_paused())
}
