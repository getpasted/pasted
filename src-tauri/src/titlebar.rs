#[cfg(target_os = "macos")]
use once_cell::sync::Lazy;
#[cfg(target_os = "macos")]
use parking_lot::Mutex;

#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TitlebarDoubleClickAction {
    Zoom,
    Fill,
    Minimize,
    None,
}

#[cfg(any(target_os = "macos", test))]
const STANDARD_ZOOM_WIDTH: f64 = 1040.0;
#[cfg(any(target_os = "macos", test))]
const STANDARD_ZOOM_HEIGHT: f64 = 640.0;

#[cfg(any(target_os = "macos", test))]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
struct WindowPoint {
    x: f64,
    y: f64,
}

#[cfg(any(target_os = "macos", test))]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
struct WindowSize {
    width: f64,
    height: f64,
}

#[cfg(any(target_os = "macos", test))]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
struct WindowFrame {
    origin: WindowPoint,
    size: WindowSize,
}

#[cfg(target_os = "macos")]
static ZOOM_RESTORE_FRAME: Lazy<Mutex<Option<WindowFrame>>> = Lazy::new(|| Mutex::new(None));
#[cfg(target_os = "macos")]
static FILL_RESTORE_FRAME: Lazy<Mutex<Option<WindowFrame>>> = Lazy::new(|| Mutex::new(None));

#[cfg(any(target_os = "macos", test))]
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

#[cfg(any(target_os = "macos", test))]
fn clamp_frame_to_visible(frame: WindowFrame, visible: WindowFrame) -> WindowFrame {
    let width = frame.size.width.min(visible.size.width).max(1.0);
    let height = frame.size.height.min(visible.size.height).max(1.0);
    let max_x = visible.origin.x + visible.size.width - width;
    let max_y = visible.origin.y + visible.size.height - height;

    WindowFrame {
        origin: WindowPoint {
            x: frame.origin.x.clamp(visible.origin.x, max_x),
            y: frame.origin.y.clamp(visible.origin.y, max_y),
        },
        size: WindowSize { width, height },
    }
}

#[cfg(any(target_os = "macos", test))]
fn standard_zoom_frame(visible: WindowFrame) -> WindowFrame {
    let width = STANDARD_ZOOM_WIDTH.min(visible.size.width);
    let height = STANDARD_ZOOM_HEIGHT.min(visible.size.height);
    WindowFrame {
        origin: WindowPoint {
            x: visible.origin.x + (visible.size.width - width) / 2.0,
            y: visible.origin.y + (visible.size.height - height) / 2.0,
        },
        size: WindowSize { width, height },
    }
}

#[cfg(any(target_os = "macos", test))]
fn frames_are_equivalent(left: WindowFrame, right: WindowFrame) -> bool {
    const TOLERANCE: f64 = 2.0;
    (left.origin.x - right.origin.x).abs() <= TOLERANCE
        && (left.origin.y - right.origin.y).abs() <= TOLERANCE
        && (left.size.width - right.size.width).abs() <= TOLERANCE
        && (left.size.height - right.size.height).abs() <= TOLERANCE
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
fn perform_frame_toggle(
    window: &tauri::WebviewWindow,
    action: TitlebarDoubleClickAction,
) -> Result<(), String> {
    use objc::runtime::Object;
    use objc::{msg_send, sel, sel_impl};
    use tauri::Manager;

    let ns_window = window.ns_window().map_err(|error| error.to_string())? as usize;
    window
        .app_handle()
        .run_on_main_thread(move || unsafe {
            let ns_window = ns_window as *mut Object;
            let mut screen: *mut Object = msg_send![ns_window, screen];
            if screen.is_null() {
                screen = msg_send![objc::class!(NSScreen), mainScreen];
            }
            if screen.is_null() {
                return;
            }

            let current: WindowFrame = msg_send![ns_window, frame];
            let visible: WindowFrame = msg_send![screen, visibleFrame];
            let target = match action {
                TitlebarDoubleClickAction::Zoom => standard_zoom_frame(visible),
                TitlebarDoubleClickAction::Fill => visible,
                TitlebarDoubleClickAction::Minimize | TitlebarDoubleClickAction::None => return,
            };
            let restore_slot = match action {
                TitlebarDoubleClickAction::Zoom => &ZOOM_RESTORE_FRAME,
                TitlebarDoubleClickAction::Fill => &FILL_RESTORE_FRAME,
                TitlebarDoubleClickAction::Minimize | TitlebarDoubleClickAction::None => return,
            };

            let next = if frames_are_equivalent(current, target) {
                restore_slot
                    .lock()
                    .take()
                    .map(|frame| clamp_frame_to_visible(frame, visible))
                    .unwrap_or(current)
            } else {
                *restore_slot.lock() = Some(current);
                target
            };

            if !frames_are_equivalent(current, next) {
                // AppKit's animated zoom resizes the native frame before WebKit's
                // layout catches up. An atomic, non-animated frame update keeps
                // the three Pasted columns visually attached to the window.
                let _: () = msg_send![ns_window, setFrame: next display: 1i8 animate: 0i8];
            }
        })
        .map_err(|error| error.to_string())
}

pub fn perform_titlebar_double_click(window: tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        match configured_titlebar_double_click_action() {
            action @ (TitlebarDoubleClickAction::Zoom | TitlebarDoubleClickAction::Fill) => {
                perform_frame_toggle(&window, action)
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
    use super::{
        clamp_frame_to_visible, frames_are_equivalent, parse_titlebar_double_click_action,
        standard_zoom_frame, TitlebarDoubleClickAction, WindowFrame, WindowPoint, WindowSize,
    };

    fn frame(x: f64, y: f64, width: f64, height: f64) -> WindowFrame {
        WindowFrame {
            origin: WindowPoint { x, y },
            size: WindowSize { width, height },
        }
    }

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
    fn defaults_unknown_or_missing_preferences_to_standard_zoom() {
        assert_eq!(
            parse_titlebar_double_click_action(None),
            TitlebarDoubleClickAction::Zoom
        );
        assert_eq!(
            parse_titlebar_double_click_action(Some("Future macOS value")),
            TitlebarDoubleClickAction::Zoom
        );
    }

    #[test]
    fn standard_zoom_is_centered_and_bounded_by_the_visible_desktop() {
        assert_eq!(
            standard_zoom_frame(frame(100.0, 40.0, 1600.0, 1000.0)),
            frame(380.0, 220.0, 1040.0, 640.0)
        );
        assert_eq!(
            standard_zoom_frame(frame(-900.0, 20.0, 800.0, 600.0)),
            frame(-900.0, 20.0, 800.0, 600.0)
        );
    }

    #[test]
    fn restored_frames_are_clamped_to_the_current_display() {
        assert_eq!(
            clamp_frame_to_visible(
                frame(-1400.0, -200.0, 1400.0, 900.0),
                frame(0.0, 25.0, 1200.0, 775.0),
            ),
            frame(0.0, 25.0, 1200.0, 775.0)
        );
    }

    #[test]
    fn frame_comparison_tolerates_fractional_appkit_rounding_only() {
        assert!(frames_are_equivalent(
            frame(10.0, 20.0, 1040.0, 640.0),
            frame(11.5, 19.0, 1041.0, 639.0),
        ));
        assert!(!frames_are_equivalent(
            frame(10.0, 20.0, 1040.0, 640.0),
            frame(13.0, 20.0, 1040.0, 640.0),
        ));
    }
}
