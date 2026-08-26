use tauri::AppHandle;

#[tauri::command]
pub fn set_linux_native_menu_theme(app: AppHandle, dark: bool) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        app.run_on_main_thread(move || {
            if let Err(error) = crate::linux_native_theme::apply_menu_theme(dark) {
                eprintln!("Could not apply the native Linux menu theme: {error}");
            }
        })
        .map_err(|error| error.to_string())?;
    }

    #[cfg(not(target_os = "linux"))]
    let _ = (app, dark);

    Ok(())
}

#[tauri::command]
pub fn set_overlay_cursor(app: AppHandle, pointing: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        app.run_on_main_thread(move || unsafe {
            use objc::runtime::Object;
            use objc::{msg_send, sel, sel_impl};

            let cursor: *mut Object = if pointing {
                msg_send![objc::class!(NSCursor), pointingHandCursor]
            } else {
                msg_send![objc::class!(NSCursor), arrowCursor]
            };
            let _: () = msg_send![cursor, set];
        })
        .map_err(|error| error.to_string())?;
    }

    #[cfg(not(target_os = "macos"))]
    let _ = (app, pointing);

    Ok(())
}

#[tauri::command]
pub fn perform_titlebar_double_click(window: tauri::WebviewWindow) -> Result<(), String> {
    crate::titlebar::perform_titlebar_double_click(window)
}

#[tauri::command]
pub fn set_titlebar_direction(window: tauri::WebviewWindow, rtl: bool) -> Result<(), String> {
    crate::titlebar::set_titlebar_direction(window, rtl)
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub fn play_system_sound(sound_id: Option<u32>) {
    let id = sound_id.unwrap_or(1057);
    unsafe {
        #[link(name = "AudioToolbox", kind = "framework")]
        extern "C" {
            fn AudioServicesPlaySystemSound(sound_id: u32);
        }
        AudioServicesPlaySystemSound(id);
    }
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn play_system_sound(_sound_id: Option<u32>) {}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    crate::app_runtime::request_app_exit(&app);
}

#[tauri::command]
pub fn set_dock_visibility(show_dock: bool, app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use tauri::ActivationPolicy;
        if show_dock {
            let _ = app.set_activation_policy(ActivationPolicy::Regular);
        } else {
            let _ = app.set_activation_policy(ActivationPolicy::Accessory);
        }
    }
    let _ = show_dock;
    let _ = app;
    Ok(())
}

#[tauri::command]
pub fn open_emoji_picker() -> bool {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("osascript")
            .arg("-e")
            .arg("tell application \"System Events\" to keystroke \" \" using {control down, command down}")
            .spawn()
            .is_ok()
    }

    #[cfg(not(target_os = "macos"))]
    false
}
