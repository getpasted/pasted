use super::*;
use crate::extractor_recipe::{ExtractorPostProcessing, DEFAULT_LABEL_CONFIDENCE_PERCENT};

fn recipe(stable_ref: &str) -> ExtractorRecipe {
    EXTRACTOR_PRESETS
        .iter()
        .find(|preset| preset.stable_ref == stable_ref)
        .expect("shipped Extractor")
        .recipe()
}

#[test]
fn only_shipped_label_recipes_declare_confidence_filtering() {
    for stable_ref in [APPLE_VISION_LABELS_REF, LLAMA_CPP_LABELS_REF] {
        assert_eq!(
            recipe(stable_ref).post_processing,
            [ExtractorPostProcessing::FilterLabelsByConfidence {
                minimum_percent: DEFAULT_LABEL_CONFIDENCE_PERCENT,
            }]
        );
    }
    for stable_ref in [
        APPLE_VISION_OCR_REF,
        TESSERACT_OCR_REF,
        WHISPER_TRANSCRIPTION_REF,
    ] {
        assert!(recipe(stable_ref).post_processing.is_empty());
    }
}
