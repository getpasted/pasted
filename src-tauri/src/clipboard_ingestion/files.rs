use std::thread;

use tauri::Emitter;

use super::CaptureContext;
use crate::app_exclusions::ExcludedCaptureKind;
use crate::clipboard_capture_policy::RecentImageCapture;

pub(crate) fn ingest_files(
    context: &CaptureContext<'_>,
    paths: Vec<String>,
    hash: String,
    last_hash: &mut String,
    coalesced_image: Option<(&RecentImageCapture, &str)>,
) {
    if !context.begin_hash(last_hash, &hash) {
        return;
    }
    if context.queue.consume_internal_clipboard_write(&hash) {
        return;
    }
    if let Some((recent, source)) = coalesced_image {
        if let Ok(Some(updated)) =
            context
                .db
                .reattribute_image_capture(recent.clip_id, &recent.content_hash, source)
        {
            let _ = context.app.emit("clip-added", updated);
        }
        return;
    }
    if !crate::resource_limits::file_list_within_limit(&paths) {
        context.report_ignored(&format!(
            "Ignored file list exceeding Pasted's limit of {} paths or {} MB of metadata",
            crate::resource_limits::MAX_FILE_LIST_ITEMS,
            crate::resource_limits::MAX_FILE_LIST_METADATA_BYTES / 1024 / 1024
        ));
        return;
    }
    if context.ignore_excluded(ExcludedCaptureKind::Files) {
        return;
    }

    let serialized = match serde_json::to_string(&paths) {
        Ok(serialized) => serialized,
        Err(error) => {
            eprintln!("[Pasted Monitor] Failed to serialize file list: {error}");
            context.report_failed();
            return;
        }
    };
    match context
        .db
        .save_clip("file", Some(&serialized), None, None, &hash, context.source)
    {
        Ok(clip) => {
            prefetch_previews(context, paths, hash);
            let clip_id = clip.id;
            let _ = context.app.emit("clip-added", clip);
            context.report_success(clip_id);
        }
        Err(error) => {
            eprintln!("[Pasted Monitor] Failed to save file clip: {error}");
            context.report_failed();
        }
    }
}

fn prefetch_previews(context: &CaptureContext<'_>, paths: Vec<String>, hash: String) {
    let preview_mode = context
        .db
        .get_setting("filePreviewMode")
        .ok()
        .flatten()
        .filter(|mode| matches!(mode.as_str(), "off" | "safe" | "all"))
        .unwrap_or_else(|| "safe".to_string());
    if preview_mode == "off" {
        return;
    }
    let preview_max_mb = context
        .db
        .get_setting("filePreviewMaxMb")
        .ok()
        .flatten()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(25)
        .clamp(1, 64);
    let app = context.app.clone();
    thread::spawn(move || {
        crate::commands::file_previews::prefetch_file_clip_previews(
            &app,
            &paths,
            &hash,
            &preview_mode,
            preview_max_mb,
        );
    });
}
