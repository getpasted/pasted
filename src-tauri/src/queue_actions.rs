//! Shared Queue paste workflows used by GUI, hotkey, and live-app adapters.

use arboard::Clipboard;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

use crate::db::DbState;
use crate::sequential_paste::SequentialQueueState;

fn ensure_paste_available() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    if !crate::platform_capabilities::accessibility_status().is_trusted {
        return Err("Paste Next needs Accessibility access. Allow Pasted (or the terminal/IDE running this development build) in System Settings, then try again.".to_string());
    }
    Ok(())
}

fn restore_after_failure(app: &AppHandle) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_focus();
    }
}

fn restore_after_ui_paste(app: &AppHandle) {
    thread::sleep(Duration::from_millis(220));
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.set_focus();
    }
}

pub fn paste_item(
    queue: &SequentialQueueState,
    db: &DbState,
    app: &AppHandle,
    index: usize,
    restore_after_success: bool,
) -> Result<Option<String>, String> {
    let Some((item_id, text)) = queue.peek_item(index) else {
        return Ok(None);
    };
    if let Err(error) = ensure_paste_available() {
        let _ = db.log_activity("queue_paste_failed", &error);
        return Err(error);
    }
    let paste_target = app.state::<Arc<crate::paste_target::PasteTargetState>>();
    let target = match paste_target.prepare_last_external() {
        Ok(target) => target,
        Err(error) => {
            let _ = db.log_activity("queue_paste_failed", &error);
            return Err(error);
        }
    };
    queue.mark_internal_clipboard_write(&text);
    let mut clipboard = match Clipboard::new() {
        Ok(clipboard) => clipboard,
        Err(error) => {
            queue.clear_internal_clipboard_write();
            let message = format!("Could not access the clipboard for Queue paste: {error}");
            let _ = db.log_activity("queue_paste_failed", &message);
            return Err(message);
        }
    };
    if let Err(error) = clipboard.set_text(&text) {
        queue.clear_internal_clipboard_write();
        let message = format!("Could not place the next Queue item on the clipboard: {error}");
        let _ = db.log_activity("queue_paste_failed", &message);
        return Err(message);
    }
    if let Err(error) = paste_target.paste_to(&target) {
        queue.clear_internal_clipboard_write();
        restore_after_failure(app);
        let _ = db.log_activity("queue_paste_failed", &error);
        return Err(error);
    }
    if let Err(error) = queue.consume_item(item_id) {
        queue.clear_internal_clipboard_write();
        restore_after_failure(app);
        let message =
            format!("The Queue item was copied but could not be committed as pasted: {error}");
        let _ = db.log_activity("queue_paste_failed", &message);
        return Err(message);
    }
    let status = queue.get_status();
    let _ = app.emit("sequential-updated", status.clone());
    let _ = db.log_activity(
        "queue_item_pasted",
        &format!(
            "Pasted the next Queue item ({} remaining)",
            status.total_count
        ),
    );
    if restore_after_success {
        restore_after_ui_paste(app);
    }
    Ok(Some(text))
}

pub fn paste_all(
    queue: &SequentialQueueState,
    db: &DbState,
    app: &AppHandle,
) -> Result<Option<String>, String> {
    let status = queue.get_status();
    if status.queue.is_empty() {
        return Ok(None);
    }
    if let Err(error) = ensure_paste_available() {
        let _ = db.log_activity("queue_paste_failed", &error);
        return Err(error);
    }
    let paste_target = app.state::<Arc<crate::paste_target::PasteTargetState>>();
    let target = match paste_target.prepare_last_external() {
        Ok(target) => target,
        Err(error) => {
            let _ = db.log_activity("queue_paste_failed", &error);
            return Err(error);
        }
    };
    let combined = status.queue.join("\n\n");
    queue.mark_internal_clipboard_write(&combined);
    let mut clipboard = match Clipboard::new() {
        Ok(clipboard) => clipboard,
        Err(error) => {
            queue.clear_internal_clipboard_write();
            let message = format!("Could not access the clipboard for Queue paste: {error}");
            let _ = db.log_activity("queue_paste_failed", &message);
            return Err(message);
        }
    };
    if let Err(error) = clipboard.set_text(&combined) {
        queue.clear_internal_clipboard_write();
        let message = format!("Could not place the Queue on the clipboard: {error}");
        let _ = db.log_activity("queue_paste_failed", &message);
        return Err(message);
    }
    if let Err(error) = paste_target.paste_to(&target) {
        queue.clear_internal_clipboard_write();
        restore_after_failure(app);
        let _ = db.log_activity("queue_paste_failed", &error);
        return Err(error);
    }
    if let Err(error) = queue.consume_prefix(&status.item_ids) {
        queue.clear_internal_clipboard_write();
        restore_after_failure(app);
        let message = format!("The Queue pasted but could not be cleared: {error}");
        let _ = db.log_activity("queue_paste_failed", &message);
        return Err(message);
    }
    let _ = app.emit("sequential-updated", queue.get_status());
    let _ = db.log_activity(
        "queue_all_pasted",
        &format!("Pasted {} Queue items together", status.total_count),
    );
    restore_after_ui_paste(app);
    Ok(Some(combined))
}
