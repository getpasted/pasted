use crate::db::tests::setup_test_db;

fn observation(
    participant_ref: &str,
    text: &str,
) -> crate::content_analysis::ExtractionObservation {
    crate::content_analysis::ExtractionObservation {
        extractor_ref: participant_ref.into(),
        extractor_name: participant_ref.into(),
        engine: "test-v1".into(),
        priority: 10,
        duplicate_of: None,
        outcome: crate::content_extraction::ExtractionOutcome::Produced { text: text.into() },
    }
}

#[test]
fn retention_is_per_clip_and_extractor_without_pruning_current_results() {
    let db = setup_test_db();
    db.enforce_analysis_attempt_retention(2).unwrap();
    let clip = db
        .save_clip(
            "image",
            None,
            Some("image"),
            None,
            "attempt-retention",
            "Tests",
        )
        .unwrap();
    for run in 0..4 {
        for participant in ["extractor:first", "extractor:second"] {
            db.record_extraction_observations(
                clip.id,
                &clip.content_hash,
                &[observation(participant, &format!("{participant}-{run}"))],
            )
            .unwrap();
        }
    }
    let history = db.get_extraction_history(clip.id, 101, 0).unwrap();
    assert_eq!(history.len(), 4);
    assert_eq!(
        history
            .iter()
            .filter(|attempt| attempt.observation.extractor_ref == "extractor:first")
            .count(),
        2
    );
    assert_eq!(db.get_extraction_observations(clip.id).unwrap().len(), 1);
}

#[test]
fn unlimited_retention_and_failed_pruning_transaction_are_lossless() {
    let db = setup_test_db();
    db.enforce_analysis_attempt_retention(0).unwrap();
    let clip = db
        .save_clip(
            "image",
            None,
            Some("image"),
            None,
            "attempt-unlimited",
            "Tests",
        )
        .unwrap();
    for run in 0..12 {
        db.record_extraction_observations(
            clip.id,
            &clip.content_hash,
            &[observation("extractor:test", &run.to_string())],
        )
        .unwrap();
    }
    assert_eq!(
        db.get_extraction_history(clip.id, 101, 0).unwrap().len(),
        12
    );
    db.conn
        .lock()
        .execute_batch(
            "CREATE TRIGGER reject_attempt_pruning BEFORE DELETE ON clip_extraction_attempts
             BEGIN SELECT RAISE(ABORT, 'keep attempts'); END;",
        )
        .unwrap();
    assert!(db.enforce_analysis_attempt_retention(1).is_err());
    let value: String = db
        .conn
        .lock()
        .query_row(
            "SELECT value FROM settings WHERE key = 'analysisAttemptsPerClip'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(value, "0");
    assert_eq!(
        db.get_extraction_history(clip.id, 101, 0).unwrap().len(),
        12
    );
}
