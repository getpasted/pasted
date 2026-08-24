use super::*;

fn unavailable_vision_recipe() -> (ExtractorRecipe, ExtractorDiagnosticReport) {
    let mut recipe = crate::content_extraction::EXTRACTOR_PRESETS[0].recipe();
    recipe.resources = vec![crate::extractor_recipe::ExtractorResource {
        id: "model".into(),
        label: "Multimodal GGUF language model".into(),
        kind: crate::extractor_recipe::ExtractorResourceKind::File,
        required: true,
        path: None,
    }];
    let diagnostic = crate::extractor_recipe::diagnose(&recipe);
    (recipe, diagnostic)
}

#[test]
fn rejects_vague_resource_selection_instructions() {
    let (recipe, diagnostic) = unavailable_vision_recipe();
    let guidance = vec!["Select a local multimodal GGUF language model file.".into()];
    assert_eq!(precision_issues(&recipe, &diagnostic, &guidance).len(), 1);
}

#[test]
fn accepts_a_named_resource_with_a_direct_artifact_url() {
    let (recipe, diagnostic) = unavailable_vision_recipe();
    let guidance = vec![
        "Download Multimodal GGUF language model from https://example.com/models/vision-q4.gguf and select it for that Pasted resource.".into(),
    ];
    assert!(precision_issues(&recipe, &diagnostic, &guidance).is_empty());
}
