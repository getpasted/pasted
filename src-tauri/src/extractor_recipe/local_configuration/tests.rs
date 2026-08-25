use super::*;

#[test]
fn removes_every_local_path_before_external_authoring() {
    let mut recipe = crate::content_extraction::EXTRACTOR_PRESETS[0].recipe();
    recipe.steps[0].executable.path = Some("/private/tool".into());
    recipe.resources = vec![super::super::ExtractorResource {
        id: "model".into(),
        label: "Model".into(),
        kind: super::super::ExtractorResourceKind::File,
        required: true,
        path: Some("/private/model".into()),
    }];
    let redacted = without_local_paths(&recipe);
    assert!(redacted
        .steps
        .iter()
        .all(|step| step.executable.path.is_none()));
    assert!(redacted
        .resources
        .iter()
        .all(|resource| resource.path.is_none()));
}
