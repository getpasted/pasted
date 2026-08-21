use std::sync::Arc;
use tauri::AppHandle;

use crate::db::DbState;

pub(crate) mod activity;
pub(crate) mod analysis;
pub(crate) mod app_lock;
pub(crate) mod backups;
pub(crate) mod bins;
pub(crate) mod capture;
pub(crate) mod cli_installation;
pub(crate) mod clip_metadata;
pub(crate) mod clip_policies;
pub(crate) mod clipboard;
pub(crate) mod clips;
pub(crate) mod content_registry;
pub(crate) mod extraction;
pub(crate) mod extractors;
pub(crate) mod factory_reset;
pub(crate) mod file_previews;
pub(crate) mod hotkeys;
pub(crate) mod hud;
pub(crate) mod imports;
pub(crate) mod intelligence;
pub(crate) mod library_access;
pub(crate) mod manual_transforms;
pub(crate) mod platform;
pub(crate) mod queue;
pub(crate) mod retention;
pub(crate) mod search_indexes;
pub(crate) mod settings;
pub(crate) mod source_apps;
pub(crate) mod storage;
pub(crate) mod transformations;

pub(crate) use backups::*;
pub(crate) use bins::*;
pub(crate) use clipboard::*;
pub(crate) use extraction::*;
pub(crate) use factory_reset::*;
pub(crate) use hotkeys::*;
pub(crate) use hud::*;
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
