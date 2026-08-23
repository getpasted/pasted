use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

use crate::db::DbState;

pub(crate) const HUD_WIDTH: f64 = 360.0;
pub(crate) const HUD_HEIGHT: f64 = 448.0;

pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("hud") {
        let _ = window.hide();
    }
}

pub fn require_unlocked(app: &AppHandle) -> Result<(), String> {
    if app
        .try_state::<Arc<crate::app_lock::AppLockState>>()
        .is_some_and(|state| state.is_locked())
    {
        hide(app);
        return Err("Pasted is locked.".to_string());
    }
    Ok(())
}

pub fn reveal(app: &AppHandle) -> Result<(), String> {
    require_unlocked(app)?;
    let db = app.state::<Arc<DbState>>();
    crate::features::require(&db, crate::features::Feature::Hud)?;
    let window = app
        .get_webview_window("hud")
        .ok_or_else(|| "HUD window is unavailable".to_string())?;
    let lock_state = app.state::<Arc<crate::app_lock::AppLockState>>();
    let lock_status = crate::app_lock::status(&db, &lock_state);
    if lock_status.locked {
        hide(app);
        return Err("Pasted is locked.".to_string());
    }
    window
        .emit("app-lock-changed", &lock_status)
        .map_err(|error| format!("Could not synchronize HUD lock state: {error}"))?;
    let _ = window.show();
    let _ = window.set_focus();
    Ok(())
}

pub fn toggle(app: &AppHandle) -> Result<(), String> {
    require_unlocked(app)?;
    let db = app.state::<Arc<DbState>>();
    crate::features::require(&db, crate::features::Feature::Hud)?;
    let Some(window) = app.get_webview_window("hud") else {
        return Err("HUD window is unavailable".to_string());
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    let mut position_payload: Option<serde_json::Value> = None;

    #[cfg(target_os = "macos")]
    {
        #[repr(C)]
        #[derive(Copy, Clone)]
        struct Point {
            x: f64,
            y: f64,
        }
        #[repr(C)]
        #[derive(Copy, Clone)]
        struct Size {
            width: f64,
            height: f64,
        }
        #[repr(C)]
        #[derive(Copy, Clone)]
        struct Rect {
            origin: Point,
            size: Size,
        }

        use objc::runtime::{Class, Object};
        use objc::{msg_send, sel, sel_impl};

        unsafe {
            if let (Some(event_class), Some(screens_class)) =
                (Class::get("NSEvent"), Class::get("NSScreen"))
            {
                let cursor: Point = msg_send![event_class, mouseLocation];
                let screens: *mut Object = msg_send![screens_class, screens];
                let count: usize = msg_send![screens, count];
                let mut active_screen: Option<*mut Object> = None;
                let mut primary_height = 1080.0;

                if count > 0 {
                    let primary: *mut Object = msg_send![screens, objectAtIndex: 0usize];
                    let frame: Rect = msg_send![primary, frame];
                    primary_height = frame.size.height;
                }
                for index in 0..count {
                    let screen: *mut Object = msg_send![screens, objectAtIndex: index];
                    let frame: Rect = msg_send![screen, frame];
                    if cursor.x >= frame.origin.x
                        && cursor.x <= frame.origin.x + frame.size.width
                        && cursor.y >= frame.origin.y
                        && cursor.y <= frame.origin.y + frame.size.height
                    {
                        active_screen = Some(screen);
                        break;
                    }
                }

                let screen = active_screen.unwrap_or_else(|| msg_send![screens_class, mainScreen]);
                if !screen.is_null() {
                    let visible: Rect = msg_send![screen, visibleFrame];
                    let cursor_y = primary_height - cursor.y;
                    let visible_top = primary_height - (visible.origin.y + visible.size.height);
                    let visible_bottom = primary_height - visible.origin.y;
                    let visible_left = visible.origin.x;
                    let visible_right = visible.origin.x + visible.size.width;
                    let width = HUD_WIDTH;
                    let height = HUD_HEIGHT;
                    let mut x = cursor.x - (width / 2.0);
                    x = x.clamp(
                        visible_left + 8.0,
                        (visible_right - width - 8.0).max(visible_left + 8.0),
                    );
                    let mut y = cursor_y + 8.0;
                    if y + height > visible_bottom - 8.0 {
                        y = cursor_y - height - 8.0;
                    }
                    y = y.clamp(
                        visible_top + 8.0,
                        (visible_bottom - height - 8.0).max(visible_top + 8.0),
                    );
                    let payload = serde_json::json!({
                        "flipped": y < cursor_y,
                        "cursorX": cursor.x,
                        "cursorY": cursor_y,
                        "targetX": x,
                        "targetY": y,
                    });
                    let _ = window.emit("hud_position_updated", payload.clone());
                    position_payload = Some(payload);

                    if let Ok(ns_window) = window.ns_window() {
                        let ns_window = ns_window as *mut Object;
                        let _: () = msg_send![ns_window, setHasShadow: 0i8];
                        let _: () = msg_send![ns_window, setAlphaValue: 0.0f64];
                        let origin = Point {
                            x,
                            y: primary_height - y - height,
                        };
                        let _: () = msg_send![ns_window, setFrameOrigin: origin];
                    }
                    let _ = window
                        .set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
                }
            }
        }
    }

    reveal(app)?;
    #[cfg(target_os = "macos")]
    {
        if let Ok(ns_window) = window.ns_window() {
            use objc::runtime::Object;
            use objc::{msg_send, sel, sel_impl};
            unsafe {
                let ns_window = ns_window as *mut Object;
                let _: () = msg_send![ns_window, setAlphaValue: 1.0f64];
            }
        }
        if let Some(payload) = position_payload {
            let _ = window.emit("hud_position_updated", payload);
        }
    }
    Ok(())
}
