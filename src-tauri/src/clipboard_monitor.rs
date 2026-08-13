use arboard::Clipboard;
use base64::Engine;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

use crate::content_detection::detect_with_detectors;
use crate::db::DbState;
use crate::sequential_paste::SequentialQueueState;

#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum CaptureFeedbackKind {
    Success,
    Ignored,
    Failure,
}

fn capture_feedback_payload(kind: CaptureFeedbackKind, clip_id: Option<i64>) -> serde_json::Value {
    match clip_id {
        Some(clip_id) => serde_json::json!({ "kind": kind, "clip_id": clip_id }),
        None => serde_json::json!({ "kind": kind }),
    }
}

fn emit_capture_feedback(
    app: &AppHandle,
    db: &DbState,
    kind: CaptureFeedbackKind,
    clip_id: Option<i64>,
) {
    if !crate::features::is_enabled(db, crate::features::Feature::Notifications) {
        return;
    }
    let _ = app.emit_to(
        "capture-feedback",
        "clipboard-capture-feedback",
        capture_feedback_payload(kind, clip_id),
    );
}

fn report_ignored_capture(app: &AppHandle, db: &DbState, reason: &str) {
    let _ = db.log_activity("clipboard_capture_ignored", reason);
    let _ = app.emit(
        "clipboard-clip-ignored",
        serde_json::json!({ "reason": reason }),
    );
    emit_capture_feedback(app, db, CaptureFeedbackKind::Ignored, None);
}

fn report_failed_capture(app: &AppHandle, db: &DbState) {
    emit_capture_feedback(app, db, CaptureFeedbackKind::Failure, None);
}

fn configured_capture_bytes(db: &DbState) -> usize {
    let configured = db.get_setting("maxClipSizeMb").ok().flatten();
    crate::resource_limits::configured_clip_capture_bytes(configured.as_deref())
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

fn is_file_manager_source(source: Option<&str>) -> bool {
    source.is_some_and(|source| {
        matches!(
            source.trim().to_ascii_lowercase().as_str(),
            "finder"
                | "file explorer"
                | "windows explorer"
                | "explorer"
                | "files"
                | "nautilus"
                | "dolphin"
                | "thunar"
                | "nemo"
                | "caja"
                | "pcmanfm"
        )
    })
}

fn inferred_screenshot_source(path: &Path) -> Option<&'static str> {
    let name = path.file_name()?.to_str()?.to_ascii_lowercase();
    if name.contains("cleanshot") {
        return Some("CleanShot X");
    }
    (name.starts_with("screenshot ") || name.starts_with("screen shot ")).then_some("Screenshot")
}

fn resolved_capture_source<'a>(
    active_app: Option<&'a str>,
    inferred_source: Option<&'a str>,
) -> Option<&'a str> {
    if is_file_manager_source(active_app) {
        active_app
    } else {
        inferred_source.or(active_app)
    }
}

fn composite_image_source(inferred_source: Option<&str>) -> &str {
    inferred_source.unwrap_or("Screenshot")
}

/// Resolve the common composite clipboard payload where one image file is
/// accompanied by bitmap bytes. Explicit file-manager copies retain their file
/// identity; screenshot and otherwise ambiguous producers prefer the bitmap so
/// previews, image paste, and OCR continue to work.
fn prefer_bitmap_for_image_file(bitmap_available: bool, source: Option<&str>) -> bool {
    bitmap_available && !is_file_manager_source(source)
}

fn is_pasted_source(source: Option<&str>) -> bool {
    source.is_some_and(|source| {
        matches!(
            source.trim().to_ascii_lowercase().as_str(),
            "pasted" | "pasted-app"
        )
    })
}

const COMPOSITE_CAPTURE_WINDOW: Duration = Duration::from_secs(2);
const FILE_IMAGE_STABILIZATION_ATTEMPTS: usize = 20;
const FILE_IMAGE_STABILIZATION_INTERVAL: Duration = Duration::from_millis(100);

