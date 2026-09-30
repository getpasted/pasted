use super::*;
#[path = "activity_archive.rs"]
mod activity_archive;
#[path = "history_total_capacity.rs"]
mod history_total_capacity;
#[test]
fn test_retention_uses_trash_and_excludes_pinned_and_protected_clips() {
    let db = setup_test_db();
    let pinned = db
        .save_clip("text", Some("Pinned"), None, None, "ret-pin", "App")
        .unwrap();
    let protected = db
        .save_clip("text", Some("Protected"), None, None, "ret-prot", "App")
        .unwrap();
    db.toggle_pin(pinned.id).unwrap();
    db.toggle_protected(protected.id).unwrap();

    for index in 0..3 {
        db.save_clip(
            "text",
            Some(&format!("Regular {index}")),
            None,
            None,
            &format!("ret-{index}"),
            "App",
        )
        .unwrap();
    }

    db.purge_old_clips(1).unwrap();

    let active = db.get_clips(None, false).unwrap();
    assert_eq!(
        active
            .iter()
            .filter(|clip| !clip.is_pinned && !clip.is_protected)
            .count(),
        1
    );
    assert!(active.iter().any(|clip| clip.id == pinned.id));
    assert!(active.iter().any(|clip| clip.id == protected.id));
    assert_eq!(active.len(), 3);
    assert_eq!(
        db.get_setting("keepClipCount").unwrap().as_deref(),
        Some("3")
    );
    assert_eq!(db.get_trashed_clips().unwrap().len(), 2);
}

#[test]
fn test_retention_without_trash_keeps_requested_unpinned_capacity() {
    let db = setup_test_db();
    db.save_setting("enableTrash", "false").unwrap();
    let pinned = db
        .save_clip("text", Some("Pinned"), None, None, "purge-pin", "App")
        .unwrap();
    db.toggle_pin(pinned.id).unwrap();
    for index in 0..4 {
        db.save_clip(
            "text",
            Some(&format!("Regular {index}")),
            None,
            None,
            &format!("purge-{index}"),
            "App",
        )
        .unwrap();
    }

    db.purge_old_clips(2).unwrap();

    let active = db.get_clips(None, false).unwrap();
    assert_eq!(active.len(), 2);
    assert_eq!(active.iter().filter(|clip| !clip.is_pinned).count(), 1);
    assert!(active.iter().any(|clip| clip.id == pinned.id));
    assert!(db.get_trashed_clips().unwrap().is_empty());
}

#[test]
fn age_retention_uses_trash_and_preserves_pinned_and_protected_clips() {
    let db = setup_test_db();
    let old = db
        .save_clip("text", Some("Old"), None, None, "age-old", "App")
        .unwrap();
    let recent = db
        .save_clip("text", Some("Recent"), None, None, "age-new", "App")
        .unwrap();
    let pinned = db
        .save_clip("text", Some("Pinned"), None, None, "age-pin", "App")
        .unwrap();
    let protected = db
        .save_clip("text", Some("Protected"), None, None, "age-prot", "App")
        .unwrap();
    db.toggle_pin(pinned.id).unwrap();
    db.toggle_protected(protected.id).unwrap();
    {
        let conn = db.conn.lock();
        conn.execute(
            "UPDATE clips SET created_at = datetime('now', '-31 days') WHERE id IN (?1, ?2, ?3)",
            params![old.id, pinned.id, protected.id],
        )
        .unwrap();
    }

    db.configure_clip_retention(0, 30).unwrap();

    let active = db.get_clips(None, false).unwrap();
    assert!(!active.iter().any(|clip| clip.id == old.id));
    assert!(active.iter().any(|clip| clip.id == recent.id));
    assert!(active.iter().any(|clip| clip.id == pinned.id));
    assert!(active.iter().any(|clip| clip.id == protected.id));
    assert_eq!(db.get_trashed_clips().unwrap()[0].id, old.id);
}

#[test]
fn unlimited_count_and_forever_age_do_not_remove_clips() {
    let db = setup_test_db();
    let clip = db
        .save_clip("text", Some("Kept"), None, None, "unlimited", "App")
        .unwrap();
    {
        let conn = db.conn.lock();
        conn.execute(
            "UPDATE clips SET created_at = datetime('now', '-100 years') WHERE id = ?1",
            [clip.id],
        )
        .unwrap();
    }

    db.configure_clip_retention(0, 0).unwrap();

    assert_eq!(db.get_clips(None, false).unwrap().len(), 1);
    assert!(db.get_trashed_clips().unwrap().is_empty());
}

#[test]
fn history_policy_change_does_not_cascade_into_trash_purging() {
    let db = setup_test_db();
    db.save_setting("trashCapacityCount", "1").unwrap();
    for index in 0..3 {
        db.save_clip(
            "text",
            Some(&format!("Grace {index}")),
            None,
            None,
            &format!("grace-{index}"),
            "App",
        )
        .unwrap();
    }

    db.enforce_clip_retention(1, 0).unwrap();

    assert_eq!(db.get_trashed_clips().unwrap().len(), 2);
    db.enforce_trash_retention(1, 0).unwrap();
    assert_eq!(db.get_trashed_clips().unwrap().len(), 1);
}

#[test]
fn trash_age_retention_purges_old_items_but_preserves_protected_clips() {
    let db = setup_test_db();
    let old = db
        .save_clip("text", Some("Old Trash"), None, None, "trash-age", "App")
        .unwrap();
    let protected = db
        .save_clip(
            "text",
            Some("Protected Trash"),
            None,
            None,
            "trash-protected",
            "App",
        )
        .unwrap();
    let recent = db
        .save_clip(
            "text",
            Some("Recent Trash"),
            None,
            None,
            "trash-recent",
            "App",
        )
        .unwrap();
    db.batch_trash_clips(vec![old.id, protected.id, recent.id])
        .unwrap();
    {
        let conn = db.conn.lock();
        conn.execute(
            "UPDATE clips
                 SET trashed_at = datetime('now', '-31 days'),
                     is_protected = CASE WHEN id = ?2 THEN 1 ELSE 0 END
                 WHERE id IN (?1, ?2)",
            params![old.id, protected.id],
        )
        .unwrap();
    }

    db.configure_trash_retention(0, 30).unwrap();

    let trashed = db.get_trashed_clips().unwrap();
    assert!(!trashed.iter().any(|clip| clip.id == old.id));
    assert!(trashed.iter().any(|clip| clip.id == protected.id));
    assert!(trashed.iter().any(|clip| clip.id == recent.id));
}
