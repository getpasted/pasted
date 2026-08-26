use super::*;
use crate::extractor_recipe::ExtractorPostProcessing;

pub fn migrate_builtin_recipe_compatibility(
    stable_ref: &str,
    current: &ExtractorRecipe,
    legacy_model_path: Option<&str>,
) -> ExtractorRecipe {
    let mut migrated = current.clone();

    if let Some(minimum_percent) = migrated.legacy_minimum_visual_label_confidence.take() {
        if matches!(stable_ref, APPLE_VISION_LABELS_REF | LLAMA_CPP_LABELS_REF)
            && migrated.post_processing.is_empty()
        {
            migrated.post_processing =
                vec![ExtractorPostProcessing::FilterLabelsByConfidence { minimum_percent }];
        }
    }

    if stable_ref == APPLE_VISION_OCR_REF {
        for step in &mut migrated.steps {
            let uses_bundled_helper = step
                .arguments
                .windows(2)
                .any(|arguments| arguments == ["--pasted-extractor-helper-v1", "apple-vision-ocr"]);
            if uses_bundled_helper && step.executable.discover == ["pasted"] {
                step.executable.discover = vec![BUNDLED_EXTRACTOR_EXECUTABLE.into()];
                step.executable.version_arguments.clear();
            }
        }
    }

    if stable_ref == WHISPER_TRANSCRIPTION_REF {
        let model_path = migrated
            .resources
            .iter()
            .find(|resource| resource.id == "model")
            .and_then(|resource| resource.path.clone())
            .or_else(|| legacy_model_path.map(str::to_string));
        let interim_recipe = migrated.steps.len() == 1
            && migrated.steps[0].executable.discover == ["whisper-cli"]
            && migrated.steps[0].arguments
                == [
                    "--model",
                    "{resource.model.path}",
                    "--file",
                    "{input.path}",
                    "--no-timestamps",
                ];
        if interim_recipe {
            let whisper_path = migrated.steps[0].executable.path.clone();
            migrated = EXTRACTOR_PRESETS
                .iter()
                .find(|preset| preset.stable_ref == WHISPER_TRANSCRIPTION_REF)
                .expect("shipped Whisper Extractor")
                .recipe();
            if let Some(step) = migrated
                .steps
                .iter_mut()
                .find(|step| step.id == "transcribe")
            {
                step.executable.path = whisper_path;
            }
        }
        if let Some(resource) = migrated
            .resources
            .iter_mut()
            .find(|resource| resource.id == "model" && resource.path.is_none())
        {
            resource.path = model_path;
        }
    }

    migrated
}
