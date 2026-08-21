use std::sync::Arc;
use tauri::{AppHandle, State};

use crate::db::{Bin, DbState};
use crate::features::{self, Feature};

use super::{refresh_native_app_menu, register_all_app_shortcuts, register_changed_hotkeys};

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
