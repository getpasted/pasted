use std::sync::Arc;

use tauri::State;

use crate::db::{ActivityLog, DbState};

#[tauri::command]
pub fn get_activity_logs(
    limit: Option<i64>,
    offset: Option<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<ActivityLog>, String> {
    db.get_activity_logs(limit, offset)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn clear_activity_logs(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.clear_activity_logs().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn export_activity_json(db: State<'_, Arc<DbState>>) -> Result<String, String> {
    let exported = db
        .export_activity_json()
        .map_err(|error| error.to_string())?;
    let _ = db.log_activity("data_export_completed", "Exported Activity as JSON");
    Ok(exported)
}

#[tauri::command]
pub fn export_activity_csv(db: State<'_, Arc<DbState>>) -> Result<String, String> {
    let exported = db
        .export_activity_csv()
        .map_err(|error| error.to_string())?;
    let _ = db.log_activity("data_export_completed", "Exported Activity as CSV");
    Ok(exported)
}
