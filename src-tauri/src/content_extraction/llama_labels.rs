use super::*;
use crate::extractor_recipe::{ExtractorPostProcessing, DEFAULT_LABEL_CONFIDENCE_PERCENT};

const MODEL_REPOSITORY: &str = "ggml-org/SmolVLM-500M-Instruct-GGUF";
const LABEL_PROMPT: &str = "Identify the visible subjects, objects, animals, foods, places, and other useful searchable concepts. Return concise plain-language labels, no duplicates, and a confidenceBasisPoints value from 0 to 10000 for each label.";
const LABEL_SCHEMA: &str = r#"{"type":"object","properties":{"text":{"type":"null"},"labels":{"type":"array","maxItems":32,"items":{"type":"object","properties":{"value":{"type":"string"},"confidenceBasisPoints":{"type":"integer","minimum":0,"maximum":10000}},"required":["value","confidenceBasisPoints"],"additionalProperties":false}}},"required":["text","labels"],"additionalProperties":false}"#;

pub(super) fn recipe() -> ExtractorRecipe {
    ExtractorRecipe {
        definition_version: EXTRACTOR_RECIPE_VERSION,
        accepts: vec![ExtractorInputKind::Image],
        accepted_file_formats: format_defaults::for_builtin(LLAMA_CPP_LABELS_REF),
        post_processing: vec![ExtractorPostProcessing::FilterLabelsByConfidence {
            minimum_percent: DEFAULT_LABEL_CONFIDENCE_PERCENT,
        }],
        legacy_minimum_visual_label_confidence: None,
        output: ExtractorOutputKind::SearchableText,
        steps: vec![ExtractorCommandStep {
            id: "label".into(),
            executable: ExtractorExecutable {
                path: None,
                discover: vec!["llama-cli".into()],
                version_arguments: vec!["--version".into()],
            },
            arguments: vec![
                "-hf".into(),
                MODEL_REPOSITORY.into(),
                "--offline".into(),
                "--image".into(),
                "{input.path}".into(),
                "--prompt".into(),
                LABEL_PROMPT.into(),
                "--json-schema".into(),
                LABEL_SCHEMA.into(),
                "--single-turn".into(),
                "--simple-io".into(),
                "--no-display-prompt".into(),
                "--no-show-timings".into(),
                "--reasoning".into(),
                "off".into(),
                "--temp".into(),
                "0".into(),
                "--seed".into(),
                "0".into(),
                "-n".into(),
                "384".into(),
            ],
            mode: ExtractorStepMode::EachInput,
            capture: ExtractorCapture::PastedJsonV1,
            output_extension: None,
            no_output_exit_codes: Vec::new(),
            timeout_seconds: 180,
        }],
        resources: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractor_recipe::validate_recipe;

    #[test]
    fn recipe_uses_official_multimodal_model_and_bounded_json() {
        let recipe = recipe();
        assert_eq!(
            recipe.post_processing,
            [ExtractorPostProcessing::FilterLabelsByConfidence {
                minimum_percent: 80,
            }]
        );
        assert_eq!(recipe.steps[0].executable.discover, ["llama-cli"]);
        assert!(recipe.steps[0]
            .arguments
            .windows(2)
            .any(|pair| { pair == ["-hf", MODEL_REPOSITORY] }));
        assert!(recipe.steps[0].arguments.contains(&"--offline".into()));
        assert!(recipe.steps[0].arguments.contains(&"--json-schema".into()));
        assert!(recipe.accepts(ExtractorInputKind::Image));
        assert_eq!(recipe.accepted_file_formats, ["bmp", "jpg", "png", "webp"]);
        validate_recipe(&recipe).unwrap();
    }
}
