use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::db::DbState;

pub(crate) mod analysis;
#[path = "retention/history.rs"]
mod history;
pub(crate) mod revisions;

#[tauri::command]
pub fn enforce_clip_retention(
    keep_count: i64,
    keep_age_days: i64,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    history::configure(keep_count, keep_age_days, &app, &db)
}

#[tauri::command]
pub fn enforce_trash_retention(
    keep_count: i64,
    keep_age_days: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.enforce_trash_retention(keep_count, keep_age_days)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn enforce_activity_retention(
    keep_count: i64,
    keep_age_days: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.enforce_activity_retention(keep_count, keep_age_days)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn trash_unpinned_clips(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.trash_unpinned_clips().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn purge_unpinned_clips(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.purge_unpinned_clips().map_err(|e| e.to_string())
}
