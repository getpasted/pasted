use super::super::*;

#[test]
fn clip_lists_defer_image_payloads_to_the_image_endpoint() {
    let db = setup_test_db();
    let image_payload = crate::resource_limits::TEST_PNG_DATA_URL;
    let clip = db
        .save_clip(
            "image",
            None,
            None,
            Some(image_payload),
            "image_hash",
            "Screenshot",
        )
        .unwrap();

    let clips = db.get_clips(None, false).unwrap();
    assert_eq!(clips.len(), 1);
    assert_eq!(clips[0].id, clip.id);
    assert!(clips[0].image_base64.is_none());
    assert_eq!(
        db.get_clip_image(clip.id).unwrap().as_deref(),
        Some(image_payload)
    );
}

#[test]
fn test_protected_clips_immunity() {
    let db = setup_test_db();
    let clip = save_plain_test_clip(&db, "text", "Protected Secret", "prot_hash", "Keeper");

    // Toggle protected
    let is_prot = db.toggle_protected(clip.id).unwrap();
    assert!(is_prot);

    // Attempt delete_clip (should be blocked)
    db.delete_clip(clip.id).unwrap();
    let active = db.get_clips(None, false).unwrap();
    assert_eq!(active.len(), 1);
    assert!(active[0].is_protected);

    // Attempt trash_unpinned_clips (should be blocked)
    db.trash_unpinned_clips().unwrap();
    let active_after_trash = db.get_clips(None, false).unwrap();
    assert_eq!(active_after_trash.len(), 1);

    // Attempt purge_unpinned_clips (should be blocked)
    db.purge_unpinned_clips().unwrap();
    let active_after_purge = db.get_clips(None, false).unwrap();
    assert_eq!(active_after_purge.len(), 1);

    // Every bulk and retention path must preserve protected clips.
    db.clear_history().unwrap();
    db.purge_old_clips(0).unwrap();
    let active_after_clear = db.get_clips(None, false).unwrap();
    assert_eq!(active_after_clear.len(), 1);
    assert!(active_after_clear[0].is_protected);

    // Unprotect and verify delete works
    db.toggle_protected(clip.id).unwrap();
    db.delete_clip(clip.id).unwrap();
    assert_eq!(db.get_clips(None, false).unwrap().len(), 0);
}
