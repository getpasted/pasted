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
        .filter(|bytes| {
            !bytes.is_empty() && bytes.len() <= crate::resource_limits::MAX_ENCODED_IMAGE_BYTES
        })
}

fn execute_task<Notify>(
    db_state: &crate::db::DbState,
    task: OcrTask,
    extractor: &crate::content_extraction::Extractor,
    registry: &crate::content_extraction::ExtractorEngineRegistry<'_>,
    notify: Notify,
) where
    Notify: FnOnce(i64),
{
    if !crate::features::is_enabled(db_state, crate::features::Feature::Ocr) {
        let _ = db_state.reset_ocr_work(Some(task.clip_id), Some(&task.content_hash));
        return;
    }
    let Ok(true) = db_state.mark_ocr_running(task.clip_id, &task.content_hash) else {
        return;
    };
    let classifiers =
        crate::features::is_enabled(db_state, crate::features::Feature::ContentClassification)
            .then(|| db_state.get_content_classifiers().ok())
            .flatten();
    let analysis = crate::extraction_execution::analyze_image_with_registry_and_policy(
        task.image_bytes,
        extractor,
        classifiers.as_deref(),
        registry,
        crate::analysis_contract::AnalysisPolicy::Background,
    );
    if !crate::features::is_enabled(db_state, crate::features::Feature::Ocr) {
        let _ = db_state.reset_ocr_work(Some(task.clip_id), Some(&task.content_hash));
        return;
    }
    let completed = crate::extraction_execution::persist_claimed_image_analysis(
        db_state,
        task.clip_id,
        &task.content_hash,
        extractor,
        classifiers.is_some(),
        analysis,
    )
    .map(|persisted| persisted.ocr_updated)
    .unwrap_or(false);
    if completed {
        notify(task.clip_id);
    }
}

