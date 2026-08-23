use std::sync::Arc;

use tauri::State;

use crate::db::DbState;

#[tauri::command]
pub async fn get_content_extractor_runtime(
    reference: String,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::content_extraction::ExtractorRuntimeStatus, String> {
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        let extractor = db
            .get_content_extractor(&reference)
            .map_err(|error| error.to_string())?;
        Ok(crate::content_extraction::inspect_extractor_runtime(
            &extractor,
        ))
    })
    .await
    .map_err(|error| error.to_string())?
}
