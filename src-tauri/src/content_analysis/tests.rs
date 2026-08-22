use super::*;

struct TestEngine;

struct FailingEngine;

struct StaticEngine {
    id: &'static str,
    outcome: ExtractionOutcome,
}

impl crate::content_extraction::ExtractorEngine for TestEngine {
    fn id(&self) -> &'static str {
        "test-v1"
    }

    fn availability(&self) -> crate::content_extraction::EngineAvailability {
        crate::content_extraction::EngineAvailability {
            is_available: true,
            unavailable_reason: None,
        }
    }

    fn extract(&self, _image_bytes: &[u8]) -> ExtractionOutcome {
        ExtractionOutcome::Produced {
            text: "agent@example.com".into(),
        }
    }
}

impl crate::content_extraction::ExtractorEngine for FailingEngine {
    fn id(&self) -> &'static str {
        "test-v1"
    }

    fn availability(&self) -> crate::content_extraction::EngineAvailability {
        crate::content_extraction::EngineAvailability {
            is_available: true,
            unavailable_reason: None,
        }
    }

    fn extract(&self, _image_bytes: &[u8]) -> ExtractionOutcome {
        ExtractionOutcome::Failed {
            failure: crate::content_extraction::ExtractionFailure {
                code: "test_failure".into(),
                message: "The test engine failed.".into(),
            },
        }
    }
}

impl crate::content_extraction::ExtractorEngine for StaticEngine {
    fn id(&self) -> &'static str {
        self.id
    }

    fn availability(&self) -> crate::content_extraction::EngineAvailability {
        crate::content_extraction::EngineAvailability {
            is_available: true,
            unavailable_reason: None,
        }
    }

    fn extract(&self, _image_bytes: &[u8]) -> ExtractionOutcome {
        self.outcome.clone()
    }
}

fn classifier(pattern: &str, content_type: &str) -> Classifier {
    Classifier {
        id: 1,
        stable_ref: format!("test:{content_type}"),
        name: content_type.into(),
        content_type: content_type.into(),
        description: String::new(),
        patterns: vec![pattern.into()],
        validator: None,
        enabled: true,
        priority: 10,
        is_builtin: false,
        defaults: None,
        is_deleted: false,
    }
}

fn extractor() -> Extractor {
    Extractor {
        id: 1,
        stable_ref: "extractor:test".into(),
        name: "Test OCR".into(),
        description: String::new(),
        engine: "test-v1".into(),
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
        runtime: crate::content_extraction::runtime_status_for("test-v1", None),
        recipe: crate::content_extraction::test_recipe("image"),
        recipe_hash: "test".into(),
        default_recipe: None,
        defaults: None,
    }
}

fn analyze_test_image(
    image_bytes: Vec<u8>,
    extractor: &Extractor,
    classifiers: Option<&[Classifier]>,
    registry: &ExtractorEngineRegistry<'_>,
) -> AnalysisReport {
    analyze(AnalysisRequest {
        input: AnalysisInput::Image {
            image_bytes,
            searchable_text: None,
            source: None,
        },
        policy: AnalysisPolicy::Interactive,
        inspector: false,
        file_format_inspector: false,
        extractors: vec![ExtractorParticipantSource {
            extractor,
            registry,
        }],
        classifiers,
        suggestion: None,
    })
}

fn analyze_test_text(text: &str, classifiers: &[Classifier]) -> AnalysisReport {
    analyze(AnalysisRequest {
        input: AnalysisInput::Text {
            text: text.into(),
            source: None,
        },
        policy: AnalysisPolicy::Capture,
        inspector: false,
        file_format_inspector: false,
        extractors: Vec::new(),
        classifiers: Some(classifiers),
        suggestion: None,
    })
}

