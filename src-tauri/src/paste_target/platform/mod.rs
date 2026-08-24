#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub(super) use linux::{
    active_application_context, active_window_id, application_for_window, paste_to_target,
    unavailable_reason,
};
#[cfg(target_os = "macos")]
pub(super) use macos::{
    active_application_context, frontmost_application, paste_to_target, unavailable_reason,
};
#[cfg(target_os = "windows")]
pub(super) use windows::{
    active_application_context, frontmost_application, paste_to_target, unavailable_reason,
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
mod tests {
    use super::*;

    #[test]
    fn wayland_is_reported_as_unavailable_even_with_xwayland() {
        let reason = linux_unavailable_reason(Some("wayland"), true, true, true).unwrap();
        assert!(reason.contains("Wayland"));
        assert!(reason.contains("not removed"));
    }

    #[test]
    fn x11_requires_display_and_xdotool() {
        assert!(linux_unavailable_reason(Some("x11"), false, false, true).is_some());
        assert!(linux_unavailable_reason(Some("x11"), false, true, false).is_some());
        assert!(linux_unavailable_reason(Some("x11"), false, true, true).is_none());
    }
}
