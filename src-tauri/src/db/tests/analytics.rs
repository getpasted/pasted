use super::*;

#[test]
fn insights_separates_clip_types_file_formats_and_content_types() {
    let db = setup_test_db();
    let workspace = crate::external_tools::PrivateWorkspace::create("insights-formats").unwrap();
    let png_path = workspace.join("misleading.txt");
    std::fs::write(
        &png_path,
        [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 0],
    )
    .unwrap();
    db.save_clip("text", Some("Plain text"), None, None, "plain", "Tests")
        .unwrap();
    db.save_clip(
        "email",
        Some("name@example.com"),
        None,
        None,
        "email",
        "Tests",
    )
    .unwrap();
    db.save_clip("image", None, None, None, "image", "Tests")
        .unwrap();
    let file_payload =
        serde_json::to_string(&vec![png_path.to_string_lossy().into_owned()]).unwrap();
    db.save_clip("file", Some(&file_payload), None, None, "files", "Tests")
        .unwrap();

    let summary = db.get_analytics_summary().unwrap();
    let clip_type_count = |name: &str| {
        summary
            .clip_types
            .iter()
            .find(|entry| entry.clip_type == name)
            .map(|entry| entry.count)
            .unwrap_or_default()
    };
    let format_count = |name: &str| {
        summary
            .file_formats
            .iter()
            .find(|entry| entry.file_format == name)
            .map(|entry| entry.count)
            .unwrap_or_default()
    };
    assert_eq!(clip_type_count("text"), 2);
    assert_eq!(clip_type_count("image"), 1);
    assert_eq!(clip_type_count("file"), 1);
    assert_eq!(format_count("png"), 1);
    assert_eq!(summary.content_types[0].content_type, "email");
    assert_eq!(summary.content_types[0].count, 1);
    let serialized = serde_json::to_value(&summary).unwrap();
    assert_eq!(serialized["clip_types"][0]["clip_type"], "text");
    assert!(serialized["clip_types"][0].get("content_type").is_none());
}

#[test]
fn insights_bounds_file_format_breakdowns() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "file",
            Some(r#"["/tmp/file.bin"]"#),
            None,
            None,
            "many-formats",
            "Tests",
        )
        .unwrap();
    let inspection = crate::content_inspection::FileFormatInspection {
        formats: (0..MAX_ANALYTICS_FILE_FORMATS + 5)
            .map(|index| crate::content_inspection::DetectedFileFormat {
                format: format!("format-{index:02}"),
                mime_type: format!("application/x-format-{index:02}"),
                count: 1,
            })
            .collect(),
        inspected_count: 1,
        unknown_count: 0,
        unavailable_count: 0,
        ..Default::default()
    };
    db.record_file_format_inspection(clip.id, &clip.content_hash, &inspection)
        .unwrap();

    let summary = db.get_analytics_summary().unwrap();
    assert_eq!(summary.file_formats.len(), MAX_ANALYTICS_FILE_FORMATS);
}
