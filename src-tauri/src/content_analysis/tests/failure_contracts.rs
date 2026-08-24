use super::*;

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
fn typed_engine_failures_fail_the_extractor_participant_closed() {
    let engine = failing_engine();
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
