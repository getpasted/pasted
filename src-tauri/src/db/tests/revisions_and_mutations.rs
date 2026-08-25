use super::*;
mod ocr;

#[test]
fn test_clip_version_history() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "text",
            Some("Original Content"),
            None,
            None,
            "HashV1",
            "App",
        )
        .unwrap();

    db.update_clip_text(clip.id, "Transformed Uppercase Content")
        .unwrap();
    db.update_clip_text(clip.id, "Transformed Uppercase Content")
        .unwrap();
    db.update_clip_text(clip.id, "Final Content").unwrap();

    let versions = db.get_clip_versions(clip.id).unwrap();
    assert_eq!(versions.len(), 2);
    assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 2);
    assert_eq!(versions[0].text_content, "Transformed Uppercase Content");
    assert_eq!(versions[1].text_content, "Original Content");

    let updated = db.get_clips(None, false).unwrap();
    assert_eq!(updated[0].text_content.as_deref(), Some("Final Content"));

    for index in 0..55 {
        db.update_clip_text(clip.id, &format!("Revision {index}"))
            .unwrap();
    }
    assert_eq!(db.get_clip_versions(clip.id).unwrap().len(), 11);
    assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 11);

    db.purge_clip_permanently(clip.id).unwrap();
    assert!(db.get_clip_versions(clip.id).unwrap().is_empty());
    assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 0);
}

#[test]
fn revision_restore_rejects_versions_from_another_clip_without_mutation() {
    let db = setup_test_db();
    let first = db
        .save_clip(
            "text",
            Some("First original"),
            None,
            None,
            "revision-boundary-first",
            "Test",
        )
        .unwrap();
    let second = db
        .save_clip(
            "text",
            Some("Second original"),
            None,
            None,
            "revision-boundary-second",
            "Test",
        )
        .unwrap();
    db.update_clip_text(first.id, "First current").unwrap();
    db.update_clip_text(second.id, "Second current").unwrap();
    let foreign_version = db.get_clip_versions(second.id).unwrap().remove(0);
    let first_version_count = db.get_clip_version_count(first.id).unwrap();
    let second_version_count = db.get_clip_version_count(second.id).unwrap();

    assert!(db
        .restore_clip_version(first.id, foreign_version.id)
        .is_err());
    assert_eq!(
        db.get_clip_by_id(first.id).unwrap().text_content.as_deref(),
        Some("First current")
    );
    assert_eq!(
        db.get_clip_by_id(second.id)
            .unwrap()
            .text_content
            .as_deref(),
        Some("Second current")
    );
    assert_eq!(
        db.get_clip_version_count(first.id).unwrap(),
        first_version_count
    );
    assert_eq!(
        db.get_clip_version_count(second.id).unwrap(),
        second_version_count
    );
}

#[test]
fn revision_deletion_preserves_current_original_and_other_clips() {
    let db = setup_test_db();
    let first = db
        .save_clip(
            "text",
            Some("First original"),
            None,
            None,
            "delete-version-first",
            "Test",
        )
        .unwrap();
    let second = db
        .save_clip(
            "text",
            Some("Second original"),
            None,
            None,
            "delete-version-second",
            "Test",
        )
        .unwrap();
    db.update_clip_text(first.id, "First edit").unwrap();
    db.update_clip_text(first.id, "First current").unwrap();
    db.update_clip_text(second.id, "Second current").unwrap();

    let first_versions = db.get_clip_versions(first.id).unwrap();
    let editable = first_versions
        .iter()
        .find(|version| !version.is_original)
        .unwrap();
    let original = first_versions
        .iter()
        .find(|version| version.is_original)
        .unwrap();
    let second_version = db.get_clip_versions(second.id).unwrap().remove(0);

    db.delete_clip_version(first.id, editable.id).unwrap();
    assert_eq!(db.get_clip_version_count(first.id).unwrap(), 1);
    assert_eq!(
        db.get_clip_by_id(first.id).unwrap().text_content.as_deref(),
        Some("First current")
    );
    assert!(db.delete_clip_version(first.id, original.id).is_err());
    assert!(db.delete_clip_version(first.id, 0).is_err());
    assert!(db.delete_clip_version(first.id, second_version.id).is_err());
    assert_eq!(db.get_clip_version_count(second.id).unwrap(), 1);
    assert!(db
        .get_activity_logs(Some(20), None)
        .unwrap()
        .iter()
        .any(|entry| entry.event_type == "clip_version_deleted"));
}

