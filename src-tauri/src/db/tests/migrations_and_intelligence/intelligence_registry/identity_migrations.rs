use super::super::super::*;

#[test]
fn legacy_apple_vision_labels_identity_is_migrated_without_losing_history() {
    let db = setup_test_db();
    let clip = save_plain_test_clip(
        &db,
        "image",
        "dog",
        "legacy-apple-labels-ref-hash",
        "Photos",
    );
    let observation = crate::content_analysis::ExtractionObservation {
        extractor_ref: crate::content_extraction::APPLE_VISION_LABELS_REF.into(),
        extractor_name: "Apple Vision Labels".into(),
        engine: crate::content_extraction::RECIPE_ENGINE.into(),
        priority: 15,
        duplicate_of: None,
        outcome: crate::content_extraction::ExtractionOutcome::Produced {
            text: String::new(),
            labels: vec![crate::content_extraction::VisualLabel {
                value: "dog".into(),
                confidence_basis_points: Some(9_500),
            }],
        },
    };
    db.record_extraction_observations(clip.id, &clip.content_hash, &[observation])
        .unwrap();
    let old_ref = crate::content_extraction::LEGACY_APPLE_VISION_LABELS_REF;
    let new_ref = crate::content_extraction::APPLE_VISION_LABELS_REF;
    {
        let conn = db.conn.lock();
        conn.execute(
            "UPDATE content_extractors SET stable_ref = ?1 WHERE stable_ref = ?2",
            rusqlite::params![old_ref, new_ref],
        )
        .unwrap();
        for table in ["clip_analysis_results", "clip_extraction_attempts"] {
            conn.execute(
                &format!(
                    "UPDATE {table}
                     SET participant_ref = ?1, result_json = REPLACE(result_json, ?2, ?1)
                     WHERE participant_ref = ?2"
                ),
                rusqlite::params![old_ref, new_ref],
            )
            .unwrap();
        }
        conn.execute(
            "UPDATE clips SET ocr_extractor_ref = ?1 WHERE id = ?2",
            rusqlite::params![old_ref, clip.id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO clip_versions (clip_id, text_content, context_json) VALUES (?1, '', ?2)",
            rusqlite::params![
                clip.id,
                serde_json::json!({ "extractorRef": old_ref }).to_string()
            ],
        )
        .unwrap();
    }
    let path = db.database_path();
    drop(db);

    let migrated = DbState::new(path).unwrap();
    assert!(migrated.get_content_extractor(old_ref).is_err());
    assert_eq!(
        migrated.get_content_extractor(new_ref).unwrap().name,
        "Apple Vision Labels"
    );
    assert_eq!(
        migrated.get_extraction_observations(clip.id).unwrap()[0]
            .observation
            .extractor_ref,
        new_ref
    );
    assert_eq!(
        migrated.get_extraction_history(clip.id, 10, 0).unwrap()[0]
            .observation
            .extractor_ref,
        new_ref
    );
    let conn = migrated.conn.lock();
    let stale_references: i64 = conn
        .query_row(
            "SELECT
                (SELECT COUNT(*) FROM clips WHERE ocr_extractor_ref = ?1) +
                (SELECT COUNT(*) FROM clip_versions WHERE context_json LIKE '%' || ?1 || '%')",
            [old_ref],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stale_references, 0);
}
