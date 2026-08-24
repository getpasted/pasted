use super::super::*;

#[test]
fn test_clip_saving_and_retrieval() {
    let db = setup_test_db();
    let clip = save_plain_test_clip(&db, "text", "Hello Rust", "hash1", "Safari");
    assert!(clip.id > 0);

    let clips = db.get_clips(None, false).unwrap();
    assert_eq!(clips.len(), 1);
    assert_eq!(clips[0].text_content.as_deref(), Some("Hello Rust"));
    assert_eq!(clips[0].source, "Safari");
    assert!(!clips[0].is_pinned);
}

#[test]
fn origin_kind_is_conservative_and_distinguishes_files_and_screenshots() {
    assert_eq!(derived_origin_kind("file", "Finder"), "file_reference");
    assert_eq!(derived_origin_kind("image", "Screenshot"), "screenshot");
    assert_eq!(derived_origin_kind("image", "screencapture"), "screenshot");
    assert_eq!(derived_origin_kind("image", "CleanShot X"), "screenshot");
    assert_eq!(derived_origin_kind("file", "CleanShot X"), "screenshot");
    assert_eq!(derived_origin_kind("image", "Preview"), "clipboard_content");
    assert_eq!(derived_origin_kind("text", "Safari"), "clipboard_content");
    assert_eq!(derived_origin_kind("text", "CLI Terminal"), "command_line");
}

#[test]
fn structural_smart_bin_rules_migrate_to_clip_types() {
    let db = setup_test_db();
    let legacy_rule = serde_json::json!({
        "conditions": [{"type": "content_type", "operator": "is", "value": "image"}],
        "match": "all"
    })
    .to_string();
    let bin = db
        .create_bin("Images", "🖼️", "default", Some(&legacy_rule))
        .unwrap();
    {
        let conn = db.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO content_types
                    (id, label, icon, group_name, is_builtin, is_archived)
                 VALUES ('image', 'Image', 'Image', 'custom', 0, 0)",
            [],
        )
        .unwrap();
        retire_structural_content_type_entries(&conn).unwrap();
    }

    assert!(db
        .get_content_types(true)
        .unwrap()
        .iter()
        .all(|content_type| content_type.id != "image"));
    let migrated = db.get_bin(bin.id).unwrap().smart_rule.unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&migrated).unwrap()["conditions"][0]["type"],
        "clip_type"
    );

    let clip = db
        .save_clip(
            "image",
            None,
            None,
            Some(crate::resource_limits::TEST_PNG_DATA_URL),
            "clip-type-smart-bin-hash",
            "Preview",
        )
        .unwrap();
    assert_eq!(db.get_clips(Some(bin.id), false).unwrap()[0].id, clip.id);
}

#[test]
fn image_capture_reattribution_is_hash_safe_and_image_only() {
    let db = setup_test_db();
    let image = db
        .save_clip(
            "image",
            None,
            None,
            Some(crate::resource_limits::TEST_PNG_DATA_URL),
            "reattribute-image-hash",
            "Safari",
        )
        .unwrap();
    let file = save_plain_test_clip(
        &db,
        "file",
        "[\"/tmp/capture.png\"]",
        "reattribute-file-hash",
        "pasted-app",
    );

    assert!(db
        .reattribute_image_capture(image.id, "wrong-hash", "Screenshot")
        .unwrap()
        .is_none());
    assert_eq!(db.get_clip_by_id(image.id).unwrap().source, "Safari");

    let updated = db
        .reattribute_image_capture(image.id, &image.content_hash, "Screenshot")
        .unwrap()
        .unwrap();
    assert_eq!(updated.source, "Screenshot");

    assert!(db
        .reattribute_image_capture(file.id, &file.content_hash, "Screenshot")
        .unwrap()
        .is_none());
    assert_eq!(db.get_clip_by_id(file.id).unwrap().source, "pasted-app");
}

