use arboard::Clipboard;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::{Bin, ClipMutationSummary, ClipSearchRequest, ClipSearchResult, DbState};
use crate::features::{self, Feature};
use crate::installation_diagnostics::InstallationDiagnostics;
use crate::sequential_paste::SequentialQueueState;
use crate::third_party_licenses::ThirdPartyLicenseDocument;

pub(crate) mod activity;
pub(crate) mod analysis;
pub(crate) mod app_lock;
pub(crate) mod backups;
pub(crate) mod clip_metadata;
pub(crate) mod clip_policies;
pub(crate) mod clips;
pub(crate) mod content_registry;
pub(crate) mod extractors;
pub(crate) mod factory_reset;
pub(crate) mod file_previews;
pub(crate) mod imports;
pub(crate) mod intelligence;
pub(crate) mod manual_transforms;
pub(crate) mod queue;
pub(crate) mod retention;
pub(crate) mod search_indexes;
pub(crate) mod source_apps;
pub(crate) mod storage;
pub(crate) mod transformations;

pub(crate) use backups::*;
pub(crate) use factory_reset::*;
pub(crate) use imports::*;
pub(crate) use intelligence::*;
pub(crate) use manual_transforms::*;
pub(crate) use source_apps::*;
pub(crate) use transformations::*;

fn refresh_native_app_menu(app: &AppHandle, db: &Arc<DbState>) {
    if let Err(error) = crate::app_menu::install(app, db) {
        eprintln!("Could not refresh the native app menu: {error}");
    }
}

fn emit_window_appearance_change(app: &AppHandle, key: &str, value: &str) {
    let _ = app.emit(
        "app-setting-changed",
        serde_json::json!({ "key": key, "value": value }),
    );

    // Retain the narrower event while older windows and integrations migrate.
    if matches!(key, "themeMode" | "textSize") {
        let _ = app.emit(
            "window-appearance-changed",
            serde_json::json!({ "key": key, "value": value }),
        );
    }
}

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

#[tauri::command]
pub fn get_installation_diagnostics(
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<InstallationDiagnostics, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let app_path = executable
        .ancestors()
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
        .map(PathBuf::from)
        .unwrap_or(executable);
    let data_path = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    Ok(InstallationDiagnostics::collect_with_database(
        app_path,
        data_path,
        db.database_path(),
    ))
}

#[tauri::command]
pub fn get_third_party_licenses() -> ThirdPartyLicenseDocument {
    crate::third_party_licenses::document().clone()
}

fn apply_feature_policy_changes(app: &AppHandle, db: &Arc<DbState>, changed: &[Feature]) {
    for feature in changed {
        if *feature == Feature::AppLock {
            if let Some(state) = app.try_state::<Arc<crate::app_lock::AppLockState>>() {
                if !features::is_enabled(db, *feature) {
                    state.unlock();
                }
                let status = crate::app_lock::status(db, &state);
                let _ = app.emit("app-lock-changed", status);
            }
            continue;
        }
        if features::is_enabled(db, *feature) {
            continue;
        }
        match feature {
            Feature::Hud => crate::hud_window::hide(app),
            Feature::Queue => {
                if let Some(queue) = app.try_state::<Arc<SequentialQueueState>>() {
                    queue.stop_queue();
                    let _ = app.emit("sequential-updated", queue.get_status());
                }
            }
            Feature::Ocr => {
                if let Some(ocr) = app.try_state::<Arc<crate::ocr::OcrService>>() {
                    ocr.cancel();
                }
            }
            Feature::Notifications => {
                if let Some(window) = app.get_webview_window("capture-feedback") {
                    let _ = window.hide();
                }
            }
            _ => {}
        }
    }
    refresh_native_app_menu(app, db);
    crate::refresh_tray_menu(app, db);
    let _ = register_all_app_shortcuts(app);
}

