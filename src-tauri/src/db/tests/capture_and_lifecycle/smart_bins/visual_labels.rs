use super::*;

#[test]
fn match_structured_extractor_results() {
    let db = setup_test_db();
    let dog = save_plain_test_clip(&db, "image", "dog-image", "dog-hash", "Photos");
    let pizza = save_plain_test_clip(&db, "image", "pizza-image", "pizza-hash", "Photos");
    db.record_extraction_observations(
        dog.id,
        &dog.content_hash,
        &[crate::content_analysis::ExtractionObservation {
            extractor_ref: crate::content_extraction::APPLE_VISION_LABELS_REF.into(),
            extractor_name: "Apple Vision Labels".into(),
            engine: crate::content_extraction::RECIPE_ENGINE.into(),
            priority: 15,
            duplicate_of: None,
            outcome: crate::content_extraction::ExtractionOutcome::Produced {
                text: "dog\nanimal".into(),
                labels: vec![crate::content_extraction::VisualLabel {
                    value: "dog".into(),
                    confidence_basis_points: Some(9_700),
                }],
            },
        }],
    )
    .unwrap();
    let bin = db
        .create_bin(
            "Dogs",
            "🐕",
            "default",
            Some(r#"{"conditions":[{"type":"visual_label","operator":"is","value":"dog"}],"match":"all"}"#),
        )
        .unwrap();

    assert_eq!(db.get_clips(Some(bin.id), false).unwrap()[0].id, dog.id);
    assert!(!db
        .get_clips(Some(bin.id), false)
        .unwrap()
        .iter()
        .any(|clip| clip.id == pizza.id));
}
