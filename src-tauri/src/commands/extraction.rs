use std::sync::Arc;

use tauri::State;

use crate::db::DbState;
use crate::features::{self, Feature};

mod ocr_backfill;
pub use ocr_backfill::*;

#[tauri::command]
pub fn extract_ocr_from_clip(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::extraction_execution::ExtractionApplicationResult, String> {
    features::require(&db, Feature::Ocr)?;
    let extractors = db
        .active_image_text_extractors_for_features(true)
        .map_err(|error| error.to_string())?;
    if extractors.is_empty() {
        return Err("No available image text Extractor is enabled".to_string());
    }
    let clip = db.get_clip_by_id(clip_id).map_err(|e| e.to_string())?;

    let image = clip
        .image_base64
        .as_deref()
        .ok_or_else(|| "Clip has no extractable image data".to_string())?;
    let bytes = crate::ocr::decode_stored_image(image)
        .ok_or_else(|| "Clip has no extractable image data".to_string())?;
    let classifiers = features::is_enabled(&db, Feature::ContentClassification)
        .then(|| db.get_content_classifiers().ok())
        .flatten();
    let registry = crate::content_extraction::system_engine_registry();
    let analysis = crate::extraction_execution::analyze_images_with_registry(
        bytes,
        &extractors,
        classifiers.as_deref(),
        &registry,
    );
    let extractor = extractors
        .iter()
        .find(|extractor| extractor.stable_ref == analysis.target_ref)
        .ok_or_else(|| "No Extractor completed the image analysis".to_string())?;
    crate::extraction_execution::apply_image_analysis(
        &db,
        clip_id,
        &clip.content_hash,
        extractor,
        classifiers.is_some(),
        analysis,
    )
    .map_err(|error| match error {
        rusqlite::Error::InvalidParameterName(message) => message,
        error => error.to_string(),
    })
}

#[tauri::command]
pub fn get_clip_searchable_text(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<crate::db::ClipSearchableText>, String> {
    db.get_clip_searchable_text(clip_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_clip_extraction_results(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::db::StoredExtractionObservation>, String> {
    db.get_extraction_observations(clip_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_clip_extraction_history(
    clip_id: i64,
    limit: usize,
    offset: usize,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::db::StoredExtractionAttempt>, String> {
    db.get_extraction_history(clip_id, limit, offset)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn extract_text_from_file_clip(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::extraction_execution::ExtractionApplicationResult, String> {
    let ocr_enabled = features::is_enabled(&db, Feature::Ocr);
    let transcriptions_enabled = features::is_enabled(&db, Feature::Transcriptions);
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let extractors = db
            .active_file_text_extractors_for_features(ocr_enabled, transcriptions_enabled)
            .map_err(|error| error.to_string())?;
        if extractors.is_empty() {
            return Err("No available file text Extractor is enabled".to_string());
        }
        let clip = db
            .get_clip_by_id(clip_id)
            .map_err(|error| error.to_string())?;
        let paths = clip
            .text_content
            .as_deref()
            .map(crate::content_inspection::parse_file_paths)
            .filter(|paths| !paths.is_empty())
            .ok_or_else(|| "Clip has no extractable file references".to_string())?;
        if !crate::resource_limits::file_list_within_limit(&paths) {
            return Err("File references exceed the extraction safety limit".to_string());
        }
        let classifiers = features::is_enabled(&db, Feature::ContentClassification)
            .then(|| db.get_content_classifiers().ok())
            .flatten();
        let registry = crate::content_extraction::system_engine_registry();
        let analysis = crate::extraction_execution::analyze_files_with_extractors_and_registry(
            paths,
            &extractors,
            classifiers.as_deref(),
            &registry,
        );
        let extractor = extractors
            .iter()
            .find(|extractor| extractor.stable_ref == analysis.target_ref)
            .ok_or_else(|| "No Extractor completed the file analysis".to_string())?;
        crate::extraction_execution::apply_file_analysis(
            &db,
            clip_id,
            &clip.content_hash,
            extractor,
            classifiers.is_some(),
            analysis,
        )
        .map_err(|error| match error {
            rusqlite::Error::InvalidParameterName(message) => message,
            error => error.to_string(),
        })
    })
    .await
    .map_err(|error| format!("File extraction task failed: {error}"))?
}
