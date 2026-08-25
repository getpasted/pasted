use super::*;

#[test]
fn shipped_extractors_declare_logical_inputs_and_file_formats() {
    assert_eq!(
        format_defaults::for_builtin(WHISPER_TRANSCRIPTION_REF),
        ["aac", "flac", "m4a", "mp3", "ogg", "wav"]
    );
    for stable_ref in [
        APPLE_VISION_OCR_REF,
        APPLE_VISION_LABELS_REF,
        TESSERACT_OCR_REF,
    ] {
        let accepted = format_defaults::for_builtin(stable_ref);
        assert!(accepted.contains(&"png".to_string()));
        assert!(accepted.contains(&"jpg".to_string()));
        assert_eq!(
            accepted.contains(&"heif".to_string()),
            matches!(stable_ref, APPLE_VISION_OCR_REF | APPLE_VISION_LABELS_REF)
        );
        assert!(!accepted.contains(&"pdf".to_string()));
        let recipe = EXTRACTOR_PRESETS
            .iter()
            .find(|preset| preset.stable_ref == stable_ref)
            .unwrap()
            .recipe();
        assert!(recipe.accepts(ExtractorInputKind::Image));
        assert!(recipe.accepts(ExtractorInputKind::FileReferences));
    }
}
