// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
fn configure_appimage_wayland_compatibility() {
    use std::path::Path;

    // AppImages bundle WebKitGTK, but Wayland's client library belongs to the
    // host graphics-driver boundary and must agree with that machine's
    // Mesa/EGL stack. SteamOS otherwise aborts its WebKitWebProcess with
    // EGL_BAD_PARAMETER, leaving the native window completely white.
    let is_appimage = std::env::var_os("APPIMAGE").is_some();
    let is_wayland = std::env::var("XDG_SESSION_TYPE")
        .map(|value| value.eq_ignore_ascii_case("wayland"))
        .unwrap_or(false)
        || std::env::var_os("WAYLAND_DISPLAY").is_some();
    if !is_appimage || !is_wayland {
        return;
    }

    let Some(host_library) = [
        "/usr/lib/libwayland-client.so.0",
        "/usr/lib64/libwayland-client.so.0",
    ]
    .into_iter()
    .find(|candidate| Path::new(candidate).is_file()) else {
        return;
    };

    let existing = std::env::var("LD_PRELOAD").unwrap_or_default();
    if existing.split(':').any(|entry| entry == host_library) {
        return;
    }
    let preload = if existing.is_empty() {
        host_library.to_string()
    } else {
        format!("{host_library}:{existing}")
    };
    std::env::set_var("LD_PRELOAD", preload);
}

fn main() {
    let arguments = std::env::args().collect::<Vec<_>>();
    if let Some(code) = pasted_lib::content_extraction::run_bundled_extractor_helper(&arguments) {
        std::process::exit(code);
    }
    #[cfg(target_os = "linux")]
    configure_appimage_wayland_compatibility();
    pasted_lib::run()
}
