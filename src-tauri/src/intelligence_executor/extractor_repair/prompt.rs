use super::*;

pub(super) fn repair_prompt(
    name: &str,
    description: &str,
    recipe: &crate::extractor_recipe::ExtractorRecipe,
    diagnostic: &crate::extractor_recipe::ExtractorDiagnosticReport,
    prior_guidance: &[String],
    guidance_issues: &[String],
    user_prompt: Option<&str>,
) -> Result<String, IntelligenceExecutionError> {
    let recipe = serde_json::to_string(&crate::extractor_recipe::without_local_paths(recipe))
        .map_err(|error| {
            IntelligenceExecutionError::new("recipe_serialization_failed", error.to_string())
        })?;
    let diagnostic = serde_json::to_string(diagnostic).map_err(|error| {
        IntelligenceExecutionError::new("diagnostic_serialization_failed", error.to_string())
    })?;
    let prior_guidance = serde_json::to_string(prior_guidance).map_err(|error| {
        IntelligenceExecutionError::new("guidance_serialization_failed", error.to_string())
    })?;
    let guidance_issues = serde_json::to_string(guidance_issues).map_err(|error| {
        IntelligenceExecutionError::new("guidance_serialization_failed", error.to_string())
    })?;
    Ok(format!(
        "Repair an existing local Pasted Extractor for the reported host. Prefer tools already discoverable on that host. Setup instructions must be directly followable without outside research: provide the exact install command or canonical HTTPS download URL, exact artifact filename, and exact named Pasted resource to select. For paired model files, link and name one verified compatible pair. Never say only to install, download, find, or select something. Preserve null local paths and never invent a path or claim setup succeeded. Do not add shell operators, network access at extraction time, or implicit installation. Return the complete revised recipe.\nUser request: {}\nName: {}\nDescription: {}\nCurrent recipe: {}\nStructured preflight: {}\nRejected setup guidance: {}\nGuidance rejection reasons: {}",
        user_prompt.unwrap_or("Make this Extractor available."), name, description, recipe,
        diagnostic, prior_guidance, guidance_issues,
    ))
}