fn perform_task(app: &tauri::AppHandle, db_state: &crate::db::DbState, task: OcrTask) {
    use tauri::Emitter;
    let Ok(Some(extractor)) = db_state.active_image_text_extractor() else {
        let _ = db_state.reset_ocr_work(Some(task.clip_id), Some(&task.content_hash));
        return;
    };
    let registry = crate::content_extraction::system_engine_registry();
    execute_task(db_state, task, &extractor, &registry, |clip_id| {
        let _ = app.emit("clip-added", serde_json::json!({ "id": clip_id }));
        let _ = app.emit(
            "ocr-status-changed",
            serde_json::json!({ "clipId": clip_id }),
        );
    });
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
                            if let Ok(Some(extractor)) = db_state.active_image_text_extractor() {
                                let _ = db_state.complete_or_reset_ocr_attempt_with_extractor(
                                    candidate.clip_id,
                                    &candidate.content_hash,
                                    None,
                                    crate::db::OcrExtractorProvenance::identified(
                                        &extractor.engine,
                                        &extractor.stable_ref,
                                        &extractor.name,
                                    ),
                                    Some("invalid_image_data"),
                                );
                            } else {
                                let _ = db_state.reset_ocr_work(
                                    Some(candidate.clip_id),
                                    Some(&candidate.content_hash),
                                );
                            }
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
                Some(crate::resource_limits::TEST_PNG_DATA_URL),
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
        struct DisablingEngine<'a> {
            db: &'a crate::db::DbState,
        }

        impl crate::content_extraction::ExtractorEngine for DisablingEngine<'_> {
            fn id(&self) -> &'static str {
                "fake-engine-v1"
            }

            fn availability(&self) -> crate::content_extraction::EngineAvailability {
                crate::content_extraction::EngineAvailability {
                    is_available: true,
                    unavailable_reason: None,
                }
            }

            fn extract(&self, _image_bytes: &[u8]) -> crate::content_extraction::ExtractionOutcome {
                self.db.save_setting("enableOcr", "false").unwrap();
                crate::content_extraction::ExtractionOutcome::Produced {
                    text: "must not be saved".into(),
                }
            }
        }

        let db = setup_test_db();
        let clip = db
            .save_clip(
                "image",
                None,
                None,
                Some(crate::resource_limits::TEST_PNG_DATA_URL),
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
            executable_path: None,
            model_path: None,
            input_contract: "image".into(),
            output_contract: "searchable_text".into(),
            enabled: true,
            priority: 10,
            revision: 1,
            is_builtin: false,
            is_available: true,
            unavailable_reason: None,
            runtime: crate::content_extraction::runtime_status_for("fake-engine-v1", None),
            defaults: None,
        };
        let engine = DisablingEngine { db: &db };
        let engines: [&dyn crate::content_extraction::ExtractorEngine; 1] = [&engine];
        let registry = crate::content_extraction::ExtractorEngineRegistry::new(&engines);

        execute_task(
            &db,
            OcrTask {
                clip_id: clip.id,
                content_hash: clip.content_hash.clone(),
                image_bytes: b"image".to_vec(),
            },
            &extractor,
            &registry,
            |_| notified.store(true, Ordering::Release),
        );

        assert!(!notified.load(Ordering::Acquire));
        let stored = db.get_clip_by_id(clip.id).unwrap();
        assert_eq!(stored.text_content, None);
        let status = db.get_ocr_backfill_status().unwrap();
        assert_eq!(status.eligible_count, 1);
        assert_eq!(status.running_count, 0);
    }

    #[test]
    fn extractor_failures_are_persisted_as_failed_attempts() {
        struct FailingEngine {
            code: String,
        }

        impl crate::content_extraction::ExtractorEngine for FailingEngine {
            fn id(&self) -> &'static str {
                "fake-engine-v1"
            }

            fn availability(&self) -> crate::content_extraction::EngineAvailability {
                crate::content_extraction::EngineAvailability {
                    is_available: true,
                    unavailable_reason: None,
                }
            }

            fn extract(&self, _image_bytes: &[u8]) -> crate::content_extraction::ExtractionOutcome {
                crate::content_extraction::ExtractionOutcome::Failed {
                    failure: crate::content_extraction::ExtractionFailure {
                        code: self.code.clone(),
                        message: "The test engine failed.".into(),
                    },
                }
            }
        }

        let db = setup_test_db();
        let clip = db
            .save_clip(
                "image",
                None,
                None,
                Some(crate::resource_limits::TEST_PNG_DATA_URL),
                "failed-ocr",
                "Screenshot",
            )
            .unwrap();
        let extractor = crate::content_extraction::Extractor {
            id: 1,
            stable_ref: "extractor:test".into(),
            name: "Test OCR".into(),
            description: String::new(),
            engine: "fake-engine-v1".into(),
            executable_path: None,
            model_path: None,
            input_contract: "image".into(),
            output_contract: "searchable_text".into(),
            enabled: true,
            priority: 10,
            revision: 1,
            is_builtin: false,
            is_available: true,
            unavailable_reason: None,
            runtime: crate::content_extraction::runtime_status_for("fake-engine-v1", None),
            defaults: None,
        };
        let engine = FailingEngine {
            code: "engine_failed".into(),
        };
        let engines: [&dyn crate::content_extraction::ExtractorEngine; 1] = [&engine];
        let registry = crate::content_extraction::ExtractorEngineRegistry::new(&engines);

        execute_task(
            &db,
            OcrTask {
                clip_id: clip.id,
                content_hash: clip.content_hash,
                image_bytes: b"image".to_vec(),
            },
            &extractor,
            &registry,
            |_| {},
        );

        let status = db.get_ocr_backfill_status().unwrap();
        assert_eq!(status.failed_count, 1);
        assert_eq!(status.no_text_count, 0);
    }

    #[test]
    fn persistence_errors_reset_running_ocr_work() {
        struct InvalidFailureEngine;

        impl crate::content_extraction::ExtractorEngine for InvalidFailureEngine {
            fn id(&self) -> &'static str {
                "fake-engine-v1"
            }

            fn availability(&self) -> crate::content_extraction::EngineAvailability {
                crate::content_extraction::EngineAvailability {
                    is_available: true,
                    unavailable_reason: None,
                }
            }

            fn extract(&self, _image_bytes: &[u8]) -> crate::content_extraction::ExtractionOutcome {
                crate::content_extraction::ExtractionOutcome::Failed {
                    failure: crate::content_extraction::ExtractionFailure {
                        code: "x".repeat(161),
                        message: "The test engine returned invalid failure metadata.".into(),
                    },
                }
            }
        }

        let db = setup_test_db();
        let clip = db
            .save_clip(
                "image",
                None,
                None,
                Some(crate::resource_limits::TEST_PNG_DATA_URL),
                "invalid-failure-metadata",
                "Screenshot",
            )
            .unwrap();
        let extractor = crate::content_extraction::Extractor {
            id: 1,
            stable_ref: "extractor:test".into(),
            name: "Test OCR".into(),
            description: String::new(),
            engine: "fake-engine-v1".into(),
            executable_path: None,
            model_path: None,
            input_contract: "image".into(),
            output_contract: "searchable_text".into(),
            enabled: true,
            priority: 10,
            revision: 1,
            is_builtin: false,
            is_available: true,
            unavailable_reason: None,
            runtime: crate::content_extraction::runtime_status_for("fake-engine-v1", None),
            defaults: None,
        };
        let engine = InvalidFailureEngine;
        let engines: [&dyn crate::content_extraction::ExtractorEngine; 1] = [&engine];
        let registry = crate::content_extraction::ExtractorEngineRegistry::new(&engines);

        execute_task(
            &db,
            OcrTask {
                clip_id: clip.id,
                content_hash: clip.content_hash,
                image_bytes: b"image".to_vec(),
            },
            &extractor,
            &registry,
            |_| {},
        );

        let status = db.get_ocr_backfill_status().unwrap();
        assert_eq!(status.eligible_count, 1);
        assert_eq!(status.running_count, 0);
        assert_eq!(status.failed_count, 0);
    }
}
