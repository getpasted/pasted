use super::*;

#[test]
fn history_count_includes_pinned_and_protected_clips() {
    let db = setup_test_db();
    let pinned = db
        .save_clip("text", Some("Pinned"), None, None, "total-pin", "App")
        .unwrap();
    let protected = db
        .save_clip("text", Some("Protected"), None, None, "total-prot", "App")
        .unwrap();
    db.toggle_pin(pinned.id).unwrap();
    db.toggle_protected(protected.id).unwrap();
    for index in 0..3 {
        db.save_clip(
            "text",
            Some(&format!("Ordinary {index}")),
            None,
            None,
            &format!("total-{index}"),
            "App",
        )
        .unwrap();
    }

    db.configure_clip_retention(4, 0).unwrap();

    assert_eq!(db.get_total_clip_count().unwrap(), 4);
    assert_eq!(db.get_trashed_clip_count().unwrap(), 1);
    assert_eq!(
        db.get_setting("keepClipCount").unwrap().as_deref(),
        Some("4")
    );
    let active = db.get_clips(None, false).unwrap();
    assert!(active.iter().any(|clip| clip.id == pinned.id));
    assert!(active.iter().any(|clip| clip.id == protected.id));
}

#[test]
fn full_protected_history_expands_for_new_capture() {
    let db = setup_test_db();
    db.configure_clip_retention(1, 0).unwrap();
    let protected = db
        .save_clip(
            "text",
            Some("Protected"),
            None,
            None,
            "full-protected",
            "App",
        )
        .unwrap();
    db.toggle_protected(protected.id).unwrap();

    let newest = db
        .save_clip("text", Some("New"), None, None, "full-new", "App")
        .unwrap();

    assert_eq!(db.get_total_clip_count().unwrap(), 2);
    assert!(db.get_clip_by_id(newest.id).is_ok());
    assert_eq!(
        db.get_setting("keepClipCount").unwrap().as_deref(),
        Some("2")
    );
    assert_eq!(
        db.get_setting("historyLimitAutoExpanded")
            .unwrap()
            .as_deref(),
        Some("2")
    );
    let expanded = db
        .get_activity_logs(None, None)
        .unwrap()
        .into_iter()
        .find(|entry| entry.event_type == "setting_history_limit_expanded")
        .unwrap();
    assert_eq!(expanded.attributes["previousLimit"], 1);
    assert_eq!(expanded.attributes["newLimit"], 2);
}

#[test]
fn pinned_and_protected_clip_uses_only_one_slot() {
    let db = setup_test_db();
    let special = db
        .save_clip(
            "text",
            Some("Special"),
            None,
            None,
            "overlap-special",
            "App",
        )
        .unwrap();
    db.toggle_pin(special.id).unwrap();
    db.toggle_protected(special.id).unwrap();
    db.save_clip("text", Some("Old"), None, None, "overlap-old", "App")
        .unwrap();
    db.save_clip("text", Some("New"), None, None, "overlap-new", "App")
        .unwrap();

    db.configure_clip_retention(2, 0).unwrap();

    assert_eq!(db.get_total_clip_count().unwrap(), 2);
    assert_eq!(db.get_trashed_clip_count().unwrap(), 1);
    assert_eq!(db.get_setting("historyLimitAutoExpanded").unwrap(), None);
}

#[test]
fn changing_limit_clears_previous_auto_expansion_notice() {
    let db = setup_test_db();
    db.configure_clip_retention(1, 0).unwrap();
    let special = db
        .save_clip("text", Some("Special"), None, None, "notice-special", "App")
        .unwrap();
    db.toggle_protected(special.id).unwrap();
    db.save_clip("text", Some("New"), None, None, "notice-new", "App")
        .unwrap();
    assert_eq!(
        db.get_setting("historyLimitAutoExpanded")
            .unwrap()
            .as_deref(),
        Some("2")
    );

    db.configure_clip_retention(3, 0).unwrap();

    assert_eq!(db.get_setting("historyLimitAutoExpanded").unwrap(), None);
    assert_eq!(
        db.get_setting("keepClipCount").unwrap().as_deref(),
        Some("3")
    );
}

#[test]
fn recapturing_a_trashed_clip_obeys_total_history_limit() {
    let db = setup_test_db();
    db.configure_clip_retention(2, 0).unwrap();
    let old = db
        .save_clip("text", Some("Old"), None, None, "recapture-old", "App")
        .unwrap();
    let middle = db
        .save_clip(
            "text",
            Some("Middle"),
            None,
            None,
            "recapture-middle",
            "App",
        )
        .unwrap();
    db.save_clip("text", Some("New"), None, None, "recapture-new", "App")
        .unwrap();
    assert!(db
        .get_trashed_clips()
        .unwrap()
        .iter()
        .any(|clip| clip.id == old.id));

    db.save_clip("text", Some("Old"), None, None, "recapture-old", "App")
        .unwrap();

    assert_eq!(db.get_total_clip_count().unwrap(), 2);
    assert!(db
        .get_trashed_clips()
        .unwrap()
        .iter()
        .any(|clip| clip.id == middle.id));
    assert!(db
        .get_clips(None, false)
        .unwrap()
        .iter()
        .any(|clip| clip.id == old.id));
}
