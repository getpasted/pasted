use crate::db::tests::setup_test_db;

fn assert_canonical_timestamp(value: &str) {
    assert_eq!(
        crate::db::canonical_utc_timestamp(value, "Test").unwrap(),
        value
    );
}

#[test]
fn extractor_observations_round_trip_per_clip_in_priority_order() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "image",
            None,
            Some("test-image"),
            None,
            "extractor-observations",
            "Tests",
        )
        .unwrap();
    let observations = vec![
        crate::content_analysis::ExtractionObservation {
            extractor_ref: "extractor:second".into(),
            extractor_name: "Second".into(),
            engine: "second-v1".into(),
            priority: 20,
            duplicate_of: None,
            outcome: crate::content_extraction::ExtractionOutcome::Produced {
                text: "Hello World!".into(),
            },
        },
        crate::content_analysis::ExtractionObservation {
            extractor_ref: "extractor:first".into(),
            extractor_name: "First".into(),
            engine: "first-v1".into(),
            priority: 10,
            duplicate_of: None,
            outcome: crate::content_extraction::ExtractionOutcome::NoOutput,
        },
    ];

    assert!(db
        .record_extraction_observations(clip.id, &clip.content_hash, &observations)
        .unwrap());
    let stored = db.get_extraction_observations(clip.id).unwrap();
    assert_eq!(stored.len(), 2);
    for observation in &stored {
        assert_canonical_timestamp(&observation.updated_at);
    }
    assert_eq!(stored[0].observation.extractor_ref, "extractor:first");
    assert_eq!(stored[1].observation.extractor_ref, "extractor:second");
    assert!(matches!(
        stored[1].observation.outcome,
        crate::content_extraction::ExtractionOutcome::Produced { ref text }
            if text == "Hello World!"
    ));
    let second_run = vec![crate::content_analysis::ExtractionObservation {
        extractor_ref: "extractor:first".into(),
        extractor_name: "First".into(),
        engine: "first-v1".into(),
        priority: 10,
        duplicate_of: None,
        outcome: crate::content_extraction::ExtractionOutcome::Failed {
            failure: crate::content_extraction::ExtractionFailure {
                code: "test_failure".into(),
                message: "The Extractor failed.".into(),
            },
        },
    }];
    assert!(db
        .record_extraction_observations(clip.id, &clip.content_hash, &second_run)
        .unwrap());
    let history = db.get_extraction_history(clip.id, 101, 0).unwrap();
    assert_eq!(history.len(), 3);
    assert_canonical_timestamp(&history[0].run_at);
    assert_eq!(history[0].observation.extractor_ref, "extractor:first");
    assert_ne!(history[0].run_id, history[1].run_id);
}

#[test]
fn permanent_clip_deletion_cascades_all_stored_analysis_records() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "file",
            Some(r#"["/tmp/stored-analysis.pdf"]"#),
            None,
            None,
            "stored-analysis-cascade",
            "Tests",
        )
        .unwrap();
    let extractor = db.get_content_extractors().unwrap().remove(0);
    db.replace_clip_searchable_text(
        clip.id,
        &clip.content_hash,
        &extractor,
        Some("searchable marker"),
    )
    .unwrap();
    let searchable = db.get_clip_searchable_text(clip.id).unwrap().unwrap();
    assert_canonical_timestamp(&searchable.updated_at);
    db.replace_analysis_classifications(
        clip.id,
        &clip.content_hash,
        &[crate::content_classification::ClassificationMatch {
            classifier_ref: "document".into(),
            classifier_name: "Document".into(),
            content_type: "document".into(),
            priority: 10,
            start_offset: 0,
            end_offset: 8,
        }],
        "searchable_text",
    )
    .unwrap();
    db.record_extraction_observations(
        clip.id,
        &clip.content_hash,
        &[crate::content_analysis::ExtractionObservation {
            extractor_ref: "extractor:cascade".into(),
            extractor_name: "Cascade".into(),
            engine: "cascade-v1".into(),
            priority: 10,
            duplicate_of: None,
            outcome: crate::content_extraction::ExtractionOutcome::NoOutput,
        }],
    )
    .unwrap();

    let tables = [
        "clip_analysis_classifications",
        "clip_analysis_results",
        "clip_extraction_attempts",
        "clip_searchable_text",
    ];
    for table in tables {
        let count: i64 = db
            .conn
            .lock()
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE clip_id = ?1"),
                [clip.id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(count > 0, "expected {table} to contain stored Analysis");
    }

    db.purge_clip_permanently(clip.id).unwrap();
    for table in tables {
        let count: i64 = db
            .conn
            .lock()
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE clip_id = ?1"),
                [clip.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "expected {table} to cascade with its clip");
    }
}
