use std::sync::Arc;

use tauri::State;

use crate::db::{ClipItem, ClipVersion, DbState};
use crate::features::{self, Feature};

#[tauri::command]
pub fn get_clip_versions(
    clip_id: i64,
    limit: Option<i64>,
    offset: Option<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<ClipVersion>, String> {
    features::require(&db, Feature::Revisions)?;
    db.get_clip_version_timeline_page(
        clip_id,
        limit.unwrap_or(50).clamp(1, 100),
        offset.unwrap_or(0).max(0),
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_clip_version_count(clip_id: i64, db: State<'_, Arc<DbState>>) -> Result<i64, String> {
    features::require(&db, Feature::Revisions)?;
    db.get_clip_version_timeline_count(clip_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn restore_clip_version(
    clip_id: i64,
    version_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<ClipItem, String> {
    features::require(&db, Feature::Revisions)?;
    db.restore_clip_version(clip_id, version_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_clip_version(
    clip_id: i64,
    version_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    features::require(&db, Feature::Revisions)?;
    db.delete_clip_version(clip_id, version_id)
        .map_err(|error| error.to_string())
}
