#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TitlebarDoubleClickAction {
    Zoom,
    Fill,
    Minimize,
    None,
}

fn parse_titlebar_double_click_action(value: Option<&str>) -> TitlebarDoubleClickAction {
    let normalized = value.unwrap_or_default().trim().to_ascii_lowercase();
    if normalized.contains("mini") {
        TitlebarDoubleClickAction::Minimize
    } else if normalized.contains("none") || normalized.contains("nothing") {
        TitlebarDoubleClickAction::None
    } else if normalized.contains("fill") {
        TitlebarDoubleClickAction::Fill
    } else {
        // Apple stores the visible Zoom choice as the legacy Maximize value.
        TitlebarDoubleClickAction::Zoom
    }
}

#[cfg(target_os = "macos")]
fn configured_titlebar_double_click_action() -> TitlebarDoubleClickAction {
    use objc::runtime::Object;
    use objc::{msg_send, sel, sel_impl};
    use std::ffi::CStr;
    use std::os::raw::c_char;

    unsafe {
        let defaults: *mut Object = msg_send![objc::class!(NSUserDefaults), standardUserDefaults];
        let key: *mut Object = msg_send![
            objc::class!(NSString),
            stringWithUTF8String: c"AppleActionOnDoubleClick".as_ptr()
        ];
        let value: *mut Object = msg_send![defaults, stringForKey: key];
        if value.is_null() {
            return TitlebarDoubleClickAction::Zoom;
        }

        let utf8: *const c_char = msg_send![value, UTF8String];
        if utf8.is_null() {
            return TitlebarDoubleClickAction::Zoom;
        }

        parse_titlebar_double_click_action(CStr::from_ptr(utf8).to_str().ok())
    }
}

#[cfg(target_os = "macos")]
fn perform_native_zoom(window: &tauri::WebviewWindow) -> Result<(), String> {
    use objc::runtime::Object;
    use objc::{msg_send, sel, sel_impl};
    use tauri::Manager;

    let ns_window = window.ns_window().map_err(|error| error.to_string())? as usize;
    window
        .app_handle()
        .run_on_main_thread(move || unsafe {
            let ns_window = ns_window as *mut Object;
            let sender = std::ptr::null_mut::<Object>();
            let _: () = msg_send![ns_window, performZoom: sender];
        })
        .map_err(|error| error.to_string())
}

pub fn perform_titlebar_double_click(window: tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        match configured_titlebar_double_click_action() {
            TitlebarDoubleClickAction::Zoom => perform_native_zoom(&window),
            TitlebarDoubleClickAction::Fill => {
                let is_maximized = window.is_maximized().map_err(|error| error.to_string())?;
                if is_maximized {
                    window.unmaximize().map_err(|error| error.to_string())
                } else {
                    window.maximize().map_err(|error| error.to_string())
                }
            }
            TitlebarDoubleClickAction::Minimize => {
                window.minimize().map_err(|error| error.to_string())
            }
            TitlebarDoubleClickAction::None => Ok(()),
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Framed platforms retain their native titlebar and never call this
        // macOS overlay helper.
        let _ = window;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_titlebar_double_click_action, TitlebarDoubleClickAction};

    #[test]
    fn recognizes_macos_titlebar_preferences() {
        assert_eq!(
            parse_titlebar_double_click_action(Some("Minimize")),
            TitlebarDoubleClickAction::Minimize
        );
        assert_eq!(
            parse_titlebar_double_click_action(Some("None")),
            TitlebarDoubleClickAction::None
        );
        assert_eq!(
            parse_titlebar_double_click_action(Some("Do Nothing")),
            TitlebarDoubleClickAction::None
        );
        assert_eq!(
            parse_titlebar_double_click_action(Some("Maximize")),
            TitlebarDoubleClickAction::Zoom
        );
        assert_eq!(
            parse_titlebar_double_click_action(Some("Fill")),
            TitlebarDoubleClickAction::Fill
        );
        assert_eq!(
            parse_titlebar_double_click_action(Some("Zoom")),
            TitlebarDoubleClickAction::Zoom
        );
    }

    #[test]
    fn defaults_unknown_or_missing_preferences_to_native_zoom() {
        assert_eq!(
            parse_titlebar_double_click_action(None),
            TitlebarDoubleClickAction::Zoom
        );
        assert_eq!(
            parse_titlebar_double_click_action(Some("Future macOS value")),
            TitlebarDoubleClickAction::Zoom
        );
    }
}