struct RecentImageCapture {
    clip_id: i64,
    content_hash: String,
    captured_at: Instant,
}

fn is_recent_capture(capture: &RecentImageCapture) -> bool {
    capture.captured_at.elapsed() <= COMPOSITE_CAPTURE_WINDOW
}

fn image_file_rgba_fingerprint(path: &Path) -> Option<String> {
    use std::io::Read;

    let metadata = std::fs::symlink_metadata(path).ok()?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > crate::resource_limits::MAX_FILE_PREVIEW_INPUT_BYTES
    {
        return None;
    }

    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(crate::resource_limits::MAX_FILE_PREVIEW_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > crate::resource_limits::MAX_FILE_PREVIEW_INPUT_BYTES {
        return None;
    }

    let dimensions = image::ImageReader::new(std::io::Cursor::new(&bytes))
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()?;
    if !crate::resource_limits::image_dimensions_within_limit(dimensions.0, dimensions.1) {
        return None;
    }

    let image = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?
        .to_rgba8();
    Some(crate::clipboard_fingerprint::image_rgba(image.as_raw()))
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

pub fn start_clipboard_monitor(
    app: AppHandle,
    db_state: Arc<DbState>,
    seq_state: Arc<SequentialQueueState>,
    ocr_service: Arc<crate::ocr::OcrService>,
) -> MonitorHandle {
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    let is_manually_paused = Arc::new(AtomicBool::new(false));
    let is_auto_paused = Arc::new(AtomicBool::new(false));

    let is_manually_paused_clone = is_manually_paused.clone();
    let is_auto_paused_clone = is_auto_paused.clone();

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
        let mut recent_image_capture: Option<RecentImageCapture> = None;

        while running_clone.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(300));

            let active_app_opt = crate::paste_target::active_application_name();

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

            let inferred_source = clipboard_files
                .first()
                .filter(|_| clipboard_files.len() == 1)
                .and_then(|path| inferred_screenshot_source(path));

            let mut composite_image =
                if clipboard_files.len() == 1 && is_image_file_path(&clipboard_files[0]) {
                    clipboard.get_image().ok()
                } else {
                    None
                };
            let mut delayed_composite_match = false;
            if composite_image.is_none() && clipboard_files.len() == 1 {
                let path = &clipboard_files[0];
                if is_image_file_path(path) {
                    let mut file_fingerprint = image_file_rgba_fingerprint(path);
                    for _ in 0..FILE_IMAGE_STABILIZATION_ATTEMPTS {
                        thread::sleep(FILE_IMAGE_STABILIZATION_INTERVAL);
                        if file_fingerprint.is_none() {
                            file_fingerprint = image_file_rgba_fingerprint(path);
                        }
                        if let Ok(image) = clipboard.get_image() {
                            let image_fingerprint =
                                crate::clipboard_fingerprint::image_rgba(image.bytes.as_ref());
                            if file_fingerprint
                                .as_deref()
                                .is_none_or(|fingerprint| fingerprint == image_fingerprint)
                            {
                                composite_image = Some(image);
                                delayed_composite_match = true;
                            }
                            break;
                        }
                        let refreshed_files = clipboard.get().file_list().unwrap_or_default();
                        // Screenshot tools may briefly withdraw the file URL before publishing
                        // bitmap bytes. A different non-empty file list is a genuinely new copy;
                        // an empty list remains inside this bounded stabilization window.
                        if !refreshed_files.is_empty() && refreshed_files != clipboard_files {
                            break;
                        }
                    }
                }
            }
            let prefer_composite_image = clipboard_files.first().is_some_and(|path| {
                is_image_file_path(path)
                    && (delayed_composite_match
                        || prefer_bitmap_for_image_file(
                            composite_image.is_some(),
                            active_app_opt.as_deref(),
                        ))
            });
            let capture_source = if prefer_composite_image {
                Some(composite_image_source(inferred_source))
            } else {
                resolved_capture_source(active_app_opt.as_deref(), inferred_source)
            };
            let matches_recent_image = clipboard_files.first().is_some_and(|path| {
                clipboard_files.len() == 1
                    && is_image_file_path(path)
                    && recent_image_capture.as_ref().is_some_and(|recent| {
                        is_recent_capture(recent)
                            && image_file_rgba_fingerprint(path).as_deref()
                                == Some(recent.content_hash.as_str())
                    })
            });
            let coalesce_with_recent_image = clipboard_files.len() == 1
                && is_image_file_path(&clipboard_files[0])
                && (matches_recent_image
                    || is_pasted_source(active_app_opt.as_deref())
                    || inferred_source.is_some())
                && recent_image_capture.as_ref().is_some_and(is_recent_capture);

            // File lists are an explicit clipboard flavor on every supported desktop OS.
            // A single image file accompanied by bitmap bytes is resolved above so screenshot
            // tools retain image/OCR behavior while explicit file-manager copies remain files.
            if !prefer_composite_image && !clipboard_files.is_empty() {
                let files = &clipboard_files;
                let paths: Vec<String> = files
                    .iter()
                    .map(|path| path.to_string_lossy().into_owned())
                    .collect();
                let hash = crate::clipboard_fingerprint::file_list(&paths);
                if hash != last_hash {
                    last_hash = hash.clone();
                    if seq_state.consume_internal_clipboard_write(&hash) {
                        continue;
                    }
                    if coalesce_with_recent_image {
                        if let Some(recent) = recent_image_capture.as_ref() {
                            let source = composite_image_source(inferred_source);
                            if let Ok(Some(updated)) = db_state.reattribute_image_capture(
                                recent.clip_id,
                                &recent.content_hash,
                                source,
                            ) {
                                let _ = app.emit("clip-added", updated);
                            }
                        }
                        continue;
                    }
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
                        emit_capture_feedback(&app, &db_state, CaptureFeedbackKind::Ignored, None);
                        continue;
                    }

                    let serialized = match serde_json::to_string(&paths) {
                        Ok(serialized) => serialized,
                        Err(error) => {
                            eprintln!("[Pasted Monitor] Failed to serialize file list: {error}");
                            report_failed_capture(&app, &db_state);
                            continue;
                        }
                    };
                    let source = capture_source.unwrap_or("System Clipboard");
                    match db_state.save_clip("file", Some(&serialized), None, None, &hash, source) {
                        Ok(clip) => {
                            let preview_mode = db_state
                                .get_setting("filePreviewMode")
                                .ok()
                                .flatten()
                                .filter(|mode| matches!(mode.as_str(), "off" | "safe" | "all"))
                                .unwrap_or_else(|| "safe".to_string());
                            let preview_max_mb = db_state
                                .get_setting("filePreviewMaxMb")
                                .ok()
                                .flatten()
                                .and_then(|value| value.parse::<u64>().ok())
                                .unwrap_or(25)
                                .clamp(1, 64);
                            if preview_mode != "off" {
                                let preview_app = app.clone();
                                let preview_paths = paths.clone();
                                let preview_hash = hash.clone();
                                thread::spawn(move || {
                                    crate::commands::prefetch_file_clip_previews(
                                        &preview_app,
                                        &preview_paths,
                                        &preview_hash,
                                        &preview_mode,
                                        preview_max_mb,
                                    );
                                });
                            }
                            let clip_id = clip.id;
                            let _ = app.emit("clip-added", clip);
                            emit_capture_feedback(
                                &app,
                                &db_state,
                                CaptureFeedbackKind::Success,
                                Some(clip_id),
                            );
                        }
                        Err(error) => {
                            eprintln!("[Pasted Monitor] Failed to save file clip: {error}");
                            report_failed_capture(&app, &db_state);
                        }
                    }
                }
                continue;
            }

            // Attempt to read text
            let clipboard_text = if prefer_composite_image {
                None
            } else {
                clipboard.get_text().ok()
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

                        // Queue paste commands write to the system clipboard so
                        // the destination app can receive a normal paste. That
                        // internal write is not a new user copy and must not be
                        // saved to history, re-queued, or sent through Smart Bin
                        // automation.
                        if seq_state.consume_internal_clipboard_write(&text) {
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
                                        emit_capture_feedback(
                                            &app,
                                            &db_state,
                                            CaptureFeedbackKind::Ignored,
                                            None,
                                        );
                                        continue;
                                    }
                                }
                            }
                        }

                        // Detect type
                        let content_type = if crate::features::is_enabled(
                            &db_state,
                            crate::features::Feature::ContentDetection,
                        ) {
                            db_state
                                .get_content_detectors()
                                .map(|detectors| detect_with_detectors(&text, &detectors))
                                .unwrap_or_else(|_| "text".to_string())
                        } else {
                            "text".to_string()
                        };

                        // If sequential mode active, push to queue as well
                        if crate::features::is_enabled(&db_state, crate::features::Feature::Queue)
                            && seq_state.capture_item(text.clone())
                        {
                            let _ = db_state.log_activity(
                                "queue_item_recorded",
                                "Recorded copied text into the Queue",
                            );
                            let _ = app.emit("sequential-updated", seq_state.get_status());
                        }

                        // Save with the best available capture-source attribution.
                        let source = capture_source.unwrap_or("System Clipboard");
                        match db_state.save_clip(
                            &content_type,
                            Some(&text),
                            None,
                            None,
                            &hash,
                            source,
                        ) {
                            Ok(clip) => {
                                let _ = app.emit("clip-added", clip.clone());
                                emit_capture_feedback(
                                    &app,
                                    &db_state,
                                    CaptureFeedbackKind::Success,
                                    Some(clip.id),
                                );
                                let automation_db = db_state.clone();
                                let automation_app = app.clone();
                                let automation_type = content_type;
                                let automation_text = text.clone();
                                let automation_source = source.to_string();
                                thread::spawn(move || {
                                    if crate::features::is_enabled(
                                        &automation_db,
                                        crate::features::Feature::Bins,
                                    ) && crate::features::is_enabled(
                                        &automation_db,
                                        crate::features::Feature::Transformations,
                                    ) {
                                        crate::intelligence_executor::apply_smart_bin_transforms_for_clip(
                                            &automation_db,
                                            clip.id,
                                            &automation_type,
                                            &automation_text,
                                            &automation_source,
                                        );
                                    }
                                    if let Ok(updated) = automation_db.get_clip_by_id(clip.id) {
                                        let _ = automation_app.emit("clip-added", updated);
                                    }
                                });
                            }
                            Err(e) => {
                                eprintln!("[Pasted Monitor] Failed to save clip: {}", e);
                                report_failed_capture(&app, &db_state);
                            }
                        }
                    }
                    continue;
                }
            }

            // Attempt to read image
            if let Some(img) = composite_image.or_else(|| clipboard.get_image().ok()) {
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

                let hash = crate::clipboard_fingerprint::image_rgba(&raw_bytes);

                if hash != last_hash {
                    last_hash = hash.clone();
                    if seq_state.consume_internal_clipboard_write(&hash) {
                        continue;
                    }
                    if is_pasted_source(active_app_opt.as_deref())
                        && recent_image_capture.as_ref().is_some_and(is_recent_capture)
                    {
                        if let Some(recent) = recent_image_capture.as_ref() {
                            if let Ok(Some(updated)) = db_state.reattribute_image_capture(
                                recent.clip_id,
                                &recent.content_hash,
                                composite_image_source(inferred_source),
                            ) {
                                let _ = app.emit("clip-added", updated);
                            }
                        }
                        continue;
                    }

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
                                    emit_capture_feedback(
                                        &app,
                                        &db_state,
                                        CaptureFeedbackKind::Ignored,
                                        None,
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

                        let source = capture_source.unwrap_or("System Clipboard");
                        match db_state.save_clip("image", None, None, Some(&b64), &hash, source) {
                            Ok(clip) => {
                                recent_image_capture = Some(RecentImageCapture {
                                    clip_id: clip.id,
                                    content_hash: clip.content_hash.clone(),
                                    captured_at: Instant::now(),
                                });
                                let _ = app.emit("clip-added", clip.clone());
                                emit_capture_feedback(
                                    &app,
                                    &db_state,
                                    CaptureFeedbackKind::Success,
                                    Some(clip.id),
                                );
                                if crate::features::is_enabled(
                                    &db_state,
                                    crate::features::Feature::Ocr,
                                ) {
                                    let _ = ocr_service.enqueue(crate::ocr::OcrTask {
                                        clip_id: clip.id,
                                        content_hash: clip.content_hash,
                                        image_bytes: img_bytes,
                                    });
                                }
                            }
                            Err(e) => {
                                eprintln!("[Pasted Monitor] Failed to save image clip: {}", e);
                                report_failed_capture(&app, &db_state);
                            }
                        }
                    } else {
                        report_failed_capture(&app, &db_state);
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
    use super::{
        capture_feedback_payload, composite_image_source, image_file_rgba_fingerprint,
        inferred_screenshot_source, is_pasted_source, prefer_bitmap_for_image_file,
        resolved_capture_source, CaptureFeedbackKind,
    };
    use std::path::Path;

    #[test]
    fn capture_feedback_never_contains_clipboard_data() {
        for (kind, expected) in [
            (CaptureFeedbackKind::Success, "success"),
            (CaptureFeedbackKind::Ignored, "ignored"),
            (CaptureFeedbackKind::Failure, "failure"),
        ] {
            let payload = capture_feedback_payload(kind, None);
            assert_eq!(payload, serde_json::json!({ "kind": expected }));
            assert_eq!(payload.as_object().map(|object| object.len()), Some(1));
        }
        assert_eq!(
            capture_feedback_payload(CaptureFeedbackKind::Success, Some(42)),
            serde_json::json!({ "kind": "success", "clip_id": 42 })
        );
    }

    #[test]
    fn screenshot_composite_payloads_prefer_bitmap_for_ocr() {
        assert!(prefer_bitmap_for_image_file(true, None));
        assert!(prefer_bitmap_for_image_file(true, Some("CleanShot X")));
        assert!(prefer_bitmap_for_image_file(true, Some("System Clipboard")));
        assert_eq!(
            inferred_screenshot_source(Path::new("/Users/pasted/Desktop/CleanShot 2026-08-11.png")),
            Some("CleanShot X")
        );
        assert_eq!(
            resolved_capture_source(Some("Safari"), Some("CleanShot X")),
            Some("CleanShot X")
        );
        assert_eq!(composite_image_source(None), "Screenshot");
        assert_eq!(composite_image_source(Some("CleanShot X")), "CleanShot X");
        assert!(is_pasted_source(Some("pasted-app")));
        assert!(is_pasted_source(Some("Pasted")));
        assert!(!is_pasted_source(Some("Preview")));
    }

    #[test]
    fn explicit_file_copies_keep_file_identity() {
        assert!(!prefer_bitmap_for_image_file(true, Some("Finder")));
        assert!(!prefer_bitmap_for_image_file(true, Some("File Explorer")));
        assert!(!prefer_bitmap_for_image_file(false, Some("CleanShot X")));
        assert_eq!(
            resolved_capture_source(Some("Finder"), Some("CleanShot X")),
            Some("Finder")
        );
    }

    #[test]
    fn copied_image_files_use_the_same_rgba_fingerprint_as_clipboard_images() {
        let path = std::env::temp_dir().join(format!(
            "pasted-composite-fingerprint-{}-{}.png",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let rgba = vec![10, 20, 30, 255, 40, 50, 60, 128];
        let image = image::RgbaImage::from_raw(2, 1, rgba.clone()).unwrap();
        image.save(&path).unwrap();

        assert_eq!(
            image_file_rgba_fingerprint(&path),
            Some(crate::clipboard_fingerprint::image_rgba(&rgba))
        );

        let _ = std::fs::remove_file(path);
    }
}
