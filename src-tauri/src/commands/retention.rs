use std::sync::Arc;

use tauri::State;

use crate::db::DbState;

#[tauri::command]
pub fn enforce_clip_retention(
    keep_count: i64,
    keep_age_days: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.enforce_clip_retention(keep_count, keep_age_days)
        .map_err(|error| error.to_string())
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
pub fn enforce_revision_retention(
    keep_count: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.enforce_revision_retention(keep_count)
        .map_err(|error| error.to_string())
}
