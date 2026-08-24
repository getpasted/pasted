use std::sync::Arc;

use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::db::DbState;

pub(crate) mod runtime;

#[tauri::command]
pub async fn get_content_extractors(
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::content_extraction::Extractor>, String> {
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        db.get_content_extractors()
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn choose_extractor_executable(app: AppHandle) -> Result<Option<String>, String> {
    let Some(selected_file) = app
        .dialog()
        .file()
        .set_title("Choose an Extractor Executable")
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    selected_file
        .into_path()
        .map(|path| Some(path.to_string_lossy().into_owned()))
        .map_err(|error| format!("The selected executable is not accessible: {error}"))
}

#[tauri::command]
pub async fn choose_extractor_resource_file(
    kind: Option<String>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<String>, String> {
    let picker = app.dialog().file().set_title(crate::localization::text(
        &db,
        "component.contentExtractorManagerDialog.chooseResource",
    ));
    let selected = if kind.as_deref() == Some("directory") {
        picker.blocking_pick_folder()
    } else {
        picker.blocking_pick_file()
    };
    let Some(selected) = selected else {
        return Ok(None);
    };
    selected
        .into_path()
        .map(|path| Some(path.to_string_lossy().into_owned()))
        .map_err(|error| format!("The selected resource is not accessible: {error}"))
}

#[tauri::command]
pub fn diagnose_content_extractor_recipe(
    recipe: crate::extractor_recipe::ExtractorRecipe,
) -> crate::extractor_recipe::ExtractorDiagnosticReport {
    crate::extractor_recipe::diagnose(&recipe)
}

#[tauri::command]
pub async fn test_content_extractor_recipe(
    recipe: crate::extractor_recipe::ExtractorRecipe,
    path: String,
) -> Result<crate::content_extraction::ExtractionOutcome, String> {
    crate::extractor_recipe::validate_recipe(&recipe)?;
    tauri::async_runtime::spawn_blocking(move || {
        let path = std::path::PathBuf::from(path);
        let metadata = std::fs::metadata(&path)
            .map_err(|_| "The selected test file is unavailable.".to_string())?;
        if !metadata.is_file() {
            return Err("Select a regular file to test this Extractor.".to_string());
        }
        if recipe.accepts(crate::extractor_recipe::ExtractorInputKind::FileReferences) {
            Ok(crate::extractor_recipe::execute_files(
                &recipe,
                &[path.to_string_lossy().into_owned()],
            ))
        } else {
            if metadata.len() > crate::resource_limits::MAX_ENCODED_IMAGE_BYTES as u64 {
                return Err("The selected image exceeds the extraction limit.".to_string());
            }
            let image = std::fs::read(path)
                .map_err(|_| "The selected image could not be read.".to_string())?;
            Ok(crate::extractor_recipe::execute_image(&recipe, &image))
        }
    })
    .await
    .map_err(|error| format!("Extractor test failed: {error}"))?
}

#[tauri::command]
pub async fn create_content_extractor_recipe(
    input: crate::extractor_recipe::ExtractorRecipeDefinitionInput,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::content_extraction::Extractor, String> {
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        db.create_content_extractor_recipe(&input)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn update_content_extractor_recipe(
    id: i64,
    input: crate::extractor_recipe::ExtractorRecipeDefinitionInput,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::content_extraction::Extractor, String> {
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        db.update_content_extractor_recipe(id, &input)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn get_extractor_authoring_sessions(
    reference: String,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::extractor_recipe::ExtractorAuthoringSession>, String> {
    db.get_extractor_authoring_sessions(&reference)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn duplicate_content_extractor(
    reference: String,
    name: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::content_extraction::Extractor, String> {
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        db.duplicate_content_extractor(&reference, name.as_deref())
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn delete_content_extractor(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.delete_content_extractor(id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn restore_default_content_extractors(
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::content_extraction::Extractor>, String> {
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        db.restore_default_content_extractors()
            .map_err(|error| error.to_string())?;
        db.get_content_extractors()
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}
