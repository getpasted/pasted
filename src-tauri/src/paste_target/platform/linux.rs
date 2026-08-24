use super::super::{ActiveApplicationContext, PasteAction, PasteTarget};

pub(in crate::paste_target) fn unavailable_reason() -> Option<String> {
    super::linux_unavailable_reason(
        std::env::var("XDG_SESSION_TYPE").ok().as_deref(),
        std::env::var_os("WAYLAND_DISPLAY").is_some(),
        std::env::var_os("DISPLAY").is_some(),
        command_is_available("xdotool"),
    )
}

pub(in crate::paste_target) fn active_application_context(
    include_private_mode_signal: bool,
) -> Option<ActiveApplicationContext> {
    if is_native_wayland_session() {
        return None;
    }
    active_window_id().and_then(|window_id| {
        let target = application_for_window(window_id);
        let name = application_name_for_window(window_id)
            .or_else(|| target.as_ref().map(|target| target.name.clone()))?;
        Some(ActiveApplicationContext {
            name,
            window_title: include_private_mode_signal
                .then(|| target.map(|target| target.name))
                .flatten(),
            window_title_is_accessible: false,
        })
    })
}

pub(in crate::paste_target) fn active_window_id() -> Option<u64> {
    let output = std::process::Command::new("xdotool")
        .arg("getactivewindow")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

pub(in crate::paste_target) fn application_for_window(window_id: u64) -> Option<PasteTarget> {
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
            "Previous app".into()
        } else {
            name
        },
    ))
}

fn application_name_for_window(window_id: u64) -> Option<String> {
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

pub(in crate::paste_target) fn paste_to_target(
    target: &PasteTarget,
    action: PasteAction,
) -> Result<(), String> {
    let status = std::process::Command::new("xdotool")
        .args([
            "windowactivate",
            "--sync",
            &target.native_handle.to_string(),
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

fn is_native_wayland_session() -> bool {
    std::env::var("XDG_SESSION_TYPE").is_ok_and(|value| value.eq_ignore_ascii_case("wayland"))
        || (std::env::var_os("WAYLAND_DISPLAY").is_some()
            && !std::env::var("XDG_SESSION_TYPE")
                .is_ok_and(|value| value.eq_ignore_ascii_case("x11")))
}

fn command_is_available(command: &str) -> bool {
    std::process::Command::new(command)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}
