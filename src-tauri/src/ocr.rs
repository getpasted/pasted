// Native macOS Vision OCR Engine using Apple's Vision Framework (VNRecognizeTextRequest)
// Runs in-process with 0 external CLI dependencies, hardware-accelerated.

#[cfg(target_os = "macos")]
pub fn perform_ocr_on_image_bytes(image_bytes: &[u8]) -> Option<String> {
    use objc::runtime::{Class, Object};
    use objc::{msg_send, sel, sel_impl};
    use std::ptr::null_mut;

    type Id = *mut Object;

    if image_bytes.is_empty() {
        return None;
    }

    unsafe {
        // Create NSData from raw image bytes
        let ns_data_class = Class::get("NSData")?;
        let ns_data: Id = msg_send![ns_data_class, dataWithBytes:image_bytes.as_ptr() length:image_bytes.len()];
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
                            let utf8: *const std::os::raw::c_char = msg_send![string_val, UTF8String];
                            if !utf8.is_null() {
                                if let Ok(s) = std::ffi::CStr::from_ptr(utf8).to_str() {
                                    let trimmed = s.trim();
                                    if !trimmed.is_empty() {
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
    pub image_bytes: Vec<u8>,
}

pub fn spawn_ocr_worker(
    app: tauri::AppHandle,
    db_state: std::sync::Arc<crate::db::DbState>,
) -> std::sync::mpsc::Sender<OcrTask> {
    use tauri::Emitter;
    let (tx, rx) = std::sync::mpsc::channel::<OcrTask>();
    std::thread::spawn(move || {
        while let Ok(task) = rx.recv() {
            if let Some(ocr_text) = perform_ocr_on_image_bytes(&task.image_bytes) {
                if !ocr_text.trim().is_empty() {
                    let _ = db_state.update_clip_text(task.clip_id, &ocr_text);
                    let _ = app.emit("clip-added", serde_json::json!({ "id": task.clip_id }));
                }
            }
        }
    });
    tx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocr_empty_or_invalid_bytes() {
        assert_eq!(perform_ocr_on_image_bytes(&[]), None);
        assert_eq!(perform_ocr_on_image_bytes(&[0, 1, 2, 3, 4]), None);
    }

    #[test]
    fn test_ocr_task_struct() {
        let task = OcrTask {
            clip_id: 42,
            image_bytes: vec![1, 2, 3],
        };
        assert_eq!(task.clip_id, 42);
        assert_eq!(task.image_bytes.len(), 3);
    }
}
