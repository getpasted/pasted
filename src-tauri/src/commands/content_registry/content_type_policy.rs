use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::db::DbState;

#[tauri::command]
pub fn create_content_type(
    input: crate::content_types::ContentTypeInput,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<crate::content_types::ContentTypeDefinition, String> {
    let created = db
        .create_content_type(&input)
        .map_err(|error| error.to_string())?;
    crate::app_events::emit_clip_library_changed(&app, Vec::new());
    Ok(created)
}

#[tauri::command]
pub fn update_content_type(
    id: String,
    input: crate::content_types::ContentTypeInput,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<crate::content_types::ContentTypeDefinition, String> {
    let updated = db
        .update_content_type(&id, &input)
        .map_err(|error| error.to_string())?;
    crate::app_events::emit_clip_library_changed(&app, Vec::new());
    Ok(updated)
}

#[tauri::command]
pub fn set_content_type_archived(
    id: String,
    archived: bool,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.set_content_type_archived(&id, archived)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn restore_default_content_types(
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<Vec<crate::content_types::ContentTypeDefinition>, String> {
    db.restore_default_content_types()
        .map_err(|error| error.to_string())?;
    let restored = db
        .get_content_types(true)
        .map_err(|error| error.to_string())?;
    crate::app_events::emit_clip_library_changed(&app, Vec::new());
    Ok(restored)
}
