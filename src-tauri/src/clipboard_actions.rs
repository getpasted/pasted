//! Shared copy and paste workflows used by GUI, hotkey, and live-app adapters.

use arboard::Clipboard;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

use crate::db::{ClipItem, DbState};
use crate::sequential_paste::SequentialQueueState;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PasteOrigin {
    Hud,
    ClipHotkey,
}

pub fn write_clip(clipboard: &mut Clipboard, clip: &ClipItem) -> Result<(), String> {
    if clip.content_type == "file" {
        let paths = clip
            .text_content
            .as_deref()
            .ok_or_else(|| "File clip has no path metadata".to_string())
            .and_then(|value| {
                serde_json::from_str::<Vec<String>>(value)
                    .map_err(|_| "File clip has invalid path metadata".to_string())
            })?;
        if paths.is_empty() || !crate::resource_limits::file_list_within_limit(&paths) {
            return Err("File list exceeds Pasted's safety limit".to_string());
        }
        return clipboard
            .set()
            .file_list(&paths)
            .map_err(|error| error.to_string());
    }
    if clip.content_type == "image" {
        let image_base64 = clip
            .image_base64
            .as_deref()
            .ok_or_else(|| "Image clip has no stored image data".to_string())?;
        let clean = image_base64.split(',').next_back().unwrap_or(image_base64);
        if clean.len() > crate::resource_limits::MAX_STORED_IMAGE_BASE64_BYTES {
            return Err("Clip image exceeds Pasted's safety limit".to_string());
        }
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, clean)
            .map_err(|error| error.to_string())?;
        let image = image::load_from_memory(&bytes).map_err(|error| error.to_string())?;
        let rgba = image.to_rgba8();
        return clipboard
            .set_image(arboard::ImageData {
                width: rgba.width() as usize,
                height: rgba.height() as usize,
                bytes: std::borrow::Cow::Owned(rgba.into_raw()),
            })
            .map_err(|error| error.to_string());
    }
    clip.text_content
        .as_deref()
        .ok_or_else(|| "Clip has no copyable content".to_string())
        .and_then(|text| clipboard.set_text(text).map_err(|error| error.to_string()))
}

pub fn internal_fingerprint(clip: &ClipItem) -> Result<String, String> {
    if clip.content_type == "file" {
        let paths = clip
            .text_content
            .as_deref()
            .ok_or_else(|| "File clip has no path metadata".to_string())
            .and_then(|value| {
                serde_json::from_str::<Vec<String>>(value)
                    .map_err(|_| "File clip has invalid path metadata".to_string())
            })?;
        return Ok(crate::clipboard_fingerprint::file_list(&paths));
    }
    if clip.content_type == "image" {
        let image_base64 = clip
            .image_base64
            .as_deref()
            .ok_or_else(|| "Image clip has no stored image data".to_string())?;
        let clean = image_base64.split(',').next_back().unwrap_or(image_base64);
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, clean)
            .map_err(|error| error.to_string())?;
        let image = image::load_from_memory(&bytes).map_err(|error| error.to_string())?;
        return Ok(crate::clipboard_fingerprint::image_rgba(
            image.to_rgba8().as_raw(),
        ));
    }
    clip.text_content
        .clone()
        .ok_or_else(|| "Clip has no copyable content".to_string())
}

pub fn copy_clip(
    db: &DbState,
    sequential: &SequentialQueueState,
    clip_id: i64,
) -> Result<(), String> {
    let clip = db
        .get_clip_by_id(clip_id)
        .map_err(|error| error.to_string())?;
    let fingerprint = internal_fingerprint(&clip)?;
    let mut clipboard = Clipboard::new().map_err(|error| error.to_string())?;
    sequential.mark_internal_clipboard_write(&fingerprint);
    if let Err(error) = write_clip(&mut clipboard, &clip) {
        sequential.clear_internal_clipboard_write();
        return Err(error);
    }
    Ok(())
}

