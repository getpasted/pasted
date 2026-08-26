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

#[test]
fn modified_shipped_label_recipes_migrate_the_legacy_filter() {
    let mut legacy = recipe(APPLE_VISION_LABELS_REF);
    legacy.post_processing.clear();
    legacy.legacy_minimum_visual_label_confidence = Some(72);
    legacy.steps[0].executable.path = Some("/custom/vision-helper".into());

    let migrated = migrate_builtin_recipe_compatibility(APPLE_VISION_LABELS_REF, &legacy, None);

    assert_eq!(
        migrated.post_processing,
        [ExtractorPostProcessing::FilterLabelsByConfidence {
            minimum_percent: 72,
        }]
    );
    assert_eq!(migrated.legacy_minimum_visual_label_confidence, None);
    assert_eq!(
        migrated.steps[0].executable.path.as_deref(),
        Some("/custom/vision-helper")
    );
}
