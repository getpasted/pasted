use crate::analysis_contract::{
    AnalysisEnvelope, AnalysisPolicy, AnalysisTargetKind, ClipApplication,
};
use crate::content_analysis::{AnalysisInput, AnalysisRequest, EnricherParticipantSource};
use crate::content_enrichment::{EnrichmentResult, SMART_ACTIONS_ENRICHER_REF};
use crate::db::DbState;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartActionEnrichmentResult {
    #[serde(flatten)]
    pub analysis: EnrichmentResult,
    #[serde(flatten)]
    pub application: ClipApplication,
}

pub fn enrich_text(
    db: &DbState,
    text: &str,
    source: Option<&str>,
) -> Result<SmartActionEnrichmentResult, String> {
    if text.len() > crate::resource_limits::MAX_CLIP_TEXT_BYTES {
        return Err("Enrichment input exceeds Pasted's safety limit".into());
    }
    if source.is_some_and(|source| source.len() > 1_024) {
        return Err("Enrichment source metadata exceeds Pasted's safety limit".into());
    }
    let detectors = db
        .get_content_detectors()
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter(|detector| detector.enabled)
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
        extractor: None,
        detectors: Some(&detectors),
        enricher: Some(EnricherParticipantSource {
            transforms: &transforms,
        }),
    });
    let resolution =
        report.resolve_participant(SMART_ACTIONS_ENRICHER_REF, AnalysisTargetKind::Enricher);
    if let Some(failure) = resolution.failure {
        return Err(failure.message);
    }
    let recommendations = report.context.recommendations.unwrap_or_default();
    Ok(SmartActionEnrichmentResult {
        analysis: AnalysisEnvelope::new(AnalysisPolicy::Interactive, recommendations, report.runs),
        application: ClipApplication::preview(),
    })
}

pub fn enrich_clip(db: &DbState, clip_id: i64) -> Result<SmartActionEnrichmentResult, String> {
    let clip = db
        .get_clip_by_id(clip_id)
        .map_err(|error| error.to_string())?;
    let text = clip
        .text_content
        .as_deref()
        .filter(|_| clip.content_type != "file" && clip.content_type != "image")
        .ok_or_else(|| "Clip has no enrichable text".to_string())?;
    enrich_text(db, text, Some(&clip.source))
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
            "pasted-enrichment-execution-{}-{nanos}-{sequence}.db",
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
                "enrichment-test-hash",
                "Terminal",
            )
            .unwrap();

        let direct =
            enrich_text(&db, clip.text_content.as_deref().unwrap(), Some("Terminal")).unwrap();
        let from_clip = enrich_clip(&db, clip.id).unwrap();
        assert_eq!(direct.analysis.result, from_clip.analysis.result);
        assert_eq!(from_clip.application, ClipApplication::preview());
        assert_eq!(from_clip.analysis.result.actions.len(), 1);
        assert_eq!(
            from_clip.analysis.result.actions[0].transform_name,
            "Format JSON"
        );
    }

    #[test]
    fn empty_results_still_return_a_versioned_enrichment_envelope() {
        let db = db();
        let result = enrich_text(&db, "ordinary words", Some("Pasted CLI")).unwrap();
        assert_eq!(result.analysis.metadata.format_version, 1);
        assert!(result.analysis.result.actions.is_empty());
        assert_eq!(
            result.analysis.participants.last().map(|run| run.outcome),
            Some(crate::analysis_contract::ParticipantOutcome::Produced)
        );
        assert_eq!(result.application, ClipApplication::preview());
        let expected = serde_json::from_str::<serde_json::Value>(include_str!(
            "../../contracts/analysis/v1/enricher-interactive-empty.json"
        ))
        .unwrap();
        assert_eq!(serde_json::to_value(result).unwrap(), expected);
    }

    #[test]
    fn oversized_source_metadata_fails_before_analysis() {
        let db = db();
        let source = "x".repeat(1_025);
        let error = enrich_text(&db, "hello", Some(&source)).unwrap_err();
        assert!(error.contains("source metadata"));
    }
}
