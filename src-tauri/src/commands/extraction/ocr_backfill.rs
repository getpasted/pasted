use std::sync::Arc;

use tauri::State;

use crate::db::DbState;
use crate::features::{self, Feature};

#[tauri::command]
pub async fn get_ocr_backfill_status(
    db: State<'_, Arc<DbState>>,
) -> Result<crate::db::OcrBackfillStatus, String> {
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        db.get_ocr_backfill_status()
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn start_ocr_backfill(
    db: State<'_, Arc<DbState>>,
    ocr: State<'_, Arc<crate::ocr::OcrService>>,
) -> Result<(), String> {
    features::require(&db, Feature::Ocr)?;
    if db
        .active_image_text_extractors_for_features(true)
        .map_err(|error| error.to_string())?
        .is_empty()
    {
        return Err("No available image text Extractor is enabled".to_string());
    }
    ocr.start_backfill()
}

#[tauri::command]
pub fn cancel_ocr_backfill(
    db: State<'_, Arc<DbState>>,
    ocr: State<'_, Arc<crate::ocr::OcrService>>,
) -> Result<(), String> {
    features::require(&db, Feature::Ocr)?;
    ocr.cancel();
    Ok(())
}

#[tauri::command]
pub fn retry_failed_ocr(
    db: State<'_, Arc<DbState>>,
    ocr: State<'_, Arc<crate::ocr::OcrService>>,
) -> Result<usize, String> {
    features::require(&db, Feature::Ocr)?;
    let count = db.reset_failed_ocr().map_err(|error| error.to_string())?;
    if count > 0 {
        ocr.start_backfill()?;
    }
    Ok(count)
}
