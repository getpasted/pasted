use std::sync::Arc;

use tauri::State;

use crate::db::DbState;

#[tauri::command]
pub fn enforce_analysis_attempt_retention(
    keep_count: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.enforce_analysis_attempt_retention(keep_count)
        .map_err(|error| error.to_string())
}
