use std::sync::Arc;

use tauri::{
    menu::{AboutMetadataBuilder, Menu, MenuBuilder, MenuEvent, MenuItem, SubmenuBuilder},
    AppHandle, Emitter, Manager, Runtime,
};

use crate::{commands, db::DbState};

#[derive(Debug, Clone, PartialEq, Eq)]
enum MenuDispatch {
    Navigate(&'static str),
    NavigateBin(i64),
    FrontendAction(&'static str),
    ShowMain,
    ToggleHud,
    CloseMain,
    MinimizeMain,
    ToggleMaximize,
    ToggleFullscreen,
    Quit,
}

fn dispatch_for_id(id: &str) -> Option<MenuDispatch> {
    let target = match id {
        "app.settings" => MenuDispatch::Navigate("settings"),
        "file.new_bin" => MenuDispatch::FrontendAction("new-bin"),
        "file.toggle_history" => MenuDispatch::FrontendAction("toggle-history"),
        "file.toggle_queue" => MenuDispatch::FrontendAction("toggle-queue"),
        "edit.clip.copy" => MenuDispatch::FrontendAction("copy-selected-clip"),
        "edit.clip.note" => MenuDispatch::FrontendAction("add-note"),
        "edit.clip.pin" => MenuDispatch::FrontendAction("toggle-pin"),
        "edit.clip.protect" => MenuDispatch::FrontendAction("toggle-protection"),
        "edit.clip.trash" => MenuDispatch::FrontendAction("trash-selected"),
        "view.all" => MenuDispatch::Navigate("all"),
        "view.search" => MenuDispatch::Navigate("search"),
        "view.queue" => MenuDispatch::Navigate("sequential"),
        "view.pinned" => MenuDispatch::Navigate("pinned"),
        "view.protected" => MenuDispatch::Navigate("protected"),
        "view.noted" => MenuDispatch::Navigate("notes"),
        "view.trashed" => MenuDispatch::Navigate("trash"),
        "view.analytics" => MenuDispatch::Navigate("analytics"),
        "view.transforms" => MenuDispatch::Navigate("transformations:transforms"),
        "view.advanced" => MenuDispatch::Navigate("transformations:advanced"),
        "view.playground" => MenuDispatch::Navigate("transformations:playground"),
        "view.activity" => MenuDispatch::Navigate("activity"),
        "view.toggle_sidebar" => MenuDispatch::FrontendAction("toggle-sidebar"),
        "view.reset_columns" => MenuDispatch::FrontendAction("reset-columns"),
        "view.refresh" => MenuDispatch::FrontendAction("refresh-data"),
        "window.show_main" => MenuDispatch::ShowMain,
        "window.quick_hud" => MenuDispatch::ToggleHud,
        "window.close" => MenuDispatch::CloseMain,
        "window.minimize" => MenuDispatch::MinimizeMain,
        "window.maximize" => MenuDispatch::ToggleMaximize,
        "window.fullscreen" => MenuDispatch::ToggleFullscreen,
        "file.quit" => MenuDispatch::Quit,
        "help.documentation" => MenuDispatch::Navigate("help:cli"),
        "help.hotkeys" => MenuDispatch::Navigate("help:hotkeys"),
        "help.privacy" => MenuDispatch::Navigate("help:autopause"),
        "help.trash" => MenuDispatch::Navigate("help:trash"),
        "help.transforms" => MenuDispatch::Navigate("help:pipelines"),
        "help.shortcut_settings" => MenuDispatch::Navigate("settings:hotkeys"),
        _ => {
            let bin_id = id.strip_prefix("view.bin.")?.parse::<i64>().ok()?;
            if bin_id <= 0 {
                return None;
            }
            MenuDispatch::NavigateBin(bin_id)
        }
    };
    Some(target)
}

fn reveal_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
    let Some(dispatch) = dispatch_for_id(event.id().as_ref()) else {
        return;
    };

    match dispatch {
        MenuDispatch::Navigate(route) => {
            reveal_main(app);
            let _ = app.emit("navigate-tab", route);
        }
        MenuDispatch::NavigateBin(bin_id) => {
            reveal_main(app);
            let _ = app.emit("navigate-bin", bin_id);
        }
        MenuDispatch::FrontendAction(action) => {
            reveal_main(app);
            let _ = app.emit("app-menu-action", action);
        }
        MenuDispatch::ShowMain => reveal_main(app),
        MenuDispatch::ToggleHud => {
            let _ = commands::toggle_hud_window(app.clone());
        }
        MenuDispatch::CloseMain => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.close();
            }
        }
        MenuDispatch::MinimizeMain => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.minimize();
            }
        }
        MenuDispatch::ToggleMaximize => {
            if let Some(window) = app.get_webview_window("main") {
                let is_maximized = window.is_maximized().unwrap_or(false);
                let _ = if is_maximized {
                    window.unmaximize()
                } else {
                    window.maximize()
                };
            }
        }
        MenuDispatch::ToggleFullscreen => {
            if let Some(window) = app.get_webview_window("main") {
                let is_fullscreen = window.is_fullscreen().unwrap_or(false);
                let _ = window.set_fullscreen(!is_fullscreen);
            }
        }
        MenuDispatch::Quit => app.exit(0),
    }
}

