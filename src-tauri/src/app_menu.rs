use std::sync::Arc;

use tauri::{
    menu::{Menu, MenuBuilder, MenuEvent, MenuItem, SubmenuBuilder},
    AppHandle, Emitter, Manager, Runtime,
};

use crate::{
    commands,
    db::DbState,
    features::{self, Feature},
};

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
        "app.about" => MenuDispatch::Navigate("settings:about"),
        "file.new_bin" => MenuDispatch::FrontendAction("new-bin"),
        "file.toggle_history" => MenuDispatch::FrontendAction("toggle-history"),
        "file.toggle_queue" => MenuDispatch::FrontendAction("toggle-queue"),
        "edit.clip.copy" => MenuDispatch::FrontendAction("copy-selected-clip"),
        "edit.clip.note" => MenuDispatch::FrontendAction("add-note"),
        "edit.clip.name" => MenuDispatch::FrontendAction("name-clip"),
        "edit.clip.pin" => MenuDispatch::FrontendAction("toggle-pin"),
        "edit.clip.protect" => MenuDispatch::FrontendAction("toggle-protection"),
        "edit.clip.trash" => MenuDispatch::FrontendAction("trash-selected"),
        "view.all" => MenuDispatch::Navigate("all"),
        "view.search" => MenuDispatch::Navigate("search"),
        "view.queue" => MenuDispatch::Navigate("sequential"),
        "view.pinned" => MenuDispatch::Navigate("pinned"),
        "view.protected" => MenuDispatch::Navigate("protected"),
        "view.named" => MenuDispatch::Navigate("named"),
        "view.noted" => MenuDispatch::Navigate("notes"),
        "view.trashed" => MenuDispatch::Navigate("trash"),
        "view.analytics" => MenuDispatch::Navigate("analytics"),
        "view.transforms" => MenuDispatch::Navigate("transformations:transforms"),
        "view.advanced" => MenuDispatch::Navigate("transformations:advanced"),
        "view.playground" => MenuDispatch::Navigate("transformations:playground"),
        "view.activity" => MenuDispatch::Navigate("activity"),
        "view.toggle_sidebar" => MenuDispatch::FrontendAction("toggle-sidebar"),
        "view.zoom_out" => MenuDispatch::FrontendAction("zoom-out"),
        "view.actual_size" => MenuDispatch::FrontendAction("actual-size"),
        "view.zoom_in" => MenuDispatch::FrontendAction("zoom-in"),
        "view.reset_columns" => MenuDispatch::FrontendAction("reset-columns"),
        "view.refresh" => MenuDispatch::FrontendAction("refresh-data"),
        "window.show_main" => MenuDispatch::ShowMain,
        "window.quick_hud" => MenuDispatch::ToggleHud,
        "window.close" => MenuDispatch::CloseMain,
        "window.minimize" => MenuDispatch::MinimizeMain,
        "window.maximize" => MenuDispatch::ToggleMaximize,
        "window.fullscreen" => MenuDispatch::ToggleFullscreen,
        "file.quit" => MenuDispatch::Quit,
        "help.getting_started" => MenuDispatch::Navigate("help:getting-started"),
        "help.cli" => MenuDispatch::Navigate("help:cli"),
        "help.shortcuts" => MenuDispatch::Navigate("help:shortcuts-hud"),
        "help.privacy" => MenuDispatch::Navigate("help:privacy-capture"),
        "help.deletion" => MenuDispatch::Navigate("help:deletion-recovery"),
        "help.analysis" => MenuDispatch::Navigate("help:analysis"),
        "help.transformations" => MenuDispatch::Navigate("help:transformations"),
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

fn dispatch_allowed_while_locked(dispatch: &MenuDispatch) -> bool {
    matches!(
        dispatch,
        MenuDispatch::ShowMain
            | MenuDispatch::CloseMain
            | MenuDispatch::MinimizeMain
            | MenuDispatch::ToggleMaximize
            | MenuDispatch::ToggleFullscreen
            | MenuDispatch::Quit
    )
}

