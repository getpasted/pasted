use super::*;

mod prompt;
mod setup_guidance;

use prompt::repair_prompt;

const DEFAULT_REPAIR_ATTEMPTS: u8 = 3;
const MAX_REPAIR_ATTEMPTS: u8 = 3;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairExtractorRecipeRequest {
    pub name: String,
    pub description: String,
    pub recipe: crate::extractor_recipe::ExtractorRecipe,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub connection_id: Option<String>,
    #[serde(default)]
    pub max_attempts: Option<u8>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractorRepairStatus {
    Ready,
    SetupRequired,
    GuidanceIncomplete,
}

impl ExtractorRepairStatus {
    pub const fn stable_name(&self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::SetupRequired => "setup_required",
            Self::GuidanceIncomplete => "guidance_incomplete",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractorRepairOutcome {
    pub name: String,
    pub description: String,
    pub recipe: crate::extractor_recipe::ExtractorRecipe,
    pub setup_guidance: Vec<String>,
    pub authoring: crate::extractor_recipe::ExtractorAuthoringManifest,
    pub diagnostic: crate::extractor_recipe::ExtractorDiagnosticReport,
    pub status: ExtractorRepairStatus,
    pub attempts: u8,
    pub connection_id: String,
    pub connection_name: String,
    pub duration_ms: i64,
}

pub fn repair_extractor_recipe(
    db: &DbState,
    request: RepairExtractorRecipeRequest,
    cancellation: Option<&AtomicBool>,
) -> Result<ExtractorRepairOutcome, IntelligenceExecutionError> {
    let max_attempts = request
        .max_attempts
        .unwrap_or(DEFAULT_REPAIR_ATTEMPTS)
        .clamp(1, MAX_REPAIR_ATTEMPTS);
    let mut name = request.name;
    let mut description = request.description;
    let mut recipe = request.recipe;
    let mut diagnostic = crate::extractor_recipe::diagnose(&recipe);
    let mut setup_guidance = Vec::new();
    let mut guidance_issues = Vec::new();
    let mut messages = vec![authoring_message(
        crate::extractor_recipe::ExtractorAuthoringRole::User,
        request
            .prompt
            .as_deref()
            .unwrap_or("Diagnose this Extractor and make it available on this system."),
        None,
    )];
    let mut connection_id = String::new();
    let mut connection_name = String::new();
    let mut provider = None;
    let mut model = None;
    let mut duration_ms = 0;
    let mut attempts = 0;

    while !diagnostic.is_available && attempts < max_attempts {
        attempts += 1;
        let diagnostic_value = serde_json::to_value(&diagnostic).map_err(|error| {
            IntelligenceExecutionError::new("diagnostic_serialization_failed", error.to_string())
        })?;
        messages.push(authoring_message(
            crate::extractor_recipe::ExtractorAuthoringRole::Tool,
            "Extractor preflight found unavailable dependencies.",
            Some(diagnostic_value),
        ));
        let prior_recipe = recipe.clone();
        let proposal = super::propose_extractor_recipe(
            db,
            super::ProposeExtractorRecipeRequest {
                prompt: repair_prompt(
                    &name,
                    &description,
                    &recipe,
                    &diagnostic,
                    &setup_guidance,
                    &guidance_issues,
                    request.prompt.as_deref(),
                )?,
                connection_id: request.connection_id.clone(),
            },
            cancellation,
        )?;
        connection_id = proposal.connection_id.clone();
        connection_name = proposal.connection_name.clone();
        provider = proposal.authoring.provider.clone();
        model = proposal.authoring.model.clone();
        duration_ms += proposal.duration_ms;
        if let Some(message) = proposal.authoring.messages.last().cloned() {
            messages.push(message);
        }
        name = proposal.name;
        description = proposal.description;
        recipe =
            crate::extractor_recipe::reset_preserving_local_paths(&prior_recipe, &proposal.recipe);
        setup_guidance = proposal.setup_guidance;
        diagnostic = crate::extractor_recipe::diagnose(&recipe);
        guidance_issues = setup_guidance::precision_issues(&recipe, &diagnostic, &setup_guidance);
        if diagnostic.is_available || guidance_issues.is_empty() {
            break;
        }
    }

    Ok(ExtractorRepairOutcome {
        name,
        description,
        recipe,
        setup_guidance: if guidance_issues.is_empty() {
            setup_guidance
        } else {
            Vec::new()
        },
        authoring: crate::extractor_recipe::ExtractorAuthoringManifest {
            manifest_version: crate::extractor_recipe::EXTRACTOR_AUTHORING_VERSION,
            source: crate::extractor_recipe::ExtractorAuthoringSource::Ai,
            original_prompt: request.prompt,
            provider,
            model,
            messages,
        },
        status: if diagnostic.is_available {
            ExtractorRepairStatus::Ready
        } else if guidance_issues.is_empty() {
            ExtractorRepairStatus::SetupRequired
        } else {
            ExtractorRepairStatus::GuidanceIncomplete
        },
        diagnostic,
        attempts,
        connection_id,
        connection_name,
        duration_ms,
    })
}

#[cfg(test)]
mod tests;

fn authoring_message(
    role: crate::extractor_recipe::ExtractorAuthoringRole,
    content: impl Into<String>,
    structured_content: Option<serde_json::Value>,
) -> crate::extractor_recipe::ExtractorAuthoringMessage {
    crate::extractor_recipe::ExtractorAuthoringMessage {
        role,
        content: content.into(),
        created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        structured_content,
    }
}
