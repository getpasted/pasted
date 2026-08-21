use super::*;
#[test]
fn relocating_database_preserves_data_and_retains_the_source() {
    let db = setup_test_db();
    let source = db.database_path();
    let destination_directory = std::env::temp_dir().join(format!(
        "pasted_relocation_{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&destination_directory).unwrap();
    let destination = destination_directory.join("pasted.db");
    db.save_clip(
        "text",
        Some("Move me without losing me"),
        None,
        None,
        "relocation-test-hash",
        "Test",
    )
    .unwrap();

    let retained = db.relocate_database(destination.clone()).unwrap();

    assert_eq!(retained, source);
    assert_eq!(db.database_path(), destination);
    assert!(retained.is_file());
    assert_eq!(
        db.get_clips(None, false).unwrap()[0]
            .text_content
            .as_deref(),
        Some("Move me without losing me")
    );
    let reopened = DbState::new(db.database_path()).unwrap();
    assert_eq!(reopened.get_clips(None, false).unwrap().len(), 1);
    let _ = fs::remove_file(retained);
    let _ = fs::remove_dir_all(destination_directory);
}

#[test]
fn relocating_database_never_overwrites_an_existing_target() {
    let db = setup_test_db();
    let destination_directory = std::env::temp_dir().join(format!(
        "pasted_relocation_existing_{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&destination_directory).unwrap();
    let destination = destination_directory.join("pasted.db");
    fs::write(&destination, b"keep this file").unwrap();

    assert!(db.relocate_database(destination.clone()).is_err());
    assert_eq!(fs::read(&destination).unwrap(), b"keep this file");
    assert_ne!(db.database_path(), destination);
    let _ = fs::remove_file(db.database_path());
    let _ = fs::remove_dir_all(destination_directory);
}

#[test]
fn factory_reset_removes_user_state_and_restores_first_launch_defaults() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "text",
            Some("Reset me completely"),
            None,
            None,
            "factory-reset-clip",
            "Test",
        )
        .unwrap();
    db.update_clip_note(clip.id, Some("A note to remove"))
        .unwrap();
    db.create_bin_with_type("Personal", "Folder", "default", None, "category")
        .unwrap();
    db.create_content_type(&crate::content_types::ContentTypeInput {
        id: "reset_custom".into(),
        label: "Reset Custom".into(),
        icon: "FileText".into(),
        group: "custom".into(),
        conceal_clips: false,
    })
    .unwrap();
    db.save_setting("themeMode", "vampire").unwrap();
    {
        let conn = db.conn.lock();
        conn.execute(
            "INSERT INTO activity_logs (event_type, description) VALUES ('test', 'remove me')",
            [],
        )
        .unwrap();
        conn.execute(
                "INSERT INTO intelligence_connections (id, name, provider_kind) VALUES ('reset-connection', 'Reset', 'cli')",
                [],
            )
            .unwrap();
        conn.execute(
                "INSERT INTO custom_operations (id, name, executor_kind) VALUES ('reset-operation', 'Reset', 'regex')",
                [],
            )
            .unwrap();
        conn.execute(
            "INSERT INTO saved_transforms
                    (id, name, plan_json, connection_id, authoring_kind)
                 VALUES
                    ('reset-transform', 'Reset', '{\"steps\":[]}', 'reset-connection', 'intent'),
                    ('reset-manual-transform', 'Reset Manual', '{\"steps\":[]}', NULL, 'manual')",
            [],
        )
        .unwrap();
    }

    let report = db.factory_reset().unwrap();
    assert_eq!(report.clips_deleted, 1);
    assert_eq!(report.bins_deleted, 4);
    assert_eq!(report.transforms_deleted, 3);
    assert_eq!(report.connections_deleted, 1);
    assert_eq!(report.activity_entries_deleted, 3);

    assert!(db.get_clips(None, false).unwrap().is_empty());
    assert!(search_test_clips(&db, "Reset me").is_empty());
    let default_bins = db.get_bins().unwrap();
    assert_eq!(default_bins.len(), 3);
    assert_eq!(
        default_bins
            .iter()
            .map(|bin| bin.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Images", "Links and Web", "Code Snippets"]
    );
    assert_eq!(
            default_bins[0].smart_rule.as_deref(),
            Some("{\"version\":1,\"conditions\":[{\"type\":\"clip_type\",\"operator\":\"is\",\"value\":\"image\"}],\"match\":\"any\"}")
        );
    assert_eq!(
        default_bins.iter().map(|bin| bin.id).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(db.get_setting("themeMode").unwrap(), None);
    let reset_types = db.get_content_types(true).unwrap();
    assert_eq!(
        reset_types.len(),
        crate::content_types::CONTENT_TYPE_PRESETS.len()
    );
    assert!(!reset_types.iter().any(|item| item.id == "reset_custom"));
    let conn = db.conn.lock();
    for table in [
        "clip_versions",
        "activity_logs",
        "custom_operations",
        "saved_transforms",
        "intelligence_connections",
    ] {
        let count: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "{table} should be empty after reset");
    }
    drop(conn);
    let reset_registry = db.get_library_items(None, true).unwrap();
    assert!(!reset_registry.iter().any(|item| {
        item.item.stable_ref == "custom:reset-operation"
            || item.item.stable_ref == "transform:reset-manual-transform"
    }));
    assert_eq!(
        reset_registry
            .iter()
            .filter(|item| item.item.kind == "operation" && item.item.is_builtin)
            .count(),
        crate::operation_registry::BUILTIN_OPERATIONS.len()
    );

    let fresh = db
        .save_clip(
            "text",
            Some("Fresh start"),
            None,
            None,
            "factory-reset-fresh",
            "Test",
        )
        .unwrap();
    assert!(fresh.id > 0);
}

#[test]
fn factory_reset_rolls_back_everything_when_a_delete_fails() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "text",
            Some("Do not partially reset me"),
            None,
            None,
            "factory-reset-rollback-clip",
            "Test",
        )
        .unwrap();
    let bin = db
        .create_bin("Keep This Bin", "Folder", "default", None)
        .unwrap();
    db.assign_to_bin(clip.id, Some(bin.id)).unwrap();
    db.save_setting("themeMode", "flux").unwrap();
    {
        let conn = db.conn.lock();
        conn.execute(
            "INSERT INTO activity_logs (event_type, description)
                 VALUES ('test', 'survive a failed reset')",
            [],
        )
        .unwrap();
        conn.execute_batch(
            "CREATE TRIGGER reject_factory_reset_clip_delete
                 BEFORE DELETE ON clips
                 BEGIN
                    SELECT RAISE(ABORT, 'simulated reset failure');
                 END;",
        )
        .unwrap();
    }

    let error = db.factory_reset().unwrap_err();
    assert!(error.to_string().contains("simulated reset failure"));

    let preserved = db.get_clip_by_id(clip.id).unwrap();
    assert_eq!(preserved.bin_id, Some(bin.id));
    assert_eq!(
        db.get_setting("themeMode").unwrap().as_deref(),
        Some("flux")
    );
    assert!(db.get_bins().unwrap().iter().any(|item| item.id == bin.id));
    let conn = db.conn.lock();
    let activity_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM activity_logs", [], |row| row.get(0))
        .unwrap();
    assert!(activity_count > 0);
}

#[test]
fn test_clip_saving_and_retrieval() {
    let db = setup_test_db();
    let clip = db
        .save_clip("text", Some("Hello Rust"), None, None, "hash1", "Safari")
        .unwrap();
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
    let file = db
        .save_clip(
            "file",
            Some("[\"/tmp/capture.png\"]"),
            None,
            None,
            "reattribute-file-hash",
            "pasted-app",
        )
        .unwrap();

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
    let clipboard = db
        .save_clip(
            "text",
            Some("ordinary clipboard text"),
            None,
            None,
            "origin_clipboard_hash",
            "Safari",
        )
        .unwrap();

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

#[test]
fn smart_bin_text_operators_distinguish_exact_and_partial_axis_values() {
    let db = setup_test_db();
    let safari = db
        .save_clip(
            "text",
            Some("first"),
            None,
            None,
            "source-exact-hash",
            "Safari",
        )
        .unwrap();
    let preview = db
        .save_clip(
            "text",
            Some("second"),
            None,
            None,
            "source-contains-hash",
            "Safari Technology Preview",
        )
        .unwrap();
    let email = db
        .save_clip(
            "text",
            Some("person@example.com"),
            None,
            None,
            "content-type-contains-hash",
            "Mail",
        )
        .unwrap();
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
    let clip = db
        .save_clip(
            "text",
            Some("Protected Secret"),
            None,
            None,
            "prot_hash",
            "Keeper",
        )
        .unwrap();

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
