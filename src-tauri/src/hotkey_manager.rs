use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::commands;
use crate::db::DbState;
use crate::sequential_paste::SequentialQueueState;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppHotkeyAction {
    ToggleHud,
    ToggleMainWindow,
    OpenTransformations,
    ToggleCopyQueue,
    PopCopyQueue,
    PasteClip(usize),
    PasteWithPipeline(String),
    CopyWithLastPipeline,
    PasteWithLastPipeline,
    OpenBin(i64),
}

pub struct HotkeyManager {
    action_map: RwLock<HashMap<Shortcut, AppHotkeyAction>>,
}

impl HotkeyManager {
    pub fn new() -> Self {
        Self {
            action_map: RwLock::new(HashMap::new()),
        }
    }

    pub fn register_all(&self, app: &AppHandle) -> Result<(), String> {
        let _ = app.global_shortcut().unregister_all();
        let mut map = self.action_map.write();
        map.clear();

        let db_opt = app.try_state::<Arc<DbState>>();
        let Some(db) = db_opt else {
            return Err("Database state not initialized".to_string());
        };

        let get_setting = |key: &str, default_val: &str| -> Option<String> {
            match db.get_setting(key) {
                Ok(Some(s)) => {
                    let trimmed = s.trim().to_string();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed)
                    }
                }
                _ => {
                    if default_val.trim().is_empty() {
                        None
                    } else {
                        Some(default_val.to_string())
                    }
                }
            }
        };

        let mut add_shortcut = |setting_str_opt: Option<String>, action: AppHotkeyAction| {
            let Some(setting_str) = setting_str_opt else {
                return;
            };
            if let Some(shortcuts) = commands::parse_shortcut_str_all_layouts(&setting_str) {
                for shortcut in shortcuts {
                    if let Err(error) = app.global_shortcut().register(shortcut) {
                        eprintln!(
                            "[Pasted Hotkeys] Could not register '{setting_str}' ({shortcut:?}): {error}"
                        );
                    }
                    map.insert(shortcut, action.clone());
                }
            } else {
                eprintln!("[Pasted Hotkeys] Could not parse shortcut setting: '{setting_str}'");
            }
        };

        // HUD shortcut (default Option+Shift+V)
        let hud_sc = get_setting("hudHotkey", "Alt+Shift+V");
        add_shortcut(hud_sc, AppHotkeyAction::ToggleHud);

        // Main window shortcut
        let main_sc = get_setting("openMainWindowHotkey", "");
        add_shortcut(main_sc, AppHotkeyAction::ToggleMainWindow);

        // Transformations shortcut
        let transformations_sc = get_setting("openTransformationsHotkey", "");
        add_shortcut(transformations_sc, AppHotkeyAction::OpenTransformations);

        // Sequential Stack toggle (default Option+Shift+C)
        let seq_toggle_sc = get_setting("seqToggleHotkey", "Alt+Shift+C");
        add_shortcut(seq_toggle_sc, AppHotkeyAction::ToggleCopyQueue);

        // Sequential Stack pop (default Option+Shift+X)
        let seq_pop_sc = get_setting("seqPopHotkey", "Alt+Shift+X");
        add_shortcut(seq_pop_sc, AppHotkeyAction::PopCopyQueue);

        // Recent clip shortcuts
        for i in 1..=9 {
            let key = format!("pasteClip{}Hotkey", i);
            let sc = get_setting(&key, "");
            add_shortcut(sc, AppHotkeyAction::PasteClip(i));
        }

        // Last-Pipeline shortcuts
        let copy_last_pipeline_sc = get_setting("copyLastPipelineHotkey", "");
        add_shortcut(copy_last_pipeline_sc, AppHotkeyAction::CopyWithLastPipeline);
        let paste_last_pipeline_sc = get_setting("pasteLastPipelineHotkey", "");
        add_shortcut(
            paste_last_pipeline_sc,
            AppHotkeyAction::PasteWithLastPipeline,
        );

        // Per-Pipeline shortcuts
        if let Ok(pipelines) = db.get_pipelines() {
            for pipeline in pipelines {
                if let Some(sc) = pipeline.shortcut {
                    if !sc.trim().is_empty() {
                        add_shortcut(
                            Some(sc),
                            AppHotkeyAction::PasteWithPipeline(pipeline.stable_ref),
                        );
                    }
                }
            }
        }

        // Bin shortcuts
        if let Ok(bins) = db.get_bins() {
            for b in bins {
                if let Some(sc) = b.shortcut {
                    if !sc.trim().is_empty() {
                        add_shortcut(Some(sc), AppHotkeyAction::OpenBin(b.id));
                    }
                }
            }
        }

        Ok(())
    }

    pub fn dispatch(&self, app: &AppHandle, shortcut: &Shortcut) {
        let action_opt = {
            let map = self.action_map.read();
            map.get(shortcut).cloned()
        };

        let Some(action) = action_opt else {
            eprintln!(
                "[Pasted Hotkeys] Ignoring unmapped shortcut: key={:?}, modifiers={:?}",
                shortcut.key, shortcut.mods
            );
            return;
        };

        let app_handle = app.clone();
        if let Err(error) = app.run_on_main_thread(move || match action {
            AppHotkeyAction::ToggleHud => {
                let _ = commands::toggle_hud_window(app_handle.clone());
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
            AppHotkeyAction::OpenTransformations => {
                if let Some(w) = app_handle.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                    let _ = app_handle.emit("navigate-tab", "transformations");
                }
            }
            AppHotkeyAction::ToggleCopyQueue => {
                let seq = app_handle.state::<Arc<SequentialQueueState>>();
                let status = seq.get_status();
                if status.is_active {
                    seq.stop_queue();
                } else {
                    seq.start_queue();
                }
                let updated = seq.get_status();
                let _ = app_handle.emit("sequential-updated", updated);
            }
            AppHotkeyAction::PopCopyQueue => {
                let seq = app_handle.state::<Arc<SequentialQueueState>>();
                if let Some(item) = seq.pop_next() {
                    if let Ok(mut cb) = arboard::Clipboard::new() {
                        let _ = cb.set_text(&item);
                    }
                    let updated = seq.get_status();
                    let _ = app_handle.emit("sequential-updated", updated);
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        commands::simulate_cmd_v_paste();
                    });
                }
            }
            AppHotkeyAction::PasteClip(index) => {
                let db_opt = app_handle.try_state::<Arc<DbState>>();
                if let Some(db) = db_opt {
                    if let Ok(clips) = db.get_clips(None, None, false) {
                        if let Some(clip) = clips.get(index - 1) {
                            if let Ok(mut cb) = arboard::Clipboard::new() {
                                let full_clip =
                                    db.get_clip_by_id(clip.id).unwrap_or_else(|_| clip.clone());
                                if commands::write_clip_to_clipboard(&mut cb, &full_clip).is_ok() {
                                    std::thread::spawn(move || {
                                        std::thread::sleep(std::time::Duration::from_millis(50));
                                        commands::simulate_cmd_v_paste();
                                    });
                                }
                            }
                        }
                    }
                }
            }
            AppHotkeyAction::PasteWithPipeline(pipeline_ref) => {
                let db_opt = app_handle.try_state::<Arc<DbState>>();
                if let Some(db) = db_opt {
                    if let Err(error) =
                        commands::execute_clipboard_pipeline(&db, Some(&pipeline_ref), true)
                    {
                        eprintln!("[Pasted Pipeline Shortcut] {error}");
                    }
                }
            }
            AppHotkeyAction::CopyWithLastPipeline => {
                let db_opt = app_handle.try_state::<Arc<DbState>>();
                if let Some(db) = db_opt {
                    if let Err(error) = commands::execute_clipboard_pipeline(&db, None, false) {
                        eprintln!("[Pasted Last Pipeline Copy] {error}");
                    }
                }
            }
            AppHotkeyAction::PasteWithLastPipeline => {
                let db_opt = app_handle.try_state::<Arc<DbState>>();
                if let Some(db) = db_opt {
                    if let Err(error) = commands::execute_clipboard_pipeline(&db, None, true) {
                        eprintln!("[Pasted Last Pipeline Paste] {error}");
                    }
                }
            }
            AppHotkeyAction::OpenBin(bin_id) => {
                if let Some(w) = app_handle.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                    let _ = app_handle.emit("navigate-bin", bin_id);
                }
            }
        }) {
            eprintln!("[Pasted Hotkeys] Could not dispatch shortcut action: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotkey_manager_maps_actions() {
        let mgr = HotkeyManager::new();
        let sc = commands::parse_shortcut_str("CmdOrCtrl+Shift+V").unwrap();

        {
            let mut map = mgr.action_map.write();
            map.insert(sc, AppHotkeyAction::ToggleHud);
        }

        {
            let map = mgr.action_map.read();
            assert_eq!(map.get(&sc), Some(&AppHotkeyAction::ToggleHud));
        }

        {
            let mut map = mgr.action_map.write();
            map.clear();
            assert_eq!(map.get(&sc), None);
        }
    }
}
