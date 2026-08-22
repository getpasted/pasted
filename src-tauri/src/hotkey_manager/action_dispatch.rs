use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::Shortcut;

use super::{AppHotkeyAction, HotkeyManager};
use crate::db::DbState;
use crate::features::{self, Feature};
use crate::sequential_paste::SequentialQueueState;

impl HotkeyManager {
    pub fn dispatch(&self, app: &AppHandle, shortcut: &Shortcut) {
        let action_opt = {
            let map = self.action_map.read();
            map.get(shortcut).cloned()
        };

        let Some(action) = action_opt else {
            eprintln!(
                "[Pasted Hotkeys] Ignoring unmapped hotkey: key={:?}, modifiers={:?}",
                shortcut.key, shortcut.mods
            );
            return;
        };

        self.dispatch_action(app, action);
    }

    pub(super) fn dispatch_action(&self, app: &AppHandle, action: AppHotkeyAction) {
        if let Some(db) = app.try_state::<Arc<DbState>>() {
            if !features::is_enabled(&db, Feature::Hotkeys) {
                return;
            }
        }
        let lock_state = app.try_state::<Arc<crate::app_lock::AppLockState>>();
        let locked = lock_state.as_ref().is_some_and(|state| state.is_locked());
        if locked {
            return;
        }
        if matches!(&action, AppHotkeyAction::LockApp) {
            let db = app.state::<Arc<DbState>>();
            let state = app.state::<Arc<crate::app_lock::AppLockState>>();
            let status = match crate::app_lock::lock_enabled(&db, &state) {
                Ok(status) => status,
                Err(error) => {
                    eprintln!("[Pasted Hotkeys] Could not lock Pasted: {error}");
                    return;
                }
            };
            crate::hud_window::hide(app);
            let _ = app.emit("app-lock-changed", &status);
            let app_handle = app.clone();
            if let Err(error) = app.run_on_main_thread(move || {
                let db = app_handle.state::<Arc<DbState>>();
                if let Err(error) = crate::app_menu::install(&app_handle, &db) {
                    eprintln!("[Pasted Hotkeys] Could not refresh the locked menu: {error}");
                }
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }) {
                eprintln!("[Pasted Hotkeys] Could not dispatch app-lock hotkey: {error}");
            }
            return;
        }
        if let Some(db) = app.try_state::<Arc<DbState>>() {
            let active_app = crate::paste_target::active_application_name();
            if crate::app_exclusions::should_ignore_hotkeys(&db, active_app.as_deref()) {
                return;
            }
        }
        let app_handle = app.clone();
        let clipboard_action_guard = Arc::clone(&self.clipboard_action_guard);
        if let Err(error) = app.run_on_main_thread(move || match action {
            AppHotkeyAction::ToggleHud => {
                let _ = crate::hud_window::toggle(&app_handle);
            }
            AppHotkeyAction::ToggleMainWindow => {
                if let Some(w) = app_handle.get_webview_window("main") {
                    if w.is_visible().unwrap_or(false) {
                        let _ = w.hide();
                    } else {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
            }
            AppHotkeyAction::LockApp => {}
            AppHotkeyAction::OpenTransformations => {
                if let Some(w) = app_handle.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                    let _ = app_handle.emit("navigate-tab", "transformations");
                }
            }
            AppHotkeyAction::ToggleCopyQueue => {
                let seq = app_handle.state::<Arc<SequentialQueueState>>();
                let db = app_handle.state::<Arc<DbState>>();
                let status = seq.get_status();
                if status.is_active {
                    seq.stop_queue();
                    let _ = db.log_activity(
                        "queue_recording_stopped",
                        "Stopped recording copies into the Queue",
                    );
                } else {
                    seq.start_queue();
                    let _ = db.log_activity(
                        "queue_recording_started",
                        "Started recording copies into the Queue",
                    );
                }
                let updated = seq.get_status();
                let _ = app_handle.emit("sequential-updated", updated);
            }
            AppHotkeyAction::PopCopyQueue => {
                let queue_app = app_handle.clone();
                let action_guard = Arc::clone(&clipboard_action_guard);
                std::thread::spawn(move || {
                    let Some(_execution) = action_guard.try_lock() else {
                        return;
                    };
                    let seq = queue_app.state::<Arc<SequentialQueueState>>();
                    let db = queue_app.state::<Arc<DbState>>();
                    let _ = crate::queue_actions::paste_item(&seq, &db, &queue_app, 0, false);
                });
            }
            AppHotkeyAction::PasteClip(index) => {
                let paste_app = app_handle.clone();
                let action_guard = Arc::clone(&clipboard_action_guard);
                std::thread::spawn(move || {
                    let Some(_execution) = action_guard.try_lock() else {
                        return;
                    };
                    let Some(db) = paste_app.try_state::<Arc<DbState>>() else {
                        return;
                    };
                    let Ok(clips) = db.get_clips_page(
                        None,
                        false,
                        Some(1),
                        Some(index.saturating_sub(1) as i64),
                    ) else {
                        return;
                    };
                    let Some(clip) = clips.first() else {
                        return;
                    };
                    if let Err(error) =
                        crate::clipboard_actions::paste_hud_clip(&db, &paste_app, clip.id)
                    {
                        eprintln!("[Pasted HUD] {error}");
                    }
                });
            }
            AppHotkeyAction::PasteClipById(clip_id) => {
                let paste_app = app_handle.clone();
                let action_guard = Arc::clone(&clipboard_action_guard);
                std::thread::spawn(move || {
                    let Some(_execution) = action_guard.try_lock() else {
                        return;
                    };
                    let Some(db) = paste_app.try_state::<Arc<DbState>>() else {
                        return;
                    };
                    if let Err(error) = crate::clipboard_actions::paste_clip(
                        &db,
                        &paste_app,
                        clip_id,
                        crate::clipboard_actions::PasteOrigin::ClipHotkey,
                    ) {
                        eprintln!("[Pasted Clip Hotkey] {error}");
                    }
                });
            }
            AppHotkeyAction::PasteWithManualTransform(manual_transform_ref) => {
                let transform_app = app_handle.clone();
                let action_guard = Arc::clone(&clipboard_action_guard);
                std::thread::spawn(move || {
                    let Some(_execution) = action_guard.try_lock() else {
                        return;
                    };
                    let Some(db) = transform_app.try_state::<Arc<DbState>>() else {
                        return;
                    };
                    if let Err(error) = crate::clipboard_actions::execute_transform(
                        &db,
                        Some(&manual_transform_ref),
                        true,
                    ) {
                        eprintln!("[Pasted Transform Hotkey] {error}");
                    }
                });
            }
            AppHotkeyAction::CopyWithLastManualTransform => {
                let transform_app = app_handle.clone();
                let action_guard = Arc::clone(&clipboard_action_guard);
                std::thread::spawn(move || {
                    let Some(_execution) = action_guard.try_lock() else {
                        return;
                    };
                    let Some(db) = transform_app.try_state::<Arc<DbState>>() else {
                        return;
                    };
                    if let Err(error) =
                        crate::clipboard_actions::execute_transform(&db, None, false)
                    {
                        eprintln!("[Pasted Last Manual Transform Copy] {error}");
                    }
                });
            }
            AppHotkeyAction::PasteWithLastManualTransform => {
                let transform_app = app_handle.clone();
                let action_guard = Arc::clone(&clipboard_action_guard);
                std::thread::spawn(move || {
                    let Some(_execution) = action_guard.try_lock() else {
                        return;
                    };
                    let Some(db) = transform_app.try_state::<Arc<DbState>>() else {
                        return;
                    };
                    if let Err(error) = crate::clipboard_actions::execute_transform(&db, None, true)
                    {
                        eprintln!("[Pasted Last Manual Transform Paste] {error}");
                    }
                });
            }
            AppHotkeyAction::OpenBin(bin_id) => {
                if let Some(w) = app_handle.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                    let _ = app_handle.emit("navigate-bin", bin_id);
                }
            }
        }) {
            eprintln!("[Pasted Hotkeys] Could not dispatch hotkey action: {error}");
        }
    }
}
