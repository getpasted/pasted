use std::sync::Arc;

use tauri::State;

use crate::db::DbState;
use crate::features::{self, Feature};

#[tauri::command]
pub async fn get_ocr_backfill_clip_ids(
    group: String,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<i64>, String> {
    features::require(&db, Feature::Search)?;
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        db.get_ocr_backfill_clip_ids(&group)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}