#[tauri::command]
pub fn save_app_setting(
    key: String,
    value: String,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<(), crate::application_error::ApplicationError> {
    let outcome = crate::settings_service::update_setting(&db, key, value)?;
    apply_settings_runtime_changes(&app, &db, outcome);
    Ok(())
}

#[tauri::command]
pub fn save_app_settings(
    values: std::collections::HashMap<String, String>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<(), crate::application_error::ApplicationError> {
    let outcome = crate::settings_service::update_settings(&db, values)?;
    apply_settings_runtime_changes(&app, &db, outcome);
    Ok(())
}

fn apply_settings_runtime_changes(
    app: &AppHandle,
    db: &Arc<DbState>,
    outcome: crate::settings_service::SettingsUpdateOutcome,
) {
    let changed_features = outcome.changed_features();
    if !changed_features.is_empty() {
        apply_feature_policy_changes(app, db, &changed_features);
    }
    let mut language_changed = false;
    for change in outcome.changes {
        if change.key == "menubarIconStyle" {
            crate::refresh_tray_icon(app, &change.value);
        }
        if change.key == crate::localization::LANGUAGE_SETTING_KEY {
            language_changed = true;
        }
        emit_window_appearance_change(app, &change.key, &change.value);
    }
    if language_changed {
        refresh_native_app_menu(app, db);
        crate::refresh_tray_menu(app, db);
    }
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
    crate::request_app_exit(&app);
}

#[tauri::command]
pub fn get_all_app_settings(
    db: State<'_, Arc<DbState>>,
) -> Result<std::collections::HashMap<String, String>, String> {
    let mut settings = db.get_all_settings().map_err(|e| e.to_string())?;
    settings.retain(|key, _| !crate::app_lock::is_private_setting(key));
    Ok(settings)
}

#[tauri::command]
pub fn copy_clip_to_system(
    text: Option<String>,
    image_base64: Option<String>,
    file_paths: Option<Vec<String>>,
) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;

    if let Some(paths) = file_paths {
        if paths.is_empty() || !crate::resource_limits::file_list_within_limit(&paths) {
            return Err("File list exceeds Pasted's safety limit".to_string());
        }
        clipboard
            .set()
            .file_list(&paths)
            .map_err(|error| error.to_string())?;
    } else if let Some(img_b64) = image_base64 {
        // Strip data:image/png;base64,
        let clean = img_b64.split(',').next_back().unwrap_or(&img_b64);
        if clean.len() > crate::resource_limits::MAX_STORED_IMAGE_BASE64_BYTES {
            return Err("Clip image exceeds Pasted's safety limit".to_string());
        }
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, clean)
            .map_err(|e| e.to_string())?;

        let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
        let rgba = img.to_rgba8();
        let img_data = arboard::ImageData {
            width: rgba.width() as usize,
            height: rgba.height() as usize,
            bytes: std::borrow::Cow::Owned(rgba.into_raw()),
        };
        clipboard.set_image(img_data).map_err(|e| e.to_string())?;
    } else if let Some(t) = text {
        if t.len() > crate::resource_limits::MAX_CLIP_TEXT_BYTES {
            return Err("Clip text exceeds Pasted's safety limit".to_string());
        }
        clipboard.set_text(t).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn copy_clip_by_id(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
    sequential: State<'_, Arc<SequentialQueueState>>,
) -> Result<(), String> {
    copy_clip_by_id_shared(&db, &sequential, clip_id)
}

pub(crate) fn copy_clip_by_id_shared(
    db: &DbState,
    sequential: &SequentialQueueState,
    clip_id: i64,
) -> Result<(), String> {
    crate::clipboard_actions::copy_clip(db, sequential, clip_id)
}

#[tauri::command]
pub fn paste_text_to_frontmost(text: String, app: AppHandle) -> Result<(), String> {
    if text.len() > crate::resource_limits::MAX_CLIP_TEXT_BYTES {
        return Err("Clip text exceeds Pasted's 8 MB safety limit".to_string());
    }
    let mut clipboard = Clipboard::new().map_err(|error| error.to_string())?;
    clipboard
        .set_text(text)
        .map_err(|error| error.to_string())?;

    if let Some(hud) = app.get_webview_window("hud") {
        let _ = hud.hide();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.hide();
    }

    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = crate::paste_automation::paste();
    });

    Ok(())
}

