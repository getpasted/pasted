use super::super::*;

#[test]
fn ocr_state_is_hash_safe_and_follows_the_clip_lifecycle() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "image",
            None,
            None,
            Some(crate::resource_limits::TEST_PNG_DATA_URL),
            "ocr-lifecycle-hash",
            "Screenshot",
        )
        .unwrap();

    let status = db.get_ocr_backfill_status().unwrap();
    assert_eq!(status.total_images, 1);
    assert_eq!(status.eligible_count, 1);
    assert_eq!(
        db.get_ocr_backfill_clip_ids("images").unwrap(),
        vec![clip.id]
    );
    assert_eq!(
        db.get_ocr_backfill_clip_ids("waiting").unwrap(),
        vec![clip.id]
    );
    assert!(db.get_ocr_backfill_clip_ids("complete").unwrap().is_empty());
    assert!(db.get_ocr_backfill_clip_ids("unknown").is_err());

    let candidate = db.claim_next_ocr_candidate().unwrap().unwrap();
    assert_eq!(candidate.clip_id, clip.id);
    assert!(db
        .complete_ocr_attempt(
            clip.id,
            "wrong-hash",
            Some("stale result"),
            "test-engine",
            None,
        )
        .is_ok());
    assert_eq!(
        db.get_clip_by_id(clip.id).unwrap().text_content.as_deref(),
        None
    );

    db.delete_clip(clip.id).unwrap();
    assert!(!db
        .complete_ocr_attempt(
            clip.id,
            &clip.content_hash,
            Some("late result"),
            "test-engine",
            None,
        )
        .unwrap());
    assert_eq!(db.get_ocr_backfill_status().unwrap().total_images, 0);

    db.restore_clip(clip.id).unwrap();
    assert_eq!(db.get_ocr_backfill_status().unwrap().eligible_count, 1);
    db.save_setting("enableOcr", "false").unwrap();
    db.purge_clip_permanently(clip.id).unwrap();
    assert!(db.get_clip_by_id(clip.id).is_err());
    assert_eq!(db.get_ocr_backfill_status().unwrap().total_images, 0);
}

#[test]
fn successful_ocr_records_state_and_revisions_only_when_text_changes() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "image",
            None,
            None,
            Some(crate::resource_limits::TEST_PNG_DATA_URL),
            "ocr-success-hash",
            "Screenshot",
        )
        .unwrap();

    assert!(db
        .complete_ocr_attempt_with_extractor(
            clip.id,
            &clip.content_hash,
            Some("First OCR"),
            OcrExtractorProvenance::identified("test-engine-v1", "extractor:test-ocr", "Test OCR",),
            None,
        )
        .unwrap());
    let completed_clip = db.get_clip_by_id(clip.id).unwrap();
    assert_eq!(
        completed_clip.ocr_extractor_ref.as_deref(),
        Some("extractor:test-ocr")
    );
    assert_eq!(
        completed_clip.ocr_extractor_name.as_deref(),
        Some("Test OCR")
    );
    assert_eq!(
        completed_clip.ocr_engine_version.as_deref(),
        Some("test-engine-v1")
    );
    assert_eq!(db.get_ocr_backfill_status().unwrap().completed_count, 1);
    assert_eq!(
        db.get_ocr_backfill_clip_ids("complete").unwrap(),
        vec![clip.id]
    );
    assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 1);
    assert_eq!(db.get_clip_version_timeline_count(clip.id).unwrap(), 2);
    let timeline = db.get_clip_version_timeline_page(clip.id, 50, 0).unwrap();
    assert_eq!(timeline.len(), 2);
    assert!(timeline[0].is_current);
    assert_eq!(timeline[0].action_kind.as_deref(), Some("current"));
    assert_eq!(timeline[0].text_content, "First OCR");
    assert!(!timeline[1].is_current);
    assert_eq!(timeline[1].action_kind.as_deref(), Some("original"));
    let original = db.get_clip_versions(clip.id).unwrap().remove(0);
    assert_eq!(original.action_kind.as_deref(), Some("original"));
    assert!(original.text_content.is_empty());

    db.force_ocr_running(clip.id, &clip.content_hash).unwrap();
    db.complete_ocr_attempt_with_extractor(
        clip.id,
        &clip.content_hash,
        Some("Improved OCR"),
        OcrExtractorProvenance::identified("test-engine-v2", "extractor:test-ocr-v2", "Test OCR 2"),
        None,
    )
    .unwrap();
    assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 2);
    assert_eq!(
        db.get_clip_versions(clip.id).unwrap()[0].text_content,
        "First OCR"
    );

    db.force_ocr_running(clip.id, &clip.content_hash).unwrap();
    db.complete_ocr_attempt_with_extractor(
        clip.id,
        &clip.content_hash,
        Some("Improved OCR"),
        OcrExtractorProvenance::identified("test-engine-v2", "extractor:test-ocr-v2", "Test OCR 2"),
        None,
    )
    .unwrap();
    assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 2);

    db.force_ocr_running(clip.id, &clip.content_hash).unwrap();
    db.complete_ocr_attempt_with_extractor(
        clip.id,
        &clip.content_hash,
        None,
        OcrExtractorProvenance::identified(
            "failed-engine-v1",
            "extractor:failed-ocr",
            "Failed OCR",
        ),
        Some("recognition_failed"),
    )
    .unwrap();
    let failed_rerun = db.get_clip_by_id(clip.id).unwrap();
    assert_eq!(failed_rerun.text_content.as_deref(), Some("Improved OCR"));
    assert_eq!(
        failed_rerun.ocr_extractor_name.as_deref(),
        Some("Test OCR 2")
    );
    assert_eq!(
        failed_rerun.ocr_engine_version.as_deref(),
        Some("test-engine-v2")
    );
}

