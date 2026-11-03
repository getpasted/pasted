use arboard::Clipboard;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager};

use crate::clipboard_capture_policy::{
    already_processed_change, clipboard_change_marker, composite_image_source,
    configured_capture_bytes, inferred_screenshot_source, is_image_file_path, is_pasted_source,
    resolved_capture_source, should_coalesce_recent_image, should_prefer_composite_image,
    RecentImageCapture, FILE_IMAGE_STABILIZATION_ATTEMPTS, FILE_IMAGE_STABILIZATION_INTERVAL,
};
use crate::clipboard_image::{image_file_clipboard_payload, image_file_rgba_fingerprint};
use crate::clipboard_ingestion::{ingest_files, ingest_image, ingest_text, CaptureContext};
use crate::db::DbState;
use crate::sequential_paste::SequentialQueueState;

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
    initially_paused: bool,
) -> MonitorHandle {
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    let is_manually_paused = Arc::new(AtomicBool::new(initially_paused));
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
        // The clipboard contents that predate this process are not a new copy.
        // Baseline the native generation so restarting Pasted cannot revive a
        // matching clip that the user already moved to Trash.
        let mut last_processed_change_marker = clipboard_change_marker();
        let mut initial_snapshot_pending = last_processed_change_marker.is_none();
        let mut auto_paused_app: Option<String> = None;
        let mut recent_image_capture: Option<RecentImageCapture> = None;

        while running_clone.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(300));

            let inspect_private_mode = crate::private_browsing::is_enabled(&db_state);
            let active_context =
                crate::paste_target::active_application_context(inspect_private_mode);
            let active_app_opt = active_context.as_ref().map(|context| context.name.clone());
            let exclusion_rules = crate::app_exclusions::load_rules(&db_state);
            let active_exclusion = active_app_opt.as_deref().and_then(|active_app| {
                crate::app_exclusions::matching_rule(&exclusion_rules, active_app)
            });
            let private_browser_excluded = active_context
                .as_ref()
                .is_some_and(|context| crate::private_browsing::should_exclude(&db_state, context));
            let fully_excluded_app = active_app_opt.as_ref().filter(|_| {
                private_browser_excluded
                    || active_exclusion.is_some_and(crate::app_exclusions::ignores_all_capture)
            });

            // Only a rule that excludes every capture kind presents as a full recording pause.
            // Partial rules remain active below and suppress only their selected clipboard kind.
            if let Some(active_app) = fully_excluded_app {
                if auto_paused_app.as_deref() != Some(active_app.as_str()) {
                    is_auto_paused_clone.store(true, Ordering::Relaxed);
                    auto_paused_app = Some(active_app.clone());
                    let _ = db_state.log_activity(
                        "recording_auto_paused",
                        &format!("Auto-paused recording for excluded app: {}", active_app),
                    );
                    crate::app_events::emit_clipboard_pause_changed(
                        &app,
                        true,
                        Some(active_app.clone()),
                    );
                }
            } else if let Some(prev_app) = auto_paused_app.take() {
                is_auto_paused_clone.store(false, Ordering::Relaxed);
                let _ = db_state.log_activity(
                    "recording_auto_resumed",
                    &format!("Auto-resumed recording after leaving {}", prev_app),
                );
                let is_still_paused = is_manually_paused_clone.load(Ordering::Relaxed);
                crate::app_events::emit_clipboard_pause_changed(&app, is_still_paused, None);
            }

            if is_manually_paused_clone.load(Ordering::Relaxed)
                || is_auto_paused_clone.load(Ordering::Relaxed)
            {
                continue;
            }

            let capture_suppressed = app
                .try_state::<Arc<crate::app_lock::AppLockState>>()
                .is_some_and(|state| !crate::app_lock::capture_allowed(&db_state, &state));

            let change_marker = clipboard_change_marker();
            if already_processed_change(change_marker, last_processed_change_marker) {
                continue;
            }

            let clipboard_files = clipboard.get().file_list().unwrap_or_default();

            let inferred_source = clipboard_files
                .first()
                .filter(|_| clipboard_files.len() == 1)
                .and_then(|path| inferred_screenshot_source(path));

            let mut composite_image =
                if clipboard_files.len() == 1 && is_image_file_path(&clipboard_files[0]) {
                    image_file_clipboard_payload(&clipboard_files[0])
                        .or_else(|| clipboard.get_image().ok())
                } else {
                    None
                };
            if composite_image.is_none()
                && inferred_source == Some("CleanShot X")
                && clipboard_files.len() == 1
            {
                composite_image = image_file_clipboard_payload(&clipboard_files[0]);
            }
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
                should_prefer_composite_image(
                    is_image_file_path(path),
                    delayed_composite_match,
                    composite_image.is_some(),
                    active_app_opt.as_deref(),
                    inferred_source,
                )
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
                        recent.is_current()
                            && image_file_rgba_fingerprint(path).as_deref()
                                == Some(recent.content_hash.as_str())
                    })
            });
            let coalesce_with_recent_image = should_coalesce_recent_image(
                clipboard_files.len() == 1 && is_image_file_path(&clipboard_files[0]),
                matches_recent_image,
                active_app_opt.as_deref(),
                inferred_source,
                recent_image_capture
                    .as_ref()
                    .is_some_and(RecentImageCapture::is_current),
            );

            // File lists are an explicit clipboard flavor on every supported desktop OS.
            // A single image file accompanied by bitmap bytes is resolved above so screenshot
            // tools retain image/OCR behavior while explicit file-manager copies remain files.
            if !prefer_composite_image && !clipboard_files.is_empty() {
                last_processed_change_marker = change_marker;
                let paths: Vec<String> = clipboard_files
                    .iter()
                    .map(|path| path.to_string_lossy().into_owned())
                    .collect();
                let hash = crate::clipboard_fingerprint::file_list(&paths);
                let context = CaptureContext {
                    app: &app,
                    db: &db_state,
                    queue: &seq_state,
                    active_app: active_app_opt.as_deref(),
                    active_exclusion,
                    source: capture_source.unwrap_or("System Clipboard"),
                    suppressed: capture_suppressed || initial_snapshot_pending,
                };
                let coalesced_image = coalesce_with_recent_image
                    .then(|| {
                        recent_image_capture
                            .as_ref()
                            .map(|recent| (recent, composite_image_source(inferred_source)))
                    })
                    .flatten();
                ingest_files(&context, paths, hash, &mut last_hash, coalesced_image);
                initial_snapshot_pending = false;
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
                    last_processed_change_marker = change_marker;
                    let hash = crate::clipboard_fingerprint::text(&text);
                    let context = CaptureContext {
                        app: &app,
                        db: &db_state,
                        queue: &seq_state,
                        active_app: active_app_opt.as_deref(),
                        active_exclusion,
                        source: capture_source.unwrap_or("System Clipboard"),
                        suppressed: capture_suppressed || initial_snapshot_pending,
                    };
                    ingest_text(
                        &context,
                        text,
                        hash,
                        &mut last_hash,
                        configured_capture_bytes(&db_state),
                    );
                    initial_snapshot_pending = false;
                    continue;
                }
            }

            // Attempt to read image
            if let Some(image) = composite_image.or_else(|| clipboard.get_image().ok()) {
                last_processed_change_marker = change_marker;
                let context = CaptureContext {
                    app: &app,
                    db: &db_state,
                    queue: &seq_state,
                    active_app: active_app_opt.as_deref(),
                    active_exclusion,
                    source: capture_source.unwrap_or("System Clipboard"),
                    suppressed: capture_suppressed || initial_snapshot_pending,
                };
                let reattribute_source = (is_pasted_source(active_app_opt.as_deref())
                    && recent_image_capture
                        .as_ref()
                        .is_some_and(RecentImageCapture::is_current))
                .then(|| composite_image_source(inferred_source));
                ingest_image(
                    &context,
                    image,
                    &ocr_service,
                    &mut last_hash,
                    &mut recent_image_capture,
                    reattribute_source,
                    configured_capture_bytes(&db_state),
                );
            }
            initial_snapshot_pending = false;
        }
    });

    MonitorHandle {
        running,
        is_manually_paused,
        is_auto_paused,
    }
}
