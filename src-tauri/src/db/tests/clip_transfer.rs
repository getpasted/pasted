use super::*;

#[test]
fn clip_exports_match_their_documented_json_and_csv_contracts() {
    let db = setup_test_db();
    let active = db
        .save_clip(
            "text",
            Some("=SUM(A1:A2), \"quoted\""),
            Some("<b>preserved in JSON</b>"),
            None,
            "clip-export-active",
            "Editor, Inc.",
        )
        .unwrap();
    db.toggle_pin(active.id).unwrap();
    db.update_clip_name(active.id, Some("Formula 📊")).unwrap();
    let trashed = db
        .save_clip(
            "text",
            Some("must not be exported"),
            None,
            None,
            "clip-export-trashed",
            "Tests",
        )
        .unwrap();
    db.delete_clip(trashed.id).unwrap();

    let json = db.export_clips_json().unwrap();
    let clips: Vec<ClipItem> = serde_json::from_str(&json).unwrap();
    assert_eq!(clips.len(), 1);
    assert_eq!(clips[0].content_hash, "clip-export-active");
    assert_eq!(
        clips[0].html_content.as_deref(),
        Some("<b>preserved in JSON</b>")
    );
    assert!(clips[0].is_pinned);
    assert_eq!(clips[0].name.as_deref(), Some("Formula 📊"));
    assert!(!json.contains("must not be exported"));

    let csv = db.export_clips_csv().unwrap();
    let mut lines = csv.lines();
    assert_eq!(
        lines.next(),
        Some("id,content_type,source,is_pinned,created_at,name,text_content")
    );
    let row = lines.next().unwrap();
    assert!(row.contains("\"Editor, Inc.\""));
    assert!(row.contains("\"'=SUM(A1:A2), \"\"quoted\"\"\""));
    assert!(row.contains(",true,"));
    assert!(row.contains("\"Formula 📊\""));
    assert!(lines.next().is_none());

    let json_target = setup_test_db();
    let json_preview = json_target.inspect_clips_json(&json).unwrap();
    assert_eq!(json_preview.imported_count, 1);
    assert!(json_target.get_all_clips_for_backup().unwrap().is_empty());
    let first_json_import = json_target.import_clips_json(&json).unwrap();
    assert_eq!(first_json_import.scanned_count, 1);
    assert_eq!(first_json_import.imported_count, 1);
    assert_eq!(first_json_import.duplicate_count, 0);
    let second_json_import = json_target.import_clips_json(&json).unwrap();
    assert_eq!(second_json_import.imported_count, 0);
    assert_eq!(second_json_import.duplicate_count, 1);
    let imported_json_clip = json_target.get_all_clips_for_backup().unwrap().remove(0);
    assert_eq!(
        imported_json_clip.html_content.as_deref(),
        Some("<b>preserved in JSON</b>")
    );
    assert!(imported_json_clip.is_pinned);
    assert_eq!(imported_json_clip.name.as_deref(), Some("Formula 📊"));

    let csv_target = setup_test_db();
    let csv_preview = csv_target.inspect_clips_csv(&csv).unwrap();
    assert_eq!(csv_preview.imported_count, 1);
    assert!(csv_target.get_clips(None, false).unwrap().is_empty());
    let first_csv_import = csv_target.import_clips_csv(&csv).unwrap();
    assert_eq!(first_csv_import.imported_count, 1);
    assert_eq!(first_csv_import.duplicate_count, 0);
    let second_csv_import = csv_target.import_clips_csv(&csv).unwrap();
    assert_eq!(second_csv_import.imported_count, 0);
    assert_eq!(second_csv_import.duplicate_count, 1);
    let imported_csv_clip = csv_target.get_clips(None, false).unwrap().remove(0);
    assert_eq!(
        imported_csv_clip.text_content.as_deref(),
        Some("=SUM(A1:A2), \"quoted\"")
    );
    assert_eq!(imported_csv_clip.source, "Editor, Inc.");
    assert_eq!(imported_csv_clip.name.as_deref(), Some("Formula 📊"));

    let invalid_target = setup_test_db();
    let invalid_csv = format!("{csv}\n\"broken\",\"row\"");
    assert!(invalid_target.import_clips_csv(&invalid_csv).is_err());
    assert!(invalid_target.get_clips(None, false).unwrap().is_empty());
}

#[test]
fn clip_json_import_round_trips_stored_images() {
    let source = setup_test_db();
    source
        .save_clip(
            "image",
            Some("recognized text"),
            None,
            Some(crate::resource_limits::TEST_PNG_DATA_URL),
            "clip-image-export-hash",
            "Screenshot",
        )
        .unwrap();
    let json = source.export_clips_json().unwrap();

    let target = setup_test_db();
    let report = target.import_clips_json(&json).unwrap();
    assert_eq!(report.imported_count, 1);
    let imported = target.get_all_clips_for_backup().unwrap().remove(0);
    assert_eq!(imported.content_type, "image");
    assert_eq!(imported.text_content.as_deref(), Some("recognized text"));
    assert_eq!(
        imported.image_base64.as_deref(),
        Some(crate::resource_limits::TEST_PNG_DATA_URL)
    );
    assert_eq!(imported.content_hash, "clip-image-export-hash");
}

#[test]
fn raster_image_boundaries_reject_active_content_without_mutation() {
    let malicious = "data:image/png;base64,PHN2ZyBvbmxvYWQ9ImFsZXJ0KDEpIj48L3N2Zz4=";
    let direct = setup_test_db();
    assert!(direct
        .save_clip(
            "image",
            None,
            None,
            Some(malicious),
            "malicious-direct-image",
            "Tests",
        )
        .is_err());
    assert!(direct.get_all_clips_for_backup().unwrap().is_empty());

    let source = setup_test_db();
    source
        .save_clip(
            "image",
            None,
            None,
            Some(crate::resource_limits::TEST_PNG_DATA_URL),
            "malicious-import-image",
            "Tests",
        )
        .unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(&source.export_clips_json().unwrap()).unwrap();
    payload[0]["image_base64"] = malicious.into();
    let payload = serde_json::to_string(&payload).unwrap();
    let target = setup_test_db();
    assert!(target.inspect_clips_json(&payload).is_err());
    assert!(target.import_clips_json(&payload).is_err());
    assert!(target.get_all_clips_for_backup().unwrap().is_empty());

    let legacy = source.get_all_clips_for_backup().unwrap().remove(0);
    source
        .conn
        .lock()
        .execute(
            "UPDATE clips SET image_base64 = ?1 WHERE id = ?2",
            params![malicious, legacy.id],
        )
        .unwrap();
    assert_eq!(source.get_clip_image(legacy.id).unwrap(), None);
    assert_eq!(
        source
            .get_all_clips_for_backup()
            .unwrap()
            .remove(0)
            .image_base64
            .as_deref(),
        Some(malicious)
    );
}
