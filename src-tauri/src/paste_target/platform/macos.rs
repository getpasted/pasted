use super::super::{ActiveApplicationContext, PasteAction, PasteTarget};

pub(in crate::paste_target) fn unavailable_reason() -> Option<String> {
    None
}

pub(in crate::paste_target) fn active_application_context(
    include_private_mode_signal: bool,
) -> Option<ActiveApplicationContext> {
    let target = frontmost_application()?;
    let window_title = include_private_mode_signal
        .then(|| focused_window_title(target.pid))
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

fn focused_window_title(pid: i32) -> Option<String> {
    use std::ffi::{c_void, CStr, CString};
    use std::ptr;

    type CfTypeRef = *const c_void;
    type CfStringRef = *const c_void;
    type AxUiElementRef = *const c_void;
    const UTF8: u32 = 0x0800_0100;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXUIElementCreateApplication(pid: i32) -> AxUiElementRef;
        fn AXUIElementCopyAttributeValue(
            element: AxUiElementRef,
            attribute: CfStringRef,
            value: *mut CfTypeRef,
        ) -> i32;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFStringCreateWithCString(
            allocator: *const c_void,
            text: *const i8,
            encoding: u32,
        ) -> CfStringRef;
        fn CFStringGetCString(
            string: CfStringRef,
            buffer: *mut i8,
            buffer_size: isize,
            encoding: u32,
        ) -> bool;
        fn CFRelease(value: CfTypeRef);
    }

    unsafe fn attribute(name: &str) -> CfStringRef {
        let name = CString::new(name).ok();
        name.map_or(ptr::null(), |name| {
            CFStringCreateWithCString(ptr::null(), name.as_ptr(), UTF8)
        })
    }

    unsafe {
        let application = AXUIElementCreateApplication(pid);
        if application.is_null() {
            return None;
        }
        let focused_key = attribute("AXFocusedWindow");
        if focused_key.is_null() {
            CFRelease(application);
            return None;
        }
        let mut window: CfTypeRef = ptr::null();
        let status = AXUIElementCopyAttributeValue(application, focused_key, &mut window);
        CFRelease(focused_key);
        CFRelease(application);
        if status != 0 || window.is_null() {
            return None;
        }

        let title_key = attribute("AXTitle");
        if title_key.is_null() {
            CFRelease(window);
            return None;
        }
        let mut title: CfTypeRef = ptr::null();
        let status = AXUIElementCopyAttributeValue(window, title_key, &mut title);
        CFRelease(title_key);
        CFRelease(window);
        if status != 0 || title.is_null() {
            return None;
        }

        let mut buffer = vec![0i8; 2049];
        let copied = CFStringGetCString(title, buffer.as_mut_ptr(), buffer.len() as isize, UTF8);
        CFRelease(title);
        copied.then(|| {
            CStr::from_ptr(buffer.as_ptr())
                .to_string_lossy()
                .into_owned()
        })
    }
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
