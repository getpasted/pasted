use super::*;

fn attempt(
    outcome: ExtractionOutcome,
    class: Option<AnalysisFailureClass>,
    retry: Option<&str>,
) -> StoredExtractionAttempt {
    StoredExtractionAttempt {
        observation: ExtractionObservation {
            extractor_ref: "extractor:test".into(),
            extractor_name: "Test".into(),
            engine: "test-v1".into(),
            priority: 10,
            duplicate_of: None,
            outcome,
        },
        run_id: "run".into(),
        run_at: "2026-08-24T00:00:00Z".into(),
        input_fingerprint: "fingerprint".into(),
        failure_class: class,
        retry_after: retry.map(str::to_string),
    }
}

fn failed(code: &str) -> ExtractionOutcome {
    ExtractionOutcome::Failed {
        failure: crate::content_extraction::ExtractionFailure {
            code: code.into(),
            message: "Failed.".into(),
        },
    }
}

#[test]
fn failure_classes_have_explicit_retry_semantics() {
    assert_eq!(
        failure_class(&failed("invalid_contract")),
        Some(AnalysisFailureClass::Terminal)
    );
    assert_eq!(
        failure_class(&failed("engine_unavailable")),
        Some(AnalysisFailureClass::Dependency)
    );
    assert_eq!(
        failure_class(&failed("engine_timeout")),
        Some(AnalysisFailureClass::Transient)
    );
    assert_eq!(failure_class(&ExtractionOutcome::NoOutput), None);

    let now = DateTime::parse_from_rfc3339("2026-08-24T00:00:10Z")
        .unwrap()
        .with_timezone(&Utc);
    let terminal = attempt(
        failed("invalid_contract"),
        Some(AnalysisFailureClass::Terminal),
        None,
    );
    let waiting = attempt(
        failed("engine_timeout"),
        Some(AnalysisFailureClass::Transient),
        Some("2026-08-24T00:00:20Z"),
    );
    assert_eq!(reuse_action(&terminal, false, now), ReuseAction::Reuse);
    assert_eq!(reuse_action(&waiting, false, now), ReuseAction::Defer);
    assert_eq!(reuse_action(&waiting, true, now), ReuseAction::Run);
}

#[test]
fn transient_backoff_is_bounded() {
    assert_eq!(
        retry_after("2026-08-24T00:00:00Z", 1).as_deref(),
        Some("2026-08-24T00:00:05Z")
    );
    assert_eq!(
        retry_after("2026-08-24T00:00:00Z", 20).as_deref(),
        Some("2026-08-24T00:05:00Z")
    );
}

#[test]
fn fingerprints_change_with_input_and_extractor_identity() {
    let mut extractor = crate::content_extraction::Extractor {
        id: 1,
        stable_ref: "extractor:test".into(),
        name: "Test".into(),
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
        recipe_hash: "recipe-v1".into(),
        default_recipe: None,
        defaults: None,
    };
    let first = image_contexts(b"first", std::slice::from_ref(&extractor));
    let changed_input = image_contexts(b"second", std::slice::from_ref(&extractor));
    extractor.revision += 1;
    let changed_extractor = image_contexts(b"first", std::slice::from_ref(&extractor));
    assert_ne!(
        first[0].input_fingerprint,
        changed_input[0].input_fingerprint
    );
    assert_ne!(
        first[0].input_fingerprint,
        changed_extractor[0].input_fingerprint
    );
}
