#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
mod macos_accessibility;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
mod windows_accessibility;

#[cfg(target_os = "linux")]
pub(super) use linux::{
    active_application_context, active_window_id, application_for_window, paste_to_target,
    unavailable_reason,
};

#[cfg(target_os = "linux")]
pub(super) fn focused_smart_paste_context() -> Result<crate::smart_paste::SmartPasteContext, String>
{
    Err("Smart Paste field detection is not yet available on Linux.".to_string())
}
#[cfg(target_os = "macos")]
pub(super) use macos::{
    active_application_context, focused_smart_paste_context, frontmost_application,
    paste_to_target, unavailable_reason,
};
#[cfg(target_os = "windows")]
pub(super) use windows::{
    active_application_context, focused_smart_paste_context, frontmost_application,
    paste_to_target, unavailable_reason,
};

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub(super) fn active_application_context(
    _include_private_mode_signal: bool,
) -> Option<super::ActiveApplicationContext> {
    None
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub(super) fn paste_to_target(
    target: &super::PasteTarget,
    action: super::PasteAction,
) -> Result<(), String> {
    Err(action.target_failure(&target.name))
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub(super) fn unavailable_reason() -> Option<String> {
    Some(
        "Automatic Queue paste is unavailable on this platform. Clip not removed from Queue."
            .to_string(),
    )
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub(super) fn focused_smart_paste_context() -> Result<crate::smart_paste::SmartPasteContext, String>
{
    Err("Smart Paste field detection is unavailable on this platform.".to_string())
}

#[cfg(any(target_os = "linux", test))]
pub(super) fn linux_unavailable_reason(
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

#[cfg(test)]
mod tests;