#[test]
fn media_inspection_is_interactive_only() {
    let request = |policy| AnalysisRequest {
        input: AnalysisInput::Files {
            paths: vec!["/missing/private-recording.wav".into()],
            source: Some("Finder".into()),
        },
        policy,
        inspector: true,
        file_format_inspector: false,
        extractors: Vec::new(),
        classifiers: None,
        suggestion: None,
    };
    let capture = analyze(request(AnalysisPolicy::Capture));
    assert_eq!(capture.runs.len(), 1);
    assert_eq!(
        capture.runs[0].stable_ref,
        crate::content_inspection::STRUCTURE_INSPECTOR_REF
    );

    let interactive = analyze(request(AnalysisPolicy::Interactive));
    assert_eq!(interactive.runs.len(), 2);
    assert!(interactive
        .runs
        .iter()
        .any(|run| run.stable_ref == crate::content_inspection::MEDIA_INSPECTOR_REF));
}

#[test]
fn extraction_makes_text_available_to_later_classification() {
    let classifiers = vec![classifier(r"^[^@]+@[^@]+\.[^@]+$", "email")];
    let engine = TestEngine;
    let engines: [&dyn crate::content_extraction::ExtractorEngine; 1] = [&engine];
    let registry = ExtractorEngineRegistry::new(&engines);
    let report = analyze_test_image(vec![1, 2, 3], &extractor(), Some(&classifiers), &registry);

    assert_eq!(
        report.context.searchable_text.as_deref(),
        Some("agent@example.com")
    );
    assert_eq!(
        report.context.classification_matches[0].content_type,
        "email"
    );
    assert_eq!(report.runs.len(), 2);
    assert_eq!(report.runs[0].pass, AnalysisPass::Extract);
    assert_eq!(report.runs[1].pass, AnalysisPass::Classify);
}

#[test]
fn unresolved_missing_representations_skip_without_execution() {
    let runs = schedule(
        AnalysisContext::for_text("hello"),
        vec![AnalysisParticipant::new(
            ParticipantContract {
                stable_ref: "needs-image".into(),
                name: "Needs Image".into(),
                pass: AnalysisPass::Suggest,
                priority: 1,
                requires: vec![RepresentationKind::ImageBytes],
                provides: vec![RepresentationKind::SearchableText],
            },
            |_| panic!("a participant with missing inputs must not execute"),
        )],
        AnalysisPass::Suggest,
    )
    .runs;

    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].outcome, ParticipantOutcome::MissingInput);
}

#[test]
fn bounded_policies_exclude_later_participants_without_reporting_fake_runs() {
    let report = schedule(
        AnalysisContext::for_text("hello"),
        vec![AnalysisParticipant::new(
            ParticipantContract {
                stable_ref: "suggestion:test".into(),
                name: "Test Suggestion".into(),
                pass: AnalysisPass::Suggest,
                priority: 1,
                requires: vec![RepresentationKind::AnalyzableText],
                provides: vec![],
            },
            |_| panic!("capture policy must not execute Suggest participants"),
        )],
        AnalysisPolicy::Capture.through(),
    );

    assert!(report.runs.is_empty());
}

#[test]
fn typed_inputs_carry_capture_source_without_changing_analyzable_text() {
    let report = analyze(AnalysisRequest {
        input: AnalysisInput::Text {
            text: "hello".into(),
            source: Some("Terminal".into()),
        },
        policy: AnalysisPolicy::Capture,
        inspector: false,
        file_format_inspector: false,
        extractors: Vec::new(),
        classifiers: None,
        suggestion: None,
    });

    assert!(report.context.has(RepresentationKind::CaptureSource));
    assert_eq!(report.context.analysis_text(), Some("hello"));
    assert!(report.runs.is_empty());
}

#[test]
fn participant_resolution_normalizes_scheduler_failures_for_every_surface() {
    let failed = AnalysisReport {
        context: AnalysisContext::for_text("text"),
        runs: vec![ParticipantRun {
            stable_ref: "participant:test".into(),
            pass: AnalysisPass::Suggest,
            outcome: ParticipantOutcome::Failed,
            failure: None,
        }],
    }
    .resolve_participant("participant:test", AnalysisTargetKind::Suggestion);
    assert_eq!(failed.outcome, ParticipantOutcome::Failed);
    assert_eq!(failed.failure.unwrap().code, "analysis_failed");

    let missing_input = AnalysisReport {
        context: AnalysisContext::for_text("text"),
        runs: vec![ParticipantRun {
            stable_ref: "participant:test".into(),
            pass: AnalysisPass::Suggest,
            outcome: ParticipantOutcome::MissingInput,
            failure: None,
        }],
    }
    .resolve_participant("participant:test", AnalysisTargetKind::Suggestion);
    assert_eq!(missing_input.failure.unwrap().code, "missing_input");

    let missing_participant = AnalysisReport {
        context: AnalysisContext::for_text("text"),
        runs: Vec::new(),
    }
    .resolve_participant("participant:test", AnalysisTargetKind::Suggestion);
    assert_eq!(
        missing_participant.failure.unwrap().code,
        "missing_participant"
    );
}

