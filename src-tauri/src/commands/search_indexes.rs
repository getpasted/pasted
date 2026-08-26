use std::sync::Arc;

use tauri::State;

use crate::db::{
    ClipSearchRequest, DbState, SearchHistoryEntry, SearchHistoryPage, SearchIndexStatus,
    DEFAULT_CLIP_SEARCH_PAGE_SIZE,
};
use crate::features::{self, Feature};

#[tauri::command]
pub async fn get_search_index_status(
    db: State<'_, Arc<DbState>>,
) -> Result<SearchIndexStatus, String> {
    features::require(&db, Feature::Search)?;
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || db.get_search_index_status())
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn rebuild_search_index(
    stable_ref: String,
    db: State<'_, Arc<DbState>>,
) -> Result<SearchIndexStatus, String> {
    features::require(&db, Feature::Search)?;
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || db.rebuild_search_index(&stable_ref))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_search_history(
    limit: Option<usize>,
    offset: Option<usize>,
    db: State<'_, Arc<DbState>>,
) -> Result<SearchHistoryPage, String> {
    features::require(&db, Feature::Search)?;
    db.list_search_history(
        limit.unwrap_or(DEFAULT_CLIP_SEARCH_PAGE_SIZE),
        offset.unwrap_or(0),
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn record_search_history(
    request: ClipSearchRequest,
    result_count: usize,
    db: State<'_, Arc<DbState>>,
) -> Result<SearchHistoryEntry, String> {
    features::require(&db, Feature::Search)?;
    db.record_search_history(&request, result_count)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_search_history(id: i64, db: State<'_, Arc<DbState>>) -> Result<bool, String> {
    features::require(&db, Feature::Search)?;
    db.delete_search_history(id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn clear_search_history(db: State<'_, Arc<DbState>>) -> Result<usize, String> {
    features::require(&db, Feature::Search)?;
    db.clear_search_history().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn enforce_search_history_retention(
    keep_count: i64,
    keep_age_days: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<usize, String> {
    features::require(&db, Feature::Search)?;
    db.enforce_search_history_retention(keep_count, keep_age_days)
        .map_err(|error| error.to_string())
}
