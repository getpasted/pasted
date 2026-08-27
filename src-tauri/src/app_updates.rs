use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::db::DbState;
use crate::features::{self, Feature};
use crate::update_manifest::{channel_for_version, UpdateChannel};

pub struct PendingUpdate(pub Mutex<Option<Update>>);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateStatus {
    pub configured: bool,
    pub enabled: bool,
    pub current_version: String,
    pub channel: UpdateChannel,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableAppUpdate {
    pub current_version: String,
    pub channel: UpdateChannel,
    pub available: bool,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
}

pub fn updater_public_key() -> &'static str {
    option_env!("PASTED_UPDATER_PUBLIC_KEY").unwrap_or("updater-public-key-not-configured")
}

pub fn updater_is_configured() -> bool {
    option_env!("PASTED_UPDATER_PUBLIC_KEY").is_some()
}

#[tauri::command]
pub fn get_app_update_status(
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<AppUpdateStatus, String> {
    let current_version = app.package_info().version.to_string();
    Ok(AppUpdateStatus {
        configured: updater_is_configured(),
        enabled: features::is_enabled(&db, Feature::Updates),
        channel: channel_for_version(&current_version)?,
        current_version,
    })
}

#[tauri::command]
pub async fn check_for_app_update(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
    db: State<'_, Arc<DbState>>,
) -> Result<AvailableAppUpdate, String> {
    features::require(&db, Feature::Updates)?;
    if !updater_is_configured() {
        return Err("Automatic updates are unavailable in this build".to_string());
    }
    let current_version = app.package_info().version.to_string();
    let channel = channel_for_version(&current_version)?;
    let endpoint = channel
        .feed_url()
        .parse()
        .map_err(|error| format!("Invalid update endpoint: {error}"))?;
    let updater = app
        .updater_builder()
        .endpoints(vec![endpoint])
        .map_err(|error| error.to_string())?
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| error.to_string())?;
    let update = updater.check().await.map_err(|error| error.to_string())?;
    let report = match update.as_ref() {
        Some(update) => AvailableAppUpdate {
            current_version,
            channel,
            available: true,
            version: Some(update.version.clone()),
            notes: update.body.clone(),
            pub_date: update.date.map(|date| date.to_string()),
        },
        None => AvailableAppUpdate {
            current_version,
            channel,
            available: false,
            version: None,
            notes: None,
            pub_date: None,
        },
    };
    *pending
        .0
        .lock()
        .map_err(|_| "Update state is unavailable")? = update;
    Ok(report)
}

#[tauri::command]
pub async fn install_app_update(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    features::require(&db, Feature::Updates)?;
    let update = pending
        .0
        .lock()
        .map_err(|_| "Update state is unavailable")?
        .take()
        .ok_or_else(|| "Check for an update before installing".to_string())?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|error| error.to_string())?;
    app.restart();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn development_builds_do_not_claim_a_release_key() {
        if option_env!("PASTED_UPDATER_PUBLIC_KEY").is_none() {
            assert!(!updater_is_configured());
            assert_eq!(updater_public_key(), "updater-public-key-not-configured");
        }
    }
}
