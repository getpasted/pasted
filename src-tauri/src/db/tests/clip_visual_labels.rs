use super::*;

fn labeled_clip(db: &DbState) -> ClipItem {
    let clip = save_plain_test_clip(db, "image", "image", "labels-hash", "Photos");
    db.record_extraction_observations(
        clip.id,
        &clip.content_hash,
        &[crate::content_analysis::ExtractionObservation {
            extractor_ref: crate::content_extraction::APPLE_VISION_LABELS_REF.into(),
            extractor_name: "Apple Vision Labels".into(),
            engine: crate::content_extraction::RECIPE_ENGINE.into(),
            priority: 15,
            duplicate_of: None,
            outcome: crate::content_extraction::ExtractionOutcome::Produced {
                text: "dog".into(),
                labels: vec![crate::content_extraction::VisualLabel {
                    value: "dog".into(),
                    confidence_basis_points: Some(9_700),
                }],
            },
        }],
    )
    .unwrap();
    clip
}

#[test]
fn manual_labels_overlay_immutable_detected_labels() {
    let db = setup_test_db();
    let clip = labeled_clip(&db);

    let added = db.add_visual_label(clip.id, " Favorite ").unwrap();
    assert_eq!(added.labels.len(), 2);
    assert_eq!(
        added.labels[1].source,
        crate::db::clip_visual_labels::VisualLabelSource::Manual
    );
    assert_eq!(search_test_clips(&db, "favorite")[0].id, clip.id);
    assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 1);
    db.add_visual_label(clip.id, "favorite").unwrap();
    assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 1);

    let removed = db.remove_visual_label(clip.id, "dog").unwrap();
    assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 2);
    assert_eq!(removed.labels[0].value, "Favorite");
    assert!(removed.has_overrides);
    assert!(search_test_clips(&db, "dog").is_empty());
    let dogs = db
        .create_bin(
            "Dogs",
            "dog",
            "default",
            Some(r#"{"conditions":[{"type":"visual_label","operator":"is","value":"dog"}],"match":"all"}"#),
        )
        .unwrap();
    let favorites = db
        .create_bin(
            "Favorites",
            "star",
            "default",
            Some(r#"{"conditions":[{"type":"visual_label","operator":"is","value":"favorite"}],"match":"all"}"#),
        )
        .unwrap();
    assert!(db.get_clips(Some(dogs.id), false).unwrap().is_empty());
    assert_eq!(
        db.get_clips(Some(favorites.id), false).unwrap()[0].id,
        clip.id
    );
    assert_eq!(db.get_extraction_observations(clip.id).unwrap().len(), 1);

    let reset = db.reset_visual_labels(clip.id).unwrap();
    assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 3);
    assert_eq!(reset.labels.len(), 1);
    assert_eq!(reset.labels[0].value, "dog");
    assert!(!reset.has_overrides);
    assert_eq!(search_test_clips(&db, "dog")[0].id, clip.id);
}

#[test]
fn adding_a_suppressed_detected_label_restores_it() {
    let db = setup_test_db();
    let clip = labeled_clip(&db);

    db.remove_visual_label(clip.id, "DOG").unwrap();
    let restored = db.add_visual_label(clip.id, "dog").unwrap();

    assert_eq!(restored.labels.len(), 1);
    assert_eq!(
        restored.labels[0].source,
        crate::db::clip_visual_labels::VisualLabelSource::Detected
    );
    assert!(!restored.has_overrides);
}

#[test]
fn history_and_organization_transfer_preserves_manual_overrides() {
    let source = setup_test_db();
    let clip = labeled_clip(&source);
    source.add_visual_label(clip.id, "favorite").unwrap();
    source.remove_visual_label(clip.id, "dog").unwrap();

    let archive = source.export_backup_json().unwrap();
    let target = setup_test_db();
    target.import_backup_json(&archive).unwrap();
    let imported = target.get_all_clips_for_backup().unwrap().remove(0);
    let labels = target.get_effective_visual_labels(imported.id).unwrap();

    assert!(labels.has_overrides);
    assert_eq!(labels.labels.len(), 1);
    assert_eq!(labels.labels[0].value, "favorite");
}
