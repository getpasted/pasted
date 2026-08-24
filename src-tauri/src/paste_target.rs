use parking_lot::Mutex;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;

mod platform;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasteTarget {
    #[serde(skip)]
    pid: i32,
    #[serde(skip)]
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    identifier: String,
    #[serde(skip)]
    #[cfg_attr(not(any(target_os = "windows", target_os = "linux")), allow(dead_code))]
    native_handle: u64,
    pub name: String,
    pub automatic_paste_available: bool,
    pub unavailable_reason: Option<String>,
}

impl PasteTarget {
    pub(super) fn available(
        pid: i32,
        identifier: String,
        native_handle: u64,
        name: String,
    ) -> Self {
        Self {
            pid,
            identifier,
            native_handle,
            name,
            automatic_paste_available: true,
            unavailable_reason: None,
        }
    }

    fn unavailable(name: &str, reason: String) -> Self {
        Self {
            pid: 0,
            identifier: String::new(),
            native_handle: 0,
            name: name.to_string(),
            automatic_paste_available: false,
            unavailable_reason: Some(reason),
        }
    }

    #[cfg(test)]
    fn matches_application(&self, other: &Self) -> bool {
        self.pid == other.pid || self.identifier == other.identifier || self.name == other.name
    }
}

pub struct PasteTargetState {
    last_external: Mutex<Option<PasteTarget>>,
    unavailable_reason: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PasteAction {
    Queue,
    Hud,
}

impl PasteAction {
    pub(super) fn target_failure(self, name: &str) -> String {
        match self {
            Self::Queue => format!("Could not target {name}. Clip not removed from Queue."),
            Self::Hud => format!("Could not target {name}. HUD paste was cancelled."),
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn accessibility_failure(self) -> String {
        let action = match self {
            Self::Queue => "Paste Next",
            Self::Hud => "HUD paste",
        };
        format!("macOS blocked {action}. Allow Accessibility access for Pasted (or the terminal/IDE running this development build), then try again.")
    }
}

impl Default for PasteTargetState {
    fn default() -> Self {
        Self {
            last_external: Mutex::new(None),
            unavailable_reason: platform::unavailable_reason(),
        }
    }
}

impl PasteTargetState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_tracking(self: &Arc<Self>) {
        if self.unavailable_reason.is_some() {
            return;
        }

        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            let state = Arc::clone(self);
            std::thread::spawn(move || loop {
                if let Some(target) = platform::frontmost_application() {
                    state.remember_if_external(target);
                }
                std::thread::sleep(Duration::from_millis(150));
            });
        }

        #[cfg(target_os = "linux")]
        {
            let state = Arc::clone(self);
            std::thread::spawn(move || {
                let mut last_window_id = 0;
                loop {
                    if let Some(window_id) = platform::active_window_id() {
                        if window_id != last_window_id {
                            last_window_id = window_id;
                            if let Some(target) = platform::application_for_window(window_id) {
                                state.remember_if_external(target);
                            }
                        }
                    }
                    std::thread::sleep(Duration::from_millis(250));
                }
            });
        }
    }

    fn remember_if_external(&self, target: PasteTarget) {
        if target.pid <= 0 {
            return;
        }
        #[cfg(target_os = "macos")]
        if target.identifier == crate::installation_diagnostics::APP_IDENTIFIER {
            return;
        }
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        if target.pid as u32 == std::process::id() {
            return;
        }
        *self.last_external.lock() = Some(target);
    }

    pub fn current(&self) -> Option<PasteTarget> {
        self.last_external.lock().clone()
    }

    pub fn snapshot(&self) -> PasteTarget {
        if let Some(reason) = &self.unavailable_reason {
            return PasteTarget::unavailable("Automatic paste unavailable", reason.clone());
        }
        self.current().unwrap_or_else(|| {
            PasteTarget::unavailable(
                "No target yet",
                "Focus another app before pasting from Queue.".to_string(),
            )
        })
    }

