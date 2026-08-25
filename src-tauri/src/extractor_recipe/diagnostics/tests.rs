use super::*;

#[test]
fn reports_every_missing_dependency_without_exposing_paths() {
    let mut recipe = crate::content_extraction::EXTRACTOR_PRESETS[0].recipe();
    recipe.steps[0].executable.path = Some("/Users/private/bin/missing-tool".into());
    recipe.steps[0].executable.discover = vec!["definitely-not-a-pasted-tool".into()];
    recipe.resources = vec![super::super::ExtractorResource {
        id: "model".into(),
        label: "Vision model".into(),
        kind: super::super::ExtractorResourceKind::File,
        required: true,
        path: None,
    }];
    let report = diagnose(&recipe);
    assert!(!report.is_available);
    assert_eq!(report.issues.len(), 2);
    assert_eq!(
        report.issues[0].code,
        ExtractorDiagnosticCode::ExecutableUnavailable
    );
    assert_eq!(
        report.issues[1].code,
        ExtractorDiagnosticCode::ResourceNotConfigured
    );
    assert!(!serde_json::to_string(&report)
        .unwrap()
        .contains("/Users/private"));
}
