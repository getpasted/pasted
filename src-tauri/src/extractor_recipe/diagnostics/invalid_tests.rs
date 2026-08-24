use super::*;

#[test]
fn invalid_recipes_still_report_unconfigured_dependencies() {
    let mut recipe = crate::content_extraction::EXTRACTOR_PRESETS[0].recipe();
    recipe.steps[0].executable.path = None;
    recipe.steps[0].executable.discover.clear();
    let report = diagnose(&recipe);
    assert_eq!(
        report.issues[0].code,
        ExtractorDiagnosticCode::InvalidRecipe
    );
    assert_eq!(
        report.issues[1].code,
        ExtractorDiagnosticCode::ExecutableNotConfigured
    );
}
