use super::*;

fn produced(text: &str) -> ExtractionOutcome {
    ExtractionOutcome::Produced {
        text: text.into(),
        labels: Vec::new(),
    }
}

#[test]
fn extraction_makes_text_available_to_later_classification() {
    let classifiers = vec![classifier(r"^[^@]+@[^@]+\.[^@]+$", "email")];
    let engine = engine(produced("agent@example.com"));
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
fn compatible_extractors_all_run_in_priority_order_and_keep_observations() {
    let first_engine = StaticEngine {
        id: "first-v1",
        outcome: ExtractionOutcome::NoOutput,
    };
    let second_engine = StaticEngine {
        id: "second-v1",
        outcome: produced("Hello World!"),
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
        outcome: produced("Hello World!"),
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
fn text_classification_uses_the_same_scheduler_contract() {
    let classifiers = vec![classifier(r"^#[0-9a-fA-F]{6}$", "color")];
    let report = analyze_test_text("#112233", &classifiers);
    assert_eq!(
        report.context.classification_matches[0].content_type,
        "color"
    );
    assert_eq!(report.runs[0].stable_ref, "analysis:content-classifiers");
}
