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
    pub classification_matches: Vec<crate::content_classification::ClassificationMatch>,
    pub failure: Option<AnalysisFailure>,
    pub participants: Vec<ParticipantRun>,
    #[serde(skip)]
    pub(crate) observations: Vec<crate::content_analysis::ExtractionObservation>,
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
        let observations = analysis.context.extraction_observations.clone();
        Self {
            metadata: AnalysisMetadata::new(policy),
            target_kind: AnalysisTargetKind::Extractor,
            target_ref: extractor.stable_ref.clone(),
            outcome,
            output: produced
                .then_some(analysis.context.searchable_text)
                .flatten(),
            classification_matches: if produced {
                analysis.context.classification_matches
            } else {
                Vec::new()
            },
            failure: resolution.failure,
            participants: analysis.runs,
            observations,
        }
    }

    pub fn failed(&self) -> bool {
        self.outcome == ExtractionResultOutcome::Failed
    }
}

pub fn analyze_image(
    image_bytes: Vec<u8>,
    extractor: &Extractor,
    classifiers: Option<&[crate::content_classification::Classifier]>,
) -> ExtractionResult {
    let registry = crate::content_extraction::system_engine_registry();
    analyze_images_with_registry(
        image_bytes,
        std::slice::from_ref(extractor),
        classifiers,
        &registry,
    )
}

pub fn analyze_image_with_registry(
    image_bytes: Vec<u8>,
    extractor: &Extractor,
    classifiers: Option<&[crate::content_classification::Classifier]>,
    registry: &ExtractorEngineRegistry<'_>,
) -> ExtractionResult {
    analyze_images_with_registry_and_policy(
        image_bytes,
        std::slice::from_ref(extractor),
        classifiers,
        registry,
        crate::analysis_contract::AnalysisPolicy::Interactive,
    )
}

pub fn analyze_images_with_registry(
    image_bytes: Vec<u8>,
    extractors: &[Extractor],
    classifiers: Option<&[crate::content_classification::Classifier]>,
    registry: &ExtractorEngineRegistry<'_>,
) -> ExtractionResult {
    analyze_images_with_registry_and_policy(
        image_bytes,
        extractors,
        classifiers,
        registry,
        AnalysisPolicy::Interactive,
    )
}

pub fn analyze_files(
    paths: Vec<String>,
    extractor: &Extractor,
    classifiers: Option<&[crate::content_classification::Classifier]>,
) -> ExtractionResult {
    let registry = crate::content_extraction::system_engine_registry();
    analyze_files_with_registry(paths, extractor, classifiers, &registry)
}

pub fn analyze_files_with_registry(
    paths: Vec<String>,
    extractor: &Extractor,
    classifiers: Option<&[crate::content_classification::Classifier]>,
    registry: &ExtractorEngineRegistry<'_>,
) -> ExtractionResult {
    analyze_files_with_extractors_and_registry(
        paths,
        std::slice::from_ref(extractor),
        classifiers,
        registry,
    )
}

pub fn analyze_files_with_extractors_and_registry(
    paths: Vec<String>,
    extractors: &[Extractor],
    classifiers: Option<&[crate::content_classification::Classifier]>,
    registry: &ExtractorEngineRegistry<'_>,
) -> ExtractionResult {
    let report = crate::content_analysis::analyze(crate::content_analysis::AnalysisRequest {
        input: crate::content_analysis::AnalysisInput::Files {
            paths,
            source: None,
        },
        policy: AnalysisPolicy::Interactive,
        inspector: false,
        extractors: extractor_sources(extractors, registry),
        classifiers,
        suggestion: None,
    });
    ExtractionResult::from_report(
        selected_extractor(extractors, &report),
        AnalysisPolicy::Interactive,
        report,
    )
}

fn extractor_sources<'a>(
    extractors: &'a [Extractor],
    registry: &'a ExtractorEngineRegistry<'a>,
) -> Vec<crate::content_analysis::ExtractorParticipantSource<'a>> {
    extractors
        .iter()
        .map(
            |extractor| crate::content_analysis::ExtractorParticipantSource {
                extractor,
                registry,
            },
        )
        .collect()
}

