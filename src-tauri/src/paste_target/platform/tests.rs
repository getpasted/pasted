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
