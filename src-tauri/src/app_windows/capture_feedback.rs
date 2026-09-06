use objc2::runtime::AnyObject;
use objc2::{msg_send, sel};
use tauri::Manager;

pub(super) fn prevent_app_activation(
    app: &tauri::AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(window) = app.get_webview_window("capture-feedback") else {
        return Ok(());
    };
    let pointer = window.ns_window()?;
    // Setup runs on the AppKit main thread, before this hidden window is shown.
    unsafe {
        let window = pointer.cast::<AnyObject>();
        let supported: bool = msg_send![window, respondsToSelector: sel!(_setPreventsActivation:)];
        if !supported {
            return Err("AppKit does not provide the window activation setting".into());
        }
        // Non-focusable NSWindows still activate their application on click.
        // This private AppKit setter updates the WindowServer activation flag;
        // overriding _isNonactivatingPanel alone does not update that flag.
        // Wine uses the same setter to synchronize native activation behavior:
        // https://github.com/wine-mirror/wine/blob/master/dlls/winemac.drv/cocoa_window.m
        // Preserve Tauri's existing class and KVO property observers entirely.
        let _: () = msg_send![window, _setPreventsActivation: true];
        let _: () = msg_send![window, setCanHide: false];
    }
    Ok(())
}
