use std::sync::Arc;

use tauri::State;

use crate::db::{AnalyticsSummary, ClipSearchRequest, ClipSearchResult, DbState};

#[tauri::command]
pub async fn search_clips(
    request: ClipSearchRequest,
    db: State<'_, Arc<DbState>>,
) -> Result<ClipSearchResult, String> {
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.search_clips(&request).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[path = "library_exports.rs"]
mod exports;
pub use exports::*;

#[tauri::command]
pub fn get_analytics_summary(db: State<'_, Arc<DbState>>) -> Result<AnalyticsSummary, String> {
    db.get_analytics_summary()
        .map_err(|error| error.to_string())
}
