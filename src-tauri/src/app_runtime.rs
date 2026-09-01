use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::{Emitter, Manager};

static EXIT_REQUESTED: AtomicBool = AtomicBool::new(false);

pub(crate) fn exit_requested() -> bool {
    EXIT_REQUESTED.load(Ordering::SeqCst)
}

pub(crate) fn request_app_exit(app: &tauri::AppHandle) {
    if EXIT_REQUESTED.swap(true, Ordering::SeqCst) {
        return;
    }
    for window in app.webview_windows().values() {
        let _ = window.hide();
    }
    let db = app.state::<Arc<crate::db::DbState>>();
    let _ = db.log_activity("app_exit_requested", "Quit Pasted");
    app.exit(0);
}

pub(crate) fn handle_single_instance(app: &tauri::AppHandle, args: &[String]) {
    if exit_requested() {
        return;
    }
    if args.iter().any(|argument| argument == "--skip-welcome") {
        if let Some(db) = app.try_state::<Arc<crate::db::DbState>>() {
            let _ = db.save_setting(
                crate::external_import::ONBOARDING_SETTING_KEY,
                &crate::external_import::ONBOARDING_VERSION.to_string(),
            );
            let _ = app.emit(
                "app-setting-changed",
                serde_json::json!({
                    "key": crate::external_import::ONBOARDING_SETTING_KEY,
                    "value": crate::external_import::ONBOARDING_VERSION.to_string(),
                }),
            );
        }
    }
    if let Some(path) = crate::live_app::request_from_args(args) {
        crate::live_app::handle_request_file(app, &path, false);
        return;
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub(crate) fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let startup_args = std::env::args().collect::<Vec<_>>();
    let is_autostart = startup_args
        .iter()
        .any(|argument| argument == "--autostart");
    let live_request = crate::live_app::request_from_args(&startup_args);

    #[cfg(target_os = "linux")]
    if let Err(error) = crate::linux_native_theme::apply_menu_theme(true) {
        eprintln!("Could not apply the initial native Linux menu theme: {error}");
    }
    crate::app_windows::configure_initial_windows(app)?;

    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("./pasted_data"));
    let preview_database_path = crate::local_webkit_preview::database_path();
    let db_path = preview_database_path
        .clone()
        .unwrap_or_else(|| crate::library_storage::resolve_database_path(&app_dir));
    let db_state =
        Arc::new(crate::db::DbState::new(db_path).expect("Failed to initialize SQLite database"));
    if startup_args
        .iter()
        .any(|argument| argument == "--skip-welcome")
    {
        let _ = db_state.save_setting(
            crate::external_import::ONBOARDING_SETTING_KEY,
            &crate::external_import::ONBOARDING_VERSION.to_string(),
        );
    }

    let queue_state = Arc::new(crate::sequential_paste::SequentialQueueState::persistent(
        db_state.clone(),
    ));
    let paste_target_state = Arc::new(crate::paste_target::PasteTargetState::new());
    paste_target_state.start_tracking();
    app.manage(db_state.clone());
    app.manage(Arc::new(crate::app_lock::AppLockState::from_db(&db_state)));
    app.manage(queue_state.clone());
    app.manage(paste_target_state);

    if let Some(path) = live_request.as_deref() {
        if crate::live_app::is_recovery_reset_request(path) {
            crate::live_app::handle_request_file(app.handle(), path, true);
            app.handle().exit(0);
            return Ok(());
        }
    }

    let launch_description = if is_autostart {
        "Opened Pasted at login"
    } else {
        "Opened Pasted"
    };
    let _ = db_state.log_activity("app_started", launch_description);

    let ocr_service = Arc::new(crate::ocr::spawn_ocr_worker(
        app.handle().clone(),
        db_state.clone(),
    ));
    app.manage(ocr_service.clone());
    crate::app_menu::install(app.handle(), &db_state)?;

    let monitor_handle = crate::clipboard_monitor::start_clipboard_monitor(
        app.handle().clone(),
        db_state.clone(),
        queue_state,
        ocr_service,
        preview_database_path.is_some(),
    );
    app.manage(Arc::new(crate::clipboard_monitor::ClipboardMonitorState {
        is_manually_paused: monitor_handle.is_manually_paused.clone(),
        is_auto_paused: monitor_handle.is_auto_paused.clone(),
    }));

    if let Some(path) = live_request.as_deref() {
        crate::live_app::handle_request_file(app.handle(), path, false);
    }
    #[cfg(target_os = "macos")]
    crate::app_windows::configure_overlay_windows(app.handle());

    crate::keyboard_layout::start_layout_monitor(app.handle().clone());
    let _ = crate::commands::register_all_app_shortcuts(app.handle());
    crate::app_tray::install(app, &db_state)?;
    crate::app_windows::mark_startup_setup_ready(app.handle());
    Ok(())
}

pub(crate) fn handle_run_event(app: &tauri::AppHandle, event: &tauri::RunEvent) {
    if !matches!(event, tauri::RunEvent::Resumed) {
        return;
    }
    let db = app.state::<Arc<crate::db::DbState>>();
    let state = app.state::<Arc<crate::app_lock::AppLockState>>();
    let enabled = db
        .get_setting(crate::app_lock::ENABLED_SETTING)
        .ok()
        .flatten()
        .as_deref()
        == Some("true");
    let lock_on_sleep = db
        .get_setting(crate::app_lock::LOCK_ON_SLEEP_SETTING)
        .ok()
        .flatten()
        .as_deref()
        != Some("false");
    if crate::features::is_enabled(&db, crate::features::Feature::AppLock)
        && enabled
        && lock_on_sleep
    {
        state.lock();
        crate::hud_window::hide(app);
        let _ = crate::app_menu::install(app, &db);
        let status = crate::app_lock::status(&db, &state);
        let _ = app.emit("app-lock-changed", status);
    }
}