#[test]
fn same_pass_consumers_run_after_their_inputs_become_available() {
    let report = schedule(
        AnalysisContext::for_image(vec![1]),
        vec![
            AnalysisParticipant::new(
                ParticipantContract {
                    stable_ref: "consumer".into(),
                    name: "Consumer".into(),
                    pass: AnalysisPass::Extract,
                    priority: 1,
                    requires: vec![RepresentationKind::SearchableText],
                    provides: vec![RepresentationKind::Classification],
                },
                |context| {
                    context.classification_matches =
                        vec![crate::content_classification::ClassificationMatch {
                            classifier_ref: "test:derived".into(),
                            classifier_name: "Derived".into(),
                            content_type: "derived".into(),
                            priority: 1,
                            start_offset: 0,
                            end_offset: 7,
                        }];
                    context.classification_complete = true;
                    Ok(ParticipantOutcome::Produced)
                },
            ),
            AnalysisParticipant::new(
                ParticipantContract {
                    stable_ref: "producer".into(),
                    name: "Producer".into(),
                    pass: AnalysisPass::Extract,
                    priority: 2,
                    requires: vec![RepresentationKind::ImageBytes],
                    provides: vec![RepresentationKind::SearchableText],
                },
                |context| {
                    context.searchable_text = Some("derived text".into());
                    Ok(ParticipantOutcome::Produced)
                },
            ),
            AnalysisParticipant::new(
                ParticipantContract {
                    stable_ref: "independent".into(),
                    name: "Independent".into(),
                    pass: AnalysisPass::Extract,
                    priority: 3,
                    requires: vec![RepresentationKind::ImageBytes],
                    provides: vec![],
                },
                |_| Ok(ParticipantOutcome::Produced),
            ),
        ],
        AnalysisPass::Suggest,
    );

    assert_eq!(
        report.context.classification_matches[0].content_type,
        "derived"
    );
    assert_eq!(report.runs.len(), 3);
    assert_eq!(report.runs[0].stable_ref, "producer");
    assert_eq!(report.runs[1].stable_ref, "consumer");
    assert_eq!(report.runs[2].stable_ref, "independent");
    assert!(report
        .runs
        .iter()
        .all(|run| run.outcome == ParticipantOutcome::Produced));
}

#[test]
fn typed_engine_failures_fail_the_extractor_participant_closed() {
    let engine = FailingEngine;
    let engines: [&dyn crate::content_extraction::ExtractorEngine; 1] = [&engine];
    let registry = ExtractorEngineRegistry::new(&engines);
    let report = analyze_test_image(vec![1], &extractor(), None, &registry);

    assert_eq!(report.context.searchable_text, None);
    assert_eq!(report.runs[0].outcome, ParticipantOutcome::Failed);
    assert_eq!(
        report.runs[0].failure.as_ref(),
        Some(&AnalysisFailure {
            code: "test_failure".into(),
            message: "The test engine failed.".into(),
        })
    );
}