fn selected_extractor<'a>(extractors: &'a [Extractor], report: &AnalysisReport) -> &'a Extractor {
    let selected_ref = report
        .context
        .extraction_observations
        .iter()
        .rev()
        .find(|observation| {
            matches!(
                observation.outcome,
                crate::content_extraction::ExtractionOutcome::Produced { .. }
            )
        })
        .or_else(|| report.context.extraction_observations.last())
        .map(|observation| observation.extractor_ref.as_str());
    selected_ref
        .and_then(|stable_ref| {
            extractors
                .iter()
                .find(|extractor| extractor.stable_ref == stable_ref)
        })
        .unwrap_or_else(|| {
            extractors
                .first()
                .expect("extraction requires at least one Extractor")
        })
}

pub(crate) fn analyze_images_with_registry_and_policy(
    image_bytes: Vec<u8>,
    extractors: &[Extractor],
    classifiers: Option<&[crate::content_classification::Classifier]>,
    registry: &ExtractorEngineRegistry<'_>,
    policy: crate::analysis_contract::AnalysisPolicy,
) -> ExtractionResult {
    let report = crate::content_analysis::analyze(crate::content_analysis::AnalysisRequest {
        input: crate::content_analysis::AnalysisInput::Image {
            image_bytes,
            searchable_text: None,
            source: None,
        },
        policy,
        inspector: false,
        extractors: extractor_sources(extractors, registry),
        classifiers,
        suggestion: None,
    });
    ExtractionResult::from_report(selected_extractor(extractors, &report), policy, report)
}

