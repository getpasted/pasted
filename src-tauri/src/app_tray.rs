use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuBuilder, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

use crate::db::DbState;

const COPYCAT_TRAY_ICON_STYLE: &str = "copycat";

fn load_tray_icon(style: &str) -> Result<tauri::image::Image<'static>, image::ImageError> {
    let bytes = if style == COPYCAT_TRAY_ICON_STYLE {
        include_bytes!("../icons/tray-icon-copycat@2x.png").as_slice()
    } else {
        include_bytes!("../icons/tray-icon@2x.png").as_slice()
    };
    let image = image::load_from_memory(bytes)?.to_rgba8();
    let (width, height) = image.dimensions();
    Ok(tauri::image::Image::new_owned(
        image.into_raw(),
        width,
        height,
    ))
}

pub(crate) fn refresh_icon(app: &tauri::AppHandle, style: &str) {
    #[cfg(target_os = "macos")]
    if let Some(tray) = app.tray_by_id("main") {
        match load_tray_icon(style) {
            Ok(icon) => {
                if let Err(error) = tray.set_icon_with_as_template(Some(icon), true) {
                    eprintln!("Could not update the menu bar icon: {error}");
                }
            }
            Err(error) => eprintln!("Could not load the menu bar icon: {error}"),
        }
    }

    #[cfg(not(target_os = "macos"))]
    let _ = (app, style);
}

fn build_menu(app: &tauri::AppHandle, db: &Arc<DbState>) -> tauri::Result<Menu<tauri::Wry>> {
    let t = |key| crate::localization::text(db, key);
    let show = MenuItem::with_id(app, "show", t("native.tray.show"), true, None::<&str>)?;
    let hud = MenuItem::with_id(
        app,
        "hud_toggle",
        t("native.tray.toggleHud"),
        true,
        None::<&str>,
    )?;
    let queue = MenuItem::with_id(
        app,
        "seq_toggle",
        t("native.tray.startQueue"),
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", t("native.tray.quit"), true, None::<&str>)?;
    let mut builder = MenuBuilder::new(app).item(&show);
    if crate::features::is_enabled(db, crate::features::Feature::Hud) {
        builder = builder.item(&hud);
    }
    if crate::features::is_enabled(db, crate::features::Feature::Queue) {
        builder = builder.item(&queue);
    }
    builder.item(&quit).build()
}

pub(crate) fn refresh_menu(app: &tauri::AppHandle, db: &Arc<DbState>) {
    let Some(tray) = app.tray_by_id("main") else {
        return;
    };
    match build_menu(app, db) {
        Ok(menu) => {
            if let Err(error) = tray.set_menu(Some(menu)) {
                eprintln!("Could not refresh the tray menu: {error}");
            }
        }
        Err(error) => eprintln!("Could not rebuild the tray menu: {error}"),
    }
}

pub(crate) fn install(
    app: &mut tauri::App,
    db: &Arc<DbState>,
) -> Result<(), Box<dyn std::error::Error>> {
    let menu = build_menu(app.handle(), db)?;
    #[cfg(target_os = "macos")]
    let tray_icon_style = db
        .get_setting("menubarIconStyle")?
        .or_else(|| crate::settings_contract::default_value("menubarIconStyle"))
        .expect("menu bar icon style must have a contract default");
    #[cfg(not(target_os = "macos"))]
    let tray_icon_style = crate::settings_contract::default_value("menubarIconStyle")
        .expect("menu bar icon style must have a contract default");

    let tray_icon = match load_tray_icon(&tray_icon_style) {
        Ok(icon) => icon,
        Err(error) => app.default_window_icon().cloned().ok_or_else(|| {
            std::io::Error::other(format!(
                "Could not load the tray icon or default application icon: {error}"
            ))
        })?,
    };

    TrayIconBuilder::with_id("main")
        .icon(tray_icon)
        .icon_as_template(true)
        .menu(&menu)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(handle_icon_event)
        .build(app)?;
    Ok(())
}

fn handle_menu_event(app: &tauri::AppHandle, event: tauri::menu::MenuEvent) {
    match event.id.as_ref() {
        "show" => show_main_window(app),
        "hud_toggle" => {
            if app
                .try_state::<Arc<crate::app_lock::AppLockState>>()
                .is_some_and(|state| state.is_locked())
            {
                return;
            }
            let _ = crate::commands::toggle_hud_window(app.clone());
        }
        "seq_toggle" => toggle_queue(app),
        "quit" => crate::app_runtime::request_app_exit(app),
        _ => {}
    }
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn toggle_queue(app: &tauri::AppHandle) {
    if app
        .try_state::<Arc<crate::app_lock::AppLockState>>()
        .is_some_and(|state| state.is_locked())
    {
        return;
    }
    let db = app.state::<Arc<DbState>>();
    if !crate::features::is_enabled(&db, crate::features::Feature::Queue) {
        return;
    }
    let queue = app.state::<Arc<crate::sequential_paste::SequentialQueueState>>();
    let is_active = *queue.is_active.lock();
    if is_active {
        queue.stop_queue();
    } else {
        queue.start_queue();
    }
    let _ = app.emit("sequential-updated", queue.get_status());
}

fn handle_icon_event(tray: &tauri::tray::TrayIcon, event: TrayIconEvent) {
    if let TrayIconEvent::Click { .. } = event {
        let app = tray.app_handle();
        if let Some(window) = app.get_webview_window("main") {
            if window.is_visible().unwrap_or(false) {
                let _ = window.hide();
                #[cfg(target_os = "macos")]
                crate::app_windows::trim_main_webview_memory(app);
            } else {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    }
}