fn safe_menu_label(value: &str) -> String {
    value.replace('&', "&&").replace(['\r', '\n'], " ")
}

pub fn install(app: &AppHandle, db: &Arc<DbState>) -> tauri::Result<()> {
    let bins = db.get_bins().unwrap_or_default();

    let new_bin = MenuItem::with_id(
        app,
        "file.new_bin",
        "New Bin…",
        true,
        Some("CmdOrCtrl+Shift+N"),
    )?;
    let settings = MenuItem::with_id(app, "app.settings", "Settings…", true, Some("CmdOrCtrl+,"))?;
    let search = MenuItem::with_id(
        app,
        "view.search",
        "Search All Clips",
        true,
        Some("CmdOrCtrl+F"),
    )?;
    let toggle_sidebar = MenuItem::with_id(
        app,
        "view.toggle_sidebar",
        "Toggle Sidebar",
        true,
        Some("CmdOrCtrl+\\"),
    )?;
    let refresh = MenuItem::with_id(app, "view.refresh", "Refresh", true, Some("CmdOrCtrl+R"))?;

    #[cfg(target_os = "macos")]
    let app_menu = SubmenuBuilder::new(app, "Pasted")
        .about(Some(
            AboutMetadataBuilder::new()
                .name(Some("Pasted"))
                .version(Some(env!("CARGO_PKG_VERSION")))
                .icon(app.default_window_icon().cloned())
                .build(),
        ))
        .separator()
        .item(&settings)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    #[cfg(target_os = "macos")]
    let file_builder = SubmenuBuilder::new(app, "File").item(&new_bin).separator();
    #[cfg(not(target_os = "macos"))]
    let file_builder = SubmenuBuilder::new(app, "File")
        .item(&new_bin)
        .separator()
        .item(&settings)
        .separator();
    #[cfg(target_os = "macos")]
    let file_menu = file_builder
        .text("file.toggle_history", "Pause or Resume History")
        .text("file.toggle_queue", "Start or Stop Copy Queue")
        .separator()
        .close_window()
        .build()?;
    #[cfg(not(target_os = "macos"))]
    let file_menu = file_builder
        .text("file.toggle_history", "Pause or Resume History")
        .text("file.toggle_queue", "Start or Stop Copy Queue")
        .separator()
        .text("window.close", "Close Window")
        .text("file.quit", "Quit Pasted")
        .build()?;

    let clip_actions = SubmenuBuilder::new(app, "Selected Clip")
        .text("edit.clip.copy", "Copy Clip")
        .text("edit.clip.note", "Add or Edit Note…")
        .separator()
        .text("edit.clip.pin", "Pin or Unpin")
        .text("edit.clip.protect", "Protect or Unprotect")
        .separator()
        .text("edit.clip.trash", "Move to Trash")
        .build()?;
    #[cfg(target_os = "macos")]
    let edit_builder = SubmenuBuilder::new(app, "Edit").undo().redo().separator();
    #[cfg(not(target_os = "macos"))]
    let edit_builder = SubmenuBuilder::new(app, "Edit");
    let edit_menu = edit_builder
        .cut()
        .copy()
        .paste()
        .select_all()
        .separator()
        .item(&clip_actions)
        .build()?;

    let bins_menu = if bins.is_empty() {
        SubmenuBuilder::with_id(app, "view.bins", "Bins")
            .enabled(false)
            .build()?
    } else {
        let mut builder = SubmenuBuilder::with_id(app, "view.bins", "Bins");
        for bin in bins {
            let name = safe_menu_label(&bin.name);
            let icon = safe_menu_label(&bin.icon);
            let label = if icon.trim().is_empty() {
                name
            } else {
                format!("{} {}", icon.trim(), name)
            };
            builder = builder.text(format!("view.bin.{}", bin.id), label);
        }
        builder.build()?
    };

    let clips_menu = SubmenuBuilder::new(app, "Clips")
        .text("view.all", "All Clips")
        .item(&search)
        .separator()
        .text("view.queue", "Queue")
        .text("view.pinned", "Pinned")
        .text("view.protected", "Protected")
        .text("view.noted", "Noted")
        .text("view.trashed", "Trashed")
        .separator()
        .item(&bins_menu)
        .build()?;
    let transforms_menu = SubmenuBuilder::new(app, "Transformations")
        .text("view.transforms", "Saved Transforms")
        .text("view.advanced", "Advanced Operations")
        .text("view.playground", "Playground")
        .build()?;
    let tools_menu = SubmenuBuilder::new(app, "Tools")
        .text("view.analytics", "Analytics & Insights")
        .item(&transforms_menu)
        .text("view.activity", "Activity Log")
        .build()?;
    let view_menu = SubmenuBuilder::new(app, "View")
        .item(&clips_menu)
        .item(&tools_menu)
        .separator()
        .item(&toggle_sidebar)
        .text("view.reset_columns", "Reset Column Widths")
        .item(&refresh)
        .build()?;

    #[cfg(target_os = "macos")]
    let window_menu = SubmenuBuilder::new(app, "Window")
        .text("window.show_main", "Show Pasted")
        .text("window.quick_hud", "Quick HUD")
        .separator()
        .minimize()
        .maximize_with_text("Zoom")
        .fullscreen()
        .separator()
        .bring_all_to_front()
        .build()?;
    #[cfg(not(target_os = "macos"))]
    let window_menu = SubmenuBuilder::new(app, "Window")
        .text("window.show_main", "Show Pasted")
        .text("window.quick_hud", "Quick HUD")
        .separator()
        .text("window.minimize", "Minimize")
        .text("window.maximize", "Maximize")
        .text("window.fullscreen", "Toggle Full Screen")
        .build()?;

    let documentation_menu = SubmenuBuilder::new(app, "Documentation")
        .text("help.documentation", "CLI Commands")
        .text("help.hotkeys", "Hotkeys & Modifiers")
        .text("help.privacy", "Auto-Pause & Privacy")
        .text("help.trash", "Soft Trash Protection")
        .text("help.transforms", "Transformations")
        .build()?;
    let help_builder = SubmenuBuilder::new(app, "Help")
        .item(&documentation_menu)
        .separator()
        .text("help.shortcut_settings", "Keyboard Shortcut Settings…");
    #[cfg(not(target_os = "macos"))]
    let help_builder = help_builder.separator().about(Some(
        AboutMetadataBuilder::new()
            .name(Some("Pasted"))
            .version(Some(env!("CARGO_PKG_VERSION")))
            .icon(app.default_window_icon().cloned())
            .build(),
    ));
    let help_menu = help_builder.build()?;

    let mut builder = MenuBuilder::new(app);
    #[cfg(target_os = "macos")]
    {
        builder = builder.item(&app_menu);
    }
    let menu: Menu<_> = builder
        .item(&file_menu)
        .item(&edit_menu)
        .item(&view_menu)
        .item(&window_menu)
        .item(&help_menu)
        .build()?;
    app.set_menu(menu)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_ids_route_to_shared_frontend_actions() {
        assert_eq!(
            dispatch_for_id("view.transforms"),
            Some(MenuDispatch::Navigate("transformations:transforms"))
        );
        assert_eq!(
            dispatch_for_id("edit.clip.pin"),
            Some(MenuDispatch::FrontendAction("toggle-pin"))
        );
        assert_eq!(
            dispatch_for_id("view.bin.42"),
            Some(MenuDispatch::NavigateBin(42))
        );
        assert_eq!(dispatch_for_id("view.bin.-1"), None);
        assert_eq!(dispatch_for_id("unknown"), None);
    }

    #[test]
    fn dynamic_labels_cannot_create_mnemonics_or_extra_lines() {
        assert_eq!(safe_menu_label("R&D\nInbox"), "R&&D Inbox");
    }
}
