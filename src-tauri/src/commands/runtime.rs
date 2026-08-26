use std::sync::Arc;

use tauri::AppHandle;

use crate::db::DbState;

pub(crate) fn refresh_native_app_menu(app: &AppHandle, db: &Arc<DbState>) {
    if let Err(error) = crate::app_menu::install(app, db) {
        eprintln!("Could not refresh the native app menu: {error}");
    }
}