pub fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
    let Some(dispatch) = dispatch_for_id(event.id().as_ref()) else {
        return;
    };
    if app
        .try_state::<Arc<crate::app_lock::AppLockState>>()
        .is_some_and(|state| state.is_locked())
        && !dispatch_allowed_while_locked(&dispatch)
    {
        return;
    }

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
        MenuDispatch::Quit => {
            crate::request_app_exit(app);
        }
    }
}

fn safe_menu_label(value: &str) -> String {
    value.replace('&', "&&").replace(['\r', '\n'], " ")
}

pub fn install(app: &AppHandle, db: &Arc<DbState>) -> tauri::Result<()> {
    let feature_enabled = |feature| features::is_enabled(db, feature);
    let t = |key| crate::localization::text(db, key);
    let unlocked = !app
        .try_state::<Arc<crate::app_lock::AppLockState>>()
        .is_some_and(|state| state.is_locked());
    let bins = if feature_enabled(Feature::Bins) {
        db.get_bins().unwrap_or_default()
    } else {
        Vec::new()
    };

    let new_bin = MenuItem::with_id(
        app,
        "file.new_bin",
        t("native.file.newBin"),
        unlocked,
        Some("CmdOrCtrl+Shift+N"),
    )?;
    let settings = MenuItem::with_id(
        app,
        "app.settings",
        t("native.app.settings"),
        unlocked,
        Some("CmdOrCtrl+,"),
    )?;
    let about = MenuItem::with_id(
        app,
        "app.about",
        t("native.app.about"),
        unlocked,
        None::<&str>,
    )?;
    #[cfg(target_os = "macos")]
    let quit = MenuItem::with_id(
        app,
        "file.quit",
        t("native.file.quit"),
        true,
        Some("CmdOrCtrl+Q"),
    )?;
    let toggle_history = MenuItem::with_id(
        app,
        "file.toggle_history",
        t("native.file.toggleHistory"),
        unlocked,
        None::<&str>,
    )?;
    let toggle_queue = MenuItem::with_id(
        app,
        "file.toggle_queue",
        t("native.file.toggleQueue"),
        unlocked,
        None::<&str>,
    )?;
    let quick_hud = MenuItem::with_id(
        app,
        "window.quick_hud",
        t("native.window.hud"),
        unlocked,
        None::<&str>,
    )?;
    let shortcut_settings = MenuItem::with_id(
        app,
        "help.shortcut_settings",
        t("native.help.hotkeys"),
        unlocked,
        None::<&str>,
    )?;
    let search = MenuItem::with_id(
        app,
        "view.search",
        t("native.view.search"),
        true,
        Some("CmdOrCtrl+F"),
    )?;
    let toggle_sidebar = MenuItem::with_id(
        app,
        "view.toggle_sidebar",
        t("native.view.toggleSidebar"),
        true,
        Some("CmdOrCtrl+\\"),
    )?;
    let refresh = MenuItem::with_id(
        app,
        "view.refresh",
        t("native.view.refresh"),
        true,
        Some("CmdOrCtrl+R"),
    )?;
    let zoom_out = MenuItem::with_id(
        app,
        "view.zoom_out",
        t("native.view.zoomOut"),
        true,
        Some("CmdOrCtrl+-"),
    )?;
    let actual_size = MenuItem::with_id(
        app,
        "view.actual_size",
        t("native.view.actualSize"),
        true,
        Some("CmdOrCtrl+0"),
    )?;
    let zoom_in = MenuItem::with_id(
        app,
        "view.zoom_in",
        t("native.view.zoomIn"),
        true,
        Some("CmdOrCtrl+="),
    )?;

    #[cfg(target_os = "macos")]
    let app_menu = SubmenuBuilder::new(app, "Pasted")
        .item(&about)
        .separator()
        .item(&settings)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .item(&quit)
        .build()?;

    let mut file_builder = SubmenuBuilder::new(app, t("native.file.title"));
    if feature_enabled(Feature::Bins) {
        file_builder = file_builder.item(&new_bin).separator();
    }
    #[cfg(not(target_os = "macos"))]
    {
        file_builder = file_builder.item(&settings).separator();
    }
    file_builder = file_builder.item(&toggle_history);
    if feature_enabled(Feature::Queue) {
        file_builder = file_builder.item(&toggle_queue);
    }
    #[cfg(target_os = "macos")]
    let file_menu = file_builder.separator().close_window().build()?;
    #[cfg(not(target_os = "macos"))]
    let file_menu = file_builder
        .separator()
        .text("window.close", t("native.file.closeWindow"))
        .text("file.quit", t("native.file.quit"))
        .build()?;

    let mut clip_actions_builder = SubmenuBuilder::new(app, t("native.edit.selectedClip"))
        .text("edit.clip.copy", t("native.edit.copyClip"));
    if feature_enabled(Feature::Notes) {
        clip_actions_builder = clip_actions_builder.text("edit.clip.note", t("native.edit.note"));
    }
    if feature_enabled(Feature::Naming) {
        clip_actions_builder = clip_actions_builder.text("edit.clip.name", t("native.edit.name"));
    }
    if feature_enabled(Feature::Pinning) || feature_enabled(Feature::Protection) {
        clip_actions_builder = clip_actions_builder.separator();
        if feature_enabled(Feature::Pinning) {
            clip_actions_builder = clip_actions_builder.text("edit.clip.pin", t("native.edit.pin"));
        }
        if feature_enabled(Feature::Protection) {
            clip_actions_builder =
                clip_actions_builder.text("edit.clip.protect", t("native.edit.protect"));
        }
    }
    clip_actions_builder = clip_actions_builder.separator().text(
        "edit.clip.trash",
        if feature_enabled(Feature::Trash) {
            t("native.edit.trash")
        } else {
            t("native.edit.delete")
        },
    );
    let clip_actions = clip_actions_builder.build()?;
    #[cfg(target_os = "macos")]
    let edit_builder = SubmenuBuilder::new(app, t("native.edit.title"))
        .undo()
        .redo()
        .separator();
    #[cfg(not(target_os = "macos"))]
    let edit_builder = SubmenuBuilder::new(app, t("native.edit.title"));
    let edit_menu = edit_builder
        .enabled(unlocked)
        .cut()
        .copy()
        .paste()
        .select_all()
        .separator()
        .item(&clip_actions)
        .build()?;

    let bins_menu = if bins.is_empty() {
        SubmenuBuilder::with_id(app, "view.bins", t("native.clips.bins"))
            .enabled(false)
            .build()?
    } else {
        let mut builder = SubmenuBuilder::with_id(app, "view.bins", t("native.clips.bins"));
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

    let mut clips_builder = SubmenuBuilder::new(app, t("native.clips.title"))
        .text("view.all", t("native.clips.history"));
    if feature_enabled(Feature::Search) {
        clips_builder = clips_builder.item(&search);
    }
    clips_builder = clips_builder.separator();
    if feature_enabled(Feature::Queue) {
        clips_builder = clips_builder.text("view.queue", t("native.clips.queue"));
    }
    if feature_enabled(Feature::Pinning) {
        clips_builder = clips_builder.text("view.pinned", t("native.clips.pinned"));
    }
    if feature_enabled(Feature::Protection) {
        clips_builder = clips_builder.text("view.protected", t("native.clips.protected"));
    }
    if feature_enabled(Feature::Naming) {
        clips_builder = clips_builder.text("view.named", t("native.clips.named"));
    }
    if feature_enabled(Feature::Notes) {
        clips_builder = clips_builder.text("view.noted", t("native.clips.noted"));
    }
    if feature_enabled(Feature::Trash) {
        clips_builder = clips_builder.text("view.trashed", t("native.clips.trashed"));
    }
    if feature_enabled(Feature::Bins) {
        clips_builder = clips_builder.separator().item(&bins_menu);
    }
    let clips_menu = clips_builder.build()?;
    let transforms_menu = SubmenuBuilder::new(app, t("native.tools.transformations"))
        .text("view.transforms", t("native.tools.savedTransforms"))
        .text("view.advanced", t("native.tools.advancedOperations"))
        .text("view.playground", t("native.tools.playground"))
        .build()?;
    let mut tools_builder = SubmenuBuilder::new(app, t("native.tools.title"));
    if feature_enabled(Feature::Transformations) {
        tools_builder = tools_builder.item(&transforms_menu);
    }
    if feature_enabled(Feature::Insights) {
        tools_builder = tools_builder.text("view.analytics", t("native.tools.insights"));
    }
    if feature_enabled(Feature::ActivityLog) {
        tools_builder = tools_builder.text("view.activity", t("native.tools.activity"));
    }
    let tools_menu = tools_builder.build()?;
    let mut view_builder = SubmenuBuilder::new(app, t("native.view.title"))
        .enabled(unlocked)
        .item(&clips_menu);
    if feature_enabled(Feature::Insights)
        || feature_enabled(Feature::Transformations)
        || feature_enabled(Feature::ActivityLog)
    {
        view_builder = view_builder.item(&tools_menu);
    }
    let view_menu = view_builder
        .separator()
        .item(&zoom_out)
        .item(&actual_size)
        .item(&zoom_in)
        .separator()
        .item(&toggle_sidebar)
        .text("view.reset_columns", t("native.view.resetColumns"))
        .item(&refresh)
        .build()?;

    let mut window_builder = SubmenuBuilder::new(app, t("native.window.title"))
        .text("window.show_main", t("native.window.show"));
    if feature_enabled(Feature::Hud) {
        window_builder = window_builder.item(&quick_hud);
    }
    window_builder = window_builder.separator();
    #[cfg(target_os = "macos")]
    let window_menu = window_builder
        .minimize()
        .maximize_with_text(t("native.window.zoom"))
        .fullscreen()
        .separator()
        .bring_all_to_front()
        .build()?;
    #[cfg(not(target_os = "macos"))]
    let window_menu = window_builder
        .text("window.minimize", t("native.window.minimize"))
        .text("window.maximize", t("native.window.maximize"))
        .text("window.fullscreen", t("native.window.fullscreen"))
        .build()?;

    let documentation_menu = SubmenuBuilder::new(app, t("native.help.documentation"))
        .enabled(unlocked)
        .text("help.getting_started", t("native.help.gettingStarted"))
        .text("help.shortcuts", t("native.help.shortcuts"))
        .text("help.privacy", t("native.help.privacy"))
        .text("help.deletion", t("native.help.deletion"))
        .text("help.analysis", t("native.help.analysis"))
        .text("help.transformations", t("native.help.transformations"))
        .text("help.cli", t("native.help.cli"))
        .build()?;
    let mut help_builder = SubmenuBuilder::new(app, t("native.help.title"));
    if feature_enabled(Feature::Help) {
        help_builder = help_builder.item(&documentation_menu).separator();
    }
    help_builder = help_builder.item(&shortcut_settings);
    #[cfg(not(target_os = "macos"))]
    let help_builder = help_builder.separator().item(&about);
    let help_menu = help_builder.build()?;

    #[allow(unused_mut)]
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
            dispatch_for_id("view.zoom_in"),
            Some(MenuDispatch::FrontendAction("zoom-in"))
        );
        assert_eq!(
            dispatch_for_id("view.actual_size"),
            Some(MenuDispatch::FrontendAction("actual-size"))
        );
        assert_eq!(
            dispatch_for_id("view.bin.42"),
            Some(MenuDispatch::NavigateBin(42))
        );
        assert_eq!(dispatch_for_id("view.bin.-1"), None);
        assert_eq!(
            dispatch_for_id("app.about"),
            Some(MenuDispatch::Navigate("settings:about"))
        );
        assert_eq!(dispatch_for_id("unknown"), None);
    }

    #[test]
    fn dynamic_labels_cannot_create_mnemonics_or_extra_lines() {
        assert_eq!(safe_menu_label("R&D\nInbox"), "R&&D Inbox");
    }

    #[test]
    fn locked_menu_allows_only_window_management_and_quit() {
        assert!(dispatch_allowed_while_locked(&MenuDispatch::ShowMain));
        assert!(dispatch_allowed_while_locked(&MenuDispatch::Quit));
        assert!(!dispatch_allowed_while_locked(&MenuDispatch::Navigate(
            "settings"
        )));
        assert!(!dispatch_allowed_while_locked(
            &MenuDispatch::FrontendAction("refresh-data")
        ));
    }
}
