use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub const APP_LOCK_CHANGED: &str = "app-lock-changed";
pub const APP_MENU_ACTION: &str = "app-menu-action";
pub const APP_SETTING_CHANGED: &str = "app-setting-changed";
pub const CLIP_ADDED: &str = "clip-added";
pub const CLIP_LIBRARY_CHANGED: &str = "clip-library-changed";
pub const CLIPBOARD_PAUSE_CHANGED: &str = "clipboard-pause-changed";
pub const HOTKEY_REGISTRATION_CHANGED: &str = "hotkey-registration-changed";
pub const NAVIGATE_BIN: &str = "navigate-bin";
pub const NAVIGATE_TAB: &str = "navigate-tab";
pub const SEQUENTIAL_UPDATED: &str = "sequential-updated";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardPauseChanged {
    pub is_paused: bool,
    pub auto_paused_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClipLibraryChanged {
    pub clip_ids: Vec<i64>,
}

pub fn emit_clipboard_pause_changed(
    app: &AppHandle,
    is_paused: bool,
    auto_paused_by: Option<String>,
) {
    let _ = app.emit(
        CLIPBOARD_PAUSE_CHANGED,
        ClipboardPauseChanged {
            is_paused,
            auto_paused_by,
        },
    );
}

pub fn emit_clip_library_changed(app: &AppHandle, clip_ids: Vec<i64>) {
    let _ = app.emit(CLIP_LIBRARY_CHANGED, ClipLibraryChanged { clip_ids });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_pause_payload_has_one_stable_camel_case_shape() {
        assert_eq!(
            serde_json::to_value(ClipboardPauseChanged {
                is_paused: true,
                auto_paused_by: Some("Password Manager".into()),
            })
            .unwrap(),
            serde_json::json!({
                "isPaused": true,
                "autoPausedBy": "Password Manager",
            })
        );
    }

    #[test]
    fn library_invalidations_are_bounded_to_clip_identity() {
        assert_eq!(
            serde_json::to_value(ClipLibraryChanged {
                clip_ids: vec![7, 11],
            })
            .unwrap(),
            serde_json::json!({ "clipIds": [7, 11] })
        );
    }
}
