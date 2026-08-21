use std::sync::Arc;

use tauri::State;

use crate::db::{DbState, SearchIndexStatus};
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