#[test]
fn disabled_revision_history_preserves_existing_versions_and_skips_new_snapshots() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "text",
            Some("Original Content"),
            None,
            None,
            "revision-feature-gate",
            "App",
        )
        .unwrap();

    db.update_clip_text(clip.id, "First Edit").unwrap();
    assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 1);

    db.save_setting("enableRevisions", "false").unwrap();
    db.update_clip_text(clip.id, "Irreversible Edit").unwrap();
    assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 1);
    assert_eq!(
        db.get_clip_by_id(clip.id).unwrap().text_content.as_deref(),
        Some("Irreversible Edit")
    );

    db.save_setting("enableRevisions", "true").unwrap();
    db.update_clip_text(clip.id, "History Resumed").unwrap();
    let versions = db.get_clip_versions(clip.id).unwrap();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].text_content, "Irreversible Edit");
    assert_eq!(versions[1].text_content, "Original Content");
}

#[test]
fn revision_retention_is_configurable_and_can_be_unlimited() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "text",
            Some("Original"),
            None,
            None,
            "revision-policy",
            "App",
        )
        .unwrap();

    db.enforce_revision_retention(10).unwrap();
    for index in 0..18 {
        db.update_clip_text(clip.id, &format!("Limited {index}"))
            .unwrap();
    }
    assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 11);

    db.enforce_revision_retention(0).unwrap();
    for index in 0..60 {
        db.update_clip_text(clip.id, &format!("Unlimited {index}"))
            .unwrap();
    }
    assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 71);

    db.enforce_revision_retention(25).unwrap();
    assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 26);
    let newest = db.get_clip_versions_page(clip.id, 10, 0).unwrap();
    let middle = db.get_clip_versions_page(clip.id, 10, 10).unwrap();
    let oldest = db.get_clip_versions_page(clip.id, 10, 20).unwrap();
    assert_eq!((newest.len(), middle.len(), oldest.len()), (10, 10, 6));
    assert_ne!(newest[0].id, middle[0].id);
}

#[test]
fn test_batch_operations() {
    let db = setup_test_db();
    let clip1 = db
        .save_clip("text", Some("Batch 1"), None, None, "HashB1", "App")
        .unwrap();
    let clip2 = db
        .save_clip("text", Some("Batch 2"), None, None, "HashB2", "App")
        .unwrap();

    db.batch_pin_clips(vec![clip1.id, clip2.id], true).unwrap();
    let pinned = db.get_clips(None, true).unwrap();
    assert_eq!(pinned.len(), 2);

    db.batch_trash_clips(vec![clip1.id]).unwrap();
    let trashed = db.get_trashed_clips().unwrap();
    assert_eq!(trashed.len(), 1);
    assert_eq!(trashed[0].id, clip1.id);
}

#[test]
fn restore_all_trashed_clips_restores_every_item_and_reports_a_stable_summary() {
    let db = setup_test_db();
    let first = db
        .save_clip("text", Some("First"), None, None, "restore-all-1", "App")
        .unwrap();
    let second = db
        .save_clip("text", Some("Second"), None, None, "restore-all-2", "App")
        .unwrap();
    let active = db
        .save_clip("text", Some("Active"), None, None, "restore-all-3", "App")
        .unwrap();

    db.batch_trash_clips(vec![first.id, second.id]).unwrap();
    let restored = db.restore_all_trashed_clips().unwrap();

    assert_eq!(restored.action, "restore_all");
    assert_eq!(restored.requested_count, 2);
    assert_eq!(restored.changed_count, 2);
    assert_eq!(restored.skipped_count, 0);
    assert_eq!(restored.clip_ids, vec![first.id, second.id]);
    assert!(db.get_trashed_clips().unwrap().is_empty());
    let active_ids = db
        .get_clips(None, false)
        .unwrap()
        .into_iter()
        .map(|clip| clip.id)
        .collect::<Vec<_>>();
    assert!(active_ids.contains(&first.id));
    assert!(active_ids.contains(&second.id));
    assert!(active_ids.contains(&active.id));

    let noop = db.restore_all_trashed_clips().unwrap();
    assert_eq!(noop.requested_count, 0);
    assert_eq!(noop.changed_count, 0);
    assert_eq!(noop.skipped_count, 0);
    assert!(noop.clip_ids.is_empty());

    let logs = db.get_activity_logs(Some(20), None).unwrap();
    assert_eq!(
        logs.iter()
            .filter(|entry| entry.event_type == "clips_restored_all")
            .count(),
        1
    );
}

