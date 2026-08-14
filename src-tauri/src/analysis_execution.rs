use crate::content_analysis::{
    AnalysisFailure, AnalysisReport, ParticipantOutcome, ParticipantRun,
};
use crate::content_extraction::{Extractor, ExtractorEngineRegistry};
use crate::db::DbState;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageAnalysisOutcome {
    Produced,
    NoOutput,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisTargetKind {
    Extractor,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageAnalysisResult {
    pub target_kind: AnalysisTargetKind,
    pub target_ref: String,
    pub outcome: ImageAnalysisOutcome,
    pub output: Option<String>,
    pub detected_type: Option<String>,
    pub matched_detector_ref: Option<String>,
    pub failure: Option<AnalysisFailure>,
    pub participants: Vec<ParticipantRun>,
}

impl ImageAnalysisResult {
    fn from_report(extractor: &Extractor, analysis: AnalysisReport) -> Self {
        let run = analysis
            .runs
            .iter()
            .find(|run| run.stable_ref == extractor.stable_ref);
        let (outcome, failure) = match run {
            Some(run) if run.outcome == ParticipantOutcome::Failed => (
                ImageAnalysisOutcome::Failed,
                run.failure.clone().or_else(|| {
                    Some(AnalysisFailure {
                        code: "analysis_failed".into(),
                        message: "The Extractor failed without a structured reason.".into(),
                    })
                }),
            ),
            Some(run) if run.outcome == ParticipantOutcome::MissingInput => (
                ImageAnalysisOutcome::Failed,
                Some(AnalysisFailure {
                    code: "missing_input".into(),
                    message: "The Extractor's required input was not available.".into(),
                }),
            ),
            Some(run) if run.outcome == ParticipantOutcome::Produced => {
                (ImageAnalysisOutcome::Produced, None)
            }
            Some(_) => (ImageAnalysisOutcome::NoOutput, None),
            None => (
                ImageAnalysisOutcome::Failed,
                Some(AnalysisFailure {
                    code: "missing_participant".into(),
                    message: "The Extractor did not participate in Analysis.".into(),
                }),
            ),
        };
        let produced = outcome == ImageAnalysisOutcome::Produced;
        Self {
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
            failure,
            participants: analysis.runs,
        }
    }

    pub fn failed(&self) -> bool {
        self.outcome == ImageAnalysisOutcome::Failed
    }
}

pub fn analyze_image(
    image_bytes: Vec<u8>,
    extractor: &Extractor,
    detectors: Option<&[crate::content_detection::Detector]>,
) -> ImageAnalysisResult {
    let registry = crate::content_extraction::system_engine_registry();
    analyze_image_with_registry(image_bytes, extractor, detectors, &registry)
}

pub fn analyze_image_with_registry(
    image_bytes: Vec<u8>,
    extractor: &Extractor,
    detectors: Option<&[crate::content_detection::Detector]>,
    registry: &ExtractorEngineRegistry<'_>,
) -> ImageAnalysisResult {
    ImageAnalysisResult::from_report(
        extractor,
        crate::content_analysis::analyze_image_with_registry(
            image_bytes,
            extractor,
            detectors,
            registry,
        ),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageAnalysisPersistence {
    pub ocr_updated: bool,
    pub classification_updated: bool,
}

pub fn persist_image_analysis(
    db: &DbState,
    clip_id: i64,
    content_hash: &str,
    extractor: &Extractor,
    classification_enabled: bool,
    analysis: &ImageAnalysisResult,
) -> rusqlite::Result<ImageAnalysisPersistence> {
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
        return Ok(ImageAnalysisPersistence {
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

    Ok(ImageAnalysisPersistence {
        ocr_updated,
        classification_updated,
    })
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
        DbState::new(std::env::temp_dir().join(format!("pasted_analysis_execution_{nonce}.db")))
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
    ) -> ImageAnalysisResult {
        ImageAnalysisResult {
            target_kind: AnalysisTargetKind::Extractor,
            target_ref: "extractor:test".into(),
            outcome: if output.is_some() {
                ImageAnalysisOutcome::Produced
            } else {
                ImageAnalysisOutcome::NoOutput
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

        assert_eq!(result.outcome, ImageAnalysisOutcome::Produced);
        assert_eq!(result.output.as_deref(), Some("recognized text"));
        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            serde_json::json!({
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

        assert_eq!(result.outcome, ImageAnalysisOutcome::NoOutput);
        assert_eq!(result.output, None);
        assert_eq!(result.failure, None);
    }

    #[test]
    fn failed_results_discard_context_mutated_before_failure() {
        let report = AnalysisReport {
            context: AnalysisContext {
                clip_kind: "image".into(),
                original_text: None,
                image_bytes: None,
                searchable_text: Some("partial output".into()),
                detected_type: Some("email".into()),
                matched_detector_ref: Some("detector:email".into()),
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

        let result = ImageAnalysisResult::from_report(&extractor(), report);

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

        let persisted = persist_image_analysis(
            &db,
            clip.id,
            &clip.content_hash,
            &extractor(),
            true,
            &analysis,
        )
        .unwrap();

        assert_eq!(
            persisted,
            ImageAnalysisPersistence {
                ocr_updated: true,
                classification_updated: true,
            }
        );
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

        let persisted = persist_image_analysis(
            &db,
            clip.id,
            &clip.content_hash,
            &extractor(),
            true,
            &analysis,
        )
        .unwrap();

        assert_eq!(
            persisted,
            ImageAnalysisPersistence {
                ocr_updated: true,
                classification_updated: false,
            }
        );
        assert_eq!(
            db.get_clip_by_id(clip.id).unwrap().text_content.as_deref(),
            Some("recognized text")
        );
        assert!(db.get_analysis_classification(clip.id).unwrap().is_none());
    }
}
