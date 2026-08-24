use std::sync::Arc;

#[cfg(target_os = "windows")]
use tauri::Manager;
use tauri::{AppHandle, Emitter, State};

use crate::db::DbState;
use crate::features::{self, Feature};

#[cfg(target_os = "windows")]
fn system_auth_window_handle(app: &AppHandle) -> Result<Option<isize>, String> {
    app.get_webview_window("main")
        .ok_or_else(|| "The Pasted window is unavailable.".to_string())?
        .hwnd()
        .map(|handle| Some(handle.0 as isize))
        .map_err(|error| error.to_string())
}

#[cfg(not(target_os = "windows"))]
fn system_auth_window_handle(_app: &AppHandle) -> Result<Option<isize>, String> {
    Ok(None)
}

#[tauri::command]
pub fn get_app_lock_status(
    db: State<'_, Arc<DbState>>,
    state: State<'_, Arc<crate::app_lock::AppLockState>>,
) -> crate::app_lock::AppLockStatus {
    crate::app_lock::status(&db, &state)
}

#[tauri::command]
pub fn configure_app_lock(
    passphrase: String,
    current_passphrase: Option<String>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
    state: State<'_, Arc<crate::app_lock::AppLockState>>,
) -> Result<crate::app_lock::AppLockStatus, String> {
    features::require(&db, Feature::AppLock)?;
    let was_enabled = db
        .get_setting(crate::app_lock::ENABLED_SETTING)
        .map_err(|error| error.to_string())?
        .as_deref()
        == Some("true");
    if was_enabled
        && !crate::app_lock::verify(&db, current_passphrase.as_deref().unwrap_or_default())?
    {
        return Err("The current passphrase is incorrect.".to_string());
    }
    crate::app_lock::configure(&db, &passphrase)?;
    state.unlock();
    let _ = if was_enabled {
        db.log_activity("app_lock_passphrase_changed", "Changed app lock passphrase")
    } else {
        db.log_activity("app_lock_enabled", "Enabled app lock")
    };
    let status = crate::app_lock::status(&db, &state);
    let _ = app.emit("app-lock-changed", &status);
    Ok(status)
}

#[tauri::command]
pub fn disable_app_lock(
    passphrase: String,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
    state: State<'_, Arc<crate::app_lock::AppLockState>>,
) -> Result<crate::app_lock::AppLockStatus, String> {
    features::require(&db, Feature::AppLock)?;
    crate::app_lock::disable(&db, &passphrase)?;
    state.unlock();
    let _ = db.log_activity("app_lock_disabled", "Disabled app lock");
    let status = crate::app_lock::status(&db, &state);
    let _ = app.emit("app-lock-changed", &status);
    Ok(status)
}

#[tauri::command]
pub fn lock_app(
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
    state: State<'_, Arc<crate::app_lock::AppLockState>>,
) -> Result<crate::app_lock::AppLockStatus, String> {
    let status = lock_app_state(&db, &state)?;
    crate::hud_window::hide(&app);
    super::refresh_native_app_menu(&app, &db);
    let _ = app.emit("app-lock-changed", &status);
    Ok(status)
}

pub(crate) fn lock_app_state(
    db: &DbState,
    state: &crate::app_lock::AppLockState,
) -> Result<crate::app_lock::AppLockStatus, String> {
    crate::app_lock::lock_enabled(db, state)
}

#[tauri::command]
pub async fn unlock_app(
    passphrase: Option<String>,
    auth_method: Option<String>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
    state: State<'_, Arc<crate::app_lock::AppLockState>>,
) -> Result<crate::app_lock::AppLockStatus, String> {
    features::require(&db, Feature::AppLock)?;
    state.check_retry()?;
    let authenticated = if let Some(method) = auth_method.as_deref() {
        let (method, enabled) = match method {
            "system" => (
                crate::app_lock::SystemAuthMethod::Primary,
                crate::app_lock::status(&db, &state).system_auth_enabled,
            ),
            "apple_watch" => (
                crate::app_lock::SystemAuthMethod::AppleWatch,
                crate::app_lock::status(&db, &state).apple_watch_enabled,
            ),
            _ => return Err("Unknown system authentication method.".to_string()),
        };
        if !enabled {
            return Err("That unlock method is not enabled.".to_string());
        }
        let window_handle = system_auth_window_handle(&app)?;
        tauri::async_runtime::spawn_blocking(move || {
            crate::app_lock::platform_authenticate(method, window_handle)
        })
        .await
        .map_err(|error| error.to_string())??
    } else {
        crate::app_lock::verify(&db, passphrase.as_deref().unwrap_or_default())?
    };
    if !authenticated {
        if auth_method.is_some() {
            return Err("Authentication canceled.".to_string());
        }
        state.record_failure();
        return Err("The passphrase is incorrect.".to_string());
    }
    state.unlock();
    super::refresh_native_app_menu(&app, &db);
    let status = crate::app_lock::status(&db, &state);
    let _ = app.emit("app-lock-changed", &status);
    Ok(status)
}

