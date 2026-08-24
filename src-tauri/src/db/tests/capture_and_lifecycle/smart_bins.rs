use super::super::*;

#[test]
fn smart_bin_text_operators_distinguish_exact_and_partial_axis_values() {
    let db = setup_test_db();
    let safari = save_plain_test_clip(&db, "text", "first", "source-exact-hash", "Safari");
    let preview = save_plain_test_clip(
        &db,
        "text",
        "second",
        "source-contains-hash",
        "Safari Technology Preview",
    );
    let email = save_plain_test_clip(
        &db,
        "text",
        "person@example.com",
        "content-type-contains-hash",
        "Mail",
    );
    db.replace_analysis_classifications(
        email.id,
        &email.content_hash,
        &[crate::content_classification::ClassificationMatch {
            classifier_ref: "email".into(),
            classifier_name: "Email".into(),
            content_type: "email".into(),
            priority: 10,
            start_offset: 0,
            end_offset: 5,
        }],
        "original_text",
    )
    .unwrap();
    let exact_rule = serde_json::json!({
        "conditions": [{"type": "source", "operator": "is", "value": "Safari"}],
        "match": "all"
    })
    .to_string();
    let contains_rule = serde_json::json!({
        "conditions": [{"type": "source", "operator": "contains", "value": "Safari"}],
        "match": "all"
    })
    .to_string();
    let exact_bin = db
        .create_bin("Exact Source", "📂", "default", Some(&exact_rule))
        .unwrap();
    let contains_bin = db
        .create_bin("Partial Source", "📂", "default", Some(&contains_rule))
        .unwrap();
    let content_type_rule = serde_json::json!({
        "conditions": [{"type": "content_type", "operator": "contains", "value": "mail"}],
        "match": "all"
    })
    .to_string();
    let content_type_bin = db
        .create_bin(
            "Partial Content Type",
            "📂",
            "default",
            Some(&content_type_rule),
        )
        .unwrap();

    assert_eq!(
        db.get_clips(Some(exact_bin.id), false)
            .unwrap()
            .iter()
            .map(|clip| clip.id)
            .collect::<Vec<_>>(),
        vec![safari.id]
    );
    let partial_ids = db
        .get_clips(Some(contains_bin.id), false)
        .unwrap()
        .iter()
        .map(|clip| clip.id)
        .collect::<HashSet<_>>();
    assert_eq!(partial_ids, HashSet::from([safari.id, preview.id]));
    assert_eq!(
        db.get_clips(Some(content_type_bin.id), false).unwrap()[0].id,
        email.id
    );

    let clip_type_rule = serde_json::json!({
        "conditions": [{"type": "clip_type", "operator": "is", "value": "text"}],
        "match": "all"
    })
    .to_string();
    let clip_type_bin = db
        .create_bin("Text Clips", "📂", "default", Some(&clip_type_rule))
        .unwrap();
    assert_eq!(
        db.get_clips(Some(clip_type_bin.id), false).unwrap().len(),
        3
    );

    db.set_bin_transform_ref(exact_bin.id, Some("transform:source-test"))
        .unwrap();
    assert_eq!(
        db.matching_smart_bin_transforms("text", &[], &[], "", "Safari")
            .unwrap(),
        vec![(exact_bin.id, "transform:source-test".into())]
    );
    db.save_setting("enableSources", "false").unwrap();
    assert!(db.get_clips(Some(exact_bin.id), false).unwrap().is_empty());
    assert!(db
        .matching_smart_bin_transforms("text", &[], &[], "", "Safari")
        .unwrap()
        .is_empty());
    db.save_setting("enableSources", "true").unwrap();
    db.save_setting("enableTypes", "false").unwrap();
    assert!(db
        .get_clips(Some(content_type_bin.id), false)
        .unwrap()
        .is_empty());
    db.save_setting("enableTypes", "true").unwrap();
    db.save_setting("enableClipTypes", "false").unwrap();
    assert!(db
        .get_clips(Some(clip_type_bin.id), false)
        .unwrap()
        .is_empty());
}

#[test]
fn file_smart_bins_match_any_selected_path_without_reordering_the_clip() {
    let db = setup_test_db();
    let paths = serde_json::json!([
        "/Users/pasted/Zebra Report.pdf",
        "/Users/pasted/Projects/Alpha Notes.txt"
    ])
    .to_string();
    let clip = db
        .save_clip("file", Some(&paths), None, None, "file_hash", "Finder")
        .unwrap();
    let pdf_rule = serde_json::json!({
        "conditions": [{"type": "file_extension", "operator": "is", "value": "pdf"}],
        "match": "any"
    })
    .to_string();
    let project_rule = serde_json::json!({
        "conditions": [{"type": "file_path", "operator": "contains", "value": "/projects/"}],
        "match": "any"
    })
    .to_string();
    let pdf_bin = db
        .create_bin("PDF Files", "📄", "default", Some(&pdf_rule))
        .unwrap();
    let project_bin = db
        .create_bin("Project Files", "📂", "default", Some(&project_rule))
        .unwrap();

    assert_eq!(
        db.get_clips(Some(pdf_bin.id), false).unwrap()[0].id,
        clip.id
    );
    assert_eq!(
        db.get_clips(Some(project_bin.id), false).unwrap()[0].id,
        clip.id
    );
    assert_eq!(
        db.get_clip_by_id(clip.id).unwrap().text_content.as_deref(),
        Some(paths.as_str())
    );
    let bins = db.get_bins().unwrap();
    assert_eq!(
        bins.iter()
            .find(|bin| bin.id == pdf_bin.id)
            .unwrap()
            .clip_count,
        Some(1)
    );
}

#[test]
fn file_format_smart_bins_match_verified_bytes_not_filename_extensions() {
    let db = setup_test_db();
    let workspace = crate::external_tools::PrivateWorkspace::create("smart-format").unwrap();
    let path = workspace.join("actually-png.txt");
    std::fs::write(
        &path,
        [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 0],
    )
    .unwrap();
    let payload = serde_json::to_string(&vec![path.to_string_lossy().into_owned()]).unwrap();
    let clip = db
        .save_clip(
            "file",
            Some(&payload),
            None,
            None,
            "verified-format",
            "Finder",
        )
        .unwrap();
    let bin = db
            .create_bin(
                "PNG Files",
                "📄",
                "default",
                Some(r#"{"conditions":[{"type":"file_format","operator":"is","value":"png"}],"match":"any"}"#),
            )
            .unwrap();
    let partial_bin = db
            .create_bin(
                "Partial Format",
                "📄",
                "default",
                Some(r#"{"conditions":[{"type":"file_format","operator":"contains","value":"pn"}],"match":"any"}"#),
            )
            .unwrap();

    let refreshed = db.get_clip_by_id(clip.id).unwrap();
    assert_eq!(refreshed.file_formats, vec!["png"]);
    assert_eq!(db.get_clips(Some(bin.id), false).unwrap()[0].id, clip.id);
    assert_eq!(
        db.get_clips(Some(partial_bin.id), false).unwrap()[0].id,
        clip.id
    );

    db.save_setting("enableFileFormats", "false").unwrap();
    assert!(db.get_clips(Some(bin.id), false).unwrap().is_empty());
}
