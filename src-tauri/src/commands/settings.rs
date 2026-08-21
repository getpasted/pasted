use std::collections::HashMap;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, State};

use crate::application_error::ApplicationError;
use crate::db::DbState;
use crate::features::{self, Feature};
use crate::sequential_paste::SequentialQueueState;
use crate::settings_service::SettingsUpdateOutcome;

use super::{refresh_native_app_menu, register_all_app_shortcuts};

pub(crate) fn emit_window_appearance_change(app: &AppHandle, key: &str, value: &str) {
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

fn apply_runtime_changes(app: &AppHandle, db: &Arc<DbState>, outcome: SettingsUpdateOutcome) {
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

#[tauri::command]
pub fn save_app_setting(
    key: String,
    value: String,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<(), ApplicationError> {
    let outcome = crate::settings_service::update_setting(&db, key, value)?;
    apply_runtime_changes(&app, &db, outcome);
    Ok(())
}

#[tauri::command]
pub fn save_app_settings(
    values: HashMap<String, String>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<(), ApplicationError> {
    let outcome = crate::settings_service::update_settings(&db, values)?;
    apply_runtime_changes(&app, &db, outcome);
    Ok(())
}

#[tauri::command]
pub fn get_all_app_settings(
    db: State<'_, Arc<DbState>>,
) -> Result<HashMap<String, String>, String> {
    let mut settings = db.get_all_settings().map_err(|error| error.to_string())?;
    settings.retain(|key, _| !crate::app_lock::is_private_setting(key));
    Ok(settings)
}
