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
    fn available(pid: i32, identifier: String, native_handle: u64, name: String) -> Self {
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
enum PasteAction {
    Queue,
    Hud,
}

impl PasteAction {
    fn target_failure(self, name: &str) -> String {
        match self {
            Self::Queue => format!("Could not target {name}. Clip not removed from Queue."),
            Self::Hud => format!("Could not target {name}. HUD paste was cancelled."),
        }
    }

    #[cfg(target_os = "macos")]
    fn accessibility_failure(self) -> String {
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
            unavailable_reason: platform_unavailable_reason(),
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
                if let Some(target) = frontmost_application() {
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
                    if let Some(window_id) = active_x11_window_id() {
                        if window_id != last_window_id {
                            last_window_id = window_id;
                            if let Some(target) = x11_application_for_window(window_id) {
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
        paste_to_target(target, PasteAction::Queue)
    }

    pub fn paste_clip_to(&self, target: &PasteTarget) -> Result<(), String> {
        if !target.automatic_paste_available {
            return Err(target
                .unavailable_reason
                .clone()
                .unwrap_or_else(|| "Automatic HUD paste is unavailable.".to_string()));
        }
        paste_to_target(target, PasteAction::Hud)
    }
}

/// Best-effort name of the application that currently owns keyboard focus.
///
/// Clipboard capture uses this shared platform adapter for App Exclusions,
/// while Queue and HUD paste retain the richer target record above.
pub(crate) fn active_application_name() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        frontmost_application_name()
    }

    #[cfg(target_os = "windows")]
    {
        frontmost_application()
            .and_then(|target| windows_application_name(target.pid).or(Some(target.name)))
    }

    #[cfg(target_os = "linux")]
    {
        // Native Wayland intentionally does not expose the globally focused
        // application. Avoid spawning xdotool in the clipboard monitor's hot
        // polling path; it can only describe XWayland windows in this session.
        if is_native_wayland_session() {
            return None;
        }
        active_x11_window_id().and_then(|window_id| {
            x11_application_name_for_window(window_id)
                .or_else(|| x11_application_for_window(window_id).map(|target| target.name))
        })
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    None
}

#[cfg(target_os = "macos")]
fn frontmost_application_name() -> Option<String> {
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
        // Clipboard attribution needs only the display name. Do not make it
        // depend on the bundle identifier required by automatic paste targets:
        // transient screenshot helpers may legitimately omit that identifier.
        ns_string(application, sel!(localizedName))
    }
}

#[cfg(target_os = "macos")]
fn platform_unavailable_reason() -> Option<String> {
    None
}

#[cfg(target_os = "windows")]
fn platform_unavailable_reason() -> Option<String> {
    None
}

#[cfg(target_os = "linux")]
fn platform_unavailable_reason() -> Option<String> {
    linux_platform_unavailable_reason(
        std::env::var("XDG_SESSION_TYPE").ok().as_deref(),
        std::env::var_os("WAYLAND_DISPLAY").is_some(),
        std::env::var_os("DISPLAY").is_some(),
        command_is_available("xdotool"),
    )
}

#[cfg(target_os = "linux")]
fn is_native_wayland_session() -> bool {
    std::env::var("XDG_SESSION_TYPE").is_ok_and(|value| value.eq_ignore_ascii_case("wayland"))
        || (std::env::var_os("WAYLAND_DISPLAY").is_some()
            && !std::env::var("XDG_SESSION_TYPE")
                .is_ok_and(|value| value.eq_ignore_ascii_case("x11")))
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn platform_unavailable_reason() -> Option<String> {
    Some(
        "Automatic Queue paste is unavailable on this platform. Clip not removed from Queue."
            .to_string(),
    )
}

#[cfg(any(target_os = "linux", test))]
fn linux_platform_unavailable_reason(
    session_type: Option<&str>,
    has_wayland_display: bool,
    has_x11_display: bool,
    has_xdotool: bool,
) -> Option<String> {
    if session_type.is_some_and(|value| value.eq_ignore_ascii_case("wayland"))
        || (has_wayland_display
            && !session_type.is_some_and(|value| value.eq_ignore_ascii_case("x11")))
    {
        return Some("This Wayland session does not allow reliable automatic pasting. Clip not removed from Queue.".to_string());
    }
    if !has_x11_display {
        return Some(
            "Automatic Queue paste needs an X11 session. Clip not removed from Queue.".to_string(),
        );
    }
    if !has_xdotool {
        return Some(
            "Automatic Queue paste needs xdotool in this X11 session. Clip not removed from Queue."
                .to_string(),
        );
    }
    None
}

#[cfg(target_os = "linux")]
fn command_is_available(command: &str) -> bool {
    std::process::Command::new(command)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
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
        (pid > 0).then(|| PasteTarget::available(pid, identifier, 0, name))
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
    (!utf8.is_null()).then(|| {
        std::ffi::CStr::from_ptr(utf8)
            .to_string_lossy()
            .into_owned()
    })
}

#[cfg(target_os = "macos")]
fn paste_to_target(target: &PasteTarget, action: PasteAction) -> Result<(), String> {
    use std::process::Command;
    const SCRIPT: &str = r#"
on run argv
    set targetName to item 1 of argv
    tell application "System Events"
        if not (exists first application process whose name is targetName) then error "target unavailable"
        set targetProcess to first application process whose name is targetName
        set frontmost of targetProcess to true
        delay 0.15
        keystroke "v" using command down
    end tell
end run
"#;
    let output = Command::new("osascript")
        .arg("-e")
        .arg(SCRIPT)
        .arg("--")
        .arg(&target.name)
        .output()
        .map_err(|error| format!("Could not start macOS paste automation: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if detail.contains("not authorized") || detail.contains("-1743") {
            Err(action.accessibility_failure())
        } else {
            Err(action.target_failure(&target.name))
        }
    }
}

#[cfg(target_os = "windows")]
fn frontmost_application() -> Option<PasteTarget> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    let handle = unsafe { GetForegroundWindow() };
    if handle == 0 {
        return None;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(handle, &mut pid) };
    if pid == 0 {
        return None;
    }
    let length = unsafe { GetWindowTextLengthW(handle) };
    let mut buffer = vec![0u16; (length.max(0) + 1) as usize];
    let copied = unsafe { GetWindowTextW(handle, buffer.as_mut_ptr(), buffer.len() as i32) };
    let name = if copied > 0 {
        OsString::from_wide(&buffer[..copied as usize])
            .to_string_lossy()
            .into_owned()
    } else {
        "Previous app".to_string()
    };
    Some(PasteTarget::available(
        pid as i32,
        format!("windows:{pid}"),
        handle as u64,
        name,
    ))
}

#[cfg(target_os = "windows")]
fn windows_application_name(pid: i32) -> Option<String> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::Path;

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid as u32) };
    if process == 0 {
        return None;
    }
    let mut buffer = vec![0u16; 32_768];
    let mut length = buffer.len() as u32;
    let success =
        unsafe { QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length) != 0 };
    unsafe { CloseHandle(process) };
    if !success || length == 0 {
        return None;
    }
    let path = OsString::from_wide(&buffer[..length as usize]);
    Path::new(&path)
        .file_stem()
        .and_then(|name| name.to_str())
        .map(str::to_string)
}

