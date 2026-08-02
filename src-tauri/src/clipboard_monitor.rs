use arboard::Clipboard;
use base64::Engine;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::db::DbState;
use crate::sequential_paste::SequentialQueueState;

pub struct ClipboardMonitorState {
    pub is_manually_paused: Arc<AtomicBool>,
    pub is_auto_paused: Arc<AtomicBool>,
}

impl ClipboardMonitorState {
    pub fn is_paused(&self) -> bool {
        self.is_manually_paused.load(Ordering::Relaxed) || self.is_auto_paused.load(Ordering::Relaxed)
    }
}

#[allow(dead_code)]
pub struct MonitorHandle {
    running: Arc<AtomicBool>,
    pub is_manually_paused: Arc<AtomicBool>,
    pub is_auto_paused: Arc<AtomicBool>,
}

impl MonitorHandle {
    #[allow(dead_code)]
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

#[cfg(target_os = "macos")]
fn get_frontmost_app_name() -> Option<String> {
    use objc::runtime::Object;
    use objc::{msg_send, sel, sel_impl};
    unsafe {
        let workspace: *mut Object = msg_send![objc::class!(NSWorkspace), sharedWorkspace];
        if workspace.is_null() {
            return None;
        }
        let app: *mut Object = msg_send![workspace, frontmostApplication];
        if app.is_null() {
            return None;
        }
        let name: *mut Object = msg_send![app, localizedName];
        if name.is_null() {
            return None;
        }
        let utf8: *const std::os::raw::c_char = msg_send![name, UTF8String];
        if utf8.is_null() {
            return None;
        }
        let c_str = std::ffi::CStr::from_ptr(utf8);
        Some(c_str.to_string_lossy().into_owned())
    }
}

pub fn start_clipboard_monitor(
    app: AppHandle,
    db_state: Arc<DbState>,
    seq_state: Arc<SequentialQueueState>,
) -> MonitorHandle {
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    let is_manually_paused = Arc::new(AtomicBool::new(false));
    let is_auto_paused = Arc::new(AtomicBool::new(false));

    let is_manually_paused_clone = is_manually_paused.clone();
    let is_auto_paused_clone = is_auto_paused.clone();

    let ocr_tx = crate::ocr::spawn_ocr_worker(app.clone(), db_state.clone());

    thread::spawn(move || {
        let mut clipboard = match Clipboard::new() {
            Ok(cb) => cb,
            Err(e) => {
                eprintln!("[Pasted Monitor] Failed to initialize arboard: {}", e);
                return;
            }
        };

        let mut last_hash = String::new();
        let mut auto_paused_app: Option<String> = None;

        while running_clone.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(300));

            let active_app_opt = get_frontmost_app_name();

            // Auto-Pause & Auto-Resume on Blacklisted Application Focus Change
            if let Some(ref active_app) = active_app_opt {
                let active_app_lower = active_app.to_lowercase();

                let mut blacklisted_names = Vec::new();
                if let Ok(Some(blacklist_json)) = db_state.get_setting("blacklistApps") {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&blacklist_json) {
                        if let Some(arr) = val.as_array() {
                            for item in arr {
                                if let Some(s) = item.as_str() {
                                    blacklisted_names.push(s.to_string());
                                } else if let Some(s) = item.get("name").and_then(|n| n.as_str()) {
                                    blacklisted_names.push(s.to_string());
                                }
                            }
                        }
                    }
                }

                // Built-in default sensitive apps if blacklist settings haven't been saved yet
                if blacklisted_names.is_empty() {
                    blacklisted_names = vec![
                        "1password".to_string(),
                        "passwords".to_string(),
                        "keychain access".to_string(),
                        "bitwarden".to_string(),
                        "dashlane".to_string(),
                        "enpass".to_string(),
                        "keepassxc".to_string(),
                    ];
                }

                let is_blacklisted = blacklisted_names.iter().any(|b| {
                    let b_lower = b.to_lowercase();
                    !b_lower.is_empty()
                        && (active_app_lower == b_lower
                            || active_app_lower.contains(&b_lower)
                            || b_lower.contains(&active_app_lower))
                });

                if is_blacklisted && auto_paused_app.is_none() {
                    is_auto_paused_clone.store(true, Ordering::Relaxed);
                    auto_paused_app = Some(active_app.clone());
                    println!("[Pasted Monitor] AUTO-PAUSED for blacklisted app: {}", active_app);
                    let _ = db_state.log_activity(
                        "recording_auto_paused",
                        &format!("Auto-paused recording for blacklisted app: {}", active_app),
                    );
                    let _ = app.emit(
                        "clipboard-pause-changed",
                        serde_json::json!({
                            "is_paused": true,
                            "auto_paused_by": active_app
                        }),
                    );
                } else if !is_blacklisted && auto_paused_app.is_some() {
                    if let Some(prev_app) = auto_paused_app.take() {
                        is_auto_paused_clone.store(false, Ordering::Relaxed);
                        println!("[Pasted Monitor] AUTO-RESUMED after leaving {}", prev_app);
                        let _ = db_state.log_activity(
                            "recording_auto_resumed",
                            &format!("Auto-resumed recording after leaving {}", prev_app),
                        );
                        let is_still_paused = is_manually_paused_clone.load(Ordering::Relaxed);
                        let _ = app.emit(
                            "clipboard-pause-changed",
                            serde_json::json!({
                                "is_paused": is_still_paused,
                                "auto_paused_by": serde_json::Value::Null
                            }),
                        );
                    }
                }
            }

            if is_manually_paused_clone.load(Ordering::Relaxed) || is_auto_paused_clone.load(Ordering::Relaxed) {
                continue;
            }

            // Attempt to read text
            if let Ok(text) = clipboard.get_text() {
                if !text.is_empty() {
                    let normalized = text.replace("\r\n", "\n").trim_end().to_string();
                    let mut hasher = Sha256::new();
                    hasher.update(normalized.as_bytes());
                    let hash = format!("{:x}", hasher.finalize());

                    if hash != last_hash {
                        last_hash = hash.clone();

                        // Check blacklist
                        if let Some(ref active_app) = active_app_opt {
                            if let Ok(Some(blacklist_json)) = db_state.get_setting("blacklistApps") {
                                if let Ok(blacklisted_list) = serde_json::from_str::<Vec<String>>(&blacklist_json) {
                                    let active_app_lower = active_app.to_lowercase();
                                    if blacklisted_list.iter().any(|b| {
                                        let b_lower = b.to_lowercase();
                                        !b_lower.is_empty() && (active_app_lower == b_lower || active_app_lower.contains(&b_lower))
                                    }) {
                                        let _ = app.emit("blacklist-clip-ignored", serde_json::json!({ "app_name": active_app }));
                                        continue;
                                    }
                                }
                            }
                        }

                        // Detect type
                        let content_type = detect_content_type(&text);

                        // If sequential mode active, push to queue as well
                        if *seq_state.is_active.lock() {
                            seq_state.push_item(text.clone());
                            let _ = app.emit("sequential-updated", seq_state.get_status());
                        }

                        // Save to database with real active source app name
                        let source_app = active_app_opt.as_deref().unwrap_or("System Clipboard");
                        match db_state.save_clip(
                            &content_type,
                            Some(&text),
                            None,
                            None,
                            &hash,
                            source_app,
                        ) {
                            Ok(clip) => {
                                let _ = app.emit("clip-added", clip);
                            }
                            Err(e) => {
                                eprintln!("[Pasted Monitor] Failed to save clip: {}", e);
                            }
                        }
                    }
                    continue;
                }
            }

            // Attempt to read image
            if let Ok(img) = clipboard.get_image() {
                let width = img.width as u32;
                let height = img.height as u32;
                let raw_bytes = img.bytes.to_vec();

                let mut hasher = Sha256::new();
                hasher.update(&raw_bytes);
                let hash = format!("{:x}", hasher.finalize());

                if hash != last_hash {
                    last_hash = hash.clone();

                    // Check blacklist
                    if let Some(ref active_app) = active_app_opt {
                        if let Ok(Some(blacklist_json)) = db_state.get_setting("blacklistApps") {
                            if let Ok(blacklisted_list) = serde_json::from_str::<Vec<String>>(&blacklist_json) {
                                let active_app_lower = active_app.to_lowercase();
                                if blacklisted_list.iter().any(|b| {
                                    let b_lower = b.to_lowercase();
                                    !b_lower.is_empty() && (active_app_lower == b_lower || active_app_lower.contains(&b_lower))
                                }) {
                                    let _ = app.emit("blacklist-clip-ignored", serde_json::json!({ "app_name": active_app }));
                                    continue;
                                }
                            }
                        }
                    }

                    if let Some(img_bytes) = rgba_to_png(width, height, &raw_bytes) {
                        let b64 = format!(
                            "data:image/webp;base64,{}",
                            base64::engine::general_purpose::STANDARD.encode(&img_bytes)
                        );

                        let source_app = active_app_opt.as_deref().unwrap_or("System Clipboard");
                        match db_state.save_clip(
                            "image",
                            None,
                            None,
                            Some(&b64),
                            &hash,
                            source_app,
                        ) {
                            Ok(clip) => {
                                let _ = app.emit("clip-added", clip.clone());
                                let _ = ocr_tx.send(crate::ocr::OcrTask {
                                    clip_id: clip.id,
                                    image_bytes: img_bytes,
                                });
                            }
                            Err(e) => {
                                eprintln!("[Pasted Monitor] Failed to save image clip: {}", e);
                            }
                        }
                    }
                }
            }
        }
    });

    MonitorHandle { running, is_manually_paused, is_auto_paused }
}

