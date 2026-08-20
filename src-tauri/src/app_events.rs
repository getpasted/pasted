use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub use crate::app_event_names::*;

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
