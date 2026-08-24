use super::super::*;

#[test]
fn test_unified_taxonomy_and_tags() {
    let db = setup_test_db();
    let tag = db
        .create_bin_with_type("CodeSnippet", "Tag", "#06b6d4", None, "tag")
        .unwrap();
    assert_eq!(tag.bin_type, "tag");

    let bins = db.get_bins().unwrap();
    assert!(bins.iter().any(|b| b.id == tag.id && b.bin_type == "tag"));
}

#[test]
fn test_pin_reordering() {
    let db = setup_test_db();
    let clip1 = save_plain_test_clip(&db, "text", "First Pinned", "HashP1", "App");
    let clip2 = save_plain_test_clip(&db, "text", "Second Pinned", "HashP2", "App");
    db.toggle_pin(clip1.id).unwrap();
    db.toggle_pin(clip2.id).unwrap();

    let newly_pinned_first = db.get_clips(None, true).unwrap();
    assert_eq!(newly_pinned_first[0].id, clip2.id);
    assert_eq!(newly_pinned_first[1].id, clip1.id);

    assert!(db.reorder_pinned_clips(vec![clip1.id]).is_err());
    assert!(db.reorder_pinned_clips(vec![clip1.id, clip1.id]).is_err());
    db.reorder_pinned_clips(vec![clip1.id, clip2.id]).unwrap();
    let clips = db.get_clips(None, true).unwrap();
    assert_eq!(clips[0].id, clip1.id);
    assert_eq!(clips[1].id, clip2.id);
}

#[test]
fn bin_clip_order_is_persistent_validated_and_independent_per_bin() {
    let db = setup_test_db();
    let first = save_plain_test_clip(&db, "text", "First", "bin-order-1", "App");
    let second = save_plain_test_clip(&db, "text", "Second", "bin-order-2", "App");
    let manual = db
        .create_bin("Manual Order", "Folder", "default", None)
        .unwrap();
    let smart = db
        .create_bin(
            "Smart Order",
            "Sparkles",
            "default",
            Some(r#"{"type":"clip_type","value":"text"}"#),
        )
        .unwrap();

    db.assign_to_bin(first.id, Some(manual.id)).unwrap();
    db.assign_to_bin(second.id, Some(manual.id)).unwrap();
    db.reorder_bin_clips(manual.id, vec![first.id, second.id])
        .unwrap();
    db.reorder_bin_clips(smart.id, vec![second.id, first.id])
        .unwrap();

    let manual_clips = db.get_clips(Some(manual.id), false).unwrap();
    let smart_clips = db.get_clips(Some(smart.id), false).unwrap();
    assert_eq!(
        manual_clips.iter().map(|clip| clip.id).collect::<Vec<_>>(),
        vec![first.id, second.id]
    );
    assert_eq!(
        smart_clips.iter().map(|clip| clip.id).collect::<Vec<_>>(),
        vec![second.id, first.id]
    );

    let bins = db.get_bins().unwrap();
    assert_eq!(
        bins.iter()
            .find(|bin| bin.id == manual.id)
            .unwrap()
            .clip_order,
        vec![first.id, second.id]
    );
    assert_eq!(
        bins.iter()
            .find(|bin| bin.id == smart.id)
            .unwrap()
            .clip_order,
        vec![second.id, first.id]
    );

    assert!(db.reorder_bin_clips(manual.id, vec![first.id]).is_err());
    assert!(db
        .reorder_bin_clips(manual.id, vec![first.id, first.id])
        .is_err());
    assert_eq!(
        db.get_bins()
            .unwrap()
            .iter()
            .find(|bin| bin.id == manual.id)
            .unwrap()
            .clip_order,
        vec![first.id, second.id]
    );
}
