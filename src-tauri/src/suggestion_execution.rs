use crate::analysis_contract::{
    AnalysisEnvelope, AnalysisPolicy, AnalysisTargetKind, ClipApplication,
};
use crate::content_analysis::{AnalysisInput, AnalysisRequest, SuggestionParticipantSource};
use crate::content_suggestions::{SuggestionResult, SMART_ACTIONS_SUGGESTION_REF};
use crate::db::DbState;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartActionSuggestionResult {
    #[serde(flatten)]
    pub analysis: SuggestionResult,
    #[serde(flatten)]
    pub application: ClipApplication,
}

pub fn suggest_text(
    db: &DbState,
    text: &str,
    source: Option<&str>,
) -> Result<SmartActionSuggestionResult, String> {
    if text.len() > crate::resource_limits::MAX_CLIP_TEXT_BYTES {
        return Err("Suggestion input exceeds Pasted's safety limit".into());
    }
    if source
        .is_some_and(|source| source.len() > crate::analysis_contract::MAX_ANALYSIS_SOURCE_BYTES)
    {
        return Err("Suggestion source metadata exceeds Pasted's safety limit".into());
    }
    let classifiers = db
        .get_content_classifiers()
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter(|classifier| classifier.enabled)
        .collect::<Vec<_>>();
    let transforms = db
        .get_transform_definitions()
        .map_err(|error| error.to_string())?;
    let report = crate::content_analysis::analyze(AnalysisRequest {
        input: AnalysisInput::Text {
            text: text.into(),
            source: source.map(str::to_owned),
        },
        policy: AnalysisPolicy::Interactive,
        inspector: true,
        file_format_inspector: false,
        extractors: Vec::new(),
        classifiers: Some(&classifiers),
        suggestion: Some(SuggestionParticipantSource {
            transforms: &transforms,
        }),
    });
    let resolution =
        report.resolve_participant(SMART_ACTIONS_SUGGESTION_REF, AnalysisTargetKind::Suggestion);
    if let Some(failure) = resolution.failure {
        return Err(failure.message);
    }
    let suggestions = report.context.suggestions.unwrap_or_default();
    Ok(SmartActionSuggestionResult {
        analysis: AnalysisEnvelope::new(AnalysisPolicy::Interactive, suggestions, report.runs),
        application: ClipApplication::preview(),
    })
}

pub fn suggest_clip(db: &DbState, clip_id: i64) -> Result<SmartActionSuggestionResult, String> {
    let clip = db
        .get_clip_by_id(clip_id)
        .map_err(|error| error.to_string())?;
    let text = clip
        .text_content
        .as_deref()
        .filter(|_| clip.content_type != "file" && clip.content_type != "image")
        .ok_or_else(|| "Clip has no analyzable text".to_string())?;
    suggest_text(db, text, Some(&clip.source))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformation_intent::{
        IntentPlanningMode, PlannedExecutor, PlannedTransformationStep, StepExecutionScope,
        StepFailurePolicy, TransformationPlan, TRANSFORMATION_PLAN_SCHEMA_VERSION,
    };
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_DATABASE: AtomicU64 = AtomicU64::new(0);

    fn db() -> DbState {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = NEXT_DATABASE.fetch_add(1, Ordering::Relaxed);
        DbState::new(std::env::temp_dir().join(format!(
            "pasted-suggestion-execution-{}-{nanos}-{sequence}.db",
            std::process::id()
        )))
        .unwrap()
    }

    #[test]
    fn clip_and_text_surfaces_share_the_same_non_mutating_result() {
        let db = db();
        db.create_saved_transform(
            "Format JSON",
            &TransformationPlan {
                schema_version: TRANSFORMATION_PLAN_SCHEMA_VERSION,
                intent: "Format JSON".into(),
                summary: "Format JSON".into(),
                planning_mode: IntentPlanningMode::Pinned,
                steps: vec![PlannedTransformationStep {
                    name: "Format".into(),
                    rationale: "Make JSON readable".into(),
                    scope: StepExecutionScope::WholeInput,
                    failure_policy: StepFailurePolicy::Stop,
                    executor: PlannedExecutor::Deterministic {
                        operation_ref: "builtin:json_format".into(),
                        config_json: None,
                    },
                }],
            },
            None,
        )
        .unwrap();
        let clip = db
            .save_clip(
                "text",
                Some("{\"hello\":\"world\"}"),
                None,
                None,
                "suggestion-test-hash",
                "Terminal",
            )
            .unwrap();

        let direct =
            suggest_text(&db, clip.text_content.as_deref().unwrap(), Some("Terminal")).unwrap();
        let from_clip = suggest_clip(&db, clip.id).unwrap();
        assert_eq!(direct.analysis.result, from_clip.analysis.result);
        assert_eq!(from_clip.application, ClipApplication::preview());
        assert_eq!(from_clip.analysis.result.actions.len(), 1);
        assert_eq!(
            from_clip.analysis.result.actions[0].transform_name,
            "Format JSON"
        );
    }

    #[test]
    fn empty_results_still_return_a_versioned_suggestion_envelope() {
        let db = db();
        let result = suggest_text(&db, "ordinary words", Some("Pasted CLI")).unwrap();
        assert_eq!(result.analysis.metadata.format_version, 1);
        assert!(result.analysis.result.actions.is_empty());
        assert_eq!(
            result.analysis.participants.last().map(|run| run.outcome),
            Some(crate::analysis_contract::ParticipantOutcome::Produced)
        );
        assert_eq!(result.application, ClipApplication::preview());
        let expected = serde_json::from_str::<serde_json::Value>(include_str!(
            "../../contracts/analysis/v1/suggestion-interactive-empty.json"
        ))
        .unwrap();
        assert_eq!(serde_json::to_value(result).unwrap(), expected);
    }

    #[test]
    fn oversized_text_and_source_metadata_fail_before_analysis() {
        let db = db();
        let text = "x".repeat(crate::resource_limits::MAX_CLIP_TEXT_BYTES + 1);
        assert!(suggest_text(&db, &text, None)
            .unwrap_err()
            .contains("safety limit"));

        let source = "x".repeat(crate::analysis_contract::MAX_ANALYSIS_SOURCE_BYTES + 1);
        let error = suggest_text(&db, "hello", Some(&source)).unwrap_err();
        assert!(error.contains("source metadata"));
    }
}
