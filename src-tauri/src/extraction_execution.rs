use crate::analysis_contract::{
    AnalysisFailure, AnalysisMetadata, AnalysisPolicy, AnalysisTargetKind, ClipApplication,
    ParticipantOutcome, ParticipantRun,
};
use crate::content_analysis::AnalysisReport;
use crate::content_extraction::{Extractor, ExtractorEngineRegistry};
use crate::db::DbState;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionResultOutcome {
    Produced,
    NoOutput,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionResult {
    #[serde(flatten)]
    pub metadata: AnalysisMetadata,
    pub target_kind: AnalysisTargetKind,
    pub target_ref: String,
    pub outcome: ExtractionResultOutcome,
    pub output: Option<String>,
    pub detected_type: Option<String>,
    pub matched_detector_ref: Option<String>,
    pub failure: Option<AnalysisFailure>,
    pub participants: Vec<ParticipantRun>,
}

impl ExtractionResult {
    fn from_report(
        extractor: &Extractor,
        policy: AnalysisPolicy,
        analysis: AnalysisReport,
    ) -> Self {
        let resolution =
            analysis.resolve_participant(&extractor.stable_ref, AnalysisTargetKind::Extractor);
        let outcome = if resolution.failure.is_some() {
            ExtractionResultOutcome::Failed
        } else if resolution.outcome == ParticipantOutcome::Produced {
            ExtractionResultOutcome::Produced
        } else {
            ExtractionResultOutcome::NoOutput
        };
        let produced = outcome == ExtractionResultOutcome::Produced;
        Self {
            metadata: AnalysisMetadata::new(policy),
            target_kind: AnalysisTargetKind::Extractor,
            target_ref: extractor.stable_ref.clone(),
            outcome,
            output: produced
                .then_some(analysis.context.searchable_text)
                .flatten(),
            detected_type: produced.then_some(analysis.context.detected_type).flatten(),
            matched_detector_ref: produced
                .then_some(analysis.context.matched_detector_ref)
                .flatten(),
            failure: resolution.failure,
            participants: analysis.runs,
        }
    }

    pub fn failed(&self) -> bool {
        self.outcome == ExtractionResultOutcome::Failed
    }
}

pub fn analyze_image(
    image_bytes: Vec<u8>,
    extractor: &Extractor,
    detectors: Option<&[crate::content_detection::Detector]>,
) -> ExtractionResult {
    let registry = crate::content_extraction::system_engine_registry();
    analyze_image_with_registry(image_bytes, extractor, detectors, &registry)
}

pub fn analyze_image_with_registry(
    image_bytes: Vec<u8>,
    extractor: &Extractor,
    detectors: Option<&[crate::content_detection::Detector]>,
    registry: &ExtractorEngineRegistry<'_>,
) -> ExtractionResult {
    analyze_image_with_registry_and_policy(
        image_bytes,
        extractor,
        detectors,
        registry,
        crate::analysis_contract::AnalysisPolicy::Interactive,
    )
}

pub(crate) fn analyze_image_with_registry_and_policy(
    image_bytes: Vec<u8>,
    extractor: &Extractor,
    detectors: Option<&[crate::content_detection::Detector]>,
    registry: &ExtractorEngineRegistry<'_>,
    policy: crate::analysis_contract::AnalysisPolicy,
) -> ExtractionResult {
    ExtractionResult::from_report(
        extractor,
        policy,
        crate::content_analysis::analyze(crate::content_analysis::AnalysisRequest {
            input: crate::content_analysis::AnalysisInput::Image {
                image_bytes,
                searchable_text: None,
                source: None,
            },
            policy,
            inspector: false,
            extractor: Some(crate::content_analysis::ExtractorParticipantSource {
                extractor,
                registry,
            }),
            detectors,
            enricher: None,
        }),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ExtractionPersistence {
    pub ocr_updated: bool,
    pub classification_updated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionApplicationResult {
    #[serde(flatten)]
    pub analysis: ExtractionResult,
    #[serde(flatten)]
    pub application: ClipApplication,
    pub ocr_updated: bool,
    pub classification_updated: bool,
}

impl ExtractionApplicationResult {
    pub fn preview(analysis: ExtractionResult) -> Self {
        Self {
            analysis,
            application: ClipApplication::preview(),
            ocr_updated: false,
            classification_updated: false,
        }
    }
}

fn persist_image_analysis(
    db: &DbState,
    clip_id: i64,
    content_hash: &str,
    extractor: &Extractor,
    classification_enabled: bool,
    analysis: &ExtractionResult,
) -> rusqlite::Result<ExtractionPersistence> {
    let extraction_error = analysis
        .failure
        .as_ref()
        .map(|failure| failure.code.as_str());
    let ocr_updated = db.complete_or_reset_ocr_attempt(
        clip_id,
        content_hash,
        analysis.output.as_deref(),
        &extractor.engine,
        extraction_error,
    )?;
    if !ocr_updated {
        return Ok(ExtractionPersistence {
            ocr_updated: false,
            classification_updated: false,
        });
    }

    let classification_updated = if classification_enabled && analysis.output.is_some() {
        db.record_analysis_classification(
            clip_id,
            content_hash,
            analysis.detected_type.as_deref(),
            analysis.matched_detector_ref.as_deref(),
            "searchable_text",
        )
        .unwrap_or(false)
    } else {
        false
    };

    Ok(ExtractionPersistence {
        ocr_updated,
        classification_updated,
    })
}

pub fn persist_claimed_image_analysis(
    db: &DbState,
    clip_id: i64,
    content_hash: &str,
    extractor: &Extractor,
    classification_enabled: bool,
    analysis: ExtractionResult,
) -> rusqlite::Result<ExtractionApplicationResult> {
    let persistence = persist_image_analysis(
        db,
        clip_id,
        content_hash,
        extractor,
        classification_enabled,
        &analysis,
    )?;
    Ok(ExtractionApplicationResult {
        application: if persistence.ocr_updated {
            ClipApplication::applied(clip_id)
        } else {
            ClipApplication::preview()
        },
        ocr_updated: persistence.ocr_updated,
        classification_updated: persistence.classification_updated,
        analysis,
    })
}

pub fn apply_image_analysis(
    db: &DbState,
    clip_id: i64,
    content_hash: &str,
    extractor: &Extractor,
    classification_enabled: bool,
    analysis: ExtractionResult,
) -> rusqlite::Result<ExtractionApplicationResult> {
    if !db.force_ocr_running(clip_id, content_hash)? {
        return Err(rusqlite::Error::InvalidParameterName(
            "The selected clip is no longer available for extraction".into(),
        ));
    }
    let result = persist_claimed_image_analysis(
        db,
        clip_id,
        content_hash,
        extractor,
        classification_enabled,
        analysis,
    )?;
    if !result.ocr_updated {
        return Err(rusqlite::Error::InvalidParameterName(
            "The selected clip changed before extraction could be applied".into(),
        ));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content_analysis::{AnalysisContext, AnalysisPass, ParticipantRun};
    use crate::content_extraction::{EngineAvailability, ExtractionFailure, ExtractionOutcome};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct FixedEngine {
        outcome: ExtractionOutcome,
    }

    impl crate::content_extraction::ExtractorEngine for FixedEngine {
        fn id(&self) -> &'static str {
            "test-engine-v1"
        }

        fn availability(&self) -> EngineAvailability {
            EngineAvailability {
                is_available: true,
                unavailable_reason: None,
            }
        }

        fn extract(&self, _image_bytes: &[u8]) -> ExtractionOutcome {
            self.outcome.clone()
        }
    }

    fn setup_test_db() -> DbState {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        DbState::new(std::env::temp_dir().join(format!("pasted_extraction_execution_{nonce}.db")))
            .unwrap()
    }

    fn extractor() -> Extractor {
        Extractor {
            id: 1,
            stable_ref: "extractor:test".into(),
            name: "Test OCR".into(),
            description: String::new(),
            engine: "test-engine-v1".into(),
            input_contract: "image".into(),
            output_contract: "searchable_text".into(),
            enabled: true,
            priority: 10,
            is_builtin: false,
            is_available: true,
            unavailable_reason: None,
            defaults: None,
        }
    }

    fn result(
        output: Option<&str>,
        detected_type: Option<&str>,
        matched_detector_ref: Option<&str>,
    ) -> ExtractionResult {
        ExtractionResult {
            metadata: AnalysisMetadata::new(AnalysisPolicy::Interactive),
            target_kind: AnalysisTargetKind::Extractor,
            target_ref: "extractor:test".into(),
            outcome: if output.is_some() {
                ExtractionResultOutcome::Produced
            } else {
                ExtractionResultOutcome::NoOutput
            },
            output: output.map(str::to_string),
            detected_type: detected_type.map(str::to_string),
            matched_detector_ref: matched_detector_ref.map(str::to_string),
            failure: None,
            participants: Vec::new(),
        }
    }

    #[test]
    fn execution_normalizes_the_shared_serializable_result() {
        let engine = FixedEngine {
            outcome: ExtractionOutcome::Produced {
                text: "recognized text".into(),
            },
        };
        let engines: [&dyn crate::content_extraction::ExtractorEngine; 1] = [&engine];
        let registry = ExtractorEngineRegistry::new(&engines);

        let result = analyze_image_with_registry(vec![1, 2, 3], &extractor(), None, &registry);

        assert_eq!(result.outcome, ExtractionResultOutcome::Produced);
        assert_eq!(result.output.as_deref(), Some("recognized text"));
        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            serde_json::json!({
                "formatVersion": 1,
                "policy": "interactive",
                "through": "enrich",
                "targetKind": "extractor",
                "targetRef": "extractor:test",
                "outcome": "produced",
                "output": "recognized text",
                "detectedType": null,
                "matchedDetectorRef": null,
                "failure": null,
                "participants": [{
                    "stableRef": "extractor:test",
                    "pass": "extract",
                    "outcome": "produced"
                }],
            })
        );

        assert_eq!(
            serde_json::to_value(ExtractionApplicationResult::preview(result)).unwrap(),
            serde_json::from_str::<serde_json::Value>(include_str!(
                "../../contracts/analysis/v1/extractor-interactive-produced.json"
            ))
            .unwrap()
        );
    }

    #[test]
    fn background_extraction_preserves_its_execution_policy() {
        let engine = FixedEngine {
            outcome: ExtractionOutcome::NoOutput,
        };
        let engines: [&dyn crate::content_extraction::ExtractorEngine; 1] = [&engine];
        let registry = ExtractorEngineRegistry::new(&engines);

        let result = analyze_image_with_registry_and_policy(
            vec![1, 2, 3],
            &extractor(),
            None,
            &registry,
            AnalysisPolicy::Background,
        );

        assert_eq!(
            result.metadata,
            AnalysisMetadata::new(AnalysisPolicy::Background)
        );
    }

    #[test]
    fn execution_preserves_typed_extractor_failures() {
        let engine = FixedEngine {
            outcome: ExtractionOutcome::Failed {
                failure: ExtractionFailure {
                    code: "test_failure".into(),
                    message: "The test engine failed.".into(),
                },
            },
        };
        let engines: [&dyn crate::content_extraction::ExtractorEngine; 1] = [&engine];
        let registry = ExtractorEngineRegistry::new(&engines);

        let result = analyze_image_with_registry(vec![1], &extractor(), None, &registry);

        assert!(result.failed());
        assert_eq!(result.output, None);
        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            serde_json::json!({
                "formatVersion": 1,
                "policy": "interactive",
                "through": "enrich",
                "targetKind": "extractor",
                "targetRef": "extractor:test",
                "outcome": "failed",
                "output": null,
                "detectedType": null,
                "matchedDetectorRef": null,
                "failure": {
                    "code": "test_failure",
                    "message": "The test engine failed.",
                },
                "participants": [{
                    "stableRef": "extractor:test",
                    "pass": "extract",
                    "outcome": "failed",
                    "failure": {
                        "code": "test_failure",
                        "message": "The test engine failed."
                    }
                }],
            })
        );
    }

    #[test]
    fn execution_keeps_no_output_distinct_from_failure() {
        let engine = FixedEngine {
            outcome: ExtractionOutcome::NoOutput,
        };
        let engines: [&dyn crate::content_extraction::ExtractorEngine; 1] = [&engine];
        let registry = ExtractorEngineRegistry::new(&engines);

        let result = analyze_image_with_registry(vec![1], &extractor(), None, &registry);

        assert_eq!(result.outcome, ExtractionResultOutcome::NoOutput);
        assert_eq!(result.output, None);
        assert_eq!(result.failure, None);
    }

    #[test]
    fn failed_results_discard_context_mutated_before_failure() {
        let report = AnalysisReport {
            context: AnalysisContext {
                clip_kind: "image".into(),
                capture_source: None,
                original_text: None,
                image_bytes: None,
                searchable_text: Some("partial output".into()),
                detected_type: Some("email".into()),
                matched_detector_ref: Some("detector:email".into()),
                structural_metadata: None,
                recommendations: None,
            },
            runs: vec![ParticipantRun {
                stable_ref: "extractor:test".into(),
                pass: AnalysisPass::Extract,
                outcome: ParticipantOutcome::Failed,
                failure: Some(AnalysisFailure {
                    code: "contract_violation".into(),
                    message: "The Extractor violated its contract.".into(),
                }),
            }],
        };

        let result =
            ExtractionResult::from_report(&extractor(), AnalysisPolicy::Interactive, report);

        assert!(result.failed());
        assert_eq!(result.output, None);
        assert_eq!(result.detected_type, None);
        assert_eq!(result.matched_detector_ref, None);
        assert_eq!(result.failure.unwrap().code, "contract_violation");
    }

    #[test]
    fn shared_persistence_applies_ocr_and_derived_classification() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "image",
                None,
                None,
                Some(crate::resource_limits::TEST_PNG_DATA_URL),
                "shared-analysis-persistence",
                "Screenshot",
            )
            .unwrap();
        assert!(db.force_ocr_running(clip.id, &clip.content_hash).unwrap());
        let analysis = result(
            Some("agent@example.com"),
            Some("email"),
            Some("detector:email"),
        );

        let persisted = persist_claimed_image_analysis(
            &db,
            clip.id,
            &clip.content_hash,
            &extractor(),
            true,
            analysis,
        )
        .unwrap();

        assert_eq!(persisted.application.applied_clip_id, Some(clip.id));
        assert!(persisted.ocr_updated);
        assert!(persisted.classification_updated);
        assert_eq!(
            db.get_clip_by_id(clip.id).unwrap().text_content.as_deref(),
            Some("agent@example.com")
        );
        let classification = db.get_analysis_classification(clip.id).unwrap().unwrap();
        assert_eq!(classification.content_type, "email");
        assert_eq!(classification.detector_ref, "detector:email");
        assert_eq!(classification.source_representation, "searchable_text");
    }

    #[test]
    fn user_application_claims_the_clip_and_rejects_stale_input() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "image",
                None,
                None,
                Some(crate::resource_limits::TEST_PNG_DATA_URL),
                "user-analysis-application",
                "Screenshot",
            )
            .unwrap();

        let stale = apply_image_analysis(
            &db,
            clip.id,
            "stale-hash",
            &extractor(),
            false,
            result(Some("must not be saved"), None, None),
        )
        .unwrap_err();
        assert!(stale.to_string().contains("no longer available"));
        assert_eq!(db.get_clip_by_id(clip.id).unwrap().text_content, None);

        let applied = apply_image_analysis(
            &db,
            clip.id,
            &clip.content_hash,
            &extractor(),
            false,
            result(Some("recognized text"), None, None),
        )
        .unwrap();
        assert_eq!(applied.application.applied_clip_id, Some(clip.id));
        assert!(applied.ocr_updated);
        assert!(!applied.classification_updated);
        assert_eq!(
            db.get_clip_by_id(clip.id).unwrap().text_content.as_deref(),
            Some("recognized text")
        );
    }

    #[test]
    fn derived_classification_failure_does_not_fail_ocr_completion() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "image",
                None,
                None,
                Some(crate::resource_limits::TEST_PNG_DATA_URL),
                "best-effort-classification",
                "Screenshot",
            )
            .unwrap();
        assert!(db.force_ocr_running(clip.id, &clip.content_hash).unwrap());
        let analysis = result(
            Some("recognized text"),
            Some(&"x".repeat(81)),
            Some("detector:test"),
        );

        let persisted = persist_claimed_image_analysis(
            &db,
            clip.id,
            &clip.content_hash,
            &extractor(),
            true,
            analysis,
        )
        .unwrap();

        assert_eq!(persisted.application.applied_clip_id, Some(clip.id));
        assert!(persisted.ocr_updated);
        assert!(!persisted.classification_updated);
        assert_eq!(
            db.get_clip_by_id(clip.id).unwrap().text_content.as_deref(),
            Some("recognized text")
        );
        assert!(db.get_analysis_classification(clip.id).unwrap().is_none());
    }
}
