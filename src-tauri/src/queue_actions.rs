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

fn commit_item_with_ports<Write, Paste>(
    queue: &SequentialQueueState,
    index: usize,
    mut write_clipboard: Write,
    mut paste_to_target: Paste,
) -> Result<Option<String>, String>
where
    Write: FnMut(&str) -> Result<(), String>,
    Paste: FnMut() -> Result<(), String>,
{
    let Some((item_id, text)) = queue.peek_item(index) else {
        return Ok(None);
    };
    queue.mark_internal_clipboard_write(&text);
    if let Err(error) = write_clipboard(&text) {
        queue.clear_internal_clipboard_write();
        return Err(error);
    }
    if let Err(error) = paste_to_target() {
        queue.clear_internal_clipboard_write();
        return Err(error);
    }
    if let Err(error) = queue.consume_item(item_id) {
        queue.clear_internal_clipboard_write();
        return Err(format!(
            "The Queue item was copied but could not be committed as pasted: {error}"
        ));
    }
    Ok(Some(text))
}

fn commit_all_with_ports<Write, Paste>(
    queue: &SequentialQueueState,
    mut write_clipboard: Write,
    mut paste_to_target: Paste,
) -> Result<Option<String>, String>
where
    Write: FnMut(&str) -> Result<(), String>,
    Paste: FnMut() -> Result<(), String>,
{
    let status = queue.get_status();
    if status.queue.is_empty() {
        return Ok(None);
    }
    let combined = status.queue.join("\n\n");
    queue.mark_internal_clipboard_write(&combined);
    if let Err(error) = write_clipboard(&combined) {
        queue.clear_internal_clipboard_write();
        return Err(error);
    }
    if let Err(error) = paste_to_target() {
        queue.clear_internal_clipboard_write();
        return Err(error);
    }
    if let Err(error) = queue.consume_prefix(&status.item_ids) {
        queue.clear_internal_clipboard_write();
        return Err(format!(
            "The Queue pasted but could not be cleared: {error}"
        ));
    }
    Ok(Some(combined))
}

pub fn paste_item(
    queue: &SequentialQueueState,
    db: &DbState,
    app: &AppHandle,
    index: usize,
    restore_after_success: bool,
) -> Result<Option<String>, String> {
    if queue.peek_item(index).is_none() {
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
    let mut clipboard = match Clipboard::new() {
        Ok(clipboard) => clipboard,
        Err(error) => {
            queue.clear_internal_clipboard_write();
            let message = format!("Could not access the clipboard for Queue paste: {error}");
            let _ = db.log_activity("queue_paste_failed", &message);
            return Err(message);
        }
    };
    let pasted = commit_item_with_ports(
        queue,
        index,
        |text| {
            clipboard.set_text(text).map_err(|error| {
                format!("Could not place the next Queue item on the clipboard: {error}")
            })
        },
        || paste_target.paste_to(&target),
    );
    let text = match pasted {
        Ok(Some(text)) => text,
        Ok(None) => return Ok(None),
        Err(error) => {
            restore_after_failure(app);
            let _ = db.log_activity("queue_paste_failed", &error);
            return Err(error);
        }
    };
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
    let mut clipboard = match Clipboard::new() {
        Ok(clipboard) => clipboard,
        Err(error) => {
            queue.clear_internal_clipboard_write();
            let message = format!("Could not access the clipboard for Queue paste: {error}");
            let _ = db.log_activity("queue_paste_failed", &message);
            return Err(message);
        }
    };
    let combined = match commit_all_with_ports(
        queue,
        |text| {
            clipboard
                .set_text(text)
                .map_err(|error| format!("Could not place the Queue on the clipboard: {error}"))
        },
        || paste_target.paste_to(&target),
    ) {
        Ok(Some(combined)) => combined,
        Ok(None) => return Ok(None),
        Err(error) => {
            restore_after_failure(app);
            let _ = db.log_activity("queue_paste_failed", &error);
            return Err(error);
        }
    };
    let _ = app.emit("sequential-updated", queue.get_status());
    let _ = db.log_activity(
        "queue_all_pasted",
        &format!("Pasted {} Queue items together", status.total_count),
    );
    restore_after_ui_paste(app);
    Ok(Some(combined))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue_with(items: &[&str]) -> SequentialQueueState {
        let queue = SequentialQueueState::new();
        for item in items {
            queue.push_item((*item).to_string()).unwrap();
        }
        queue
    }

    #[test]
    fn failed_target_does_not_consume_an_item() {
        let queue = queue_with(&["first", "second"]);
        let error = commit_item_with_ports(
            &queue,
            0,
            |_| Ok(()),
            || Err("target unavailable".to_string()),
        )
        .unwrap_err();
        assert_eq!(error, "target unavailable");
        assert_eq!(queue.get_status().queue, vec!["first", "second"]);
        assert!(!queue.consume_internal_clipboard_write("first"));
    }

    #[test]
    fn successful_target_consumes_only_the_committed_item() {
        let queue = queue_with(&["first", "second"]);
        let mut written = String::new();
        let pasted = commit_item_with_ports(
            &queue,
            0,
            |text| {
                written = text.to_string();
                Ok(())
            },
            || Ok(()),
        )
        .unwrap();
        assert_eq!(pasted.as_deref(), Some("first"));
        assert_eq!(written, "first");
        assert_eq!(queue.get_status().queue, vec!["second"]);
    }

    #[test]
    fn paste_all_is_atomic_at_the_target_boundary() {
        let queue = queue_with(&["one", "two"]);
        assert!(commit_all_with_ports(&queue, |_| Ok(()), || Err("blocked".into())).is_err());
        assert_eq!(queue.get_status().queue, vec!["one", "two"]);
        let result = commit_all_with_ports(&queue, |_| Ok(()), || Ok(())).unwrap();
        assert_eq!(result.as_deref(), Some("one\n\ntwo"));
        assert!(queue.get_status().queue.is_empty());
    }
}
