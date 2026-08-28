use std::thread;

use tauri::Emitter;

use super::CaptureContext;
use crate::app_exclusions::ExcludedCaptureKind;

pub(crate) fn ingest_text(
    context: &CaptureContext<'_>,
    text: String,
    hash: String,
    last_hash: &mut String,
    capture_limit: usize,
) {
    if !context.begin_hash(last_hash, &hash) {
        return;
    }
    if text.len() > capture_limit {
        context.report_ignored(&format!(
            "Ignored clipboard text larger than the configured {} MB limit",
            capture_limit / 1024 / 1024
        ));
        return;
    }
    if context.queue.consume_internal_clipboard_write(&text) {
        return;
    }
    if context.ignore_excluded(ExcludedCaptureKind::Text) {
        return;
    }

    if crate::features::is_enabled(context.db, crate::features::Feature::Queue)
        && context.queue.capture_item(text.clone())
    {
        let _ = context
            .db
            .log_activity("queue_item_recorded", "Recorded copied text into the Queue");
        let _ = context
            .app
            .emit("sequential-updated", context.queue.get_status());
    }

    match context.db.save_text_clip(&text, context.source) {
        Ok(clip) => {
            let _ = context.app.emit("clip-added", clip.clone());
            context.report_success(clip.id);
            apply_smart_bin_transforms(context, clip, text);
        }
        Err(error) => {
            eprintln!("[Pasted Monitor] Failed to save clip: {error}");
            context.report_failed();
        }
    }
}

fn apply_smart_bin_transforms(
    context: &CaptureContext<'_>,
    clip: crate::db::ClipItem,
    text: String,
) {
    let db = context.db.clone();
    let app = context.app.clone();
    let content_type = clip.content_type.clone();
    let content_types = clip.content_types.clone();
    let source = context.source.to_string();
    thread::spawn(move || {
        if crate::features::is_enabled(&db, crate::features::Feature::Bins)
            && crate::features::is_enabled(&db, crate::features::Feature::Transformations)
        {
            crate::intelligence_executor::apply_smart_bin_transforms_for_clip(
                &db,
                clip.id,
                &content_type,
                &content_types,
                &text,
                &source,
            );
        }
        if let Ok(updated) = db.get_clip_by_id(clip.id) {
            // The user can move the clip to Trash from capture feedback while
            // post-capture processing is still running. Do not publish that
            // stale completion back into the active History collection.
            if updated.is_trashed {
                return;
            }
            // Reconcile from the database instead of sending a snapshot that
            // can become active-looking while the event is in flight.
            let _ = app.emit("clip-added", serde_json::json!({ "id": updated.id }));
        }
    });
}