    pub fn prepare_last_external(&self) -> Result<PasteTarget, String> {
        self.prepare(PasteAction::Queue)
    }

    pub fn prepare_last_external_for_hud(&self) -> Result<PasteTarget, String> {
        self.prepare(PasteAction::Hud)
    }

    fn prepare(&self, action: PasteAction) -> Result<PasteTarget, String> {
        let snapshot = self.snapshot();
        if snapshot.automatic_paste_available {
            return Ok(snapshot);
        }
        let reason = snapshot
            .unavailable_reason
            .unwrap_or_else(|| "Automatic paste is unavailable.".to_string());
        match action {
            PasteAction::Queue => Err(reason),
            PasteAction::Hud => Err(reason
                .replace("Queue paste", "HUD paste")
                .replace("pasting from Queue", "using HUD")
                .replace("Clip not removed from Queue.", "HUD paste was cancelled.")),
        }
    }

    pub fn paste_to(&self, target: &PasteTarget) -> Result<(), String> {
        if !target.automatic_paste_available {
            return Err(target.unavailable_reason.clone().unwrap_or_else(|| {
                "Automatic Queue paste is unavailable. Clip not removed from Queue.".to_string()
            }));
        }
        platform::paste_to_target(target, PasteAction::Queue)
    }

    pub fn paste_clip_to(&self, target: &PasteTarget) -> Result<(), String> {
        if !target.automatic_paste_available {
            return Err(target
                .unavailable_reason
                .clone()
                .unwrap_or_else(|| "Automatic HUD paste is unavailable.".to_string()));
        }
        platform::paste_to_target(target, PasteAction::Hud)
    }
}

/// Best-effort name of the application that currently owns keyboard focus.
///
/// Clipboard capture uses this shared platform adapter for App Exclusions,
/// while Queue and HUD paste retain the richer target record above.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ActiveApplicationContext {
    pub name: String,
    // This value is used only for exact private-mode markers and is never
    // persisted, logged, or included in an event payload.
    pub window_title: Option<String>,
    pub window_title_is_accessible: bool,
}

pub(crate) fn active_application_context(
    include_private_mode_signal: bool,
) -> Option<ActiveApplicationContext> {
    platform::active_application_context(include_private_mode_signal)
}

pub(crate) fn active_application_name() -> Option<String> {
    active_application_context(false).map(|context| context.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(pid: i32, identifier: &str, name: &str) -> PasteTarget {
        PasteTarget::available(pid, identifier.to_string(), 0, name.to_string())
    }

    #[test]
    fn focusing_pasted_does_not_replace_the_last_external_target() {
        let state = PasteTargetState::new();
        state.remember_if_external(target(42, "com.example.Editor", "Editor"));
        state.remember_if_external(target(
            std::process::id() as i32,
            crate::installation_diagnostics::APP_IDENTIFIER,
            "Pasted",
        ));
        let remembered = state.current().unwrap();
        assert_eq!(remembered.pid, 42);
        assert_eq!(remembered.name, "Editor");
    }

    #[test]
    fn target_failure_names_the_app_and_preserves_queue_semantics() {
        assert_eq!(
            PasteAction::Queue.target_failure("ChatGPT"),
            "Could not target ChatGPT. Clip not removed from Queue."
        );
        assert_eq!(
            PasteAction::Hud.target_failure("ChatGPT"),
            "Could not target ChatGPT. HUD paste was cancelled."
        );
    }

    #[test]
    fn target_matching_allows_multi_process_apps_but_not_unrelated_apps() {
        let original = target(42, "com.openai.chat", "ChatGPT");
        assert!(original.matches_application(&target(77, "com.openai.chat.helper", "ChatGPT")));
        assert!(original.matches_application(&target(81, "com.openai.chat", "ChatGPT Helper")));
        assert!(!original.matches_application(&target(99, "com.apple.finder", "Finder")));
    }
}