#[cfg(test)]
pub(crate) fn analyze_image_with_registry_and_policy(
    image_bytes: Vec<u8>,
    extractor: &Extractor,
    classifiers: Option<&[crate::content_classification::Classifier]>,
    registry: &ExtractorEngineRegistry<'_>,
    policy: AnalysisPolicy,
) -> ExtractionResult {
    analyze_images_with_registry_and_policy(
        image_bytes,
        std::slice::from_ref(extractor),
        classifiers,
        registry,
        policy,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ExtractionPersistence {
    pub ocr_updated: bool,
    pub searchable_text_updated: bool,
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
    pub searchable_text_updated: bool,
    pub classification_updated: bool,
}

impl ExtractionApplicationResult {
    pub fn preview(analysis: ExtractionResult) -> Self {
        Self {
            analysis,
            application: ClipApplication::preview(),
            ocr_updated: false,
            searchable_text_updated: false,
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
    let ocr_updated = db.complete_or_reset_ocr_attempt_with_extractor(
        clip_id,
        content_hash,
        analysis.output.as_deref(),
        crate::db::OcrExtractorProvenance::identified(
            &extractor.engine,
            &extractor.stable_ref,
            &extractor.name,
        ),
        extraction_error,
    )?;
    if !ocr_updated {
        return Ok(ExtractionPersistence {
            ocr_updated: false,
            searchable_text_updated: false,
            classification_updated: false,
        });
    }
    db.record_extraction_observations(clip_id, content_hash, &analysis.observations)?;

    let classification_updated = if classification_enabled && analysis.output.is_some() {
        db.replace_analysis_classifications(
            clip_id,
            content_hash,
            &analysis.classification_matches,
            "searchable_text",
        )
        .unwrap_or(false)
    } else {
        false
    };

    Ok(ExtractionPersistence {
        ocr_updated,
        searchable_text_updated: false,
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
        searchable_text_updated: persistence.searchable_text_updated,
        classification_updated: persistence.classification_updated,
        analysis,
    })
}

pub fn apply_file_analysis(
    db: &DbState,
    clip_id: i64,
    content_hash: &str,
    extractor: &Extractor,
    classification_enabled: bool,
    analysis: ExtractionResult,
) -> rusqlite::Result<ExtractionApplicationResult> {
    let observations_updated =
        db.record_extraction_observations(clip_id, content_hash, &analysis.observations)?;
    if analysis.outcome != ExtractionResultOutcome::Produced {
        return Ok(ExtractionApplicationResult::preview(analysis));
    }
    if !observations_updated {
        return Err(rusqlite::Error::InvalidParameterName(
            "The selected clip changed before extraction could be applied".into(),
        ));
    }
    let searchable_text_updated = db.replace_clip_searchable_text(
        clip_id,
        content_hash,
        extractor,
        analysis.output.as_deref(),
    )?;
    if !searchable_text_updated {
        return Err(rusqlite::Error::InvalidParameterName(
            "The selected clip changed before extraction could be applied".into(),
        ));
    }
    let classification_updated = if classification_enabled {
        db.replace_analysis_classifications(
            clip_id,
            content_hash,
            &analysis.classification_matches,
            "searchable_text",
        )
        .unwrap_or(false)
    } else {
        false
    };
    Ok(ExtractionApplicationResult {
        application: ClipApplication::applied(clip_id),
        ocr_updated: false,
        searchable_text_updated,
        classification_updated,
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

    struct FixedFileEngine {
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

    impl crate::content_extraction::ExtractorEngine for FixedFileEngine {
        fn id(&self) -> &'static str {
            "test-file-engine-v1"
        }

        fn availability(&self) -> EngineAvailability {
            EngineAvailability {
                is_available: true,
                unavailable_reason: None,
            }
        }

        fn extract(&self, _image_bytes: &[u8]) -> ExtractionOutcome {
            ExtractionOutcome::NoOutput
        }

        fn extract_files(
            &self,
            _paths: &[String],
            _model_path: Option<&std::path::Path>,
        ) -> ExtractionOutcome {
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
            executable_path: None,
            model_path: None,
            input_contract: "image".into(),
            output_contract: "searchable_text".into(),
            enabled: true,
            priority: 10,
            revision: 1,
            is_builtin: false,
            is_available: true,
            unavailable_reason: None,
            runtime: crate::content_extraction::runtime_status_for("test-engine-v1", None),
            recipe: crate::content_extraction::test_recipe("image"),
            recipe_hash: "test".into(),
            default_recipe: None,
            defaults: None,
        }
    }

    fn file_extractor() -> Extractor {
        Extractor {
            id: 2,
            stable_ref: "extractor:test-file".into(),
            name: "Test Transcription".into(),
            description: String::new(),
            engine: "test-file-engine-v1".into(),
            executable_path: None,
            model_path: None,
            input_contract: "file_references".into(),
            output_contract: "searchable_text".into(),
            enabled: true,
            priority: 20,
            revision: 1,
            is_builtin: false,
            is_available: true,
            unavailable_reason: None,
            runtime: crate::content_extraction::runtime_status_for("test-file-engine-v1", None),
            recipe: crate::content_extraction::test_recipe("file_references"),
            recipe_hash: "test".into(),
            default_recipe: None,
            defaults: None,
        }
    }

    fn result(
        output: Option<&str>,
        classified_type: Option<&str>,
        matched_classifier_ref: Option<&str>,
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
            classification_matches: classified_type
                .zip(matched_classifier_ref)
                .map(|(content_type, classifier_ref)| {
                    vec![crate::content_classification::ClassificationMatch {
                        classifier_ref: classifier_ref.into(),
                        classifier_name: "Test Classifier".into(),
                        content_type: content_type.into(),
                        priority: 10,
                        start_offset: 0,
                        end_offset: output.map(str::len).unwrap_or(1).max(1),
                    }]
                })
                .unwrap_or_default(),
            failure: None,
            participants: Vec::new(),
            observations: Vec::new(),
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
                "through": "suggest",
                "targetKind": "extractor",
                "targetRef": "extractor:test",
                "outcome": "produced",
                "output": "recognized text",
                "classificationMatches": [],
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
                "through": "suggest",
                "targetKind": "extractor",
                "targetRef": "extractor:test",
                "outcome": "failed",
                "output": null,
                "classificationMatches": [],
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
    fn unavailable_engine_and_missing_downstream_input_match_the_public_fixture() {
        let failure = AnalysisFailure {
            code: "engine_not_installed".into(),
            message: "This extraction engine is not installed.".into(),
        };
        let result = ExtractionApplicationResult::preview(ExtractionResult {
            metadata: AnalysisMetadata::new(AnalysisPolicy::Interactive),
            target_kind: AnalysisTargetKind::Extractor,
            target_ref: "extractor:test".into(),
            outcome: ExtractionResultOutcome::Failed,
            output: None,
            classification_matches: Vec::new(),
            failure: Some(failure.clone()),
            participants: vec![
                ParticipantRun {
                    stable_ref: "extractor:test".into(),
                    pass: AnalysisPass::Extract,
                    outcome: ParticipantOutcome::Failed,
                    failure: Some(failure),
                },
                ParticipantRun {
                    stable_ref: crate::content_analysis::CLASSIFIER_PARTICIPANT_REF.into(),
                    pass: AnalysisPass::Classify,
                    outcome: ParticipantOutcome::MissingInput,
                    failure: None,
                },
            ],
            observations: Vec::new(),
        });
        let expected = serde_json::from_str::<serde_json::Value>(include_str!(
            "../../contracts/analysis/v1/extractor-interactive-unavailable.json"
        ))
        .unwrap();
        assert_eq!(serde_json::to_value(result).unwrap(), expected);
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
                file_references: None,
                image_bytes: None,
                searchable_text: Some("partial output".into()),
                extraction_observations: Vec::new(),
                classification_matches: vec![crate::content_classification::ClassificationMatch {
                    classifier_ref: "classifier:email".into(),
                    classifier_name: "Email".into(),
                    content_type: "email".into(),
                    priority: 10,
                    start_offset: 0,
                    end_offset: 10,
                }],
                classification_complete: true,
                structural_metadata: None,
                media_metadata: None,
                suggestions: None,
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
        assert!(result.classification_matches.is_empty());
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
            Some("classifier:email"),
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
        let classification = db.get_analysis_classifications(clip.id).unwrap().remove(0);
        assert_eq!(classification.content_type, "email");
        assert_eq!(classification.classifier_ref, "classifier:email");
        assert_eq!(classification.source_representation, "searchable_text");
    }

    #[test]
    fn file_transcription_application_preserves_paths_and_adds_searchable_text() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "file",
                Some(r#"["/tmp/interview.wav"]"#),
                None,
                None,
                "file-analysis-persistence",
                "Tests",
            )
            .unwrap();
        let engine = FixedFileEngine {
            outcome: ExtractionOutcome::Produced {
                text: "Recorded discussion about nebulae".into(),
            },
        };
        let engines: [&dyn crate::content_extraction::ExtractorEngine; 1] = [&engine];
        let registry = ExtractorEngineRegistry::new(&engines);
        let extractor = file_extractor();
        let analysis = analyze_files_with_registry(
            vec!["/tmp/interview.wav".into()],
            &extractor,
            None,
            &registry,
        );

        let applied = apply_file_analysis(
            &db,
            clip.id,
            &clip.content_hash,
            &extractor,
            false,
            analysis,
        )
        .unwrap();

        assert_eq!(applied.application.applied_clip_id, Some(clip.id));
        assert!(applied.searchable_text_updated);
        assert!(!applied.ocr_updated);
        assert_eq!(
            db.get_clip_by_id(clip.id).unwrap().text_content.as_deref(),
            Some(r#"["/tmp/interview.wav"]"#)
        );
        let stored = db.get_clip_searchable_text(clip.id).unwrap().unwrap();
        assert_eq!(stored.extractor_ref, extractor.stable_ref);
        assert_eq!(stored.searchable_text, "Recorded discussion about nebulae");
    }

    #[test]
    fn file_transcription_no_output_preserves_existing_searchable_text() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "file",
                Some(r#"["/tmp/interview.wav"]"#),
                None,
                None,
                "file-analysis-no-output",
                "Tests",
            )
            .unwrap();
        let extractor = file_extractor();
        assert!(db
            .replace_clip_searchable_text(
                clip.id,
                &clip.content_hash,
                &extractor,
                Some("Previously transcribed discussion"),
            )
            .unwrap());

        let applied = apply_file_analysis(
            &db,
            clip.id,
            &clip.content_hash,
            &extractor,
            false,
            result(None, None, None),
        )
        .unwrap();

        assert_eq!(applied.application.applied_clip_id, None);
        assert!(!applied.searchable_text_updated);
        assert_eq!(
            db.get_clip_searchable_text(clip.id)
                .unwrap()
                .unwrap()
                .searchable_text,
            "Previously transcribed discussion"
        );
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
            Some("classifier:test"),
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
        assert!(db.get_analysis_classifications(clip.id).unwrap().is_empty());
    }
}
