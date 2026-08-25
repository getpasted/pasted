use crate::content_extraction::VisualLabel;

#[cfg(target_os = "macos")]
pub(crate) fn perform(image_bytes: &[u8]) -> Vec<VisualLabel> {
    use objc::runtime::{Class, Object};
    use objc::{msg_send, sel, sel_impl};
    use std::ptr::null_mut;

    type Id = *mut Object;

    if image_bytes.is_empty() || image_bytes.len() > crate::resource_limits::MAX_ENCODED_IMAGE_BYTES
    {
        return Vec::new();
    }

    unsafe {
        let Some(ns_data_class) = Class::get("NSData") else {
            return Vec::new();
        };
        let ns_data: Id =
            msg_send![ns_data_class, dataWithBytes:image_bytes.as_ptr() length:image_bytes.len()];
        let Some(ns_image_class) = Class::get("NSImage") else {
            return Vec::new();
        };
        let ns_image: Id = msg_send![ns_image_class, alloc];
        let ns_image: Id = msg_send![ns_image, initWithData: ns_data];
        if ns_image.is_null() {
            return Vec::new();
        }
        let cg_image: Id = msg_send![
            ns_image,
            CGImageForProposedRect: null_mut::<Object>()
            context: null_mut::<Object>()
            hints: null_mut::<Object>()
        ];
        let Some(handler_class) = Class::get("VNImageRequestHandler") else {
            return Vec::new();
        };
        let handler: Id = msg_send![handler_class, alloc];
        let handler: Id = msg_send![handler, initWithCGImage:cg_image options:null_mut::<Object>()];
        let Some(request_class) = Class::get("VNClassifyImageRequest") else {
            return Vec::new();
        };
        let request: Id = msg_send![request_class, new];
        let Some(array_class) = Class::get("NSArray") else {
            return Vec::new();
        };
        let requests: Id = msg_send![array_class, arrayWithObject: request];
        let mut error: Id = null_mut();
        let success: bool = msg_send![handler, performRequests: requests error: &mut error];
        if !success {
            return Vec::new();
        }
        observations(msg_send![request, results])
    }
}

#[cfg(target_os = "macos")]
unsafe fn observations(results: *mut objc::runtime::Object) -> Vec<VisualLabel> {
    use objc::runtime::Object;
    use objc::{msg_send, sel, sel_impl};

    type Id = *mut Object;
    if results.is_null() {
        return Vec::new();
    }
    let count: usize = msg_send![results, count];
    let mut labels = Vec::new();
    for index in 0..count {
        let observation: Id = msg_send![results, objectAtIndex: index];
        let confidence: f32 = msg_send![observation, confidence];
        let meets_search_threshold: bool =
            msg_send![observation, hasMinimumPrecision: 0.1f32 forRecall: 0.8f32];
        if !meets_search_threshold {
            continue;
        }
        let identifier: Id = msg_send![observation, identifier];
        let utf8: *const std::os::raw::c_char = msg_send![identifier, UTF8String];
        if utf8.is_null() {
            continue;
        }
        if let Ok(value) = std::ffi::CStr::from_ptr(utf8).to_str() {
            labels.push(VisualLabel {
                value: plain_language_identifier(value),
                confidence_basis_points: Some((confidence * 10_000.0).round() as u16),
            });
        }
    }
    crate::content_extraction::normalize_visual_labels(labels)
}

fn plain_language_identifier(identifier: &str) -> String {
    identifier
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn perform(_image_bytes: &[u8]) -> Vec<VisualLabel> {
    Vec::new()
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    #[test]
    fn accepts_the_shipped_app_icon() {
        let labels = super::perform(include_bytes!("../../../../app-icon.png"));
        assert!(labels.len() <= crate::content_extraction::visual_labels::MAX_VISUAL_LABELS);
        assert!(labels.iter().all(|label| !label.value.trim().is_empty()));
    }
}

#[cfg(test)]
mod identifier_tests {
    #[test]
    fn converts_apple_taxonomy_identifiers_to_plain_language() {
        assert_eq!(
            super::plain_language_identifier("jack_russell_terrier"),
            "jack russell terrier"
        );
        assert_eq!(
            super::plain_language_identifier("optical__equipment"),
            "optical equipment"
        );
        assert_eq!(super::plain_language_identifier("dog"), "dog");
    }
}
