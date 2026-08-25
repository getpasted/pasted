use crate::extractor_recipe::{
    ExtractorDiagnosticCode, ExtractorDiagnosticReport, ExtractorRecipe,
};

mod actions;
mod matching;
use actions::{has_direct_artifact_url, has_install_action};
use matching::matching_items;

pub(super) fn precision_issues(
    recipe: &ExtractorRecipe,
    diagnostic: &ExtractorDiagnosticReport,
    guidance: &[String],
) -> Vec<String> {
    let normalized = guidance
        .iter()
        .map(|item| item.to_lowercase())
        .collect::<Vec<_>>();
    let mut issues = Vec::new();
    for issue in &diagnostic.issues {
        match issue.code {
            ExtractorDiagnosticCode::ExecutableNotConfigured
            | ExtractorDiagnosticCode::ExecutableUnavailable => {
                let names = recipe
                    .steps
                    .iter()
                    .find(|step| step.id == issue.subject_id)
                    .map(|step| step.executable.discover.as_slice())
                    .unwrap_or_default();
                let matching = matching_items(&normalized, &issue.label, names);
                if !matching.iter().any(|item| has_install_action(item)) {
                    issues.push(format!(
                        "Executable '{}' needs an exact install command or direct HTTPS artifact URL.",
                        issue.label
                    ));
                }
            }
            ExtractorDiagnosticCode::ResourceNotConfigured
            | ExtractorDiagnosticCode::ResourceUnavailable => {
                let matching = matching_items(&normalized, &issue.label, &[&issue.subject_id]);
                if !matching.iter().any(|item| has_direct_artifact_url(item)) {
                    issues.push(format!(
                        "Resource '{}' needs a canonical HTTPS URL ending in an exact artifact filename and must name this Pasted resource.",
                        issue.label
                    ));
                }
            }
            ExtractorDiagnosticCode::InvalidRecipe => {}
        }
    }
    issues
}

#[cfg(test)]
mod tests;
