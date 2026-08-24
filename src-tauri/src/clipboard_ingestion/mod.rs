mod files;
mod image;
mod text;

use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use crate::app_exclusions::{AppExclusionRule, ExcludedCaptureKind};
use crate::db::DbState;
use crate::sequential_paste::SequentialQueueState;

pub(crate) use files::ingest_files;
pub(crate) use image::ingest_image;
pub(crate) use text::ingest_text;

#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum CaptureFeedbackKind {
    Success,
    Ignored,
    Failure,
}

fn capture_feedback_payload(kind: CaptureFeedbackKind, clip_id: Option<i64>) -> serde_json::Value {
    match clip_id {
        Some(clip_id) => serde_json::json!({ "kind": kind, "clip_id": clip_id }),
        None => serde_json::json!({ "kind": kind }),
    }
}

pub(crate) struct CaptureContext<'a> {
    pub(crate) app: &'a AppHandle,
    pub(crate) db: &'a Arc<DbState>,
    pub(crate) queue: &'a Arc<SequentialQueueState>,
    pub(crate) active_app: Option<&'a str>,
    pub(crate) active_exclusion: Option<&'a AppExclusionRule>,
    pub(crate) source: &'a str,
    pub(crate) suppressed: bool,
}

impl CaptureContext<'_> {
    pub(crate) fn begin_hash(&self, last_hash: &mut String, hash: &str) -> bool {
        capture_preflight(last_hash, hash, self.suppressed) == CapturePreflight::Ready
    }

    pub(crate) fn ignore_excluded(&self, kind: ExcludedCaptureKind) -> bool {
        if !self
            .active_exclusion
            .is_some_and(|rule| crate::app_exclusions::ignores_capture(rule, kind))
        {
            return false;
        }
        if let Some(active_app) = self.active_app {
            let _ = self.app.emit(
                "blacklist-clip-ignored",
                serde_json::json!({ "app_name": active_app }),
            );
        }
        self.feedback(CaptureFeedbackKind::Ignored, None);
        true
    }

    pub(crate) fn report_ignored(&self, reason: &str) {
        let _ = self.db.log_activity("clipboard_capture_ignored", reason);
        let _ = self.app.emit(
            "clipboard-clip-ignored",
            serde_json::json!({ "reason": reason }),
        );
        self.feedback(CaptureFeedbackKind::Ignored, None);
    }

    pub(crate) fn report_failed(&self) {
        self.feedback(CaptureFeedbackKind::Failure, None);
    }

    pub(crate) fn report_success(&self, clip_id: i64) {
        self.feedback(CaptureFeedbackKind::Success, Some(clip_id));
    }

    fn feedback(&self, kind: CaptureFeedbackKind, clip_id: Option<i64>) {
        if !crate::features::is_enabled(self.db, crate::features::Feature::Notifications) {
            return;
        }
        let _ = self.app.emit_to(
            "capture-feedback",
            "clipboard-capture-feedback",
            capture_feedback_payload(kind, clip_id),
        );
    }
}

#[derive(Debug, PartialEq, Eq)]
enum CapturePreflight {
    Duplicate,
    Suppressed,
    Ready,
}

fn capture_preflight(
    last_hash: &mut String,
    candidate_hash: &str,
    suppressed: bool,
) -> CapturePreflight {
    if last_hash == candidate_hash {
        return CapturePreflight::Duplicate;
    }
    candidate_hash.clone_into(last_hash);
    if suppressed {
        CapturePreflight::Suppressed
    } else {
        CapturePreflight::Ready
    }
}

#[cfg(test)]
mod tests {
    use super::{
        capture_feedback_payload, capture_preflight, CaptureFeedbackKind, CapturePreflight,
    };

    #[test]
    fn capture_preflight_distinguishes_ready_suppressed_and_duplicate_payloads() {
        let mut last_hash = String::new();
        assert_eq!(
            capture_preflight(&mut last_hash, "first", false),
            CapturePreflight::Ready
        );
        assert_eq!(last_hash, "first");
        assert_eq!(
            capture_preflight(&mut last_hash, "first", false),
            CapturePreflight::Duplicate
        );
        assert_eq!(
            capture_preflight(&mut last_hash, "second", true),
            CapturePreflight::Suppressed
        );
        assert_eq!(last_hash, "second");
    }

    #[test]
    fn capture_feedback_never_contains_clipboard_data() {
        for (kind, expected) in [
            (CaptureFeedbackKind::Success, "success"),
            (CaptureFeedbackKind::Ignored, "ignored"),
            (CaptureFeedbackKind::Failure, "failure"),
        ] {
            let payload = capture_feedback_payload(kind, None);
            assert_eq!(payload, serde_json::json!({ "kind": expected }));
            assert_eq!(payload.as_object().map(|object| object.len()), Some(1));
        }
        assert_eq!(
            capture_feedback_payload(CaptureFeedbackKind::Success, Some(42)),
            serde_json::json!({ "kind": "success", "clip_id": 42 })
        );
    }
}
