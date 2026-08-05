use arboard::Clipboard;
use base64::Engine;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::db::DbState;
use crate::sequential_paste::SequentialQueueState;

fn report_ignored_capture(app: &AppHandle, db: &DbState, reason: &str) {
    let _ = db.log_activity("clipboard_capture_ignored", reason);
    let _ = app.emit(
        "clipboard-clip-ignored",
        serde_json::json!({ "reason": reason }),
    );
}

fn configured_capture_bytes(db: &DbState) -> usize {
    let configured = db.get_setting("maxClipSizeMb").ok().flatten();
    crate::resource_limits::configured_clip_capture_bytes(configured.as_deref())
}

#[derive(Clone, Copy)]
struct ContentDetectionSettings {
    colors: bool,
    links: bool,
    code: bool,
}

impl Default for ContentDetectionSettings {
    fn default() -> Self {
        Self {
            colors: true,
            links: true,
            code: true,
        }
    }
}

impl ContentDetectionSettings {
    fn from_db(db: &DbState) -> Self {
        let enabled = |key: &str| {
            db.get_setting(key)
                .ok()
                .flatten()
                .map(|value| value != "false")
                .unwrap_or(true)
        };
        Self {
            colors: enabled("detectColors"),
            links: enabled("detectLinks"),
            code: enabled("detectCode"),
        }
    }
}

fn is_image_file_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "avif"
                    | "bmp"
                    | "gif"
                    | "heic"
                    | "heif"
                    | "ico"
                    | "jpeg"
                    | "jpg"
                    | "png"
                    | "tif"
                    | "tiff"
                    | "webp"
            )
        })
}

pub struct ClipboardMonitorState {
    pub is_manually_paused: Arc<AtomicBool>,
    pub is_auto_paused: Arc<AtomicBool>,
}

