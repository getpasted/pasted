use arboard::Clipboard;
use base64::Engine;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::{Board, ClipItem, DbState, FilterRule};
use crate::filter_engine::apply_filter;
use crate::sequential_paste::{SequentialQueueState, SequentialStatus};

#[tauri::command]
pub fn get_clips(
    search_query: Option<String>,
    board_id: Option<i64>,
    only_pinned: bool,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<ClipItem>, String> {
    db.get_clips(search_query.as_deref(), board_id, only_pinned)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_total_clip_count(db: State<'_, Arc<DbState>>) -> Result<i64, String> {
    db.get_total_clip_count().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_clip_image(db: State<'_, Arc<DbState>>, id: i64) -> Result<Option<String>, String> {
    db.get_clip_image(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_trashed_clips(db: State<'_, Arc<DbState>>) -> Result<Vec<ClipItem>, String> {
    db.get_trashed_clips().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn restore_clip(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.restore_clip(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn purge_clip_permanently(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.purge_clip_permanently(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn empty_trash(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.empty_trash().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_activity_logs(
    limit: Option<i64>,
    offset: Option<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::db::ActivityLog>, String> {
    db.get_activity_logs(limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_activity_logs(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.clear_activity_logs().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_app_setting(
    key: String,
    value: String,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.save_setting(&key, &value).map_err(|e| e.to_string())
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
pub fn get_app_setting(
    key: String,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<String>, String> {
    db.get_setting(&key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_all_app_settings(
    db: State<'_, Arc<DbState>>,
) -> Result<std::collections::HashMap<String, String>, String> {
    db.get_all_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn enforce_clip_retention(
    keep_count: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.purge_old_clips(keep_count).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_clip_note(
    clip_id: i64,
    note: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.update_clip_note(clip_id, note.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_clip(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.delete_clip(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_pin_clip(id: i64, db: State<'_, Arc<DbState>>) -> Result<bool, String> {
    db.toggle_pin(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn assign_clip_board(
    clip_id: i64,
    board_id: Option<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.assign_to_board(clip_id, board_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_clip_to_board(
    clip_id: i64,
    board_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.add_clip_to_board(clip_id, board_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_clip_from_board(
    clip_id: i64,
    board_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.remove_clip_from_board(clip_id, board_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn copy_clip_to_system(
    text: Option<String>,
    image_base64: Option<String>,
) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;

    if let Some(t) = text {
        clipboard.set_text(t).map_err(|e| e.to_string())?;
    } else if let Some(img_b64) = image_base64 {
        // Strip data:image/png;base64,
        let clean = img_b64.split(',').next_back().unwrap_or(&img_b64);
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
    }

    Ok(())
}

#[tauri::command]
pub fn get_boards(db: State<'_, Arc<DbState>>) -> Result<Vec<Board>, String> {
    db.get_boards().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_board(
    name: String,
    icon: String,
    color: String,
    smart_rule: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<Board, String> {
    db.create_board(&name, &icon, &color, smart_rule.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_board(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.delete_board(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_board(
    id: i64,
    name: String,
    icon: String,
    color: String,
    smart_rule: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.update_board(id, &name, &icon, &color, smart_rule.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_filters(db: State<'_, Arc<DbState>>) -> Result<Vec<FilterRule>, String> {
    db.get_filters().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_filter(
    name: String,
    filter_type: String,
    config: Option<String>,
    shortcut: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<FilterRule, String> {
    db.create_filter(&name, &filter_type, config.as_deref(), shortcut.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_filter_shortcut(
    id: i64,
    shortcut: Option<String>,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<(), String> {
    db.update_filter_shortcut(id, shortcut.as_deref())
        .map_err(|e| e.to_string())?;
    let _ = register_all_app_shortcuts(&app);
    Ok(())
}

#[tauri::command]
pub fn update_board_shortcut(
    id: i64,
    shortcut: Option<String>,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<(), String> {
    db.update_board_shortcut(id, shortcut.as_deref())
        .map_err(|e| e.to_string())?;
    let _ = register_all_app_shortcuts(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_filter(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.delete_filter(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_operations(db: State<'_, Arc<DbState>>) -> Result<Vec<crate::db::Operation>, String> {
    db.get_operations().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_operation(
    name: String,
    op_type: String,
    config: Option<String>,
    category: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::db::Operation, String> {
    db.create_operation(&name, &op_type, config.as_deref(), category.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_operation(
    id: i64,
    name: String,
    op_type: String,
    config: Option<String>,
    category: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.update_operation(id, &name, &op_type, config.as_deref(), category.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_operation(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.delete_operation(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn transform_text(
    input: String,
    filter_type: String,
    config: Option<String>,
) -> Result<String, String> {
    apply_filter(&input, &filter_type, config.as_deref())
}

#[tauri::command]
pub fn clear_history(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.clear_history().map_err(|e| e.to_string())
}

// Sequential Paste Commands
#[tauri::command]
pub fn start_sequential_paste(
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<SequentialStatus, String> {
    seq.start_queue();
    let status = seq.get_status();
    let _ = app.emit("sequential-updated", status.clone());
    Ok(status)
}

#[tauri::command]
pub fn push_sequential_item(
    item: String,
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<SequentialStatus, String> {
    seq.push_item(item);
    let status = seq.get_status();
    let _ = app.emit("sequential-updated", status.clone());
    Ok(status)
}

#[cfg(target_os = "macos")]
pub fn simulate_cmd_v_paste() {
    use std::process::Command;
    let _ = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to keystroke \"v\" using command down")
        .spawn();
}

#[cfg(target_os = "windows")]
pub fn simulate_cmd_v_paste() {
    use std::process::Command;
    let _ = Command::new("powershell")
        .arg("-Command")
        .arg("$wshell = New-Object -ComObject wscript.shell; $wshell.SendKeys('^v')")
        .spawn();
}

#[cfg(target_os = "linux")]
pub fn simulate_cmd_v_paste() {
    use std::process::Command;
    let _ = Command::new("xdotool")
        .arg("key")
        .arg("ctrl+v")
        .spawn();
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub fn simulate_cmd_v_paste() {}

#[tauri::command]
pub fn pop_sequential_paste(
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<Option<String>, String> {
    let item = seq.pop_next();
    if let Some(ref text) = item {
        if let Ok(mut cb) = Clipboard::new() {
            let _ = cb.set_text(text);
        }

        // Hide main window if visible so focus returns to target app
        if let Some(main_win) = app.get_webview_window("main") {
            if main_win.is_visible().unwrap_or(false) {
                let _ = main_win.hide();
            }
        }

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            simulate_cmd_v_paste();
        });
    }
    let status = seq.get_status();
    let _ = app.emit("sequential-updated", status);
    Ok(item)
}

#[tauri::command]
pub fn remove_sequential_item_by_index(
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
    index: usize,
) -> Result<SequentialStatus, String> {
    let _ = seq.remove_item_by_index(index);
    let status = seq.get_status();
    let _ = app.emit("sequential-updated", status.clone());
    Ok(status)
}

#[tauri::command]
pub fn stop_sequential_paste(
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<SequentialStatus, String> {
    seq.stop_queue();
    let status = seq.get_status();
    let _ = app.emit("sequential-updated", status.clone());
    Ok(status)
}

#[tauri::command]
pub fn paste_all_sequential(
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<Option<String>, String> {
    let status = seq.get_status();
    if status.queue.is_empty() {
        return Ok(None);
    }
    let combined = status.queue.join("\n\n");
    if let Ok(mut cb) = Clipboard::new() {
        let _ = cb.set_text(&combined);
    }
    seq.stop_queue();
    let updated = seq.get_status();
    let _ = app.emit("sequential-updated", updated);

    // Hide main window if visible so focus returns to target app
    if let Some(main_win) = app.get_webview_window("main") {
        if main_win.is_visible().unwrap_or(false) {
            let _ = main_win.hide();
        }
    }

    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        simulate_cmd_v_paste();
    });

    Ok(Some(combined))
}

#[tauri::command]
pub fn get_sequential_status(
    seq: State<'_, Arc<SequentialQueueState>>,
) -> Result<SequentialStatus, String> {
    Ok(seq.get_status())
}

// Window & Activation Policy Commands
#[tauri::command]
pub fn toggle_quick_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
    Ok(())
}

#[tauri::command]
pub fn toggle_hud_window(app: AppHandle) -> Result<(), String> {
    println!("[Pasted HUD] toggle_hud_window invoked!");
    if let Some(window) = app.get_webview_window("hud") {
        let is_vis = window.is_visible().unwrap_or(false);
        println!("[Pasted HUD] Window 'hud' found! Currently visible: {}", is_vis);
        if is_vis {
            let _ = window.hide();
            println!("[Pasted HUD] Hidden HUD window.");
        } else {
            let mut pos_payload = None;

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
                                let first_screen: *mut Object = msg_send![screens_array, objectAtIndex: 0usize];
                                let first_frame: LocalRect = msg_send![first_screen, frame];
                                primary_height = first_frame.size.height;
                            }

                            for i in 0..screen_count {
                                let screen: *mut Object = msg_send![screens_array, objectAtIndex: i];
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

                            let active_screen = target_screen.unwrap_or_else(|| {
                                msg_send![screens_cls, mainScreen]
                            });

                            if !active_screen.is_null() {
                                let vis_frame: LocalRect = msg_send![active_screen, visibleFrame];

                                let mouse_top_y = primary_height - loc.y;
                                let vis_top = primary_height - (vis_frame.origin.y + vis_frame.size.height);
                                let vis_bottom = primary_height - vis_frame.origin.y;
                                let vis_left = vis_frame.origin.x;
                                let vis_right = vis_frame.origin.x + vis_frame.size.width;

                                let hud_width = 360.0;
                                let hud_height = 440.0;

                                // Horizontal positioning (centered on cursor) & clamping
                                let mut target_x = loc.x - (hud_width / 2.0);
                                target_x = target_x.clamp(vis_left + 8.0, (vis_right - hud_width - 8.0).max(vis_left + 8.0));

                                // Vertical positioning & dynamic flip if near bottom edge
                                let mut target_y = mouse_top_y + 8.0;
                                if target_y + hud_height > vis_bottom - 8.0 {
                                    target_y = mouse_top_y - hud_height - 8.0;
                                }
                                target_y = target_y.clamp(vis_top + 8.0, (vis_bottom - hud_height - 8.0).max(vis_top + 8.0));

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

                                println!(
                                    "[Pasted HUD] Smart positioning: target_x={}, target_y={} (Flipped: {})",
                                    target_x, target_y, is_flipped
                                );

                                if let Ok(ns_win_ptr) = window.ns_window() {
                                    let ns_win = ns_win_ptr as *mut Object;
                                    let _: () = msg_send![ns_win, setHasShadow: 0i8];
                                    let _: () = msg_send![ns_win, setAlphaValue: 0.0f64];
                                    let cocoa_y = primary_height - target_y - hud_height;
                                    let origin = LocalPoint { x: target_x, y: cocoa_y };
                                    let _: () = msg_send![ns_win, setFrameOrigin: origin];
                                }

                                let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
                                    x: target_x,
                                    y: target_y,
                                }));
                            }
                        }
                    }
                }
            }

            let _ = window.show();
            let _ = window.set_focus();
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
            println!("[Pasted HUD] Successfully showed and focused HUD window!");
        }
    } else {
        println!("[Pasted HUD] Could not find window 'hud'");
    }
    Ok(())
}

#[tauri::command]
pub fn paste_clip_by_id(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<(), String> {
    let clips = db.get_clips(None, None, false).map_err(|e| e.to_string())?;
    if let Some(clip) = clips.into_iter().find(|c| c.id == clip_id) {
        let mut cb = Clipboard::new().map_err(|e| e.to_string())?;
        if let Some(txt) = &clip.text_content {
            let _ = cb.set_text(txt);
        } else if let Some(b64) = &clip.image_base64 {
            let _ = cb.set_text(b64);
        }

        if let Some(hud) = app.get_webview_window("hud") {
            let _ = hud.hide();
        }
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.hide();
        }

        thread::sleep(Duration::from_millis(50));
        simulate_cmd_v_paste();
    }
    Ok(())
}

#[tauri::command]
pub fn get_protected_clips(db: State<'_, Arc<DbState>>) -> Result<Vec<ClipItem>, String> {
    db.get_protected_clips().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_clip_protected(clip_id: i64, db: State<'_, Arc<DbState>>) -> Result<bool, String> {
    db.toggle_protected(clip_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn trash_unpinned_clips(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.trash_unpinned_clips().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn purge_unpinned_clips(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.purge_unpinned_clips().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_all_clips(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.clear_all_clips().map_err(|e| e.to_string())
}

fn get_dvorak_code_for_char(ch: char) -> Option<tauri_plugin_global_shortcut::Code> {
    use tauri_plugin_global_shortcut::Code;

    match ch.to_ascii_uppercase() {
        'A' => Some(Code::KeyA),
        'B' => Some(Code::KeyN),
        'C' => Some(Code::KeyI),
        'D' => Some(Code::KeyH),
        'E' => Some(Code::KeyD),
        'F' => Some(Code::KeyW),
        'G' => Some(Code::KeyE),
        'H' => Some(Code::KeyJ),
        'I' => Some(Code::KeyG),
        'J' => Some(Code::KeyP),
        'K' => Some(Code::BracketLeft),
        'L' => Some(Code::KeyU),
        'M' => Some(Code::KeyM),
        'N' => Some(Code::KeyL),
        'O' => Some(Code::KeyS),
        'P' => Some(Code::KeyR),
        'Q' => Some(Code::KeyO),
        'R' => Some(Code::KeyY),
        'S' => Some(Code::Semicolon),
        'T' => Some(Code::KeyK),
        'U' => Some(Code::KeyF),
        'V' => Some(Code::Period),
        'W' => Some(Code::Comma),
        'X' => Some(Code::KeyQ),
        'Y' => Some(Code::KeyT),
        'Z' => Some(Code::Slash),
        '1' => Some(Code::Digit1),
        '2' => Some(Code::Digit2),
        '3' => Some(Code::Digit3),
        '4' => Some(Code::Digit4),
        '5' => Some(Code::Digit5),
        '6' => Some(Code::Digit6),
        '7' => Some(Code::Digit7),
        '8' => Some(Code::Digit8),
        '9' => Some(Code::Digit9),
        '0' => Some(Code::Digit0),
        '`' => Some(Code::Backquote),
        _ => None,
    }
}

pub fn parse_shortcut_str(sc_str: &str) -> Option<tauri_plugin_global_shortcut::Shortcut> {
    use std::str::FromStr;
    use tauri_plugin_global_shortcut::Shortcut;

    let s = sc_str.trim();
    if s.is_empty() {
        return None;
    }

    if let Ok(sc) = Shortcut::from_str(s) {
        return Some(sc);
    }

    let mut clean = s
        .replace("CmdOrCtrl", "Super")
        .replace("Command", "Super")
        .replace("Cmd", "Super")
        .replace("Option", "Alt")
        .replace("Control", "Ctrl");

    clean = clean
        .replace('ç', "C").replace('Ç', "C")
        .replace('√', "V").replace('◊', "V")
        .replace('µ', "M").replace('Â', "M")
        .replace('≈', "X")
        .replace('ß', "S")
        .replace('∂', "D")
        .replace('ƒ', "F")
        .replace('©', "G")
        .replace('®', "R")
        .replace('†', "T")
        .replace('¥', "Y")
        .replace('ø', "O").replace('Ø', "O")
        .replace('π', "P").replace('∏', "P")
        .replace('å', "A").replace('Å', "A")
        .replace('∫', "B")
        .replace('∆', "J")
        .replace('˚', "K")
        .replace('¬', "L")
        .replace('Ω', "Z")
        .replace('œ', "Q")
        .replace('∑', "W");

    if let Ok(sc) = Shortcut::from_str(&clean) {
        return Some(sc);
    }

    let parts: Vec<&str> = clean.split('+').collect();
    if let Some(last) = parts.last() {
        let last_trim = last.trim();
        if last_trim.len() == 1 && last_trim.chars().next().unwrap().is_ascii_alphabetic() {
            let key_str = format!("Key{}", last_trim.to_ascii_uppercase());
            let converted = format!("{}+{}", parts[..parts.len() - 1].join("+"), key_str);
            if let Ok(sc) = Shortcut::from_str(&converted) {
                return Some(sc);
            }
        }
        if last_trim.len() == 1 && last_trim.chars().next().unwrap().is_ascii_digit() {
            let key_str = format!("Digit{}", last_trim);
            let converted = format!("{}+{}", parts[..parts.len() - 1].join("+"), key_str);
            if let Ok(sc) = Shortcut::from_str(&converted) {
                return Some(sc);
            }
        }
    }

    None
}

pub fn parse_shortcut_str_all_layouts(sc_str: &str) -> Option<Vec<tauri_plugin_global_shortcut::Shortcut>> {
    use tauri_plugin_global_shortcut::{Shortcut, Modifiers};

    let s = sc_str.trim();
    if s.is_empty() {
        return None;
    }

    let mut clean = s
        .replace("CmdOrCtrl", "Super")
        .replace("Command", "Super")
        .replace("Cmd", "Super")
        .replace("Option", "Alt")
        .replace("Control", "Ctrl");

    clean = clean
        .replace('ç', "C").replace('Ç', "C")
        .replace('√', "V").replace('◊', "V")
        .replace('µ', "M").replace('Â', "M")
        .replace('≈', "X")
        .replace('ß', "S")
        .replace('∂', "D")
        .replace('ƒ', "F")
        .replace('©', "G")
        .replace('®', "R")
        .replace('†', "T")
        .replace('¥', "Y")
        .replace('ø', "O").replace('Ø', "O")
        .replace('π', "P").replace('∏', "P")
        .replace('å', "A").replace('Å', "A")
        .replace('∫', "B")
        .replace('∆', "J")
        .replace('˚', "K")
        .replace('¬', "L")
        .replace('Ω', "Z")
        .replace('œ', "Q")
        .replace('∑', "W");

    let mut shortcuts = Vec::new();

    if let Some(sc) = parse_shortcut_str(&clean) {
        shortcuts.push(sc);
    }

    let parts: Vec<&str> = clean.split('+').collect();
    if let Some(last) = parts.last() {
        let last_trim = last.trim();
        if last_trim.len() == 1 {
            let ch = last_trim.chars().next().unwrap();
            let mut mods = Modifiers::empty();
            for m in &parts[..parts.len() - 1] {
                match m.trim() {
                    "Super" => mods |= Modifiers::SUPER,
                    "Alt" => mods |= Modifiers::ALT,
                    "Ctrl" => mods |= Modifiers::CONTROL,
                    "Shift" => mods |= Modifiers::SHIFT,
                    _ => {}
                }
            }

            if let Some(dvorak_code) = get_dvorak_code_for_char(ch) {
                let dvorak_sc = Shortcut::new(Some(mods), dvorak_code);
                if !shortcuts.contains(&dvorak_sc) {
                    shortcuts.push(dvorak_sc);
                }
            }
        }
    }

    if shortcuts.is_empty() {
        None
    } else {
        Some(shortcuts)
    }
}

#[allow(dead_code)]
fn try_register_shortcut(app: &AppHandle, sc_str: &str) {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    if let Some(shortcut) = parse_shortcut_str(sc_str) {
        match app.global_shortcut().register(shortcut) {
            Ok(_) => println!("[Pasted Shortcut Register Success] Registered '{}' -> {:?}", sc_str, shortcut),
            Err(e) => eprintln!("[Pasted Shortcut Register Error] Failed to register '{}' -> {:?}", sc_str, e),
        }
    } else {
        eprintln!("[Pasted Shortcut Parse Error] Could not parse shortcut string: '{}'", sc_str);
    }
}

pub fn register_all_app_shortcuts(app: &AppHandle) -> Result<(), String> {
    if let Some(mgr) = app.try_state::<Arc<crate::hotkey_manager::HotkeyManager>>() {
        mgr.register_all(app)
    } else {
        Err("HotkeyManager state not initialized".to_string())
    }
}

#[derive(serde::Serialize)]
pub struct AccessibilityStatus {
    pub is_trusted: bool,
    pub is_dev_mode: bool,
}

#[tauri::command]
pub fn check_accessibility_permission() -> AccessibilityStatus {
    let is_trusted = {
        #[cfg(target_os = "macos")]
        {
            use std::ptr;
            #[link(name = "ApplicationServices", kind = "framework")]
            extern "C" {
                fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
            }
            unsafe { AXIsProcessTrustedWithOptions(ptr::null()) }
        }
        #[cfg(not(target_os = "macos"))]
        true
    };

    let is_dev_mode = cfg!(debug_assertions);

    AccessibilityStatus {
        is_trusted,
        is_dev_mode,
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
        let _ = Command::new("gnome-control-center")
            .spawn();
        true
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    true
}

#[tauri::command]
pub fn register_app_setting_hotkey(key: String, value: String, app: AppHandle) -> Result<(), String> {
    let db = app.state::<Arc<DbState>>();
    let _ = db.save_setting(&key, &value);
    register_all_app_shortcuts(&app)
}

#[tauri::command]
pub fn register_hud_shortcut(shortcut_str: String, app: AppHandle) -> Result<(), String> {
    let db = app.state::<Arc<DbState>>();
    let _ = db.save_setting("hudHotkey", &shortcut_str);
    register_all_app_shortcuts(&app)
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
pub fn open_emoji_picker() {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg("tell application \"System Events\" to keystroke \" \" using {control down, command down}")
            .spawn();
    }
}

#[tauri::command]
pub fn get_installed_applications(db: State<'_, Arc<DbState>>) -> Result<Vec<String>, String> {
    let mut apps = std::collections::BTreeSet::new();

    if let Ok(history_apps) = db.get_distinct_source_apps() {
        for app in history_apps {
            if !app.trim().is_empty() {
                apps.insert(app);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let dirs = ["/Applications", "/System/Applications", "/System/Applications/Utilities"];
        for dir in &dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "app") {
                        if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                            apps.insert(name.to_string());
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let dirs = ["/usr/share/applications", "/usr/local/share/applications"];
        for dir in &dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |ext| ext == "desktop") {
                        if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                            let clean_name = name.trim_end_matches(".desktop");
                            apps.insert(clean_name.to_string());
                        }
                    }
                }
            }
        }
    }

    let common = [
        "1Password", "Bitwarden", "Safari", "Google Chrome", "Firefox", "Slack",
        "Signal", "Telegram", "VS Code", "Terminal", "Warp", "Xcode", "Discord",
        "Keychain Access", "Passwords"
    ];
    for c in &common {
        apps.insert(c.to_string());
    }

    Ok(apps.into_iter().collect())
}

#[tauri::command]
pub fn extract_ocr_from_clip(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<String, String> {
    let clips = db.get_clips(None, None, false).map_err(|e| e.to_string())?;
    let clip = clips.into_iter().find(|c| c.id == clip_id).ok_or("Clip not found")?;

    if let Some(b64) = clip.image_base64 {
        let clean_b64 = if let Some(idx) = b64.find(',') {
            &b64[idx + 1..]
        } else {
            &b64
        };

        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(clean_b64) {
            if let Some(ocr_text) = crate::ocr::perform_ocr_on_image_bytes(&bytes) {
                let _ = db.update_clip_text(clip_id, &ocr_text);
                return Ok(ocr_text);
            }
        }
    }
    Err("No text recognized in image".to_string())
}

#[tauri::command]
pub fn toggle_clipboard_pause(
    monitor_state: State<'_, Arc<crate::clipboard_monitor::ClipboardMonitorState>>,
    db: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let current = monitor_state.is_manually_paused.load(std::sync::atomic::Ordering::Relaxed);
    let new_val = !current;
    monitor_state.is_manually_paused.store(new_val, std::sync::atomic::Ordering::Relaxed);

    if new_val {
        let _ = db.log_activity("recording_manually_paused", "Clipboard recording manually paused");
    } else {
        let _ = db.log_activity("recording_manually_resumed", "Clipboard recording manually resumed");
    }

    Ok(monitor_state.is_paused())
}

#[tauri::command]
pub fn is_clipboard_paused(
    monitor_state: State<'_, Arc<crate::clipboard_monitor::ClipboardMonitorState>>,
) -> Result<bool, String> {
    Ok(monitor_state.is_paused())
}

#[tauri::command]
pub fn export_clips_json(db: State<'_, Arc<DbState>>) -> Result<String, String> {
    let clips = db.get_clips(None, None, false).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&clips).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_clips_csv(db: State<'_, Arc<DbState>>) -> Result<String, String> {
    let clips = db.get_clips(None, None, false).map_err(|e| e.to_string())?;
    let mut csv = String::from("id,content_type,source_app,is_pinned,created_at,text_content\n");
    for c in clips {
        let text = c.text_content.unwrap_or_default().replace('"', "\"\"");
        let line = format!(
            "{},\"{}\",\"{}\",{},\"{}\",\"{}\"\n",
            c.id, c.content_type, c.source_app, c.is_pinned, c.created_at, text
        );
        csv.push_str(&line);
    }
    Ok(csv)
}

#[tauri::command]
pub fn import_clips_json(json_str: String, db: State<'_, Arc<DbState>>) -> Result<usize, String> {
    let items: Vec<ClipItem> = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;
    let mut count = 0;
    for item in items {
        if db.save_clip(
            &item.content_type,
            item.text_content.as_deref(),
            item.html_content.as_deref(),
            item.image_base64.as_deref(),
            &item.content_hash,
            &item.source_app,
        ).is_ok() {
            count += 1;
        }
    }
    Ok(count)
}

#[tauri::command]
pub fn get_analytics_summary(
    db: State<'_, Arc<DbState>>,
) -> Result<crate::db::AnalyticsSummary, String> {
    db.get_analytics_summary().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn install_cli_to_path() -> Result<String, String> {
    use std::fs;

    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let bin_dir = exe_path.parent().ok_or("Cannot locate binary directory")?;
    let cli_exe = bin_dir.join("pasted-cli");

    if !cli_exe.exists() {
        return Err(format!(
            "pasted-cli binary not found at '{:?}'. Run 'cargo build --bin pasted-cli' first.",
            cli_exe
        ));
    }

    let target_dir = dirs::home_dir()
        .map(|h| h.join(".local/bin"))
        .unwrap_or_else(|| std::path::PathBuf::from("/usr/local/bin"));

    let _ = fs::create_dir_all(&target_dir);
    let symlink_path = target_dir.join("pasted-cli");

    if symlink_path.exists() {
        let _ = fs::remove_file(&symlink_path);
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(&cli_exe, &symlink_path)
            .map_err(|e| format!("Failed to create symlink at '{:?}': {}", symlink_path, e))?;
    }

    Ok(format!(
        "✓ Successfully linked pasted-cli to '{}'! Make sure standard bin dir is in your $PATH.",
        symlink_path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shortcut_str_variations() {
        assert!(parse_shortcut_str("CmdOrCtrl+Shift+V").is_some());
        assert!(parse_shortcut_str("Control+Alt+C").is_some());
        assert!(parse_shortcut_str("Ctrl+Alt+KeyC").is_some());
        assert!(parse_shortcut_str("Alt+Super+KeyV").is_some());
        assert!(parse_shortcut_str("Option+Cmd+C").is_some());
        assert!(parse_shortcut_str("Command+Shift+V").is_some());
        assert!(parse_shortcut_str("Control+Option+C").is_some());
        assert!(parse_shortcut_str("Control+Option+V").is_some());
        assert!(parse_shortcut_str("Super+Alt+KeyC").is_some());
        assert!(parse_shortcut_str("").is_none());
        assert!(parse_shortcut_str("   ").is_none());

        // Equivalence checks for key representations
        let sc1 = parse_shortcut_str("Option+Command+C").unwrap();
        let sc2 = parse_shortcut_str("Alt+Super+KeyC").unwrap();
        assert_eq!(sc1, sc2, "Option+Command+C should resolve to identical Shortcut struct as Alt+Super+KeyC");

        // Option unicode character resolution tests
        let sc_unicode_c = parse_shortcut_str("Alt+ç").unwrap();
        let sc_ascii_c = parse_shortcut_str("Alt+KeyC").unwrap();
        assert_eq!(sc_unicode_c, sc_ascii_c, "Alt+ç should map to Alt+KeyC");
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
            let parsed = parse_shortcut_str(s);
            println!("parse_shortcut_str('{s}') = {:?}", parsed);
        }
    }

    #[test]
    fn test_accessibility_status_check() {
        let status = check_accessibility_permission();
        println!("Accessibility test status: trusted={}, dev_mode={}", status.is_trusted, status.is_dev_mode);
        assert_eq!(status.is_dev_mode, cfg!(debug_assertions));
    }
}
