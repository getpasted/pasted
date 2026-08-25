use super::*;

mod labels;
mod request;

pub(crate) use labels::perform as perform_apple_vision_labels;

pub(super) struct AppleVisionOcrEngine;

impl ExtractorEngine for AppleVisionOcrEngine {
    fn id(&self) -> &'static str {
        APPLE_VISION_ENGINE
    }

    fn availability(&self) -> EngineAvailability {
        if cfg!(target_os = "macos") {
            EngineAvailability {
                is_available: true,
                unavailable_reason: None,
            }
        } else {
            EngineAvailability {
                is_available: false,
                unavailable_reason: Some("Apple Vision is available only on macOS.".into()),
            }
        }
    }

    fn extract(&self, image_bytes: &[u8]) -> ExtractionOutcome {
        perform_apple_vision_ocr(image_bytes)
            .filter(|text| !text.trim().is_empty())
            .map_or(ExtractionOutcome::NoOutput, |text| {
                ExtractionOutcome::Produced {
                    text,
                    labels: Vec::new(),
                }
            })
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn perform_apple_vision_ocr(image_bytes: &[u8]) -> Option<String> {
    use objc::runtime::{Class, Object};
    use objc::{msg_send, sel, sel_impl};
    use std::ptr::null_mut;

    type Id = *mut Object;

    if image_bytes.is_empty() || image_bytes.len() > crate::resource_limits::MAX_ENCODED_IMAGE_BYTES
    {
        return None;
    }

    unsafe {
        let ns_data_class = Class::get("NSData")?;
        let ns_data: Id =
            msg_send![ns_data_class, dataWithBytes:image_bytes.as_ptr() length:image_bytes.len()];
        if ns_data.is_null() {
            return None;
        }
        let ns_image_class = Class::get("NSImage")?;
        let ns_image: Id = msg_send![ns_image_class, alloc];
        let ns_image: Id = msg_send![ns_image, initWithData: ns_data];
        if ns_image.is_null() {
            return None;
        }
        let cg_image: Id = msg_send![
            ns_image,
            CGImageForProposedRect: null_mut::<Object>()
            context: null_mut::<Object>()
            hints: null_mut::<Object>()
        ];
        if cg_image.is_null() {
            return None;
        }
        let handler_class = Class::get("VNImageRequestHandler")?;
        let handler: Id = msg_send![handler_class, alloc];
        let handler: Id = msg_send![handler, initWithCGImage:cg_image options:null_mut::<Object>()];
        if handler.is_null() {
            return None;
        }
        let request_class = Class::get("VNRecognizeTextRequest")?;
        let request: Id = msg_send![request_class, alloc];
        let request: Id = msg_send![request, init];
        if request.is_null() {
            return None;
        }
        let _: () = msg_send![request, setRecognitionLevel: 1i64];
        let array_class = Class::get("NSArray")?;
        let requests: Id = msg_send![array_class, arrayWithObject: request];
        let mut error: Id = null_mut();
        let success: bool = msg_send![handler, performRequests: requests error: &mut error];
        if !success {
            return None;
        }
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
            if top_candidates.is_null() {
                continue;
            }
            let candidate_count: usize = msg_send![top_candidates, count];
            if candidate_count == 0 {
                continue;
            }
            let candidate: Id = msg_send![top_candidates, objectAtIndex: 0usize];
            if candidate.is_null() {
                continue;
            }
            let string_value: Id = msg_send![candidate, string];
            if string_value.is_null() {
                continue;
            }
            let utf8: *const std::os::raw::c_char = msg_send![string_value, UTF8String];
            if utf8.is_null() {
                continue;
            }
            if let Ok(value) = std::ffi::CStr::from_ptr(utf8).to_str() {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    recognized_bytes = recognized_bytes
                        .saturating_add(trimmed.len())
                        .saturating_add(1);
                    if recognized_bytes > crate::resource_limits::MAX_OCR_TEXT_BYTES {
                        return None;
                    }
                    lines.push(trimmed.to_string());
                }
            }
        }
        (!lines.is_empty()).then(|| lines.join("\n"))
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn perform_apple_vision_ocr(_image_bytes: &[u8]) -> Option<String> {
    None
}

pub fn run_bundled_extractor_helper(arguments: &[String]) -> Option<i32> {
    let marker = arguments
        .iter()
        .position(|argument| argument == "--pasted-extractor-helper-v1")?;
    let method = arguments.get(marker + 1).map(String::as_str);
    let request_path = arguments.get(marker + 2).map(Path::new);
    let result = match (method, request_path) {
        (Some(method @ ("apple-vision-ocr" | "apple-vision-labels")), Some(request_path)) => {
            request::read_images(request_path).map_or_else(
                || Err("invalid_input"),
                |images| {
                    if method == "apple-vision-labels" {
                        let labels = crate::content_extraction::normalize_visual_labels(
                            images
                                .iter()
                                .flat_map(|image| perform_apple_vision_labels(image))
                                .collect(),
                        );
                        Ok(serde_json::json!({ "text": null, "labels": labels }))
                    } else {
                        let text = images
                            .iter()
                            .filter_map(|image| perform_apple_vision_ocr(image))
                            .collect::<Vec<_>>()
                            .join("\n");
                        Ok(serde_json::json!({ "text": text }))
                    }
                },
            )
        }
        _ => Err("unsupported_helper"),
    };
    match result {
        Ok(value) => match serde_json::to_string(&value) {
            Ok(output) => {
                println!("{output}");
                Some(0)
            }
            Err(_) => Some(1),
        },
        Err(code) => {
            eprintln!("{code}");
            Some(2)
        }
    }
}