#[cfg(target_os = "windows")]
fn paste_to_target(target: &PasteTarget, action: PasteAction) -> Result<(), String> {
    let handle = target.native_handle as isize;
    if handle == 0
        || unsafe { IsWindow(handle) } == 0
        || unsafe { SetForegroundWindow(handle) } == 0
    {
        return Err(action.target_failure(&target.name));
    }
    std::thread::sleep(Duration::from_millis(120));
    if unsafe { GetForegroundWindow() } != handle {
        return Err(action.target_failure(&target.name));
    }
    unsafe {
        keybd_event(VK_CONTROL, 0, 0, 0);
        keybd_event(b'V', 0, 0, 0);
        keybd_event(b'V', 0, KEYEVENTF_KEYUP, 0);
        keybd_event(VK_CONTROL, 0, KEYEVENTF_KEYUP, 0);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
const VK_CONTROL: u8 = 0x11;
#[cfg(target_os = "windows")]
const KEYEVENTF_KEYUP: u32 = 0x0002;

#[cfg(target_os = "windows")]
#[link(name = "user32")]
extern "system" {
    fn GetForegroundWindow() -> isize;
    fn GetWindowThreadProcessId(window: isize, process_id: *mut u32) -> u32;
    fn GetWindowTextLengthW(window: isize) -> i32;
    fn GetWindowTextW(window: isize, text: *mut u16, max_count: i32) -> i32;
    fn IsWindow(window: isize) -> i32;
    fn SetForegroundWindow(window: isize) -> i32;
    fn keybd_event(virtual_key: u8, scan_code: u8, flags: u32, extra_info: usize);
}

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
extern "system" {
    fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> isize;
    fn QueryFullProcessImageNameW(
        process: isize,
        flags: u32,
        executable_name: *mut u16,
        size: *mut u32,
    ) -> i32;
    fn CloseHandle(object: isize) -> i32;
}

#[cfg(target_os = "linux")]
fn active_x11_window_id() -> Option<u64> {
    let window_output = std::process::Command::new("xdotool")
        .arg("getactivewindow")
        .output()
        .ok()?;
    if !window_output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&window_output.stdout)
        .trim()
        .parse::<u64>()
        .ok()
}

#[cfg(target_os = "linux")]
fn x11_application_for_window(window_id: u64) -> Option<PasteTarget> {
    use std::process::Command;
    let pid_output = Command::new("xdotool")
        .args(["getwindowpid", &window_id.to_string()])
        .output()
        .ok()?;
    let pid = String::from_utf8_lossy(&pid_output.stdout)
        .trim()
        .parse::<i32>()
        .ok()?;
    let name_output = Command::new("xdotool")
        .args(["getwindowname", &window_id.to_string()])
        .output()
        .ok()?;
    let name = String::from_utf8_lossy(&name_output.stdout)
        .trim()
        .to_string();
    Some(PasteTarget::available(
        pid,
        format!("x11:{window_id}"),
        window_id,
        if name.is_empty() {
            "Previous app".to_string()
        } else {
            name
        },
    ))
}

#[cfg(target_os = "linux")]
fn x11_application_name_for_window(window_id: u64) -> Option<String> {
    let output = std::process::Command::new("xdotool")
        .args(["getwindowclassname", &window_id.to_string()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!name.is_empty()).then_some(name)
}

#[cfg(target_os = "linux")]
fn paste_to_target(target: &PasteTarget, action: PasteAction) -> Result<(), String> {
    use std::process::Command;
    let window_id = target.native_handle.to_string();
    let status = Command::new("xdotool")
        .args([
            "windowactivate",
            "--sync",
            &window_id,
            "key",
            "--clearmodifiers",
            "ctrl+v",
        ])
        .status()
        .map_err(|_| action.target_failure(&target.name))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| action.target_failure(&target.name))
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn paste_to_target(target: &PasteTarget, action: PasteAction) -> Result<(), String> {
    Err(action.target_failure(&target.name))
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

    #[test]
    fn wayland_is_reported_as_unavailable_even_with_xwayland() {
        let reason = linux_platform_unavailable_reason(Some("wayland"), true, true, true).unwrap();
        assert!(reason.contains("Wayland"));
        assert!(reason.contains("not removed"));
    }

    #[test]
    fn x11_requires_display_and_xdotool() {
        assert!(linux_platform_unavailable_reason(Some("x11"), false, false, true).is_some());
        assert!(linux_platform_unavailable_reason(Some("x11"), false, true, false).is_some());
        assert!(linux_platform_unavailable_reason(Some("x11"), false, true, true).is_none());
    }
}