#[test]
fn restoring_image_revisions_restores_visual_labels_and_the_original_state() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "image",
            None,
            None,
            Some(crate::resource_limits::TEST_PNG_DATA_URL),
            "visual-label-revision-hash",
            "Screenshot",
        )
        .unwrap();
    let observation = |label: &str| crate::content_analysis::ExtractionObservation {
        extractor_ref: crate::content_extraction::APPLE_VISION_LABELS_REF.into(),
        extractor_name: "Apple Vision Labels".into(),
        engine: crate::content_extraction::RECIPE_ENGINE.into(),
        priority: 15,
        duplicate_of: None,
        outcome: crate::content_extraction::ExtractionOutcome::Produced {
            text: String::new(),
            labels: vec![crate::content_extraction::VisualLabel {
                value: label.into(),
                confidence_basis_points: Some(9_500),
            }],
        },
    };

    db.complete_ocr_attempt_with_extractor_and_revision(
        clip.id,
        &clip.content_hash,
        None,
        OcrExtractorProvenance::identified(
            crate::content_extraction::RECIPE_ENGINE,
            crate::content_extraction::APPLE_VISION_LABELS_REF,
            "Apple Vision Labels",
        ),
        None,
        true,
    )
    .unwrap();
    db.record_extraction_observations(clip.id, &clip.content_hash, &[observation("dog")])
        .unwrap();
    db.add_visual_label(clip.id, "favorite").unwrap();

    db.force_ocr_running(clip.id, &clip.content_hash).unwrap();
    db.complete_ocr_attempt_with_extractor_and_revision(
        clip.id,
        &clip.content_hash,
        None,
        OcrExtractorProvenance::identified(
            crate::content_extraction::RECIPE_ENGINE,
            crate::content_extraction::APPLE_VISION_LABELS_REF,
            "Apple Vision Labels",
        ),
        None,
        true,
    )
    .unwrap();
    db.record_extraction_observations(clip.id, &clip.content_hash, &[observation("cat")])
        .unwrap();

    let versions = db.get_clip_versions(clip.id).unwrap();
    assert_eq!(versions.len(), 3);
    assert_eq!(
        versions[0]
            .visual_labels
            .as_ref()
            .unwrap()
            .labels
            .iter()
            .map(|label| label.value.as_str())
            .collect::<Vec<_>>(),
        vec!["dog", "favorite"]
    );

    db.restore_clip_version(clip.id, versions[0].id).unwrap();
    let restored = db.get_effective_visual_labels(clip.id).unwrap();
    assert_eq!(
        restored
            .labels
            .iter()
            .map(|label| label.value.as_str())
            .collect::<Vec<_>>(),
        vec!["dog", "favorite"]
    );

    let inverse = db
        .get_clip_versions(clip.id)
        .unwrap()
        .into_iter()
        .find(|version| version.action_kind.as_deref() == Some("restore"))
        .unwrap();
    db.restore_clip_version(clip.id, inverse.id).unwrap();
    let restored_inverse = db.get_effective_visual_labels(clip.id).unwrap();
    assert_eq!(
        restored_inverse
            .labels
            .iter()
            .map(|label| label.value.as_str())
            .collect::<Vec<_>>(),
        vec!["cat", "favorite"]
    );

    let original = versions
        .iter()
        .find(|version| version.action_kind.as_deref() == Some("original"))
        .unwrap();
    db.restore_clip_version(clip.id, original.id).unwrap();
    assert!(db
        .get_effective_visual_labels(clip.id)
        .unwrap()
        .labels
        .is_empty());
    assert!(db.get_extraction_observations(clip.id).unwrap().is_empty());

    db.enforce_revision_retention(1).unwrap();
    let retained = db.get_clip_versions(clip.id).unwrap();
    assert_eq!(
        retained
            .iter()
            .filter(|version| version.action_kind.as_deref() == Some("original"))
            .count(),
        1
    );
    assert!(retained.len() <= 2);
}

#[test]
fn legacy_image_revision_restore_clears_unversioned_extraction_metadata() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "image",
            Some("Current OCR"),
            None,
            Some(crate::resource_limits::TEST_PNG_DATA_URL),
            "legacy-image-revision-hash",
            "Screenshot",
        )
        .unwrap();
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
    let context = serde_json::json!({
        "schema_version": 1,
        "action_kind": "ocr",
        "action_label": "Updated OCR text",
        "organization": null,
        "current_transformation_id": null
    });
    db.conn
        .lock()
        .execute(
            "INSERT INTO clip_versions (clip_id, text_content, context_json)
             VALUES (?1, 'Earlier OCR', ?2)",
            rusqlite::params![clip.id, context.to_string()],
        )
        .unwrap();
    let version = db.get_clip_versions(clip.id).unwrap().remove(0);

    let restored = db.restore_clip_version(clip.id, version.id).unwrap();
    assert_eq!(restored.text_content.as_deref(), Some("Earlier OCR"));
    assert_eq!(restored.ocr_extractor_name.as_deref(), Some("Legacy OCR"));
    assert!(db.get_extraction_observations(clip.id).unwrap().is_empty());
    assert!(db
        .get_effective_visual_labels(clip.id)
        .unwrap()
        .labels
        .is_empty());
}
