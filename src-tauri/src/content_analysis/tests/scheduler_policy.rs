use super::*;

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
