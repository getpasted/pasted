// Native macOS Vision OCR Engine using Apple's Vision Framework (VNRecognizeTextRequest)
// Runs in-process with 0 external CLI dependencies, hardware-accelerated.

#[cfg(target_os = "macos")]
pub fn perform_ocr_on_image_bytes(image_bytes: &[u8]) -> Option<String> {
    use objc::runtime::{Class, Object};
    use objc::{msg_send, sel, sel_impl};
    use std::ptr::null_mut;

    type Id = *mut Object;

    if image_bytes.is_empty() || image_bytes.len() > crate::resource_limits::MAX_ENCODED_IMAGE_BYTES
    {
        return None;
    }

    unsafe {
        // Create NSData from raw image bytes
        let ns_data_class = Class::get("NSData")?;
        let ns_data: Id =
            msg_send![ns_data_class, dataWithBytes:image_bytes.as_ptr() length:image_bytes.len()];
        if ns_data.is_null() {
            return None;
        }

        // Create NSImage from NSData
        let ns_image_class = Class::get("NSImage")?;
        let ns_image: Id = msg_send![ns_image_class, alloc];
        let ns_image: Id = msg_send![ns_image, initWithData: ns_data];
        if ns_image.is_null() {
            return None;
        }

        // Get CGImageRef from NSImage
        let cg_image: Id = msg_send![
            ns_image,
            CGImageForProposedRect: null_mut::<Object>()
            context: null_mut::<Object>()
            hints: null_mut::<Object>()
        ];
        if cg_image.is_null() {
            return None;
        }

        // Create VNImageRequestHandler with CGImageRef
        let handler_class = Class::get("VNImageRequestHandler")?;
        let handler: Id = msg_send![handler_class, alloc];
        let handler: Id = msg_send![handler, initWithCGImage:cg_image options:null_mut::<Object>()];
        if handler.is_null() {
            return None;
        }

        // Create VNRecognizeTextRequest
        let request_class = Class::get("VNRecognizeTextRequest")?;
        let request: Id = msg_send![request_class, alloc];
        let request: Id = msg_send![request, init];
        if request.is_null() {
            return None;
        }

        // Set recognition level to 1 (VNRequestTextRecognitionLevelAccurate)
        let _: () = msg_send![request, setRecognitionLevel: 1i64];

        // Create NSArray containing the request
        let array_class = Class::get("NSArray")?;
        let requests: Id = msg_send![array_class, arrayWithObject: request];

        // Perform Vision request
        let mut error: Id = null_mut();
        let success: bool = msg_send![handler, performRequests: requests error: &mut error];
        if !success {
            return None;
        }

        // Retrieve OCR results array (VNRecognizedTextObservation items)
        let results: Id = msg_send![request, results];
        if results.is_null() {
            return None;
        }

        let count: usize = msg_send![results, count];
        if count == 0 {
            return None;
        }

        let mut lines = Vec::new();
        let mut recognized_bytes = 0usize;
        for i in 0..count {
            let observation: Id = msg_send![results, objectAtIndex: i];
            if observation.is_null() {
                continue;
            }

            let top_candidates: Id = msg_send![observation, topCandidates: 1usize];
            if !top_candidates.is_null() {
                let cand_count: usize = msg_send![top_candidates, count];
                if cand_count > 0 {
                    let candidate: Id = msg_send![top_candidates, objectAtIndex: 0usize];
                    if !candidate.is_null() {
                        let string_val: Id = msg_send![candidate, string];
                        if !string_val.is_null() {
                            let utf8: *const std::os::raw::c_char =
                                msg_send![string_val, UTF8String];
                            if !utf8.is_null() {
                                if let Ok(s) = std::ffi::CStr::from_ptr(utf8).to_str() {
                                    let trimmed = s.trim();
                                    if !trimmed.is_empty() {
                                        recognized_bytes = recognized_bytes
                                            .saturating_add(trimmed.len())
                                            .saturating_add(1);
                                        if recognized_bytes
                                            > crate::resource_limits::MAX_OCR_TEXT_BYTES
                                        {
                                            return None;
                                        }
                                        lines.push(trimmed.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if lines.is_empty() {
            None
        } else {
            Some(lines.join("\n"))
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn perform_ocr_on_image_bytes(_image_bytes: &[u8]) -> Option<String> {
    None
}

pub struct OcrTask {
    pub clip_id: i64,
    pub content_hash: String,
    pub image_bytes: Vec<u8>,
}

enum OcrRequest {
    Clip(OcrTask),
    Backfill,
}

pub struct OcrService {
    sender: std::sync::mpsc::Sender<OcrRequest>,
    backfill_cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl OcrService {
    pub fn enqueue(&self, task: OcrTask) -> Result<(), String> {
        self.sender
            .send(OcrRequest::Clip(task))
            .map_err(|_| "OCR worker is not available".to_string())
    }

    pub fn start_backfill(&self) -> Result<(), String> {
        self.backfill_cancelled
            .store(false, std::sync::atomic::Ordering::Release);
        self.sender
            .send(OcrRequest::Backfill)
            .map_err(|_| "OCR worker is not available".to_string())
    }

    pub fn cancel(&self) {
        self.backfill_cancelled
            .store(true, std::sync::atomic::Ordering::Release);
    }
}

pub fn decode_stored_image(value: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    let payload = value.split_once(',').map_or(value, |(_, payload)| payload);
    if payload.len() > crate::resource_limits::MAX_STORED_IMAGE_BASE64_BYTES {
        return None;
    }
    base64::engine::general_purpose::STANDARD
        .decode(payload)
        .ok()
}

fn execute_task<Recognize, Notify>(
    db_state: &crate::db::DbState,
    task: OcrTask,
    extractor: &crate::content_extraction::Extractor,
    recognize: Recognize,
    notify: Notify,
) where
    Recognize: FnOnce(&[u8]) -> Option<String>,
    Notify: FnOnce(i64),
{
    if !crate::features::is_enabled(db_state, crate::features::Feature::Ocr) {
        let _ = db_state.reset_ocr_work(Some(task.clip_id), Some(&task.content_hash));
        return;
    }
    let Ok(true) = db_state.mark_ocr_running(task.clip_id, &task.content_hash) else {
        return;
    };
    let detectors =
        crate::features::is_enabled(db_state, crate::features::Feature::ContentDetection)
            .then(|| db_state.get_content_detectors().ok())
            .flatten();
    let analysis = crate::content_analysis::analyze_image(
        task.image_bytes,
        extractor,
        detectors.as_deref(),
        recognize,
    );
    if !crate::features::is_enabled(db_state, crate::features::Feature::Ocr) {
        let _ = db_state.reset_ocr_work(Some(task.clip_id), Some(&task.content_hash));
        return;
    }
    let text = analysis.context.searchable_text.as_deref();
    let detected_type = analysis.context.detected_type.as_deref();
    let detector_ref = analysis.context.matched_detector_ref.as_deref();
    if db_state
        .complete_ocr_attempt(
            task.clip_id,
            &task.content_hash,
            text,
            &extractor.engine,
            None,
        )
        .unwrap_or(false)
    {
        if detectors.is_some() && text.is_some() {
            let _ = db_state.record_analysis_classification(
                task.clip_id,
                &task.content_hash,
                detected_type,
                detector_ref,
                "searchable_text",
            );
        }
        notify(task.clip_id);
    }
}

fn perform_task(app: &tauri::AppHandle, db_state: &crate::db::DbState, task: OcrTask) {
    use tauri::Emitter;
    let Ok(Some(extractor)) = db_state.active_image_text_extractor() else {
        let _ = db_state.reset_ocr_work(Some(task.clip_id), Some(&task.content_hash));
        return;
    };
    execute_task(
        db_state,
        task,
        &extractor,
        perform_ocr_on_image_bytes,
        |clip_id| {
            let _ = app.emit("clip-added", serde_json::json!({ "id": clip_id }));
            let _ = app.emit(
                "ocr-status-changed",
                serde_json::json!({ "clipId": clip_id }),
            );
        },
    );
}

fn run_backfill_candidates<Ready, Process>(
    db_state: &crate::db::DbState,
    cancelled: &std::sync::atomic::AtomicBool,
    mut ready: Ready,
    mut process: Process,
) where
    Ready: FnMut(&crate::db::DbState) -> bool,
    Process: FnMut(crate::db::OcrCandidate),
{
    loop {
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            break;
        }
        if !ready(db_state) {
            let _ = db_state.reset_ocr_work(None, None);
            break;
        }
        let Ok(Some(candidate)) = db_state.claim_next_ocr_candidate() else {
            break;
        };
        process(candidate);
    }
}

pub fn spawn_ocr_worker(
    app: tauri::AppHandle,
    db_state: std::sync::Arc<crate::db::DbState>,
) -> OcrService {
    let (tx, rx) = std::sync::mpsc::channel::<OcrRequest>();
    let backfill_cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let worker_cancelled = backfill_cancelled.clone();
    std::thread::spawn(move || {
        while let Ok(request) = rx.recv() {
            match request {
                OcrRequest::Clip(task) => perform_task(&app, &db_state, task),
                OcrRequest::Backfill => run_backfill_candidates(
                    &db_state,
                    &worker_cancelled,
                    |db_state| {
                        crate::features::is_enabled(db_state, crate::features::Feature::Ocr)
                            && db_state
                                .active_image_text_extractor()
                                .ok()
                                .flatten()
                                .is_some()
                    },
                    |candidate| {
                        let Some(image_bytes) = decode_stored_image(&candidate.image_base64) else {
                            let _ = db_state.complete_ocr_attempt(
                                candidate.clip_id,
                                &candidate.content_hash,
                                None,
                                "macos-vision-v1",
                                Some("invalid_image_data"),
                            );
                            return;
                        };
                        perform_task(
                            &app,
                            &db_state,
                            OcrTask {
                                clip_id: candidate.clip_id,
                                content_hash: candidate.content_hash,
                                image_bytes,
                            },
                        );
                    },
                ),
            }
        }
    });
    OcrService {
        sender: tx,
        backfill_cancelled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn setup_test_db() -> crate::db::DbState {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        crate::db::DbState::new(std::env::temp_dir().join(format!("pasted_ocr_worker_{nonce}.db")))
            .unwrap()
    }

    #[test]
    fn test_ocr_empty_or_invalid_bytes() {
        assert_eq!(perform_ocr_on_image_bytes(&[]), None);
        assert_eq!(perform_ocr_on_image_bytes(&[0, 1, 2, 3, 4]), None);
    }

    #[test]
    fn test_ocr_task_struct() {
        let task = OcrTask {
            clip_id: 42,
            content_hash: "image-hash".to_string(),
            image_bytes: vec![1, 2, 3],
        };
        assert_eq!(task.clip_id, 42);
        assert_eq!(task.content_hash, "image-hash");
        assert_eq!(task.image_bytes.len(), 3);
    }

    #[test]
    fn backfill_stops_between_items_and_resumes_without_reprocessing() {
        let db = setup_test_db();
        for index in 1..=3 {
            db.save_clip(
                "image",
                None,
                None,
                Some("aW1hZ2U="),
                &format!("backfill-image-{index}"),
                "Screenshot",
            )
            .unwrap();
        }

        let cancelled = AtomicBool::new(false);
        let mut first_pass = Vec::new();
        run_backfill_candidates(
            &db,
            &cancelled,
            |_| true,
            |candidate| {
                first_pass.push(candidate.clip_id);
                db.complete_ocr_attempt(
                    candidate.clip_id,
                    &candidate.content_hash,
                    Some("recognized"),
                    "fake-engine-v1",
                    None,
                )
                .unwrap();
                cancelled.store(true, Ordering::Release);
            },
        );

        assert_eq!(first_pass.len(), 1);
        let paused = db.get_ocr_backfill_status().unwrap();
        assert_eq!(paused.completed_count, 1);
        assert_eq!(paused.eligible_count, 2);

        cancelled.store(false, Ordering::Release);
        let mut second_pass = Vec::new();
        run_backfill_candidates(
            &db,
            &cancelled,
            |_| true,
            |candidate| {
                second_pass.push(candidate.clip_id);
                db.complete_ocr_attempt(
                    candidate.clip_id,
                    &candidate.content_hash,
                    Some("recognized"),
                    "fake-engine-v1",
                    None,
                )
                .unwrap();
            },
        );

        assert_eq!(second_pass.len(), 2);
        assert!(!second_pass.contains(&first_pass[0]));
        let completed = db.get_ocr_backfill_status().unwrap();
        assert_eq!(completed.completed_count, 3);
        assert_eq!(completed.eligible_count, 0);
        assert_eq!(completed.running_count, 0);
    }

    #[test]
    fn disabling_ocr_during_recognition_discards_the_late_result() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "image",
                None,
                None,
                Some("aW1hZ2U="),
                "disable-during-ocr",
                "Screenshot",
            )
            .unwrap();
        let notified = AtomicBool::new(false);
        let extractor = crate::content_extraction::Extractor {
            id: 1,
            stable_ref: "extractor:test".into(),
            name: "Test OCR".into(),
            description: String::new(),
            engine: "fake-engine-v1".into(),
            input_contract: "image".into(),
            output_contract: "searchable_text".into(),
            enabled: true,
            priority: 10,
            is_builtin: false,
            is_available: true,
            unavailable_reason: None,
            defaults: None,
        };

        execute_task(
            &db,
            OcrTask {
                clip_id: clip.id,
                content_hash: clip.content_hash.clone(),
                image_bytes: b"image".to_vec(),
            },
            &extractor,
            |_| {
                db.save_setting("enableOcr", "false").unwrap();
                Some("must not be saved".to_string())
            },
            |_| notified.store(true, Ordering::Release),
        );

        assert!(!notified.load(Ordering::Acquire));
        let stored = db.get_clip_by_id(clip.id).unwrap();
        assert_eq!(stored.text_content, None);
        let status = db.get_ocr_backfill_status().unwrap();
        assert_eq!(status.eligible_count, 1);
        assert_eq!(status.running_count, 0);
    }
}
