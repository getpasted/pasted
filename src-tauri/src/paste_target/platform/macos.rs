use super::super::{ActiveApplicationContext, PasteAction, PasteTarget};

pub(in crate::paste_target) fn unavailable_reason() -> Option<String> {
    None
}

pub(in crate::paste_target) fn active_application_context(
    include_private_mode_signal: bool,
) -> Option<ActiveApplicationContext> {
    let target = frontmost_application()?;
    let window_title = include_private_mode_signal
        .then(|| super::macos_accessibility::focused_window_title(target.pid))
        .flatten();
    Some(ActiveApplicationContext {
        name: target.name,
        window_title_is_accessible: window_title.is_some(),
        window_title,
    })
}

pub(in crate::paste_target) fn frontmost_application() -> Option<PasteTarget> {
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

pub(in crate::paste_target) fn focused_smart_paste_context(
) -> Result<crate::smart_paste::SmartPasteContext, String> {
    let target = frontmost_application()
        .ok_or_else(|| "Smart Paste could not identify the focused application.".to_string())?;
    let attributes = super::macos_accessibility::focused_element_attributes(target.pid)
        .ok_or_else(|| {
            "The focused field does not expose context to macOS Accessibility.".to_string()
        })?;
    Ok(crate::smart_paste::SmartPasteContext {
        application: target.name,
        role: attributes[0].clone(),
        label: attributes[1].clone(),
        description: attributes[2].clone(),
        help: attributes[3].clone(),
        placeholder: attributes[4].clone(),
    })
}

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

pub(in crate::paste_target) fn paste_to_target(
    target: &PasteTarget,
    action: PasteAction,
) -> Result<(), String> {
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
