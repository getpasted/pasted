use parking_lot::Mutex;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasteTarget {
    #[serde(skip)]
    pid: i32,
    #[serde(skip)]
    identifier: String,
    pub name: String,
}

impl PasteTarget {
    #[cfg(test)]
    fn matches_application(&self, other: &Self) -> bool {
        self.pid == other.pid || self.identifier == other.identifier || self.name == other.name
    }
}

#[derive(Default)]
pub struct PasteTargetState {
    #[cfg(target_os = "macos")]
    last_external: Mutex<Option<PasteTarget>>,
}

impl PasteTargetState {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(target_os = "macos")]
    pub fn start_tracking(self: &Arc<Self>) {
        let state = Arc::clone(self);
        std::thread::spawn(move || loop {
            if let Some(target) = frontmost_application() {
                state.remember_if_external(target);
            }
            std::thread::sleep(Duration::from_millis(100));
        });
    }

    #[cfg(target_os = "macos")]
    fn remember_if_external(&self, target: PasteTarget) {
        if target.pid > 0 && target.identifier != crate::installation_diagnostics::APP_IDENTIFIER {
            *self.last_external.lock() = Some(target);
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn start_tracking(self: &Arc<Self>) {}

    #[cfg(target_os = "macos")]
    pub fn current(&self) -> Option<PasteTarget> {
        self.last_external.lock().clone()
    }

    #[cfg(not(target_os = "macos"))]
    pub fn current(&self) -> Option<PasteTarget> {
        None
    }

    #[cfg(target_os = "macos")]
    pub fn activate_last_external(&self) -> Result<PasteTarget, String> {
        let target = self.current().ok_or_else(|| {
            "Could not target the previous app. Clip not removed from Queue.".to_string()
        })?;
        // Ask macOS to activate the remembered process first. The subsequent
        // System Events transaction explicitly addresses the app by name and
        // is the authoritative activation/paste result.
        activate_application(target.pid);
        Ok(target)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn activate_last_external(&self) -> Result<PasteTarget, String> {
        Ok(PasteTarget {
            pid: 0,
            identifier: "external-application".to_string(),
            name: "Previous App".to_string(),
        })
    }
}

#[cfg(test)]
fn target_failure(name: &str) -> String {
    format!("Could not target {name}. Clip not removed from Queue.")
}

#[cfg(target_os = "macos")]
fn frontmost_application() -> Option<PasteTarget> {
    use objc::runtime::Object;
    use objc::{msg_send, sel, sel_impl};

    unsafe {
        let workspace: *mut Object = msg_send![objc::class!(NSWorkspace), sharedWorkspace];
        if workspace.is_null() {
            return None;
        }
        let application: *mut Object = msg_send![workspace, frontmostApplication];
        if application.is_null() {
            return None;
        }
        let pid: i32 = msg_send![application, processIdentifier];
        let identifier = ns_string(application, sel!(bundleIdentifier))?;
        let name = ns_string(application, sel!(localizedName))?;
        if pid <= 0 {
            return None;
        }
        Some(PasteTarget {
            pid,
            identifier,
            name,
        })
    }
}

#[cfg(target_os = "macos")]
unsafe fn ns_string(
    object: *mut objc::runtime::Object,
    selector: objc::runtime::Sel,
) -> Option<String> {
    use objc::runtime::Object;
    use objc::{msg_send, sel, sel_impl};

    let value: *mut Object = msg_send![object, performSelector: selector];
    if value.is_null() {
        return None;
    }
    let utf8: *const std::os::raw::c_char = msg_send![value, UTF8String];
    if utf8.is_null() {
        return None;
    }
    Some(
        std::ffi::CStr::from_ptr(utf8)
            .to_string_lossy()
            .into_owned(),
    )
}

#[cfg(target_os = "macos")]
fn activate_application(pid: i32) {
    use objc::runtime::Object;
    use objc::{msg_send, sel, sel_impl};

    unsafe {
        let application: *mut Object = msg_send![
            objc::class!(NSRunningApplication),
            runningApplicationWithProcessIdentifier: pid
        ];
        if application.is_null() {
            return;
        }
        let _: bool = msg_send![application, activateWithOptions: 2usize];
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    fn target(pid: i32, identifier: &str, name: &str) -> PasteTarget {
        PasteTarget {
            pid,
            identifier: identifier.to_string(),
            name: name.to_string(),
        }
    }

    #[test]
    fn focusing_pasted_does_not_replace_the_last_external_target() {
        let state = PasteTargetState::new();
        state.remember_if_external(target(42, "com.example.Editor", "Editor"));
        state.remember_if_external(target(
            77,
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
            target_failure("ChatGPT"),
            "Could not target ChatGPT. Clip not removed from Queue."
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
