use super::*;

#[test]
fn visual_labels_share_search_and_extraction_history_contracts() {
    let outcome = ExtractionOutcome::Produced {
        text: "dog\nanimal".into(),
        labels: vec![crate::content_extraction::VisualLabel {
            value: "dog".into(),
            confidence_basis_points: Some(9_600),
        }],
    };
    let engine = engine(outcome);
    let engines: [&dyn crate::content_extraction::ExtractorEngine; 1] = [&engine];
    let registry = ExtractorEngineRegistry::new(&engines);
    let report = analyze_test_image(vec![1, 2, 3], &extractor(), None, &registry);

    assert_eq!(
        report.context.searchable_text.as_deref(),
        Some("dog\nanimal")
    );
    let ExtractionOutcome::Produced { labels, .. } =
        &report.context.extraction_observations[0].outcome
    else {
        panic!("expected structured Visual Labels");
    };
    assert_eq!(labels[0].value, "dog");
}
