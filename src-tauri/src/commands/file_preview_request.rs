use std::sync::Arc;

use tauri::{AppHandle, Manager, State};

use super::{
    attach_file_reference_health, collect_file_clip_previews_with_health,
    healthy_cached_file_preview, parse_file_clip_paths, FileClipPreview,
};
use crate::db::DbState;

#[tauri::command]
pub async fn get_file_clip_previews(
    clip_id: i64,
    mode: String,
    max_size_mb: u64,
    only_index: Option<usize>,
    force_recheck: Option<bool>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<FileClipPreview>, String> {
    if !matches!(mode.as_str(), "off" | "safe" | "all") {
        return Err("Unknown file preview mode".to_string());
    }
    let cache_directory = app
        .path()
        .app_cache_dir()
        .ok()
        .map(|directory| directory.join("file-previews/thumbnails"));
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        let clip = db
            .get_clip_by_id(clip_id)
            .map_err(|error| error.to_string())?;
        if clip.content_type != "file" {
            return Err("Clip is not a file list".to_string());
        }
        let paths = clip
            .text_content
            .as_deref()
            .map(parse_file_clip_paths)
            .filter(|paths| !paths.is_empty())
            .ok_or_else(|| "File clip has no valid path metadata".to_string())?;
        if !crate::resource_limits::file_list_within_limit(&paths) {
            return Err("File list exceeds Pasted's safety limit".to_string());
        }
        let health = crate::file_reference_health::resolve_file_reference_health(
            &db,
            clip.id,
            &paths,
            force_recheck.unwrap_or(false),
            only_index,
        )
        .map_err(|error| error.to_string())?;
        if !force_recheck.unwrap_or(false) {
            if let Some(index) = only_index {
                if let Some(cached) = healthy_cached_file_preview(
                    &paths,
                    &mode,
                    cache_directory.as_deref(),
                    &clip.content_hash,
                    &health,
                    index,
                ) {
                    return Ok(cached);
                }
            }
        }
        let previews = collect_file_clip_previews_with_health(
            &paths,
            &mode,
            max_size_mb.saturating_mul(1024 * 1024),
            cache_directory.as_deref(),
            only_index,
            Some(&clip.content_hash),
            Some(&health),
        );
        Ok(attach_file_reference_health(
            &paths, previews, &health, only_index,
        ))
    })
    .await
    .map_err(|error| error.to_string())?
}
