use std::sync::Arc;

use tauri::State;

use crate::bin_assignment::BinAssignmentOutcome;
use crate::db::{ClipItem, ClipMutationSummary, DbState};
use crate::features::{self, Feature};

use super::file_previews::parse_file_clip_paths;

pub mod versions;

#[tauri::command]
pub fn get_clips(
    bin_id: Option<i64>,
    only_pinned: bool,
    limit: Option<i64>,
    offset: Option<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<ClipItem>, String> {
    db.get_clips_page(bin_id, only_pinned, limit, offset)
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureFeedbackClip {
    id: i64,
    content_type: String,
    preview_text: Option<String>,
    source: String,
    is_pinned: bool,
    is_protected: bool,
    is_trashed: bool,
}

fn bounded_preview_text(value: &str) -> String {
    value.chars().take(280).collect()
}

#[tauri::command]
pub fn get_capture_feedback_clip(
    id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<CaptureFeedbackClip, String> {
    features::require(&db, Feature::Notifications)?;
    let clip = db.get_clip_by_id(id).map_err(|error| error.to_string())?;
    let preview_text = if clip.content_type == "file" {
        clip.text_content
            .as_deref()
            .map(parse_file_clip_paths)
            .map(|paths| {
                bounded_preview_text(
                    &paths
                        .iter()
                        .filter_map(|path| std::path::Path::new(path).file_name())
                        .filter_map(|name| name.to_str())
                        .collect::<Vec<_>>()
                        .join(" · "),
                )
            })
    } else {
        clip.text_content
            .as_deref()
            .map(bounded_preview_text)
            .filter(|text| !text.is_empty())
    };

    Ok(CaptureFeedbackClip {
        id: clip.id,
        content_type: clip.content_type,
        preview_text,
        source: clip.source,
        is_pinned: clip.is_pinned,
        is_protected: clip.is_protected,
        is_trashed: clip.is_trashed,
    })
}

#[tauri::command]
pub fn get_clip_collection_summary(
    db: State<'_, Arc<DbState>>,
) -> Result<crate::db::ClipCollectionSummary, String> {
    db.get_clip_collection_summary()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_clip_image(db: State<'_, Arc<DbState>>, id: i64) -> Result<Option<String>, String> {
    db.get_clip_image(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_trashed_clips(
    limit: Option<i64>,
    offset: Option<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<ClipItem>, String> {
    db.get_trashed_clips_page(limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn restore_clip(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.restore_clip(id).map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn restore_all_trashed_clips(
    db: State<'_, Arc<DbState>>,
) -> Result<ClipMutationSummary, String> {
    features::require(&db, Feature::Trash)?;
    db.restore_all_trashed_clips()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn purge_clip_permanently(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.purge_clip_permanently(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn empty_trash(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.empty_trash().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_clip_note(
    clip_id: i64,
    note: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    features::require(&db, Feature::Notes)?;
    db.update_clip_note(clip_id, note.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_clip(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.delete_clip(id).map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_pin_clip(id: i64, db: State<'_, Arc<DbState>>) -> Result<bool, String> {
    features::require(&db, Feature::Pinning)?;
    db.toggle_pin(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn assign_clip_bin(
    clip_id: i64,
    bin_id: Option<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<ClipItem>, String> {
    features::require(&db, Feature::Bins)?;
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        crate::bin_assignment::assign_clips_to_bin(&db, vec![clip_id], bin_id)
            .map(|outcome| outcome.updated_clips.into_iter().next())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn remove_clip_bin(
    clip_id: i64,
    bin_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<BinAssignmentOutcome, String> {
    features::require(&db, Feature::Bins)?;
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        crate::bin_assignment::remove_clips_from_bin(&db, vec![clip_id], bin_id)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn reorder_pinned_clips(ids: Vec<i64>, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    features::require(&db, Feature::Pinning)?;
    db.reorder_pinned_clips(ids).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_bin_clips(
    bin_id: i64,
    clip_ids: Vec<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    features::require(&db, Feature::Bins)?;
    db.reorder_bin_clips(bin_id, clip_ids)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn batch_pin_clips(
    ids: Vec<i64>,
    pin_state: bool,
    db: State<'_, Arc<DbState>>,
) -> Result<ClipMutationSummary, String> {
    features::require(&db, Feature::Pinning)?;
    db.batch_pin_clips(ids, pin_state)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn batch_trash_clips(
    ids: Vec<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<ClipMutationSummary, String> {
    db.batch_trash_clips(ids).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn batch_assign_bin_clips(
    ids: Vec<i64>,
    bin_id: Option<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<BinAssignmentOutcome, String> {
    features::require(&db, Feature::Bins)?;
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        crate::bin_assignment::assign_clips_to_bin(&db, ids, bin_id)
    })
    .await
    .map_err(|error| error.to_string())?
}
