pub mod analysis_contract;
pub mod analysis_execution;
mod app_menu;
pub mod bin_assignment;
mod clipboard_fingerprint;
mod clipboard_monitor;
mod commands;
pub mod content_analysis;
pub mod content_detection;
pub mod content_extraction;
pub mod content_types;
pub mod db;
pub mod external_import;
pub mod features;
mod filter_engine;
mod hotkey_manager;
pub mod installation_diagnostics;
pub mod intelligence_connections;
pub mod intelligence_executor;
mod intelligence_provider;
mod intelligence_scheduler;
pub mod library_items;
pub mod library_storage;
#[cfg(target_os = "linux")]
mod linux_native_theme;
pub mod live_app;
pub mod ocr;
#[cfg(test)]
mod operation_plugins;
mod operation_registry;
mod paste_target;
pub mod resource_limits;
mod sequential_paste;
pub mod settings_activity;
pub mod third_party_licenses;
mod titlebar;
pub mod transformation_intent;
pub mod transformation_service;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::{
    menu::{Menu, MenuBuilder, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tauri_plugin_window_state::{StateFlags, WindowExt};

static EXIT_REQUESTED: AtomicBool = AtomicBool::new(false);

const DEFAULT_TRAY_ICON_STYLE: &str = "clipboard";
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

pub(crate) fn refresh_tray_icon(app: &tauri::AppHandle, style: &str) {
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

pub(crate) fn request_app_exit(app: &tauri::AppHandle) {
    if EXIT_REQUESTED.swap(true, Ordering::SeqCst) {
        return;
    }

    for window in app.webview_windows().values() {
        let _ = window.hide();
    }
    let db = app.state::<Arc<db::DbState>>();
    let _ = db.log_activity("app_exit_requested", "Quit Pasted");
    app.exit(0);
}

fn build_tray_menu(
    app: &tauri::AppHandle,
    db: &Arc<db::DbState>,
) -> tauri::Result<Menu<tauri::Wry>> {
    let show = MenuItem::with_id(app, "show", "Show Pasted", true, None::<&str>)?;
    let hud = MenuItem::with_id(app, "hud_toggle", "Toggle HUD", true, None::<&str>)?;
    let queue = MenuItem::with_id(
        app,
        "seq_toggle",
        "Start Sequential Paste",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit Pasted", true, None::<&str>)?;
    let mut builder = MenuBuilder::new(app).item(&show);
    if features::is_enabled(db, features::Feature::Hud) {
        builder = builder.item(&hud);
    }
    if features::is_enabled(db, features::Feature::Queue) {
        builder = builder.item(&queue);
    }
    builder.item(&quit).build()
}

pub(crate) fn refresh_tray_menu(app: &tauri::AppHandle, db: &Arc<db::DbState>) {
    let Some(tray) = app.tray_by_id("main") else {
        return;
    };
    match build_tray_menu(app, db) {
        Ok(menu) => {
            if let Err(error) = tray.set_menu(Some(menu)) {
                eprintln!("Could not refresh the tray menu: {error}");
            }
        }
        Err(error) => eprintln!("Could not rebuild the tray menu: {error}"),
    }
}

fn main_window_state_flags() -> StateFlags {
    StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED | StateFlags::FULLSCREEN
}

#[cfg(target_os = "macos")]
fn setup_window_vibrancy(window: &tauri::WebviewWindow) {
    use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};
    let _ = apply_vibrancy(
        window,
        NSVisualEffectMaterial::UnderWindowBackground,
        Some(NSVisualEffectState::FollowsWindowActiveState),
        Some(12.0),
    );
}

#[cfg(target_os = "macos")]
fn trim_webview_memory(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.eval("if (window.gc) { window.gc(); }");
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

pub fn run() {
    let hotkey_manager = Arc::new(hotkey_manager::HotkeyManager::new());

    tauri::Builder::default()
        .manage(hotkey_manager.clone())
        .on_menu_event(app_menu::handle_menu_event)
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_filter(|label| label == "main")
                .skip_initial_state("main")
                .with_state_flags(main_window_state_flags())
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if EXIT_REQUESTED.load(Ordering::SeqCst) {
                return;
            }
            if args.iter().any(|argument| argument == "--skip-welcome") {
                if let Some(db) = app.try_state::<Arc<db::DbState>>() {
                    let _ = db.save_setting(
                        external_import::ONBOARDING_SETTING_KEY,
                        &external_import::ONBOARDING_VERSION.to_string(),
                    );
                    let _ = app.emit(
                        "app-setting-changed",
                        serde_json::json!({
                            "key": external_import::ONBOARDING_SETTING_KEY,
                            "value": external_import::ONBOARDING_VERSION.to_string(),
                        }),
                    );
                }
            }
            if let Some(path) = live_app::request_from_args(&args) {
                live_app::handle_request_file(app, &path);
                return;
            }
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        if let Some(mgr) = app.try_state::<Arc<hotkey_manager::HotkeyManager>>() {
                            mgr.dispatch(app, shortcut);
                        } else {
                            eprintln!("HotkeyManager state not found while dispatching a shortcut");
                        }
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::AppleScript,
            Some(vec!["--autostart"]),
        ))
        .setup(|app| {
            let startup_args = std::env::args().collect::<Vec<_>>();
            let is_autostart = startup_args
                .iter()
                .any(|argument| argument == "--autostart");
            let live_request = live_app::request_from_args(&startup_args);

            #[cfg(target_os = "linux")]
            if let Err(error) = linux_native_theme::apply_menu_theme(true) {
                eprintln!("Could not apply the initial native Linux menu theme: {error}");
            }

            // Restore while hidden, then reveal the main window. Automatic restore
            // is skipped for this window so a later webview-ready event cannot move
            // an already-visible window. Visibility itself is intentionally not
            // persisted because Pasted is commonly hidden from its tray lifecycle.
            if let Some(main_win) = app.get_webview_window("main") {
                let _ = main_win.restore_state(main_window_state_flags());
                // Window-state restoration dispatches native geometry updates to
                // the event loop. Read the resulting frame back before revealing
                // the window so macOS cannot paint the configured default frame
                // for a moment and then visibly snap to the restored one.
                let _ = main_win.outer_position();
                let _ = main_win.outer_size();
                #[cfg(target_os = "macos")]
                {
                    setup_window_vibrancy(&main_win);
                }
                if live_request.is_some() {
                    let _ = main_win.hide();
                } else if let Err(error) = main_win.show() {
                    eprintln!("Could not show the main window during startup: {error}");
                } else if !is_autostart {
                    if let Err(error) = main_win.set_focus() {
                        eprintln!("Could not focus the main window during startup: {error}");
                    }
                }
            }

            // Determine app data path for SQLite DB
            let app_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("./pasted_data"));
            let db_path = library_storage::resolve_database_path(&app_dir);

            let db_state =
                Arc::new(db::DbState::new(db_path).expect("Failed to initialize SQLite database"));
            if std::env::args().any(|argument| argument == "--skip-welcome") {
                let _ = db_state.save_setting(
                    external_import::ONBOARDING_SETTING_KEY,
                    &external_import::ONBOARDING_VERSION.to_string(),
                );
            }
            let seq_state = Arc::new(sequential_paste::SequentialQueueState::persistent(
                db_state.clone(),
            ));
            let paste_target_state = Arc::new(paste_target::PasteTargetState::new());
            paste_target_state.start_tracking();

            app.manage(db_state.clone());
            app.manage(seq_state.clone());
            app.manage(paste_target_state);

            let launch_description = if is_autostart {
                "Opened Pasted at login"
            } else {
                "Opened Pasted"
            };
            let _ = db_state.log_activity("app_started", launch_description);

            let ocr_service = Arc::new(ocr::spawn_ocr_worker(
                app.handle().clone(),
                db_state.clone(),
            ));
            app.manage(ocr_service.clone());

            app_menu::install(app.handle(), &db_state)?;

            // Start background clipboard monitor
            let handle = app.handle().clone();
            let monitor_handle = clipboard_monitor::start_clipboard_monitor(
                handle,
                db_state.clone(),
                seq_state,
                ocr_service,
            );
            let monitor_state = Arc::new(clipboard_monitor::ClipboardMonitorState {
                is_manually_paused: monitor_handle.is_manually_paused.clone(),
                is_auto_paused: monitor_handle.is_auto_paused.clone(),
            });
            app.manage(monitor_state);

            if let Some(path) = live_request.as_deref() {
                live_app::handle_request_file(app.handle(), path);
            }

            #[cfg(target_os = "macos")]
            {
                if let Some(hud_win) = app.get_webview_window("hud") {
                    setup_overlay_window_transparency(&hud_win);
                }
                if let Some(feedback_win) = app.get_webview_window("capture-feedback") {
                    setup_overlay_window_transparency(&feedback_win);
                }
            }

            // Register all saved HUD, Pipeline, and Bin shortcuts
            let _ = commands::register_all_app_shortcuts(app.handle());

            // Create Menu Bar / System Tray Icon
            let menu = build_tray_menu(app.handle(), &db_state)?;

            #[cfg(target_os = "macos")]
            let tray_icon_style = db_state
                .get_setting("menubarIconStyle")?
                .unwrap_or_else(|| DEFAULT_TRAY_ICON_STYLE.to_string());
            #[cfg(not(target_os = "macos"))]
            let tray_icon_style = DEFAULT_TRAY_ICON_STYLE.to_string();

            let tray_icon = match load_tray_icon(&tray_icon_style) {
                Ok(icon) => icon,
                Err(error) => app.default_window_icon().cloned().ok_or_else(|| {
                    std::io::Error::other(format!(
                        "Could not load the tray icon or default application icon: {error}"
                    ))
                })?,
            };

            let _tray = TrayIconBuilder::with_id("main")
                .icon(tray_icon)
                .icon_as_template(true)
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "hud_toggle" => {
                        let _ = commands::toggle_hud_window(app.clone());
                    }
                    "seq_toggle" => {
                        let db = app.state::<Arc<db::DbState>>();
                        if !features::is_enabled(&db, features::Feature::Queue) {
                            return;
                        }
                        let seq = app.state::<Arc<sequential_paste::SequentialQueueState>>();
                        let is_active = *seq.is_active.lock();
                        if is_active {
                            seq.stop_queue();
                        } else {
                            seq.start_queue();
                        }
                        let status = seq.get_status();
                        let _ = app.emit("sequential-updated", status);
                    }
                    "quit" => {
                        request_app_exit(app);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { .. } = event {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                                #[cfg(target_os = "macos")]
                                trim_webview_memory(app);
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
                #[cfg(target_os = "macos")]
                trim_webview_memory(window.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_clips,
            commands::get_capture_feedback_clip,
            commands::get_clip_image,
            commands::get_file_clip_metadata,
            commands::get_file_clip_previews,
            commands::get_trashed_clips,
            commands::restore_clip,
            commands::restore_all_trashed_clips,
            commands::purge_clip_permanently,
            commands::empty_trash,
            commands::get_activity_logs,
            commands::clear_activity_logs,
            commands::export_activity_json,
            commands::export_activity_csv,
            commands::get_content_detectors,
            commands::get_content_extractors,
            commands::create_content_extractor,
            commands::update_content_extractor_definition,
            commands::duplicate_content_extractor,
            commands::delete_content_extractor,
            commands::restore_default_content_extractors,
            commands::get_library_items,
            commands::set_library_item_enabled,
            commands::get_content_types,
            commands::get_content_type_groups,
            commands::create_content_type_group,
            commands::update_content_type_group,
            commands::set_content_type_group_archived,
            commands::delete_content_type_group,
            commands::restore_default_content_type_groups,
            commands::create_content_type,
            commands::update_content_type,
            commands::set_content_type_archived,
            commands::restore_default_content_types,
            commands::create_content_detector,
            commands::update_content_detector,
            commands::duplicate_content_detector,
            commands::delete_content_detector,
            commands::restore_default_content_detectors,
            commands::rescan_content_detection_history,
            commands::test_content_detector,
            commands::play_system_sound,
            commands::get_clip_collection_summary,
            commands::save_app_setting,
            commands::save_app_settings,
            commands::get_all_app_settings,
            commands::set_linux_native_menu_theme,
            commands::set_overlay_cursor,
            commands::enforce_clip_retention,
            commands::enforce_trash_retention,
            commands::enforce_activity_retention,
            commands::enforce_revision_retention,
            commands::update_clip_note,
            commands::delete_clip,
            commands::toggle_pin_clip,
            commands::assign_clip_bin,
            commands::remove_clip_bin,
            commands::reorder_pinned_clips,
            commands::reorder_bin_clips,
            commands::get_clip_versions,
            commands::get_clip_version_count,
            commands::restore_clip_version,
            commands::get_ocr_backfill_status,
            commands::start_ocr_backfill,
            commands::cancel_ocr_backfill,
            commands::retry_failed_ocr,
            commands::batch_pin_clips,
            commands::batch_protect_clips,
            commands::batch_trash_clips,
            commands::batch_assign_bin_clips,
            commands::copy_clip_to_system,
            commands::copy_clip_by_id,
            commands::paste_text_to_frontmost,
            commands::get_bins,
            commands::create_bin,
            commands::update_bin,
            commands::delete_bin,
            commands::get_pipelines,
            commands::create_pipeline,
            commands::update_pipeline,
            commands::update_pipeline_shortcut,
            commands::delete_pipeline,
            commands::preview_pipeline_steps,
            commands::get_operations,
            commands::get_intelligence_connections,
            commands::detect_intelligence_connections,
            commands::create_intelligence_connection,
            commands::update_intelligence_connection,
            commands::delete_intelligence_connection,
            commands::reorder_intelligence_connections,
            commands::plan_transformation_intent,
            commands::test_transformation_plan,
            commands::get_intent_transforms,
            commands::get_transforms,
            commands::save_saved_transform,
            commands::update_saved_transform,
            commands::delete_saved_transform,
            commands::apply_transform_preview_to_clip,
            commands::get_clip_transformation_provenance,
            commands::create_operation,
            commands::update_operation,
            commands::duplicate_operation,
            commands::delete_operation,
            commands::transform_text,
            commands::execute_transformation,
            commands::cancel_transformation_execution,
            commands::get_intelligence_scheduler_snapshot,
            commands::get_installation_diagnostics,
            commands::get_third_party_licenses,
            commands::get_library_location,
            commands::move_library,
            commands::restore_default_library_location,
            commands::toggle_clip_protected,
            commands::trash_unpinned_clips,
            commands::purge_unpinned_clips,
            commands::start_sequential_paste,
            commands::push_sequential_item,
            commands::pop_sequential_paste,
            commands::paste_sequential_item_by_index,
            commands::remove_sequential_item_by_index,
            commands::reorder_sequential_items,
            commands::stop_sequential_paste,
            commands::paste_all_sequential,
            commands::get_sequential_status,
            commands::get_queue_paste_target,
            commands::toggle_hud_window,
            commands::paste_clip_by_id,
            commands::set_dock_visibility,
            commands::get_source_icons,
            commands::get_installed_applications,
            commands::open_emoji_picker,
            commands::extract_ocr_from_clip,
            commands::register_hud_shortcut,
            commands::update_bin_shortcut,
            commands::get_bin_transform_ref,
            commands::set_bin_transform_ref,
            commands::register_app_setting_hotkey,
            commands::toggle_clipboard_pause,
            commands::is_clipboard_paused,
            commands::export_clips_json,
            commands::export_clips_csv,
            commands::export_backup_file,
            commands::choose_import_file,
            commands::import_inspected_file,
            commands::export_full_backup_file,
            commands::restore_full_backup_file,
            commands::consume_pending_full_restore_client_state,
            commands::get_external_import_sources,
            commands::import_external_history,
            commands::factory_reset_app,
            commands::get_analytics_summary,
            commands::install_cli_to_path,
            commands::get_hotkey_capability_status,
            commands::request_accessibility_permission,
            commands::perform_titlebar_double_click
        ])
        .run(tauri::generate_context!())
        .expect("error while running Pasted application");
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
