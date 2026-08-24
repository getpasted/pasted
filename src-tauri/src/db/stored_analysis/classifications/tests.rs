use crate::db::tests::setup_test_db;

#[test]
fn derived_analysis_classification_is_hash_safe_and_non_destructive() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "image",
            None,
            None,
            Some(crate::resource_limits::TEST_PNG_DATA_URL),
            "analysis-image-hash",
            "Screenshot",
        )
        .unwrap();

    assert!(db
        .replace_analysis_classifications(
            clip.id,
            &clip.content_hash,
            &[crate::content_classification::ClassificationMatch {
                classifier_ref: "email".into(),
                classifier_name: "Email".into(),
                content_type: "email".into(),
                priority: 10,
                start_offset: 0,
                end_offset: 5,
            }],
            "searchable_text",
        )
        .unwrap());
    let classification = db.get_analysis_classifications(clip.id).unwrap().remove(0);
    assert_eq!(classification.content_type, "email");
    assert_eq!(classification.source_representation, "searchable_text");
    assert_eq!(
        crate::db::canonical_utc_timestamp(&classification.updated_at, "Test").unwrap(),
        classification.updated_at
    );
    assert_eq!(db.get_clip_by_id(clip.id).unwrap().content_type, "image");

    assert!(!db
        .replace_analysis_classifications(
            clip.id,
            "stale-hash",
            &[crate::content_classification::ClassificationMatch {
                classifier_ref: "credential".into(),
                classifier_name: "Credential".into(),
                content_type: "credential".into(),
                priority: 10,
                start_offset: 0,
                end_offset: 5,
            }],
            "searchable_text",
        )
        .unwrap());
    assert_eq!(
        db.get_analysis_classifications(clip.id).unwrap()[0].content_type,
        "email"
    );

    db.replace_analysis_classifications(clip.id, &clip.content_hash, &[], "searchable_text")
        .unwrap();
    assert!(db.get_analysis_classifications(clip.id).unwrap().is_empty());
}
