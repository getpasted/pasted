use super::*;

mod prompt;
mod schema;
#[cfg(test)]
mod tests;
use prompt::extractor_recipe_prompt;
use schema::extractor_recipe_schema;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposeExtractorRecipeRequest {
    pub prompt: String,
    #[serde(default)]
    pub connection_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExtractorRecipeProposalPayload {
    name: String,
    description: String,
    recipe: crate::extractor_recipe::ExtractorRecipe,
    setup_guidance: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractorRecipeProposal {
    pub name: String,
    pub description: String,
    pub recipe: crate::extractor_recipe::ExtractorRecipe,
    pub setup_guidance: Vec<String>,
    pub authoring: crate::extractor_recipe::ExtractorAuthoringManifest,
    pub connection_id: String,
    pub connection_name: String,
    pub duration_ms: i64,
}

pub fn propose_extractor_recipe(
    db: &DbState,
    request: ProposeExtractorRecipeRequest,
    cancellation: Option<&AtomicBool>,
) -> Result<ExtractorRecipeProposal, IntelligenceExecutionError> {
    if request.prompt.trim().is_empty() {
        return Err(IntelligenceExecutionError::new(
            "invalid_request",
            "Describe what the Extractor should do",
        ));
    }
    let connections = select_connections(db, request.connection_id.as_deref())?;
    let prompt = extractor_recipe_prompt(&request.prompt);
    let schema = extractor_recipe_schema();
    let allow_fallback = request.connection_id.is_none();
    let mut last_error = None;
    for (index, connection) in connections.iter().enumerate() {
        let response = crate::intelligence_provider::execute(
            connection,
            crate::intelligence_provider::ProviderRequest {
                prompt: &prompt,
                output_schema: Some(&schema),
                cancellation_message: "Extractor draft was cancelled",
            },
            cancellation,
        );
        match response {
            Ok(response) => {
                let payload =
                    serde_json::from_str::<ExtractorRecipeProposalPayload>(response.output.trim())
                        .map_err(|error| {
                            IntelligenceExecutionError::new(
                                "invalid_provider_output",
                                error.to_string(),
                            )
                        })?;
                crate::extractor_recipe::validate_recipe(&payload.recipe).map_err(|error| {
                    IntelligenceExecutionError::new("invalid_provider_output", error)
                })?;
                let timestamp =
                    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                let structured_content = serde_json::to_value(&payload).map_err(|error| {
                    IntelligenceExecutionError::new("invalid_provider_output", error.to_string())
                })?;
                return Ok(ExtractorRecipeProposal {
                    name: payload.name,
                    description: payload.description,
                    recipe: payload.recipe,
                    setup_guidance: payload.setup_guidance,
                    authoring: crate::extractor_recipe::ExtractorAuthoringManifest {
                        manifest_version: crate::extractor_recipe::EXTRACTOR_AUTHORING_VERSION,
                        source: crate::extractor_recipe::ExtractorAuthoringSource::Ai,
                        original_prompt: Some(request.prompt.trim().to_string()),
                        provider: Some(connection.name.clone()),
                        model: connection.model.clone(),
                        messages: vec![
                            crate::extractor_recipe::ExtractorAuthoringMessage {
                                role: crate::extractor_recipe::ExtractorAuthoringRole::User,
                                content: request.prompt.trim().to_string(),
                                created_at: timestamp.clone(),
                                structured_content: None,
                            },
                            crate::extractor_recipe::ExtractorAuthoringMessage {
                                role: crate::extractor_recipe::ExtractorAuthoringRole::Assistant,
                                content: response.output,
                                created_at: timestamp,
                                structured_content: Some(structured_content),
                            },
                        ],
                    },
                    connection_id: connection.id.clone(),
                    connection_name: connection.name.clone(),
                    duration_ms: response.duration_ms,
                });
            }
            Err(error)
                if allow_fallback
                    && index + 1 < connections.len()
                    && is_retryable_provider_error(&error) =>
            {
                last_error = Some(error);
            }
            Err(error) => return Err(error),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        IntelligenceExecutionError::new(
            "no_enabled_connection",
            "Power on a provider and try again.",
        )
    }))
}