#[test]
fn clip_mutations_report_changes_skip_noops_and_log_user_actions() {
    let db = setup_test_db();
    let first = db
        .save_clip("text", Some("First"), None, None, "mutation-1", "App")
        .unwrap();
    let second = db
        .save_clip("text", Some("Second"), None, None, "mutation-2", "App")
        .unwrap();
    let bin = db
        .create_bin("Destination", "Folder", "#3b82f6", None)
        .unwrap();

    let pinned = db
        .batch_pin_clips(vec![first.id, second.id, first.id], true)
        .unwrap();
    assert_eq!(pinned.action, "pin");
    assert_eq!(pinned.requested_count, 3);
    assert_eq!(pinned.changed_count, 2);
    assert_eq!(pinned.skipped_count, 1);

    let pin_noop = db.batch_pin_clips(vec![first.id], true).unwrap();
    assert_eq!(pin_noop.changed_count, 0);

    let protected = db.batch_protect_clips(vec![first.id], true).unwrap();
    assert_eq!(protected.changed_count, 1);

    let assigned = db
        .batch_assign_bin_clips(vec![first.id, second.id], Some(bin.id))
        .unwrap();
    assert_eq!(assigned.changed_count, 2);

    let trashed = db.batch_trash_clips(vec![first.id, second.id]).unwrap();
    assert_eq!(trashed.changed_count, 1);
    assert_eq!(trashed.skipped_count, 1);
    assert_eq!(trashed.clip_ids, vec![second.id]);

    let logs = db.get_activity_logs(Some(20), None).unwrap();
    let event_types = logs
        .iter()
        .map(|log| log.event_type.as_str())
        .collect::<Vec<_>>();
    assert!(event_types.contains(&"clips_pinned"));
    assert!(event_types.contains(&"clip_protected_toggled"));
    assert!(event_types.contains(&"clips_bin_assigned"));
    assert!(event_types.contains(&"clip_trashed"));
    assert_eq!(
        event_types
            .iter()
            .filter(|event| **event == "clips_pinned")
            .count(),
        1
    );
}

#[test]
fn test_manual_bin_assignment_is_additive_and_individually_removable() {
    let db = setup_test_db();
    let clip1 = db
        .save_clip("text", Some("Exclusive 1"), None, None, "HashE1", "App")
        .unwrap();
    let clip2 = db
        .save_clip("text", Some("Exclusive 2"), None, None, "HashE2", "App")
        .unwrap();
    let first_bin = db
        .create_bin("First Bin", "Folder", "#3b82f6", None)
        .unwrap();
    let second_bin = db
        .create_bin("Second Bin", "Folder", "#10b981", None)
        .unwrap();
    let tag = db
        .create_bin_with_type("Important", "Tag", "#f59e0b", None, "tag")
        .unwrap();

    assert!(db.toggle_pin(clip1.id).unwrap());
    assert!(db.toggle_protected(clip1.id).unwrap());
    db.assign_to_bin(clip1.id, Some(first_bin.id)).unwrap();
    db.add_clip_to_bin(clip1.id, tag.id).unwrap();
    db.assign_to_bin(clip1.id, Some(second_bin.id)).unwrap();

    assert_eq!(db.get_clips(Some(first_bin.id), false).unwrap().len(), 1);
    let second_bin_clips = db.get_clips(Some(second_bin.id), false).unwrap();
    assert_eq!(second_bin_clips.len(), 1);
    assert_eq!(second_bin_clips[0].id, clip1.id);
    assert!(second_bin_clips[0].is_pinned);
    assert!(second_bin_clips[0].is_protected);
    assert!(second_bin_clips[0]
        .bin_ids
        .as_ref()
        .unwrap()
        .contains(&tag.id));

    db.assign_to_bin(clip1.id, None).unwrap();
    let unassigned = db.get_clips(None, false).unwrap();
    let clip1_after_unassign = unassigned.iter().find(|clip| clip.id == clip1.id).unwrap();
    assert_eq!(clip1_after_unassign.bin_id, None);
    assert!(clip1_after_unassign.is_pinned);
    assert!(clip1_after_unassign.is_protected);
    assert!(clip1_after_unassign.bin_ids.as_ref().unwrap().is_empty());

    db.batch_assign_bin_clips(vec![clip1.id, clip2.id], Some(first_bin.id))
        .unwrap();
    db.batch_assign_bin_clips(vec![clip1.id, clip2.id], Some(second_bin.id))
        .unwrap();
    assert_eq!(db.get_clips(Some(first_bin.id), false).unwrap().len(), 2);
    let batch_assigned = db.get_clips(Some(second_bin.id), false).unwrap();
    assert_eq!(batch_assigned.len(), 2);
    let protected_pinned = batch_assigned
        .iter()
        .find(|clip| clip.id == clip1.id)
        .unwrap();
    assert!(protected_pinned.is_pinned);
    assert!(protected_pinned.is_protected);

    let removed = db
        .batch_remove_bin_clips(vec![clip1.id], second_bin.id)
        .unwrap();
    assert_eq!(removed.changed_count, 1);
    let clip1_after_remove = db.get_clip_by_id(clip1.id).unwrap();
    assert!(!clip1_after_remove
        .bin_ids
        .as_ref()
        .unwrap()
        .contains(&second_bin.id));
    assert!(clip1_after_remove
        .bin_ids
        .as_ref()
        .unwrap()
        .contains(&first_bin.id));
}
