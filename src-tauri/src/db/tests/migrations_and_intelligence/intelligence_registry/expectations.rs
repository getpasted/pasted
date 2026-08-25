pub(super) fn assert_active_image_extractor(actual: Option<&str>, tesseract_available: bool) {
    let expected = if cfg!(target_os = "macos") {
        Some(crate::content_extraction::APPLE_VISION_LABELS_REF)
    } else if tesseract_available {
        Some(crate::content_extraction::TESSERACT_OCR_REF)
    } else {
        None
    };
    assert_eq!(actual, expected);
}