#[test]
fn origin_smart_bins_match_lists_counts_and_transform_automation() {
    let db = setup_test_db();
    let screenshot = db
        .save_clip(
            "image",
            None,
            None,
            Some(crate::resource_limits::TEST_PNG_DATA_URL),
            "origin_screenshot_hash",
            "Screenshot",
        )
        .unwrap();
    let paths = serde_json::json!(["/Users/pasted/Downloads/report.pdf"]).to_string();
    let file = db
        .save_clip(
            "file",
            Some(&paths),
            None,
            None,
            "origin_file_hash",
            "Finder",
        )
        .unwrap();
    let cleanshot_paths =
        serde_json::json!(["/Users/pasted/Desktop/CleanShot 2026-08-07.png"]).to_string();
    let cleanshot_file = db
        .save_clip(
            "file",
            Some(&cleanshot_paths),
            None,
            None,
            "origin_cleanshot_file_hash",
            "CleanShot X",
        )
        .unwrap();
    let clipboard = save_plain_test_clip(
        &db,
        "text",
        "ordinary clipboard text",
        "origin_clipboard_hash",
        "Safari",
    );

    let screenshot_rule = serde_json::json!({
        "conditions": [{"type": "origin_kind", "operator": "is", "value": "screenshot"}],
        "match": "all"
    })
    .to_string();
    let file_rule = serde_json::json!({
        "conditions": [{"type": "origin_kind", "operator": "is", "value": "file_reference"}],
        "match": "all"
    })
    .to_string();
    let clipboard_rule = serde_json::json!({
        "conditions": [{"type": "origin_kind", "operator": "is", "value": "clipboard_content"}],
        "match": "all"
    })
    .to_string();
    let screenshot_bin = db
        .create_bin("Screenshots", "📸", "default", Some(&screenshot_rule))
        .unwrap();
    let file_bin = db
        .create_bin("File References", "📎", "default", Some(&file_rule))
        .unwrap();
    let clipboard_bin = db
        .create_bin("Clipboard Content", "📋", "default", Some(&clipboard_rule))
        .unwrap();

    let screenshot_clips = db.get_clips(Some(screenshot_bin.id), false).unwrap();
    assert_eq!(screenshot_clips.len(), 2);
    assert!(screenshot_clips.iter().any(|clip| clip.id == screenshot.id));
    assert!(screenshot_clips
        .iter()
        .any(|clip| clip.id == cleanshot_file.id));
    assert!(db
        .get_clip_by_id(screenshot.id)
        .unwrap()
        .bin_ids
        .unwrap()
        .contains(&screenshot_bin.id));
    assert!(db
        .assign_to_bin(screenshot.id, Some(screenshot_bin.id))
        .is_err());
    assert_eq!(
        db.get_clips(Some(file_bin.id), false).unwrap()[0].id,
        file.id
    );
    assert_eq!(
        db.get_clips(Some(clipboard_bin.id), false).unwrap()[0].id,
        clipboard.id
    );
    let bins = db.get_bins().unwrap();
    assert_eq!(
        bins.iter()
            .find(|bin| bin.id == screenshot_bin.id)
            .unwrap()
            .clip_count,
        Some(2)
    );
    for bin_id in [file_bin.id, clipboard_bin.id] {
        assert_eq!(
            bins.iter().find(|bin| bin.id == bin_id).unwrap().clip_count,
            Some(1)
        );
    }

    db.set_bin_transform_ref(screenshot_bin.id, Some("transform:test-origin"))
        .unwrap();
    assert_eq!(
        db.matching_smart_bin_transforms("image", &[], &[], "", "Screenshot")
            .unwrap(),
        vec![(screenshot_bin.id, "transform:test-origin".to_string())]
    );
    assert_eq!(
        db.matching_smart_bin_transforms("file", &[], &[], &cleanshot_paths, "CleanShot X")
            .unwrap(),
        vec![(screenshot_bin.id, "transform:test-origin".to_string())]
    );
    assert!(db
        .matching_smart_bin_transforms("image", &[], &[], "", "Preview")
        .unwrap()
        .is_empty());
}