#[test]
fn compatible_extractors_all_run_in_priority_order_and_keep_observations() {
    let first_engine = StaticEngine {
        id: "first-v1",
        outcome: ExtractionOutcome::NoOutput,
    };
    let second_engine = StaticEngine {
        id: "second-v1",
        outcome: ExtractionOutcome::Produced {
            text: "Hello World!".into(),
        },
    };
    let third_engine = StaticEngine {
        id: "third-v1",
        outcome: ExtractionOutcome::Failed {
            failure: crate::content_extraction::ExtractionFailure {
                code: "test_failure".into(),
                message: "Could not inspect the image.".into(),
            },
        },
    };
    let duplicate_engine = StaticEngine {
        id: "duplicate-v1",
        outcome: ExtractionOutcome::Produced {
            text: "Hello World!".into(),
        },
    };
    let engines: [&dyn crate::content_extraction::ExtractorEngine; 4] = [
        &first_engine,
        &second_engine,
        &third_engine,
        &duplicate_engine,
    ];
    let registry = ExtractorEngineRegistry::new(&engines);
    let mut first = extractor();
    first.stable_ref = "extractor:first".into();
    first.name = "First".into();
    first.engine = "first-v1".into();
    first.priority = 10;
    let mut second = extractor();
    second.stable_ref = "extractor:second".into();
    second.name = "Second".into();
    second.engine = "second-v1".into();
    second.priority = 20;
    let mut third = extractor();
    third.stable_ref = "extractor:third".into();
    third.name = "Third".into();
    third.engine = "third-v1".into();
    third.priority = 30;
    let mut duplicate = extractor();
    duplicate.stable_ref = "extractor:duplicate".into();
    duplicate.name = "Duplicate".into();
    duplicate.engine = "duplicate-v1".into();
    duplicate.priority = 25;

    let report = analyze(AnalysisRequest {
        input: AnalysisInput::Image {
            image_bytes: vec![1],
            searchable_text: None,
            source: None,
        },
        policy: AnalysisPolicy::Interactive,
        inspector: false,
        file_format_inspector: false,
        extractors: vec![
            ExtractorParticipantSource {
                extractor: &third,
                registry: &registry,
            },
            ExtractorParticipantSource {
                extractor: &first,
                registry: &registry,
            },
            ExtractorParticipantSource {
                extractor: &second,
                registry: &registry,
            },
            ExtractorParticipantSource {
                extractor: &duplicate,
                registry: &registry,
            },
        ],
        classifiers: None,
        suggestion: None,
    });

    assert_eq!(
        report.context.searchable_text.as_deref(),
        Some("Hello World!")
    );
    assert_eq!(
        report
            .runs
            .iter()
            .map(|run| run.stable_ref.as_str())
            .collect::<Vec<_>>(),
        vec![
            "extractor:first",
            "extractor:second",
            "extractor:duplicate",
            "extractor:third"
        ]
    );
    assert_eq!(report.context.extraction_observations.len(), 4);
    assert!(matches!(
        report.context.extraction_observations[0].outcome,
        ExtractionOutcome::NoOutput
    ));
    assert!(matches!(
        report.context.extraction_observations[1].outcome,
        ExtractionOutcome::Produced { .. }
    ));
    assert!(matches!(
        report.context.extraction_observations[2].outcome,
        ExtractionOutcome::Produced { .. }
    ));
    assert_eq!(
        report.context.extraction_observations[2]
            .duplicate_of
            .as_deref(),
        Some("extractor:second")
    );
    assert!(matches!(
        report.context.extraction_observations[3].outcome,
        ExtractionOutcome::Failed { .. }
    ));
}

#[test]
fn participants_fail_closed_when_declared_outputs_are_missing() {
    let runs = schedule(
        AnalysisContext::for_text("hello"),
        vec![AnalysisParticipant::new(
            ParticipantContract {
                stable_ref: "broken-extractor".into(),
                name: "Broken Extractor".into(),
                pass: AnalysisPass::Extract,
                priority: 1,
                requires: vec![RepresentationKind::OriginalText],
                provides: vec![RepresentationKind::SearchableText],
            },
            |_| Ok(ParticipantOutcome::Produced),
        )],
        AnalysisPass::Suggest,
    )
    .runs;

    assert_eq!(runs[0].outcome, ParticipantOutcome::Failed);
    assert_eq!(
        runs[0]
            .failure
            .as_ref()
            .map(|failure| failure.code.as_str()),
        Some("contract_violation")
    );
}

#[test]
fn text_classification_uses_the_same_scheduler_contract() {
    let classifiers = vec![classifier(r"^#[0-9a-fA-F]{6}$", "color")];
    let report = analyze_test_text("#112233", &classifiers);
    assert_eq!(
        report.context.classification_matches[0].content_type,
        "color"
    );
    assert_eq!(report.runs[0].stable_ref, "analysis:content-classifiers");
}
