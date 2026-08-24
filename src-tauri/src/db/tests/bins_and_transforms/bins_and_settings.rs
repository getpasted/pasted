use super::super::*;

#[test]
fn test_clip_pinning_and_notes() {
    let db = setup_test_db();
    let clip = save_plain_test_clip(&db, "text", "Pasted Pin Test", "hash2", "VSCode");

    // Pin clip
    let is_pinned = db.toggle_pin(clip.id).unwrap();
    assert!(is_pinned);

    // Add note
    db.update_clip_note(clip.id, Some("Important note"))
        .unwrap();

    let clips = db.get_clips(None, false).unwrap();
    assert!(clips[0].is_pinned);
    assert_eq!(clips[0].note.as_deref(), Some("Important note"));
}

#[test]
fn test_bins_crud() {
    let db = setup_test_db();
    let initial_count = db.get_bins().unwrap().len();

    let bin = db.create_bin("Work", "💼", "#3b82f6", None).unwrap();
    assert!(bin.id > 0);
    db.update_bin_hotkey(bin.id, Some("Alt+W")).unwrap();
    assert_eq!(
        db.get_bin_hotkeys().unwrap(),
        vec![(bin.id, "Work".into(), "Alt+W".into())]
    );

    let bins = db.get_bins().unwrap();
    assert_eq!(bins.len(), initial_count + 1);

    db.delete_bin(bin.id, "keep", None).unwrap();
    let bins_after = db.get_bins().unwrap();
    assert_eq!(bins_after.len(), initial_count);
}

#[test]
fn deleting_a_bin_can_keep_move_or_trash_its_clips() {
    let db = setup_test_db();

    let keep_bin = db.create_bin("Keep", "📁", "default", None).unwrap();
    let kept = save_plain_test_clip(&db, "text", "kept", "keep_hash", "App");
    db.assign_to_bin(kept.id, Some(keep_bin.id)).unwrap();
    db.delete_bin(keep_bin.id, "keep", None).unwrap();
    assert_eq!(db.get_clip_by_id(kept.id).unwrap().bin_id, None);

    let source_bin = db.create_bin("Source", "📁", "default", None).unwrap();
    let destination_bin = db.create_bin("Destination", "📁", "default", None).unwrap();
    let moved = save_plain_test_clip(&db, "text", "moved", "move_hash", "App");
    db.assign_to_bin(moved.id, Some(source_bin.id)).unwrap();
    db.delete_bin(source_bin.id, "move", Some(destination_bin.id))
        .unwrap();
    assert_eq!(
        db.get_clip_by_id(moved.id).unwrap().bin_id,
        Some(destination_bin.id)
    );

    let trash_bin = db.create_bin("Trash", "📁", "default", None).unwrap();
    let trashed = save_plain_test_clip(&db, "text", "trashed", "trash_hash", "App");
    let protected = save_plain_test_clip(&db, "text", "protected", "protected_hash", "App");
    db.assign_to_bin(trashed.id, Some(trash_bin.id)).unwrap();
    db.assign_to_bin(protected.id, Some(trash_bin.id)).unwrap();
    db.toggle_protected(protected.id).unwrap();
    db.delete_bin(trash_bin.id, "trash", None).unwrap();

    assert!(db
        .get_trashed_clips()
        .unwrap()
        .iter()
        .any(|clip| clip.id == trashed.id));
    let protected_after = db.get_clip_by_id(protected.id).unwrap();
    assert!(protected_after.is_protected);
    assert!(!protected_after.is_trashed);
    assert_eq!(protected_after.bin_id, None);
}

#[test]
fn deleting_a_bin_rejects_invalid_move_destinations_atomically() {
    let db = setup_test_db();
    let source_bin = db.create_bin("Source", "📁", "default", None).unwrap();
    let clip = save_plain_test_clip(&db, "text", "clip", "clip_hash", "App");
    db.assign_to_bin(clip.id, Some(source_bin.id)).unwrap();

    assert!(db.delete_bin(source_bin.id, "move", None).is_err());
    assert!(db
        .get_bins()
        .unwrap()
        .iter()
        .any(|bin| bin.id == source_bin.id));
    assert_eq!(
        db.get_clip_by_id(clip.id).unwrap().bin_id,
        Some(source_bin.id)
    );
}

#[test]
fn test_settings_storage() {
    let db = setup_test_db();
    db.save_setting("hudHotkey", "CmdOrCtrl+Shift+V").unwrap();
    let val = db.get_setting("hudHotkey").unwrap();
    assert_eq!(val.as_deref(), Some("CmdOrCtrl+Shift+V"));
}