pub fn execute_transform(
    db: &DbState,
    transform_ref: Option<&str>,
    paste_result: bool,
) -> Result<crate::transformation_service::ExecutionOutcome, String> {
    crate::features::require(db, crate::features::Feature::Transformations)?;
    let mut clipboard = Clipboard::new().map_err(|error| error.to_string())?;
    let input = clipboard.get_text().map_err(|error| error.to_string())?;
    let outcome =
        crate::transformation_service::execute_shortcut_manual_transform(db, input, transform_ref)
            .map_err(|error| error.to_string())?;
    clipboard
        .set_text(&outcome.output)
        .map_err(|error| error.to_string())?;
    if paste_result {
        // Keep the caller's clipboard-action guard until the synthetic paste
        // fires so another hotkey cannot replace the transformed output.
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = crate::paste_automation::paste();
    }
    Ok(outcome)
}

pub fn paste_clip(
    db: &DbState,
    app: &AppHandle,
    clip_id: i64,
    origin: PasteOrigin,
) -> Result<(), String> {
    let (failed_event, pasted_event, source) = match origin {
        PasteOrigin::Hud => ("hud_paste_failed", "hud_clip_pasted", "HUD"),
        PasteOrigin::ClipHotkey => (
            "app_hotkey_clip_paste_failed",
            "app_hotkey_clip_pasted",
            "a clip hotkey",
        ),
    };
    let clip = db
        .get_clip_by_id(clip_id)
        .map_err(|error| error.to_string())?;
    #[cfg(target_os = "macos")]
    if !crate::platform_capabilities::accessibility_status().is_trusted {
        let message = match origin {
            PasteOrigin::Hud => "HUD paste needs Accessibility access. Allow Pasted (or the terminal/IDE running this development build) in System Settings, then try again.",
            PasteOrigin::ClipHotkey => "Clip hotkey paste needs Accessibility access. Allow Pasted (or the terminal/IDE running this development build) in System Settings, then try again.",
        };
        return Err(message.to_string());
    }

    let paste_target = app.state::<Arc<crate::paste_target::PasteTargetState>>();
    let target = paste_target.prepare_last_external_for_hud()?;
    let fingerprint = internal_fingerprint(&clip)?;
    let mut clipboard = Clipboard::new()
        .map_err(|_| "The system clipboard is unavailable right now.".to_string())?;
    let sequential = app.state::<Arc<SequentialQueueState>>();
    sequential.mark_internal_clipboard_write(&fingerprint);
    if let Err(error) = write_clip(&mut clipboard, &clip) {
        sequential.clear_internal_clipboard_write();
        let explanation = match clip.content_type.as_str() {
            "file" => "This clip contains unavailable files.",
            "image" => "This clip's image cannot be prepared for pasting.",
            _ => "This clip's text cannot be prepared for pasting.",
        };
        let _ = db.log_activity(
            failed_event,
            &format!("{explanation} System detail: {error}"),
        );
        return Err(explanation.to_string());
    }

    if origin == PasteOrigin::Hud {
        if let Some(hud) = app.get_webview_window("hud") {
            let _ = hud.hide();
        }
    }
    if let Err(error) = paste_target.paste_clip_to(&target) {
        if origin == PasteOrigin::Hud {
            if let Some(hud) = app.get_webview_window("hud") {
                let _ = hud.show();
                let _ = hud.set_focus();
            }
        }
        let _ = db.log_activity(failed_event, &error);
        return Err(error);
    }

    let _ = db.log_activity(
        pasted_event,
        &format!("Pasted clip {} into {} from {source}", clip.id, target.name),
    );
    Ok(())
}

pub fn paste_hud_clip(db: &DbState, app: &AppHandle, clip_id: i64) -> Result<(), String> {
    crate::features::require(db, crate::features::Feature::Hud)?;
    paste_clip(db, app, clip_id, PasteOrigin::Hud)
}
