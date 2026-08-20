use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::DbState;
use crate::features::{self, Feature};
use crate::sequential_paste::{SequentialQueueState, SequentialStatus};

#[tauri::command]
pub fn start_sequential_paste(
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<SequentialStatus, String> {
    let db = app.state::<Arc<DbState>>();
    features::require(&db, Feature::Queue)?;
    seq.start_queue();
    let _ = db.log_activity(
        "queue_recording_started",
        "Started recording copies into the Queue",
    );
    let status = seq.get_status();
    let _ = app.emit("sequential-updated", status.clone());
    Ok(status)
}

#[tauri::command]
pub fn push_sequential_item(
    item: String,
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<SequentialStatus, String> {
    let db = app.state::<Arc<DbState>>();
    features::require(&db, Feature::Queue)?;
    if item.is_empty() {
        return Err("Only clips containing text can be added to the Copy Queue".to_string());
    }
    seq.push_item(item)?;
    let _ = db.log_activity("queue_item_added", "Added a text clip to the Queue");
    let status = seq.get_status();
    let _ = app.emit("sequential-updated", status.clone());
    Ok(status)
}

fn paste_queue_item(
    seq: &SequentialQueueState,
    db: &DbState,
    app: &AppHandle,
    index: usize,
    restore_after_success: bool,
) -> Result<Option<String>, String> {
    crate::queue_actions::paste_item(seq, db, app, index, restore_after_success)
}

#[tauri::command]
pub fn pop_sequential_paste(
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<Option<String>, String> {
    let db = app.state::<Arc<DbState>>();
    features::require(&db, Feature::Queue)?;
    paste_queue_item(&seq, &db, &app, 0, true)
}

#[tauri::command]
pub fn paste_sequential_item_by_index(
    index: usize,
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<Option<String>, String> {
    let db = app.state::<Arc<DbState>>();
    features::require(&db, Feature::Queue)?;
    paste_queue_item(&seq, &db, &app, index, true)
}

#[tauri::command]
pub fn remove_sequential_item_by_index(
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
    index: usize,
) -> Result<SequentialStatus, String> {
    let db = app.state::<Arc<DbState>>();
    features::require(&db, Feature::Queue)?;
    if seq.remove_item_by_index(index).is_some() {
        let _ = db.log_activity("queue_item_removed", "Removed an item from the Queue");
    }
    let status = seq.get_status();
    let _ = app.emit("sequential-updated", status.clone());
    Ok(status)
}

#[tauri::command]
pub fn reorder_sequential_items(
    item_ids: Vec<u64>,
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<SequentialStatus, String> {
    let db = app.state::<Arc<DbState>>();
    features::require(&db, Feature::Queue)?;
    seq.reorder_items(&item_ids)?;
    let _ = db.log_activity("queue_reordered", "Reordered the Queue");
    let status = seq.get_status();
    let _ = app.emit("sequential-updated", status.clone());
    Ok(status)
}

#[tauri::command]
pub fn stop_sequential_paste(
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<SequentialStatus, String> {
    let db = app.state::<Arc<DbState>>();
    features::require(&db, Feature::Queue)?;
    seq.stop_queue();
    let _ = db.log_activity(
        "queue_recording_stopped",
        "Stopped recording copies into the Queue",
    );
    let status = seq.get_status();
    let _ = app.emit("sequential-updated", status.clone());
    Ok(status)
}

#[tauri::command]
pub fn paste_all_sequential(
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<Option<String>, String> {
    let db = app.state::<Arc<DbState>>();
    features::require(&db, Feature::Queue)?;
    crate::queue_actions::paste_all(&seq, &db, &app)
}

#[tauri::command]
pub fn get_sequential_status(
    seq: State<'_, Arc<SequentialQueueState>>,
) -> Result<SequentialStatus, String> {
    Ok(seq.get_status())
}

#[tauri::command]
pub fn get_queue_paste_target(app: AppHandle) -> crate::paste_target::PasteTarget {
    app.state::<Arc<crate::paste_target::PasteTargetState>>()
        .snapshot()
}
