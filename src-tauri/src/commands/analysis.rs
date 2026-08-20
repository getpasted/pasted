use std::sync::Arc;

use tauri::State;

use crate::db::DbState;
use crate::features::{self, Feature};

#[tauri::command]
pub async fn analyze_content(
    request: AnalyzeContentRequest,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::analysis_execution::AnalyzerPreview, String> {
    let AnalyzeContentRequest {
        text,
        clip_id,
        source,
        policy,
        include_extractor,
        include_classifiers,
        include_suggestions,
    } = request;
    if text.is_some() == clip_id.is_some() {
        return Err("Provide exactly one of text or clipId".into());
    }
    let policy = policy
        .as_deref()
        .unwrap_or("interactive")
        .parse::<crate::analysis_contract::AnalysisPolicy>()?;
    let include_suggestions = include_suggestions.unwrap_or(true);
    if include_suggestions
        && policy.includes(crate::analysis_contract::AnalysisPass::Suggest)
        && !features::is_enabled(&db, Feature::Transformations)
    {
        return Err("Transformations is disabled in Settings → Functionality".into());
    }
    let options = crate::analysis_execution::AnalyzerOptions {
        policy,
        include_extractor: include_extractor.unwrap_or(false),
        include_classifiers: include_classifiers.unwrap_or(true)
            && features::is_enabled(&db, Feature::ContentClassification),
        include_suggestions,
    };
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || match (text, clip_id) {
        (Some(text), None) => {
            crate::analysis_execution::analyze_text(&db, &text, source.as_deref(), options)
        }
        (None, Some(clip_id)) => crate::analysis_execution::analyze_clip(&db, clip_id, options),
        _ => unreachable!("input combination validated"),
    })
    .await
    .map_err(|error| error.to_string())?
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeContentRequest {
    text: Option<String>,
    clip_id: Option<i64>,
    source: Option<String>,
    policy: Option<String>,
    include_extractor: Option<bool>,
    include_classifiers: Option<bool>,
    include_suggestions: Option<bool>,
}

#[tauri::command]
pub fn get_content_inspectors() -> Vec<crate::content_inspection::InspectorDefinition> {
    crate::content_inspection::inspector_definitions()
}
