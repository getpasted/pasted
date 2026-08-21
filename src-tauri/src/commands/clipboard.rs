use arboard::Clipboard;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

use crate::db::DbState;
use crate::sequential_paste::SequentialQueueState;

#[tauri::command]
pub fn copy_clip_to_system(
    text: Option<String>,
    image_base64: Option<String>,
    file_paths: Option<Vec<String>>,
) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;

    if let Some(paths) = file_paths {
        if paths.is_empty() || !crate::resource_limits::file_list_within_limit(&paths) {
            return Err("File list exceeds Pasted's safety limit".to_string());
        }
        clipboard
            .set()
            .file_list(&paths)
            .map_err(|error| error.to_string())?;
    } else if let Some(img_b64) = image_base64 {
        // Strip data:image/png;base64,
        let clean = img_b64.split(',').next_back().unwrap_or(&img_b64);
        if clean.len() > crate::resource_limits::MAX_STORED_IMAGE_BASE64_BYTES {
            return Err("Clip image exceeds Pasted's safety limit".to_string());
        }
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, clean)
            .map_err(|e| e.to_string())?;

        let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
        let rgba = img.to_rgba8();
        let img_data = arboard::ImageData {
            width: rgba.width() as usize,
            height: rgba.height() as usize,
            bytes: std::borrow::Cow::Owned(rgba.into_raw()),
        };
        clipboard.set_image(img_data).map_err(|e| e.to_string())?;
    } else if let Some(t) = text {
        if t.len() > crate::resource_limits::MAX_CLIP_TEXT_BYTES {
            return Err("Clip text exceeds Pasted's safety limit".to_string());
        }
        clipboard.set_text(t).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn copy_clip_by_id(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
    sequential: State<'_, Arc<SequentialQueueState>>,
) -> Result<(), String> {
    copy_clip_by_id_shared(&db, &sequential, clip_id)
}

pub(crate) fn copy_clip_by_id_shared(
    db: &DbState,
    sequential: &SequentialQueueState,
    clip_id: i64,
) -> Result<(), String> {
    crate::clipboard_actions::copy_clip(db, sequential, clip_id)
}

#[tauri::command]
pub fn paste_text_to_frontmost(text: String, app: AppHandle) -> Result<(), String> {
    if text.len() > crate::resource_limits::MAX_CLIP_TEXT_BYTES {
        return Err("Clip text exceeds Pasted's 8 MB safety limit".to_string());
    }
    let mut clipboard = Clipboard::new().map_err(|error| error.to_string())?;
    clipboard
        .set_text(text)
        .map_err(|error| error.to_string())?;

    if let Some(hud) = app.get_webview_window("hud") {
        let _ = hud.hide();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.hide();
    }

    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = crate::paste_automation::paste();
    });

    Ok(())
}

#[tauri::command]
pub fn paste_clip_by_id(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<(), String> {
    paste_clip_from_hud(&db, &app, clip_id)
}

pub(crate) fn paste_clip_from_hud(
    db: &DbState,
    app: &AppHandle,
    clip_id: i64,
) -> Result<(), String> {
    crate::clipboard_actions::paste_hud_clip(db, app, clip_id)
}
