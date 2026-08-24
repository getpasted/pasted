use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;
use tauri_plugin_window_state::{StateFlags, WindowExt};

static MAIN_PAGE_LOADED: AtomicBool = AtomicBool::new(false);
static STARTUP_SETUP_READY: AtomicBool = AtomicBool::new(false);
static MAIN_WINDOW_REVEALED: AtomicBool = AtomicBool::new(false);

pub(crate) fn main_window_state_flags() -> StateFlags {
    StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED | StateFlags::FULLSCREEN
}

fn reveal_main_window_when_ready(app: &tauri::AppHandle) {
    if !MAIN_PAGE_LOADED.load(Ordering::Acquire)
        || !STARTUP_SETUP_READY.load(Ordering::Acquire)
        || MAIN_WINDOW_REVEALED.swap(true, Ordering::AcqRel)
    {
        return;
    }

    let startup_args = std::env::args().collect::<Vec<_>>();
    if crate::live_app::request_from_args(&startup_args).is_some() {
        return;
    }
    let is_autostart = startup_args
        .iter()
        .any(|argument| argument == "--autostart");
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.eval(
            "document.getElementById('startup-splash')?.getAnimations({ subtree: true }).forEach((animation) => { animation.currentTime = 0; });",
        );
        if let Err(error) = window.show() {
            eprintln!("Could not show the main window during startup: {error}");
        } else if !is_autostart {
            if let Err(error) = window.set_focus() {
                eprintln!("Could not focus the main window during startup: {error}");
            }
        }
    }
}

pub(crate) fn mark_main_page_loaded(app: &tauri::AppHandle) {
    MAIN_PAGE_LOADED.store(true, Ordering::Release);
    reveal_main_window_when_ready(app);
}

pub(crate) fn mark_startup_setup_ready(app: &tauri::AppHandle) {
    STARTUP_SETUP_READY.store(true, Ordering::Release);
    reveal_main_window_when_ready(app);
}

pub(crate) fn configure_initial_windows(
    app: &mut tauri::App,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(main_window) = app.get_webview_window("main") {
        let _ = main_window.restore_state(main_window_state_flags());
        let _ = main_window.outer_position();
        let _ = main_window.outer_size();
        #[cfg(target_os = "macos")]
        {
            setup_window_vibrancy(&main_window);
            crate::titlebar::install_focus_observers(&main_window)?;
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub(crate) fn configure_overlay_windows(app: &tauri::AppHandle) {
    for label in ["hud", "capture-feedback"] {
        if let Some(window) = app.get_webview_window(label) {
            setup_overlay_window_transparency(&window);
        }
    }
}

pub(crate) fn handle_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    if let tauri::WindowEvent::Focused(true) = event {
        if let Some(webview) = window.app_handle().get_webview_window(window.label()) {
            let _ =
                webview.eval("document.documentElement.removeAttribute('data-window-inactive');");
        }
    }
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        let _ = window.hide();
        api.prevent_close();
        #[cfg(target_os = "macos")]
        trim_main_webview_memory(window.app_handle());
    }
}

#[cfg(target_os = "macos")]
fn setup_window_vibrancy(window: &tauri::WebviewWindow) {
    use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};
    let _ = apply_vibrancy(
        window,
        NSVisualEffectMaterial::UnderWindowBackground,
        Some(NSVisualEffectState::Active),
        Some(12.0),
    );
}

#[cfg(target_os = "macos")]
pub(crate) fn trim_main_webview_memory(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.eval("if (window.gc) { window.gc(); }");
    }
}

#[cfg(target_os = "macos")]
fn setup_overlay_window_transparency(window: &tauri::WebviewWindow) {
    use objc::runtime::Object;
    use objc::{msg_send, sel, sel_impl};
    if let Ok(ns_window_ptr) = window.ns_window() {
        unsafe {
            let ns_window = ns_window_ptr as *mut Object;
            let clear_color: *mut Object = msg_send![objc::class!(NSColor), clearColor];
            let _: () = msg_send![ns_window, setBackgroundColor: clear_color];
            let _: () = msg_send![ns_window, setOpaque: false];
            let _: () = msg_send![ns_window, setHasShadow: false];
        }
    }
    let _ = window.with_webview(|webview| unsafe {
        let wk_webview = webview.inner() as *mut Object;
        let no_num: *mut Object = msg_send![objc::class!(NSNumber), numberWithBool: false];
        let key_str: *mut Object = msg_send![
            objc::class!(NSString),
            stringWithUTF8String: c"drawsBackground".as_ptr()
        ];
        let _: () = msg_send![wk_webview, setValue: no_num forKey: key_str];
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_window_state_excludes_visibility_and_decorations() {
        let flags = main_window_state_flags();

        assert!(flags.contains(StateFlags::SIZE));
        assert!(flags.contains(StateFlags::POSITION));
        assert!(flags.contains(StateFlags::MAXIMIZED));
        assert!(flags.contains(StateFlags::FULLSCREEN));
        assert!(!flags.contains(StateFlags::VISIBLE));
        assert!(!flags.contains(StateFlags::DECORATIONS));
    }
}
