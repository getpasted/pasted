use std::sync::Arc;
use tauri::{AppHandle, State};

use crate::db::{ClipMutationSummary, DbState};
use crate::features::{self, Feature};

#[tauri::command]
pub fn update_bin_protection(
    id: i64,
    protect_clips: bool,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<(), String> {
    features::require(&db, Feature::Protection)?;
    features::require(&db, Feature::Bins)?;
    db.update_bin_protection(id, protect_clips)
        .map_err(|error| error.to_string())?;
    crate::app_events::emit_clip_library_changed(&app, Vec::new());
    Ok(())
}

#[tauri::command]
pub fn update_bin_concealment(
    id: i64,
    conceal_clips: bool,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<(), String> {
    features::require(&db, Feature::Concealment)?;
    features::require(&db, Feature::Bins)?;
    db.update_bin_concealment(id, conceal_clips)
        .map_err(|error| error.to_string())?;
    crate::app_events::emit_clip_library_changed(&app, Vec::new());
    Ok(())
}

#[tauri::command]
pub fn toggle_clip_concealed(clip_id: i64, db: State<'_, Arc<DbState>>) -> Result<bool, String> {
    features::require(&db, Feature::Concealment)?;
    db.toggle_concealed(clip_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn batch_conceal_clips(
    ids: Vec<i64>,
    concealed_state: bool,
    db: State<'_, Arc<DbState>>,
) -> Result<ClipMutationSummary, String> {
    features::require(&db, Feature::Concealment)?;
    db.batch_conceal_clips(ids, concealed_state)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn toggle_clip_protected(clip_id: i64, db: State<'_, Arc<DbState>>) -> Result<bool, String> {
    features::require(&db, Feature::Protection)?;
    db.toggle_protected(clip_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn batch_protect_clips(
    ids: Vec<i64>,
    protected_state: bool,
    db: State<'_, Arc<DbState>>,
) -> Result<ClipMutationSummary, String> {
    features::require(&db, Feature::Protection)?;
    db.batch_protect_clips(ids, protected_state)
        .map_err(|e| e.to_string())
}