impl ClipboardMonitorState {
    pub fn is_paused(&self) -> bool {
        self.is_manually_paused.load(Ordering::Relaxed)
            || self.is_auto_paused.load(Ordering::Relaxed)
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

            if is_manually_paused_clone.load(Ordering::Relaxed)
                || is_auto_paused_clone.load(Ordering::Relaxed)
            {
                continue;
            }

            let clipboard_files = clipboard.get().file_list().unwrap_or_default();

            // Finder and other file managers can publish both a native file reference and the
            // bitmap itself for a copied image. Prefer that bitmap for one recognized image so
            // image copy/paste and OCR keep working; preserve multi-file selections as file clips.
            let preferred_file_image =
                if clipboard_files.len() == 1 && is_image_file_path(&clipboard_files[0]) {
                    clipboard.get_image().ok()
                } else {
                    None
                };

            // File lists are an explicit clipboard flavor on every supported desktop OS.
            // Store only bounded path metadata; never read file contents into the database.
            if preferred_file_image.is_none() && !clipboard_files.is_empty() {
                let files = &clipboard_files;
                let paths: Vec<String> = files
                    .iter()
                    .map(|path| path.to_string_lossy().into_owned())
                    .collect();
                let mut hasher = Sha256::new();
                for path in &paths {
                    hasher.update(path.as_bytes());
                    hasher.update([0]);
                }
                let hash = format!("files:{:x}", hasher.finalize());
                if hash != last_hash {
                    last_hash = hash.clone();
                    if !crate::resource_limits::file_list_within_limit(&paths) {
                        report_ignored_capture(
                                &app,
                                &db_state,
                                &format!(
                                    "Ignored file list exceeding Pasted's limit of {} paths or {} MB of metadata",
                                    crate::resource_limits::MAX_FILE_LIST_ITEMS,
                                    crate::resource_limits::MAX_FILE_LIST_METADATA_BYTES / 1024 / 1024
                                ),
                            );
                        continue;
                    }

                    let is_blacklisted = active_app_opt.as_ref().is_some_and(|active_app| {
                        db_state
                            .get_setting("blacklistApps")
                            .ok()
                            .flatten()
                            .and_then(|json| serde_json::from_str::<Vec<String>>(&json).ok())
                            .is_some_and(|entries| {
                                let active_app = active_app.to_lowercase();
                                entries.iter().any(|entry| {
                                    let entry = entry.to_lowercase();
                                    !entry.is_empty()
                                        && (active_app == entry || active_app.contains(&entry))
                                })
                            })
                    });
                    if is_blacklisted {
                        if let Some(active_app) = active_app_opt.as_ref() {
                            let _ = app.emit(
                                "blacklist-clip-ignored",
                                serde_json::json!({ "app_name": active_app }),
                            );
                        }
                        continue;
                    }

                    let serialized = match serde_json::to_string(&paths) {
                        Ok(serialized) => serialized,
                        Err(error) => {
                            eprintln!("[Pasted Monitor] Failed to serialize file list: {error}");
                            continue;
                        }
                    };
                    let source_app = active_app_opt.as_deref().unwrap_or("System Clipboard");
                    match db_state.save_clip(
                        "file",
                        Some(&serialized),
                        None,
                        None,
                        &hash,
                        source_app,
                    ) {
                        Ok(clip) => {
                            let _ = app.emit("clip-added", clip);
                        }
                        Err(error) => {
                            eprintln!("[Pasted Monitor] Failed to save file clip: {error}");
                        }
                    }
                }
                continue;
            }

            // Attempt to read text
            let clipboard_text = if preferred_file_image.is_none() {
                clipboard.get_text().ok()
            } else {
                None
            };
            if let Some(text) = clipboard_text {
                if !text.is_empty() {
                    let mut hasher = Sha256::new();
                    hasher.update(text.as_bytes());
                    let hash = format!("{:x}", hasher.finalize());

                    if hash != last_hash {
                        last_hash = hash.clone();
                        let capture_limit = configured_capture_bytes(&db_state);
                        if text.len() > capture_limit {
                            report_ignored_capture(
                                &app,
                                &db_state,
                                &format!(
                                    "Ignored clipboard text larger than the configured {} MB limit",
                                    capture_limit / 1024 / 1024
                                ),
                            );
                            continue;
                        }

                        // Check blacklist
                        if let Some(ref active_app) = active_app_opt {
                            if let Ok(Some(blacklist_json)) = db_state.get_setting("blacklistApps")
                            {
                                if let Ok(blacklisted_list) =
                                    serde_json::from_str::<Vec<String>>(&blacklist_json)
                                {
                                    let active_app_lower = active_app.to_lowercase();
                                    if blacklisted_list.iter().any(|b| {
                                        let b_lower = b.to_lowercase();
                                        !b_lower.is_empty()
                                            && (active_app_lower == b_lower
                                                || active_app_lower.contains(&b_lower))
                                    }) {
                                        let _ = app.emit(
                                            "blacklist-clip-ignored",
                                            serde_json::json!({ "app_name": active_app }),
                                        );
                                        continue;
                                    }
                                }
                            }
                        }

                        // Detect type
                        let detection_settings = ContentDetectionSettings::from_db(&db_state);
                        let content_type = detect_content_type(&text, detection_settings);

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
                                let _ = app.emit("clip-added", clip.clone());
                                let automation_db = db_state.clone();
                                let automation_app = app.clone();
                                let automation_type = content_type.clone();
                                let automation_text = text.clone();
                                let automation_source = source_app.to_string();
                                thread::spawn(move || {
                                    crate::intelligence_executor::apply_smart_bin_transforms_for_clip(
                                        &automation_db,
                                        clip.id,
                                        &automation_type,
                                        &automation_text,
                                        &automation_source,
                                    );
                                    if let Ok(Some(updated)) =
                                        automation_db.get_clips(None, None, false).map(|clips| {
                                            clips.into_iter().find(|item| item.id == clip.id)
                                        })
                                    {
                                        let _ = automation_app.emit("clip-added", updated);
                                    }
                                });
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
            if let Some(img) = preferred_file_image.or_else(|| clipboard.get_image().ok()) {
                let (Ok(width), Ok(height)) = (u32::try_from(img.width), u32::try_from(img.height))
                else {
                    report_ignored_capture(
                        &app,
                        &db_state,
                        "Ignored clipboard image with invalid dimensions",
                    );
                    continue;
                };
                if !crate::resource_limits::image_dimensions_within_limit(width, height) {
                    let mut hasher = Sha256::new();
                    hasher.update(img.bytes.as_ref());
                    let hash = format!("{:x}", hasher.finalize());
                    if hash != last_hash {
                        last_hash = hash;
                        report_ignored_capture(
                            &app,
                            &db_state,
                            "Ignored clipboard image larger than 24 megapixels",
                        );
                    }
                    continue;
                }
                let raw_bytes = img.bytes.to_vec();

                let mut hasher = Sha256::new();
                hasher.update(&raw_bytes);
                let hash = format!("{:x}", hasher.finalize());

                if hash != last_hash {
                    last_hash = hash.clone();

                    // Check blacklist
                    if let Some(ref active_app) = active_app_opt {
                        if let Ok(Some(blacklist_json)) = db_state.get_setting("blacklistApps") {
                            if let Ok(blacklisted_list) =
                                serde_json::from_str::<Vec<String>>(&blacklist_json)
                            {
                                let active_app_lower = active_app.to_lowercase();
                                if blacklisted_list.iter().any(|b| {
                                    let b_lower = b.to_lowercase();
                                    !b_lower.is_empty()
                                        && (active_app_lower == b_lower
                                            || active_app_lower.contains(&b_lower))
                                }) {
                                    let _ = app.emit(
                                        "blacklist-clip-ignored",
                                        serde_json::json!({ "app_name": active_app }),
                                    );
                                    continue;
                                }
                            }
                        }
                    }

                    if let Some(img_bytes) = rgba_to_png(width, height, &raw_bytes) {
                        let capture_limit = configured_capture_bytes(&db_state)
                            .min(crate::resource_limits::MAX_ENCODED_IMAGE_BYTES);
                        if img_bytes.len() > capture_limit {
                            report_ignored_capture(
                                &app,
                                &db_state,
                                &format!(
                                    "Ignored clipboard image larger than the configured {} MB limit",
                                    capture_limit / 1024 / 1024
                                ),
                            );
                            continue;
                        }
                        let b64 = format!(
                            "data:image/webp;base64,{}",
                            base64::engine::general_purpose::STANDARD.encode(&img_bytes)
                        );

                        let source_app = active_app_opt.as_deref().unwrap_or("System Clipboard");
                        match db_state.save_clip("image", None, None, Some(&b64), &hash, source_app)
                        {
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

    MonitorHandle {
        running,
        is_manually_paused,
        is_auto_paused,
    }
}

fn detect_content_type(text: &str, settings: ContentDetectionSettings) -> String {
    let trimmed = text.trim();

    // Check color hex
    if settings.colors
        && (trimmed.len() == 4 || trimmed.len() == 7 || trimmed.len() == 9)
        && trimmed.starts_with('#')
        && trimmed[1..].chars().all(|c| c.is_ascii_hexdigit())
    {
        return "color".to_string();
    }

    // Check RGB / HSL
    if settings.colors
        && (trimmed.starts_with("rgb(")
            || trimmed.starts_with("rgba(")
            || trimmed.starts_with("hsl("))
        && trimmed.ends_with(')')
    {
        return "color".to_string();
    }

    // Check URL / link
    if settings.links
        && (trimmed.starts_with("http://")
            || trimmed.starts_with("https://")
            || trimmed.starts_with("file://"))
    {
        return "link".to_string();
    }

    // Check Code snippet heuristics
    if settings.code
        && (trimmed.contains("function ")
            || trimmed.contains("const ")
            || trimmed.contains("let ")
            || trimmed.contains("var ")
            || trimmed.contains("import ")
            || trimmed.contains("pub fn ")
            || trimmed.contains("class ")
            || trimmed.contains("def ")
            || trimmed.contains("SELECT ")
            || (trimmed.contains('{') && trimmed.contains('}') && trimmed.contains(';')))
    {
        return "code".to_string();
    }

    "text".to_string()
}

fn rgba_to_png(width: u32, height: u32, rgba_data: &[u8]) -> Option<Vec<u8>> {
    use image::{ImageBuffer, Rgba};
    let imgbuf: ImageBuffer<Rgba<u8>, _> =
        ImageBuffer::from_raw(width, height, rgba_data.to_vec())?;
    let mut cursor = std::io::Cursor::new(Vec::new());
    if imgbuf
        .write_to(&mut cursor, image::ImageFormat::WebP)
        .is_ok()
    {
        return Some(cursor.into_inner());
    }
    let mut fallback_cursor = std::io::Cursor::new(Vec::new());
    imgbuf
        .write_to(&mut fallback_cursor, image::ImageFormat::Png)
        .ok()?;
    Some(fallback_cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::{detect_content_type, is_image_file_path, ContentDetectionSettings};
    use std::path::Path;

    #[test]
    fn image_file_detection_is_case_insensitive_and_extension_bounded() {
        assert!(is_image_file_path(Path::new("/tmp/photo.PNG")));
        assert!(is_image_file_path(Path::new("/tmp/photo.heic")));
        assert!(is_image_file_path(Path::new("/tmp/photo.webp")));
        assert!(!is_image_file_path(Path::new("/tmp/photo.png.txt")));
        assert!(!is_image_file_path(Path::new("/tmp/document.pdf")));
    }

    #[test]
    fn six_digit_codes_are_text_not_colors() {
        let settings = ContentDetectionSettings::default();
        assert_eq!(detect_content_type("313041", settings), "text");
        assert_eq!(detect_content_type("#313041", settings), "color");
        assert_eq!(detect_content_type("rgb(31, 30, 41)", settings), "color");
    }

    #[test]
    fn content_detection_categories_can_be_disabled_independently() {
        let none = ContentDetectionSettings {
            colors: false,
            links: false,
            code: false,
        };
        assert_eq!(detect_content_type("#313041", none), "text");
        assert_eq!(detect_content_type("https://pasted.app", none), "text");
        assert_eq!(detect_content_type("const pasted = true;", none), "text");

        let links_only = ContentDetectionSettings {
            links: true,
            ..none
        };
        assert_eq!(
            detect_content_type("https://pasted.app", links_only),
            "link"
        );
        assert_eq!(detect_content_type("#313041", links_only), "text");
    }
}