fn detect_content_type(text: &str) -> String {
    let trimmed = text.trim();
    
    // Check color hex
    if (trimmed.len() == 4 || trimmed.len() == 7 || trimmed.len() == 9)
        && trimmed.starts_with('#')
        && trimmed[1..].chars().all(|c| c.is_ascii_hexdigit())
    {
        return "color".to_string();
    }

    // Check RGB / HSL
    if (trimmed.starts_with("rgb(") || trimmed.starts_with("rgba(") || trimmed.starts_with("hsl(")) && trimmed.ends_with(')') {
        return "color".to_string();
    }

    // Check URL / link
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") || trimmed.starts_with("file://") {
        return "link".to_string();
    }

    // Check Code snippet heuristics
    if trimmed.contains("function ")
        || trimmed.contains("const ")
        || trimmed.contains("let ")
        || trimmed.contains("var ")
        || trimmed.contains("import ")
        || trimmed.contains("pub fn ")
        || trimmed.contains("class ")
        || trimmed.contains("def ")
        || trimmed.contains("SELECT ")
        || (trimmed.contains('{') && trimmed.contains('}') && trimmed.contains(';'))
    {
        return "code".to_string();
    }

    "text".to_string()
}

fn rgba_to_png(width: u32, height: u32, rgba_data: &[u8]) -> Option<Vec<u8>> {
    use image::{ImageBuffer, Rgba};
    let imgbuf: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(width, height, rgba_data.to_vec())?;
    let mut cursor = std::io::Cursor::new(Vec::new());
    if imgbuf.write_to(&mut cursor, image::ImageFormat::WebP).is_ok() {
        return Some(cursor.into_inner());
    }
    let mut fallback_cursor = std::io::Cursor::new(Vec::new());
    imgbuf.write_to(&mut fallback_cursor, image::ImageFormat::Png).ok()?;
    Some(fallback_cursor.into_inner())
}
