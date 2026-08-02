use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tauri::{AppHandle, Manager, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::db::DbState;
use crate::sequential_paste::SequentialQueueState;
use crate::commands;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppHotkeyAction {
    ToggleHud,
    ToggleMainWindow,
    OpenFilterWindow,
    ToggleCopyQueue,
    PopCopyQueue,
    PasteClip(usize),
    ApplyFilter(i64),
    PasteLastFilter,
    OpenBoard(i64),
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
                    match app.global_shortcut().register(shortcut) {
                        Ok(_) => println!("[Pasted HotkeyManager] Successfully registered '{setting_str}' -> {:?} for action {:?}", shortcut, action),
                        Err(err) => println!("[Pasted HotkeyManager WARNING] Global shortcut registration: '{setting_str}' -> {:?}: {:?}", shortcut, err),
                    }
                    map.insert(shortcut, action.clone());
                }
            } else {
                println!("[Pasted HotkeyManager ERROR] Could not parse shortcut string: '{setting_str}'");
            }
        };

        // 1. HUD Hotkey (default Option+Shift+V)
        let hud_sc = get_setting("hudHotkey", "Alt+Shift+V");
        add_shortcut(hud_sc, AppHotkeyAction::ToggleHud);

        // 2. Toggle Main Window Hotkey
        let main_sc = get_setting("openMainWindowHotkey", "");
        add_shortcut(main_sc, AppHotkeyAction::ToggleMainWindow);

        // 3. Open Filter Window Hotkey
        let filter_sc = get_setting("openFilterWindowHotkey", "");
        add_shortcut(filter_sc, AppHotkeyAction::OpenFilterWindow);

        // 4. Sequential Stack Toggle Hotkey (default Option+Shift+C)
        let seq_toggle_sc = get_setting("seqToggleHotkey", "Alt+Shift+C");
        add_shortcut(seq_toggle_sc, AppHotkeyAction::ToggleCopyQueue);

        // 5. Sequential Stack Pop Hotkey (default Option+Shift+X)
        let seq_pop_sc = get_setting("seqPopHotkey", "Alt+Shift+X");
        add_shortcut(seq_pop_sc, AppHotkeyAction::PopCopyQueue);

        // 6. Paste Recent Clippings Hotkeys (1..=9)
        for i in 1..=9 {
            let key = format!("pasteClip{}Hotkey", i);
            let sc = get_setting(&key, "");
            add_shortcut(sc, AppHotkeyAction::PasteClip(i));
        }

        // 7. Paste Last Filter Hotkey
        let last_filter_sc = get_setting("pasteLastFilterHotkey", "");
        add_shortcut(last_filter_sc, AppHotkeyAction::PasteLastFilter);

        // 8. Filter Pipeline Hotkeys
        if let Ok(filters) = db.get_filters() {
            for f in filters {
                if let Some(sc) = f.shortcut {
                    if !sc.trim().is_empty() {
                        add_shortcut(Some(sc), AppHotkeyAction::ApplyFilter(f.id));
                    }
                }
            }
        }

        // 9. Pasteboard Hotkeys
        if let Ok(boards) = db.get_boards() {
            for b in boards {
                if let Some(sc) = b.shortcut {
                    if !sc.trim().is_empty() {
                        add_shortcut(Some(sc), AppHotkeyAction::OpenBoard(b.id));
                    }
                }
            }
        }

        println!("[Pasted HotkeyManager] Successfully registered {} shortcuts in memory", map.len());
        Ok(())
    }

    pub fn dispatch(&self, app: &AppHandle, shortcut: &Shortcut) {
        let mods = shortcut.mods;
        let is_ctrl_alt = mods.contains(tauri_plugin_global_shortcut::Modifiers::CONTROL)
            && mods.contains(tauri_plugin_global_shortcut::Modifiers::ALT);
        let is_super = mods.contains(tauri_plugin_global_shortcut::Modifiers::SUPER);

        if is_ctrl_alt || is_super {
            let map = self.action_map.read();
            println!(
                "[Pasted HOTKEY DISPATCH] Lookup Received: Key={:?}, Mods={:?}, Id={} against {} registered entries",
                shortcut.key,
                shortcut.mods,
                shortcut.id,
                map.len()
            );
            for (registered_sc, action) in map.iter() {
                println!(
                    "   - Registered in Memory: Key={:?}, Mods={:?}, Id={} -> Action: {:?} (Match: {})",
                    registered_sc.key,
                    registered_sc.mods,
                    registered_sc.id,
                    action,
                    registered_sc == shortcut
                );
            }
        }

        let action_opt = {
            let map = self.action_map.read();
            map.get(shortcut).cloned()
        };

        let Some(action) = action_opt else {
            println!("[Pasted HotkeyManager] Unmapped shortcut pressed: Key={:?}, Mods={:?}, Full={:?}", shortcut.key, shortcut.mods, shortcut);
            return;
        };

        println!("[Pasted HotkeyManager] Executing action on main thread: {:?}", action);

        let app_handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            match action {
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
                AppHotkeyAction::OpenFilterWindow => {
                    if let Some(w) = app_handle.get_webview_window("main") {
                        let _ = w.show();
                        let _ = w.set_focus();
                        let _ = app_handle.emit("navigate-tab", "filters");
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
                                    if let Some(ref t) = clip.text_content {
                                        let _ = cb.set_text(t);
                                    } else if let Some(ref img_b64) = clip.image_base64 {
                                        let _ = cb.set_text(img_b64);
                                    }
                                    std::thread::spawn(move || {
                                        std::thread::sleep(std::time::Duration::from_millis(50));
                                        commands::simulate_cmd_v_paste();
                                    });
                                }
                            }
                        }
                    }
                }
                AppHotkeyAction::ApplyFilter(filter_id) => {
                    let db_opt = app_handle.try_state::<Arc<DbState>>();
                    if let Some(db) = db_opt {
                        if let Ok(filters) = db.get_filters() {
                            if let Some(f) = filters.into_iter().find(|flt| flt.id == filter_id) {
                                if let Ok(mut cb) = arboard::Clipboard::new() {
                                    if let Ok(text) = cb.get_text() {
                                        if let Ok(transformed) = crate::filter_engine::apply_filter(&text, &f.filter_type, f.config.as_deref()) {
                                            let _ = cb.set_text(&transformed);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                AppHotkeyAction::PasteLastFilter => {
                    let _ = app_handle.emit("paste-last-filter-requested", ());
                }
                AppHotkeyAction::OpenBoard(board_id) => {
                    if let Some(w) = app_handle.get_webview_window("main") {
                        let _ = w.show();
                        let _ = w.set_focus();
                        let _ = app_handle.emit("navigate-board", board_id);
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotkey_manager_map_operations() {
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