async fn set_system_auth_enabled(
    method: crate::app_lock::SystemAuthMethod,
    setting: &str,
    enabled: bool,
    app: &AppHandle,
    db: &DbState,
    state: &crate::app_lock::AppLockState,
) -> Result<crate::app_lock::AppLockStatus, String> {
    features::require(db, Feature::AppLock)?;
    if enabled {
        let window_handle = system_auth_window_handle(app)?;
        let authenticated = tauri::async_runtime::spawn_blocking(move || {
            crate::app_lock::platform_authenticate(method, window_handle)
        })
        .await
        .map_err(|error| error.to_string())??;
        if !authenticated {
            return Err("System authentication was not completed.".to_string());
        }
    }
    crate::app_lock::set_bool_policy(db, setting, enabled)?;
    let status = crate::app_lock::status(db, state);
    let _ = app.emit("app-lock-changed", &status);
    Ok(status)
}

#[tauri::command]
pub async fn set_app_lock_system_auth(
    enabled: bool,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
    state: State<'_, Arc<crate::app_lock::AppLockState>>,
) -> Result<crate::app_lock::AppLockStatus, String> {
    set_system_auth_enabled(
        crate::app_lock::SystemAuthMethod::Primary,
        crate::app_lock::SYSTEM_AUTH_SETTING,
        enabled,
        &app,
        &db,
        &state,
    )
    .await
}

#[tauri::command]
pub async fn set_app_lock_apple_watch(
    enabled: bool,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
    state: State<'_, Arc<crate::app_lock::AppLockState>>,
) -> Result<crate::app_lock::AppLockStatus, String> {
    set_system_auth_enabled(
        crate::app_lock::SystemAuthMethod::AppleWatch,
        crate::app_lock::APPLE_WATCH_SETTING,
        enabled,
        &app,
        &db,
        &state,
    )
    .await
}

fn update_policy(
    app: &AppHandle,
    db: &DbState,
    state: &crate::app_lock::AppLockState,
    setting: &str,
    enabled: bool,
) -> Result<crate::app_lock::AppLockStatus, String> {
    features::require(db, Feature::AppLock)?;
    crate::app_lock::set_bool_policy(db, setting, enabled)?;
    let status = crate::app_lock::status(db, state);
    let _ = app.emit("app-lock-changed", &status);
    Ok(status)
}

#[tauri::command]
pub fn set_app_lock_idle_minutes(
    minutes: u32,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
    state: State<'_, Arc<crate::app_lock::AppLockState>>,
) -> Result<crate::app_lock::AppLockStatus, String> {
    features::require(&db, Feature::AppLock)?;
    crate::app_lock::set_idle_minutes(&db, minutes)?;
    let status = crate::app_lock::status(&db, &state);
    let _ = app.emit("app-lock-changed", &status);
    Ok(status)
}

#[tauri::command]
pub fn set_app_lock_lock_on_sleep(
    enabled: bool,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
    state: State<'_, Arc<crate::app_lock::AppLockState>>,
) -> Result<crate::app_lock::AppLockStatus, String> {
    update_policy(
        &app,
        &db,
        &state,
        crate::app_lock::LOCK_ON_SLEEP_SETTING,
        enabled,
    )
}

#[tauri::command]
pub fn set_app_lock_lock_on_restart(
    enabled: bool,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
    state: State<'_, Arc<crate::app_lock::AppLockState>>,
) -> Result<crate::app_lock::AppLockStatus, String> {
    update_policy(
        &app,
        &db,
        &state,
        crate::app_lock::LOCK_ON_RESTART_SETTING,
        enabled,
    )
}

#[tauri::command]
pub fn set_app_lock_capture_while_locked(
    enabled: bool,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
    state: State<'_, Arc<crate::app_lock::AppLockState>>,
) -> Result<crate::app_lock::AppLockStatus, String> {
    update_policy(
        &app,
        &db,
        &state,
        crate::app_lock::CAPTURE_WHILE_LOCKED_SETTING,
        enabled,
    )
}

#[tauri::command]
pub fn reset_app_lock_policy(
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
    state: State<'_, Arc<crate::app_lock::AppLockState>>,
) -> Result<crate::app_lock::AppLockStatus, String> {
    features::require(&db, Feature::AppLock)?;
    crate::app_lock::reset_policy(&db)?;
    let _ = db.log_activity("settings_changed", "Reset security preferences");
    let status = crate::app_lock::status(&db, &state);
    let _ = app.emit("app-lock-changed", &status);
    Ok(status)
}

#[cfg(test)]
mod tests;