#[tauri::command]
pub fn get_bins(db: State<'_, Arc<DbState>>) -> Result<Vec<Bin>, String> {
    db.get_bins().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_bin(
    name: String,
    icon: String,
    color: String,
    smart_rule: Option<String>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<Bin, String> {
    features::require(&db, Feature::Bins)?;
    let bin = db
        .create_bin(&name, &icon, &color, smart_rule.as_deref())
        .map_err(|e| e.to_string())?;
    refresh_native_app_menu(&app, &db);
    Ok(bin)
}

#[tauri::command]
pub fn delete_bin(
    id: i64,
    disposition: Option<String>,
    destination_bin_id: Option<i64>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    features::require(&db, Feature::Bins)?;
    db.delete_bin(
        id,
        disposition.as_deref().unwrap_or("keep"),
        destination_bin_id,
    )
    .map_err(|e| e.to_string())?;
    refresh_native_app_menu(&app, &db);
    Ok(())
}

#[tauri::command]
pub fn update_bin(
    id: i64,
    name: String,
    icon: String,
    color: String,
    smart_rule: Option<String>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    features::require(&db, Feature::Bins)?;
    db.update_bin(id, &name, &icon, &color, smart_rule.as_deref())
        .map_err(|e| e.to_string())?;
    refresh_native_app_menu(&app, &db);
    Ok(())
}

#[tauri::command]
pub fn update_bin_hotkey(
    id: i64,
    hotkey: Option<String>,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<(), String> {
    features::require(&db, Feature::Bins)?;
    features::require(&db, Feature::Hotkeys)?;
    let previous = db.get_bin(id).map_err(|error| error.to_string())?.shortcut;
    db.update_bin_hotkey(id, hotkey.as_deref())
        .map_err(|e| e.to_string())?;
    let changed_hotkeys: Vec<String> = hotkey.clone().into_iter().collect();
    if let Err(error) = register_changed_hotkeys(&app, &changed_hotkeys) {
        db.update_bin_hotkey(id, previous.as_deref())
            .map_err(|rollback| {
                format!("{error}; restoring the previous Bin hotkey failed: {rollback}")
            })?;
        let _ = register_all_app_shortcuts(&app);
        return Err(error);
    }
    Ok(())
}

#[tauri::command]
pub fn get_clip_hotkey_assignments(
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<ClipHotkeyAssignment>, String> {
    features::require(&db, Feature::Hotkeys)?;
    db.get_clip_hotkeys()
        .map(|assignments| {
            assignments
                .into_iter()
                .map(|(clip_id, hotkey)| ClipHotkeyAssignment { clip_id, hotkey })
                .collect()
        })
        .map_err(|error| error.to_string())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipHotkeyAssignment {
    clip_id: i64,
    hotkey: String,
}

#[tauri::command]
pub fn update_clip_hotkey(
    clip_id: i64,
    hotkey: Option<String>,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<crate::db::ClipItem, String> {
    features::require(&db, Feature::Protection)?;
    features::require(&db, Feature::Hotkeys)?;
    let previous = db
        .get_clip_by_id(clip_id)
        .map_err(|error| error.to_string())?;
    let previous_shortcut = previous.shortcut.clone();
    let previous_explicit = previous
        .is_explicitly_protected
        .unwrap_or(previous.is_protected);
    db.update_clip_hotkey(clip_id, hotkey.as_deref())
        .map_err(|error| error.to_string())?;
    let changed_hotkeys: Vec<String> = hotkey.clone().into_iter().collect();
    if let Err(error) = register_changed_hotkeys(&app, &changed_hotkeys) {
        db.restore_clip_hotkey_state(clip_id, previous_shortcut.as_deref(), previous_explicit)
            .map_err(|rollback| {
                format!("{error}; restoring the previous clip hotkey failed: {rollback}")
            })?;
        let _ = register_all_app_shortcuts(&app);
        return Err(error);
    }
    let assigned = hotkey
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    let activity_description = if assigned {
        format!("Assigned a hotkey to clip #{clip_id}")
    } else {
        format!("Removed the hotkey from clip #{clip_id}")
    };
    let _ = db.log_activity("clip_hotkey_changed", &activity_description);
    let clip = db
        .get_clip_by_id(clip_id)
        .map_err(|error| error.to_string())?;
    crate::app_events::emit_clip_library_changed(&app, vec![clip_id]);
    Ok(clip)
}

// Window & Activation Policy Commands
#[tauri::command]
pub fn toggle_hud_window(app: AppHandle) -> Result<(), String> {
    crate::hud_window::require_unlocked(&app)?;
    let db = app.state::<Arc<DbState>>();
    features::require(&db, Feature::Hud)?;
    if let Some(window) = app.get_webview_window("hud") {
        let is_vis = window.is_visible().unwrap_or(false);
        if is_vis {
            let _ = window.hide();
        } else {
            #[cfg(target_os = "macos")]
            let mut pos_payload: Option<serde_json::Value> = None;

            #[cfg(target_os = "macos")]
            {
                #[repr(C)]
                #[derive(Copy, Clone, Debug)]
                struct LocalPoint {
                    x: f64,
                    y: f64,
                }

                #[repr(C)]
                #[derive(Copy, Clone, Debug)]
                struct LocalSize {
                    width: f64,
                    height: f64,
                }

                #[repr(C)]
                #[derive(Copy, Clone, Debug)]
                struct LocalRect {
                    origin: LocalPoint,
                    size: LocalSize,
                }

                use objc::runtime::{Class, Object};
                use objc::{msg_send, sel, sel_impl};

                unsafe {
                    if let Some(event_class) = Class::get("NSEvent") {
                        let loc: LocalPoint = msg_send![event_class, mouseLocation];

                        let screens_class = Class::get("NSScreen");
                        if let Some(screens_cls) = screens_class {
                            let screens_array: *mut Object = msg_send![screens_cls, screens];
                            let screen_count: usize = msg_send![screens_array, count];

                            let mut target_screen: Option<*mut Object> = None;
                            let mut primary_height = 1080.0;

                            if screen_count > 0 {
                                let first_screen: *mut Object =
                                    msg_send![screens_array, objectAtIndex: 0usize];
                                let first_frame: LocalRect = msg_send![first_screen, frame];
                                primary_height = first_frame.size.height;
                            }

                            for i in 0..screen_count {
                                let screen: *mut Object =
                                    msg_send![screens_array, objectAtIndex: i];
                                let frame: LocalRect = msg_send![screen, frame];
                                if loc.x >= frame.origin.x
                                    && loc.x <= frame.origin.x + frame.size.width
                                    && loc.y >= frame.origin.y
                                    && loc.y <= frame.origin.y + frame.size.height
                                {
                                    target_screen = Some(screen);
                                    break;
                                }
                            }

                            let active_screen =
                                target_screen.unwrap_or_else(|| msg_send![screens_cls, mainScreen]);

                            if !active_screen.is_null() {
                                let vis_frame: LocalRect = msg_send![active_screen, visibleFrame];

                                let mouse_top_y = primary_height - loc.y;
                                let vis_top =
                                    primary_height - (vis_frame.origin.y + vis_frame.size.height);
                                let vis_bottom = primary_height - vis_frame.origin.y;
                                let vis_left = vis_frame.origin.x;
                                let vis_right = vis_frame.origin.x + vis_frame.size.width;

                                let hud_width = 360.0;
                                let hud_height = 440.0;

                                // Horizontal positioning (centered on cursor) & clamping
                                let mut target_x = loc.x - (hud_width / 2.0);
                                target_x = target_x.clamp(
                                    vis_left + 8.0,
                                    (vis_right - hud_width - 8.0).max(vis_left + 8.0),
                                );

                                // Vertical positioning & dynamic flip if near bottom edge
                                let mut target_y = mouse_top_y + 8.0;
                                if target_y + hud_height > vis_bottom - 8.0 {
                                    target_y = mouse_top_y - hud_height - 8.0;
                                }
                                target_y = target_y.clamp(
                                    vis_top + 8.0,
                                    (vis_bottom - hud_height - 8.0).max(vis_top + 8.0),
                                );

                                let is_flipped = target_y < mouse_top_y;
                                let payload = serde_json::json!({
                                    "flipped": is_flipped,
                                    "cursorX": loc.x,
                                    "cursorY": mouse_top_y,
                                    "targetX": target_x,
                                    "targetY": target_y
                                });
                                let _ = window.emit("hud_position_updated", payload.clone());
                                pos_payload = Some(payload);

                                if let Ok(ns_win_ptr) = window.ns_window() {
                                    let ns_win = ns_win_ptr as *mut Object;
                                    let _: () = msg_send![ns_win, setHasShadow: 0i8];
                                    let _: () = msg_send![ns_win, setAlphaValue: 0.0f64];
                                    let cocoa_y = primary_height - target_y - hud_height;
                                    let origin = LocalPoint {
                                        x: target_x,
                                        y: cocoa_y,
                                    };
                                    let _: () = msg_send![ns_win, setFrameOrigin: origin];
                                }

                                let _ = window.set_position(tauri::Position::Logical(
                                    tauri::LogicalPosition {
                                        x: target_x,
                                        y: target_y,
                                    },
                                ));
                            }
                        }
                    }
                }
            }

            crate::hud_window::reveal(&app)?;
            #[cfg(target_os = "macos")]
            {
                if let Ok(ns_win_ptr) = window.ns_window() {
                    use objc::runtime::Object;
                    use objc::{msg_send, sel, sel_impl};
                    unsafe {
                        let ns_win = ns_win_ptr as *mut Object;
                        let _: () = msg_send![ns_win, setAlphaValue: 1.0f64];
                    }
                }
                if let Some(payload) = pos_payload {
                    let _ = window.emit("hud_position_updated", payload);
                }
            }
        }
    } else {
        return Err("HUD window is unavailable".to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn paste_clip_by_id(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<(), String> {
    paste_clip_from_hud(&db, &app, clip_id)
}

pub(crate) fn paste_clip_from_hud(
    db: &DbState,
    app: &AppHandle,
    clip_id: i64,
) -> Result<(), String> {
    crate::clipboard_actions::paste_hud_clip(db, app, clip_id)
}

#[tauri::command]
pub fn toggle_clip_protected(clip_id: i64, db: State<'_, Arc<DbState>>) -> Result<bool, String> {
    features::require(&db, Feature::Protection)?;
    db.toggle_protected(clip_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn batch_protect_clips(
    ids: Vec<i64>,
    protected_state: bool,
    db: State<'_, Arc<DbState>>,
) -> Result<ClipMutationSummary, String> {
    features::require(&db, Feature::Protection)?;
    db.batch_protect_clips(ids, protected_state)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn trash_unpinned_clips(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.trash_unpinned_clips().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn purge_unpinned_clips(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.purge_unpinned_clips().map_err(|e| e.to_string())
}

pub fn register_all_app_shortcuts(app: &AppHandle) -> Result<(), String> {
    if let Some(mgr) = app.try_state::<Arc<crate::hotkey_manager::HotkeyManager>>() {
        mgr.register_all(app)
    } else {
        Err("HotkeyManager state not initialized".to_string())
    }
}

fn register_changed_hotkeys(app: &AppHandle, changed_hotkeys: &[String]) -> Result<(), String> {
    let Err(error) = register_all_app_shortcuts(app) else {
        return Ok(());
    };
    let Some(manager) = app.try_state::<Arc<crate::hotkey_manager::HotkeyManager>>() else {
        return Err(error);
    };
    let status = manager.registration_status();
    if status.state != "conflict" {
        return Err(error);
    }
    if changed_hotkeys_have_registration_issue(changed_hotkeys, &status.issues) {
        Err(error)
    } else {
        Ok(())
    }
}

fn changed_hotkeys_have_registration_issue(
    changed_hotkeys: &[String],
    issues: &[crate::hotkey_manager::HotkeyRegistrationIssue],
) -> bool {
    changed_hotkeys.iter().any(|changed| {
        let changed = changed.trim();
        !changed.is_empty() && issues.iter().any(|issue| issue.hotkey.trim() == changed)
    })
}

pub type AccessibilityStatus = crate::platform_capabilities::AccessibilityStatus;

pub fn check_accessibility_permission() -> AccessibilityStatus {
    crate::platform_capabilities::accessibility_status()
}

#[derive(serde::Serialize)]
pub struct HotkeyCapabilityStatus {
    pub platform: String,
    pub backend: String,
    pub state: String,
    pub is_trusted: bool,
    pub is_dev_mode: bool,
    pub configured_count: usize,
    pub registered_count: usize,
    pub issues: Vec<crate::hotkey_manager::HotkeyRegistrationIssue>,
    pub bindings: Vec<crate::hotkey_manager::HotkeyRegisteredBinding>,
}

#[tauri::command]
pub fn get_hotkey_capability_status(app: AppHandle) -> HotkeyCapabilityStatus {
    let accessibility = check_accessibility_permission();
    let registration = app
        .try_state::<Arc<crate::hotkey_manager::HotkeyManager>>()
        .map(|manager| manager.registration_status())
        .unwrap_or_default();
    let platform = if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unsupported"
    };

    HotkeyCapabilityStatus {
        platform: platform.into(),
        backend: registration.backend,
        state: registration.state,
        is_trusted: accessibility.is_trusted,
        is_dev_mode: accessibility.is_dev_mode,
        configured_count: registration.configured_count,
        registered_count: registration.registered_count,
        issues: registration.issues,
        bindings: registration.bindings,
    }
}

#[tauri::command]
pub fn request_accessibility_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let _ = Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();
        let _ = Command::new("open")
            .arg("x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_Accessibility")
            .spawn();

        let status = check_accessibility_permission();
        status.is_trusted
    }
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let _ = Command::new("cmd")
            .arg("/c")
            .arg("start ms-settings:privacy-accessibility")
            .spawn();
        true
    }
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        let _ = Command::new("gnome-control-center").spawn();
        true
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    true
}

const BACKING_URL: &str = "https://back.getpasted.app";

#[tauri::command]
pub fn open_backing_page() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = std::process::Command::new("open");
        command.arg(BACKING_URL);
        command
    };

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = std::process::Command::new("cmd");
        command.args(["/c", "start", "", BACKING_URL]);
        command
    };

    #[cfg(target_os = "linux")]
    let mut command = {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(BACKING_URL);
        command
    };

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    return command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open the backing page: {error}"));

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    Err("Opening the backing page is unavailable on this platform".to_string())
}

#[tauri::command]
pub fn register_app_setting_hotkey(
    key: String,
    value: String,
    app: AppHandle,
) -> Result<(), String> {
    if !is_app_setting_hotkey_key(&key) {
        return Err("Unknown app hotkey setting.".to_string());
    }
    persist_hotkey_settings_and_register(std::iter::once((key, value)).collect(), &app)
}

fn is_app_setting_hotkey_key(key: &str) -> bool {
    matches!(
        key,
        "hudHotkey"
            | "seqToggleHotkey"
            | "seqPopHotkey"
            | "copyLastPipelineHotkey"
            | "pasteLastPipelineHotkey"
            | "openTransformationsHotkey"
            | "openMainWindowHotkey"
            | "lockAppHotkey"
    ) || key
        .strip_prefix("pasteClip")
        .and_then(|suffix| suffix.strip_suffix("Hotkey"))
        .and_then(|position| position.parse::<usize>().ok())
        .is_some_and(|position| (1..=9).contains(&position))
}

#[tauri::command]
pub fn register_app_setting_hotkeys(
    values: std::collections::HashMap<String, String>,
    app: AppHandle,
) -> Result<(), String> {
    if values.keys().any(|key| !is_app_setting_hotkey_key(key)) {
        return Err("Unknown app hotkey setting.".to_string());
    }
    persist_hotkey_settings_and_register(values, &app)
}

fn persist_hotkey_settings_and_register(
    values: std::collections::HashMap<String, String>,
    app: &AppHandle,
) -> Result<(), String> {
    let db = app.state::<Arc<DbState>>();
    features::require(&db, Feature::Hotkeys)?;
    let previous: std::collections::HashMap<String, Option<String>> = values
        .keys()
        .map(|key| {
            db.get_setting(key)
                .map(|value| (key.clone(), value))
                .map_err(|error| error.to_string())
        })
        .collect::<Result<_, _>>()?;
    if values.iter().all(|(key, value)| {
        previous
            .get(key)
            .and_then(|previous_value| previous_value.as_deref())
            == Some(value.as_str())
    }) {
        return Ok(());
    }
    let changed_hotkeys: Vec<String> = values
        .iter()
        .filter(|(key, value)| {
            previous
                .get(*key)
                .and_then(|previous_value| previous_value.as_deref())
                != Some(value.as_str())
        })
        .map(|(_, value)| value.clone())
        .collect();
    db.save_settings(&values)
        .map_err(|error| error.to_string())?;
    if let Err(registration_error) = register_changed_hotkeys(app, &changed_hotkeys) {
        let restored: std::collections::HashMap<String, String> = previous
            .iter()
            .filter_map(|(key, value)| value.clone().map(|value| (key.clone(), value)))
            .collect();
        let deleted: Vec<&str> = previous
            .iter()
            .filter_map(|(key, value)| value.is_none().then_some(key.as_str()))
            .collect();
        db.save_and_delete_settings(&restored, &deleted)
            .map_err(|error| {
                format!(
                    "{registration_error}; restoring the previous shortcut settings failed: {error}"
                )
            })?;
        if let Err(rollback_error) = register_all_app_shortcuts(app) {
            return Err(format!(
                "{registration_error}; restoring the previous native shortcuts failed: {rollback_error}"
            ));
        }
        return Err(registration_error);
    }
    Ok(())
}

#[tauri::command]
pub fn resolve_logical_shortcut_key(code: String, fallback: String) -> String {
    use std::str::FromStr;

    tauri_plugin_global_shortcut::Code::from_str(&code)
        .ok()
        .and_then(crate::keyboard_layout::logical_key_for_code)
        .unwrap_or(fallback)
}

#[tauri::command]
pub fn register_hud_hotkey(hotkey: String, app: AppHandle) -> Result<(), String> {
    persist_hotkey_settings_and_register(
        std::iter::once(("hudHotkey".to_string(), hotkey)).collect(),
        &app,
    )
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

#[tauri::command]
pub fn extract_ocr_from_clip(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::extraction_execution::ExtractionApplicationResult, String> {
    features::require(&db, Feature::Ocr)?;
    let extractors = db
        .active_image_text_extractors_for_features(true)
        .map_err(|error| error.to_string())?;
    if extractors.is_empty() {
        return Err("No available image text Extractor is enabled".to_string());
    }
    let clip = db.get_clip_by_id(clip_id).map_err(|e| e.to_string())?;

    let image = clip
        .image_base64
        .as_deref()
        .ok_or_else(|| "Clip has no extractable image data".to_string())?;
    let bytes = crate::ocr::decode_stored_image(image)
        .ok_or_else(|| "Clip has no extractable image data".to_string())?;
    let classifiers = features::is_enabled(&db, Feature::ContentClassification)
        .then(|| db.get_content_classifiers().ok())
        .flatten();
    let registry = crate::content_extraction::system_engine_registry();
    let analysis = crate::extraction_execution::analyze_images_with_registry(
        bytes,
        &extractors,
        classifiers.as_deref(),
        &registry,
    );
    let extractor = extractors
        .iter()
        .find(|extractor| extractor.stable_ref == analysis.target_ref)
        .ok_or_else(|| "No Extractor completed the image analysis".to_string())?;
    crate::extraction_execution::apply_image_analysis(
        &db,
        clip_id,
        &clip.content_hash,
        extractor,
        classifiers.is_some(),
        analysis,
    )
    .map_err(|error| match error {
        rusqlite::Error::InvalidParameterName(message) => message,
        error => error.to_string(),
    })
}

#[tauri::command]
pub fn get_clip_searchable_text(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<crate::db::ClipSearchableText>, String> {
    db.get_clip_searchable_text(clip_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_clip_extraction_results(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::db::StoredExtractionObservation>, String> {
    db.get_extraction_observations(clip_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_clip_extraction_history(
    clip_id: i64,
    limit: usize,
    offset: usize,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::db::StoredExtractionAttempt>, String> {
    db.get_extraction_history(clip_id, limit, offset)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn search_clips(
    request: ClipSearchRequest,
    db: State<'_, Arc<DbState>>,
) -> Result<ClipSearchResult, String> {
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.search_clips(&request).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn extract_text_from_file_clip(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::extraction_execution::ExtractionApplicationResult, String> {
    let transcriptions_enabled = features::is_enabled(&db, Feature::Transcriptions);
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let extractors = db
            .active_file_text_extractors_for_features(transcriptions_enabled)
            .map_err(|error| error.to_string())?;
        if extractors.is_empty() {
            return Err("No available file text Extractor is enabled".to_string());
        }
        let clip = db
            .get_clip_by_id(clip_id)
            .map_err(|error| error.to_string())?;
        let paths = clip
            .text_content
            .as_deref()
            .map(crate::content_inspection::parse_file_paths)
            .filter(|paths| !paths.is_empty())
            .ok_or_else(|| "Clip has no extractable file references".to_string())?;
        if !crate::resource_limits::file_list_within_limit(&paths) {
            return Err("File references exceed the extraction safety limit".to_string());
        }
        let classifiers = features::is_enabled(&db, Feature::ContentClassification)
            .then(|| db.get_content_classifiers().ok())
            .flatten();
        let registry = crate::content_extraction::system_engine_registry();
        let analysis = crate::extraction_execution::analyze_files_with_extractors_and_registry(
            paths,
            &extractors,
            classifiers.as_deref(),
            &registry,
        );
        let extractor = extractors
            .iter()
            .find(|extractor| extractor.stable_ref == analysis.target_ref)
            .ok_or_else(|| "No Extractor completed the file analysis".to_string())?;
        crate::extraction_execution::apply_file_analysis(
            &db,
            clip_id,
            &clip.content_hash,
            extractor,
            classifiers.is_some(),
            analysis,
        )
        .map_err(|error| match error {
            rusqlite::Error::InvalidParameterName(message) => message,
            error => error.to_string(),
        })
    })
    .await
    .map_err(|error| format!("File extraction task failed: {error}"))?
}

#[tauri::command]
pub fn get_ocr_backfill_status(
    db: State<'_, Arc<DbState>>,
) -> Result<crate::db::OcrBackfillStatus, String> {
    db.get_ocr_backfill_status()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn start_ocr_backfill(
    db: State<'_, Arc<DbState>>,
    ocr: State<'_, Arc<crate::ocr::OcrService>>,
) -> Result<(), String> {
    features::require(&db, Feature::Ocr)?;
    if db
        .active_image_text_extractors_for_features(true)
        .map_err(|error| error.to_string())?
        .is_empty()
    {
        return Err("No available image text Extractor is enabled".to_string());
    }
    ocr.start_backfill()
}

#[tauri::command]
pub fn cancel_ocr_backfill(
    db: State<'_, Arc<DbState>>,
    ocr: State<'_, Arc<crate::ocr::OcrService>>,
) -> Result<(), String> {
    features::require(&db, Feature::Ocr)?;
    ocr.cancel();
    Ok(())
}

#[tauri::command]
pub fn retry_failed_ocr(
    db: State<'_, Arc<DbState>>,
    ocr: State<'_, Arc<crate::ocr::OcrService>>,
) -> Result<usize, String> {
    features::require(&db, Feature::Ocr)?;
    let count = db.reset_failed_ocr().map_err(|error| error.to_string())?;
    if count > 0 {
        ocr.start_backfill()?;
    }
    Ok(count)
}

#[tauri::command]
pub fn toggle_clipboard_pause(
    monitor_state: State<'_, Arc<crate::clipboard_monitor::ClipboardMonitorState>>,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<bool, String> {
    let current = monitor_state
        .is_manually_paused
        .load(std::sync::atomic::Ordering::Relaxed);
    let new_val = !current;
    monitor_state
        .is_manually_paused
        .store(new_val, std::sync::atomic::Ordering::Relaxed);

    if new_val {
        let _ = db.log_activity(
            "recording_manually_paused",
            "Clipboard recording manually paused",
        );
    } else {
        let _ = db.log_activity(
            "recording_manually_resumed",
            "Clipboard recording manually resumed",
        );
    }

    let effective = monitor_state.is_paused();
    crate::app_events::emit_clipboard_pause_changed(&app, effective, None);
    Ok(effective)
}

#[tauri::command]
pub fn is_clipboard_paused(
    monitor_state: State<'_, Arc<crate::clipboard_monitor::ClipboardMonitorState>>,
) -> Result<bool, String> {
    Ok(monitor_state.is_paused())
}

#[tauri::command]
pub fn export_clips_json(db: State<'_, Arc<DbState>>) -> Result<String, String> {
    let exported = db.export_clips_json().map_err(|error| error.to_string())?;
    let _ = db.log_activity("data_export_completed", "Exported Clips as JSON");
    Ok(exported)
}

#[cfg(test)]
fn csv_cell(value: &str) -> String {
    let escaped = value.replace('"', "\"\"");
    let neutralized = if matches!(
        value.chars().next(),
        Some('=' | '+' | '-' | '@' | '\t' | '\r')
    ) {
        format!("'{escaped}")
    } else {
        escaped
    };
    format!("\"{neutralized}\"")
}

#[tauri::command]
pub fn export_clips_csv(db: State<'_, Arc<DbState>>) -> Result<String, String> {
    let exported = db.export_clips_csv().map_err(|error| error.to_string())?;
    let _ = db.log_activity("data_export_completed", "Exported Clips as CSV");
    Ok(exported)
}

#[tauri::command]
pub fn get_analytics_summary(
    db: State<'_, Arc<DbState>>,
) -> Result<crate::db::AnalyticsSummary, String> {
    db.get_analytics_summary().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn install_cli_to_path() -> Result<String, String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let bin_dir = exe_path.parent().ok_or("Cannot locate binary directory")?;
    let cli_exe = bin_dir.join("pasted");

    if !cli_exe.exists() {
        return Err(format!(
            "pasted binary not found at '{:?}'. Run 'cargo build --bin pasted' first.",
            cli_exe
        ));
    }

    #[cfg(unix)]
    {
        let target_dir = dirs::home_dir()
            .map(|home| home.join(".local/bin"))
            .ok_or("Cannot locate your home directory")?;
        let symlink_path = install_cli_symlink(&cli_exe, &target_dir)?;
        Ok(format!(
            "Successfully installed the pasted command at '{}'. Make sure that directory is in your PATH.",
            symlink_path.display()
        ))
    }

    #[cfg(not(unix))]
    {
        Err("Automatic CLI installation is not supported on this platform yet".to_string())
    }
}

#[cfg(unix)]
fn install_cli_symlink(
    cli_exe: &std::path::Path,
    target_dir: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    use std::fs;
    use std::os::unix::fs::symlink;

    fs::create_dir_all(target_dir).map_err(|error| {
        format!(
            "Failed to create CLI directory '{}': {error}",
            target_dir.display()
        )
    })?;
    let symlink_path = target_dir.join("pasted");
    match fs::symlink_metadata(&symlink_path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            let existing_target = fs::read_link(&symlink_path).map_err(|error| {
                format!(
                    "Failed to inspect existing CLI link '{}': {error}",
                    symlink_path.display()
                )
            })?;
            if existing_target == cli_exe {
                return Ok(symlink_path);
            }
            return Err(format!(
                "Refusing to replace existing CLI link '{}' (currently points to '{}')",
                symlink_path.display(),
                existing_target.display()
            ));
        }
        Ok(_) => {
            return Err(format!(
                "Refusing to replace existing file '{}'",
                symlink_path.display()
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "Failed to inspect CLI destination '{}': {error}",
                symlink_path.display()
            ));
        }
    }

    symlink(cli_exe, &symlink_path).map_err(|error| {
        format!(
            "Failed to create CLI link '{}': {error}",
            symlink_path.display()
        )
    })?;
    Ok(symlink_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn ocr_text_never_replaces_an_image_clips_copy_fingerprint() {
        let rgba = vec![12, 34, 56, 255];
        let image = image::RgbaImage::from_raw(1, 1, rgba.clone()).unwrap();
        let mut encoded = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();
        let image_base64 = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(encoded.into_inner())
        );
        let clip = crate::db::ClipItem {
            id: 1,
            name: None,
            content_type: "image".to_string(),
            content_types: Vec::new(),
            file_formats: Vec::new(),
            text_content: Some("recognized OCR text".to_string()),
            html_content: None,
            image_base64: Some(image_base64),
            image_path: None,
            content_hash: "stored-image-hash".to_string(),
            source: "Screenshot".to_string(),
            is_pinned: false,
            is_protected: false,
            is_explicitly_protected: Some(false),
            protecting_bin_ids: Vec::new(),
            is_concealed: false,
            is_explicitly_concealed: Some(false),
            is_explicitly_revealed: false,
            concealing_bin_ids: Vec::new(),
            concealing_content_types: Vec::new(),
            shortcut: None,
            is_transformed: false,
            pin_order: 0,
            bin_id: None,
            bin_ids: None,
            note: None,
            is_trashed: false,
            trashed_at: None,
            created_at: "2026-08-11T00:00:00Z".to_string(),
            ocr_extractor_ref: None,
            ocr_extractor_name: None,
            ocr_engine_version: None,
        };

        assert_eq!(
            crate::clipboard_actions::internal_fingerprint(&clip).unwrap(),
            crate::clipboard_fingerprint::image_rgba(&rgba)
        );
    }

    fn unique_test_directory(label: &str) -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("pasted-{label}-{}-{nonce}", std::process::id()))
    }

    #[test]
    #[cfg(unix)]
    fn cli_install_never_overwrites_an_existing_file() {
        let root = unique_test_directory("cli-preserve");
        let bin_dir = root.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let destination = bin_dir.join("pasted");
        std::fs::write(&destination, "user-owned").unwrap();

        let error = install_cli_symlink(&root.join("source"), &bin_dir).unwrap_err();
        assert!(error.contains("Refusing to replace existing file"));
        assert_eq!(std::fs::read_to_string(&destination).unwrap(), "user-owned");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn cli_install_is_idempotent_for_its_existing_link() {
        let root = unique_test_directory("cli-idempotent");
        let source = root.join("pasted-source");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(&source, "binary").unwrap();
        let bin_dir = root.join("bin");

        let first = install_cli_symlink(&source, &bin_dir).unwrap();
        let second = install_cli_symlink(&source, &bin_dir).unwrap();
        assert_eq!(first, second);
        assert_eq!(std::fs::read_link(second).unwrap(), source);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn test_parse_shortcut_str_variations() {
        assert!(crate::keyboard_shortcuts::parse("CmdOrCtrl+Shift+V").is_some());
        assert!(crate::keyboard_shortcuts::parse("Control+Alt+C").is_some());
        assert!(crate::keyboard_shortcuts::parse("Ctrl+Alt+KeyC").is_some());
        assert!(crate::keyboard_shortcuts::parse("Alt+Super+KeyV").is_some());
        assert!(crate::keyboard_shortcuts::parse("Option+Cmd+C").is_some());
        assert!(crate::keyboard_shortcuts::parse("Command+Shift+V").is_some());
        assert!(crate::keyboard_shortcuts::parse("Control+Option+C").is_some());
        assert!(crate::keyboard_shortcuts::parse("Control+Option+V").is_some());
        assert!(crate::keyboard_shortcuts::parse("Super+Alt+KeyC").is_some());
        assert!(crate::keyboard_shortcuts::parse("").is_none());
        assert!(crate::keyboard_shortcuts::parse("   ").is_none());

        // Equivalence checks for key representations
        let sc1 = crate::keyboard_shortcuts::parse("Option+Command+C").unwrap();
        let sc2 = crate::keyboard_shortcuts::parse("Alt+Super+KeyC").unwrap();
        assert_eq!(
            sc1, sc2,
            "Option+Command+C should resolve to identical Shortcut struct as Alt+Super+KeyC"
        );
    }

    #[test]
    fn app_setting_hotkey_keys_are_narrowly_scoped() {
        assert!(is_app_setting_hotkey_key("hudHotkey"));
        assert!(is_app_setting_hotkey_key("lockAppHotkey"));
        assert!(is_app_setting_hotkey_key("pasteClip1Hotkey"));
        assert!(is_app_setting_hotkey_key("pasteClip9Hotkey"));
        assert!(!is_app_setting_hotkey_key("unlockAppHotkey"));
        assert!(!is_app_setting_hotkey_key("pasteClip0Hotkey"));
        assert!(!is_app_setting_hotkey_key("pasteClip10Hotkey"));
        assert!(!is_app_setting_hotkey_key("enableAppLock"));
    }

    #[test]
    fn unrelated_hotkey_conflicts_do_not_reject_a_change() {
        let issues = vec![crate::hotkey_manager::HotkeyRegistrationIssue {
            hotkey: "Alt+Shift+V".into(),
            description: "HUD".into(),
            message: "Unavailable".into(),
        }];
        assert!(!changed_hotkeys_have_registration_issue(
            &["Alt+Shift+L".into()],
            &issues
        ));
        assert!(changed_hotkeys_have_registration_issue(
            &[" Alt+Shift+V ".into()],
            &issues
        ));
        assert!(!changed_hotkeys_have_registration_issue(
            &[String::new()],
            &issues
        ));
    }

    #[test]
    fn intelligence_credentials_must_remain_references() {
        for reference in [
            "env:OPENAI_API_KEY",
            "env:_LOCAL_MODEL_TOKEN",
            "op://Private/OpenAI/credential",
            "keychain:pasted.openai",
        ] {
            assert!(
                crate::intelligence_connections::validate_credential_reference(Some(reference))
                    .is_ok()
            );
        }
        for value in [
            "sk-proj-literal-secret",
            "env:NOT VALID",
            "env:123_INVALID",
            "op://",
            " keychain:pasted.openai",
            "",
        ] {
            assert!(
                crate::intelligence_connections::validate_credential_reference(Some(value))
                    .is_err()
            );
        }
        assert!(crate::intelligence_connections::validate_credential_reference(None).is_ok());
    }

    #[test]
    fn test_print_parsed_shortcuts() {
        let strings = vec![
            "Command+1",
            "Command+Digit1",
            "Super+Digit1",
            "Command+C",
            "Command+KeyC",
            "Super+KeyC",
            "Alt+Shift+V",
            "Alt+Shift+KeyV",
            "Control+Alt+C",
            "Control+Alt+KeyC",
        ];
        for s in strings {
            let parsed = crate::keyboard_shortcuts::parse(s);
            println!("parse_shortcut_str('{s}') = {:?}", parsed);
        }
    }

    #[test]
    fn test_accessibility_status_check() {
        let status = check_accessibility_permission();
        println!(
            "Accessibility test status: trusted={}, dev_mode={}",
            status.is_trusted, status.is_dev_mode
        );
        assert_eq!(status.is_dev_mode, cfg!(debug_assertions));
    }

    #[test]
    fn csv_cells_escape_structure_and_neutralize_formulas() {
        assert_eq!(csv_cell("plain text"), "\"plain text\"");
        assert_eq!(
            csv_cell("commas, quotes \" and\nlines"),
            "\"commas, quotes \"\" and\nlines\""
        );
        assert_eq!(csv_cell("=2+2"), "\"'=2+2\"");
        assert_eq!(csv_cell("+SUM(A1:A2)"), "\"'+SUM(A1:A2)\"");
        assert_eq!(csv_cell("-1+2"), "\"'-1+2\"");
        assert_eq!(csv_cell("@SUM(A1:A2)"), "\"'@SUM(A1:A2)\"");
        assert_eq!(csv_cell("\t=2+2"), "\"'\t=2+2\"");
        assert_eq!(csv_cell("\r=2+2"), "\"'\r=2+2\"");
    }

    #[test]
    fn file_clip_metadata_reports_availability_without_crawling_directories() {
        let root = std::env::temp_dir().join(format!(
            "pasted_file_metadata_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let directory = root.join("Folder");
        std::fs::create_dir_all(&directory).unwrap();
        let file = root.join("first.txt");
        std::fs::write(&file, b"pasted").unwrap();
        let missing = root.join("missing.mp4");
        let paths = vec![
            file.to_string_lossy().into_owned(),
            directory.to_string_lossy().into_owned(),
            missing.to_string_lossy().into_owned(),
        ];

        let inspection = crate::content_inspection::inspect_files(paths.clone(), None).unwrap();
        let structure = inspection.result.files.unwrap();
        let observations = crate::content_inspection::observe_files(&paths);
        assert_eq!(structure.item_count, 3);
        assert_eq!(observations.available_count, 2);
        assert_eq!(observations.file_count, 1);
        assert_eq!(observations.directory_count, 1);
        assert_eq!(observations.total_size_bytes, 6);
        assert_eq!(structure.extensions, vec!["TXT", "MP4"]);

        std::fs::remove_dir_all(root).unwrap();
    }
}
