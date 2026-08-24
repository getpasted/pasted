use super::super::*;

#[test]
fn test_clip_search_and_deletion() {
    let db = setup_test_db();
    let clip1 = save_plain_test_clip(&db, "text", "Unique Search Secret", "h1", "Terminal");
    let _clip2 = save_plain_test_clip(&db, "text", "Unrelated text", "h2", "Finder");
    let classified = db.save_text_clip("person@example.com", "Mail").unwrap();

    // Search by query
    let search_results = search_test_clips(&db, "Secret");
    assert_eq!(search_results.len(), 1);
    assert_eq!(
        search_results[0].text_content.as_deref(),
        Some("Unique Search Secret")
    );
    let type_results = search_test_clips(&db, "email");
    assert_eq!(type_results.len(), 1);
    assert_eq!(type_results[0].id, classified.id);
    assert_eq!(type_results[0].content_types, vec!["email"]);

    // Test distinct apps
    let apps = db.get_distinct_sources().unwrap();
    assert!(apps.contains(&"Terminal".to_string()));
    assert!(apps.contains(&"Finder".to_string()));

    // Delete single clip (moves to trash)
    db.delete_clip(clip1.id).unwrap();
    let after_delete = db.get_clips(None, false).unwrap();
    assert_eq!(after_delete.len(), 2);

    // Verify clip is in Trash
    let trashed = db.get_trashed_clips().unwrap();
    assert_eq!(trashed.len(), 1);
    assert_eq!(trashed[0].id, clip1.id);
    assert_eq!(db.get_total_clip_count().unwrap(), 2);

    // Restore clip
    db.restore_clip(clip1.id).unwrap();
    let after_restore = db.get_clips(None, false).unwrap();
    assert_eq!(after_restore.len(), 3);
}

#[test]
fn untrusted_clip_and_metadata_text_cannot_become_sql() {
    let db = setup_test_db();
    let hostile = "'); DROP TABLE clips; DELETE FROM bins; -- \" * OR 1=1";
    let hostile_transform = "AI output: '; UPDATE clips SET is_protected = 0; --";
    let hostile_rule = serde_json::json!({
        "type": "contains",
        "value": hostile,
    })
    .to_string();

    let clip = db
        .save_clip("text", Some(hostile), None, None, "hostile-hash", hostile)
        .unwrap();
    db.update_clip_text(clip.id, hostile_transform).unwrap();
    db.update_clip_note(clip.id, Some(hostile)).unwrap();
    let bin = db
        .create_bin(hostile, hostile, hostile, Some(&hostile_rule))
        .unwrap();

    // Search input is also untrusted. It may use FTS syntax internally, but it must
    // remain a bound value and must never alter the surrounding SQL statement.
    let _ = search_test_clips(&db, hostile);

    let conn = db.conn.lock();
    let clip_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM clips", [], |row| row.get(0))
        .unwrap();
    let stored: (String, String, String) = conn
        .query_row(
            "SELECT text_content, source, note FROM clips WHERE id = ?1",
            params![clip.id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    let stored_bin_name: String = conn
        .query_row(
            "SELECT name FROM bins WHERE id = ?1",
            params![bin.id],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(clip_count, 1);
    assert_eq!(
        stored,
        (hostile_transform.into(), hostile.into(), hostile.into())
    );
    assert_eq!(stored_bin_name, hostile);
}

#[test]
fn oversized_note_updates_are_rejected_without_mutating_the_clip() {
    let db = setup_test_db();
    let clip = save_plain_test_clip(&db, "text", "original", "bounded", "Tests");
    db.update_clip_note(clip.id, Some("original note")).unwrap();
    let oversized = "x".repeat(crate::resource_limits::MAX_CLIP_NOTE_BYTES + 1);

    assert!(db.update_clip_note(clip.id, Some(&oversized)).is_err());
    let stored = db
        .get_clips(None, false)
        .unwrap()
        .into_iter()
        .find(|item| item.id == clip.id)
        .unwrap();
    assert_eq!(stored.note.as_deref(), Some("original note"));
}

#[test]
fn clip_names_are_bounded_searchable_counted_and_feature_gated() {
    let db = setup_test_db();
    let clip = save_plain_test_clip(&db, "text", "ordinary body", "named-clip", "Tests");

    let named = db
        .update_clip_name(clip.id, Some("  📌 Deploy token  "))
        .unwrap();
    assert_eq!(named.name.as_deref(), Some("📌 Deploy token"));
    assert_eq!(db.get_clip_collection_summary().unwrap().named_count, 1);
    assert_eq!(search_test_clips(&db, "deploy")[0].id, clip.id);
    assert_eq!(search_test_clips(&db, "is:named")[0].id, clip.id);
    assert_eq!(search_test_clips(&db, "has:name")[0].id, clip.id);

    db.save_setting("enableNaming", "false").unwrap();
    assert!(search_test_clips(&db, "deploy").is_empty());
    assert!(search_test_clips(&db, "is:named").is_empty());
    assert_eq!(
        db.get_clip_by_id(clip.id).unwrap().name.as_deref(),
        Some("📌 Deploy token")
    );

    db.save_setting("enableNaming", "true").unwrap();
    let oversized = "x".repeat(clip_names::MAX_CLIP_NAME_CHARS + 1);
    assert!(db.update_clip_name(clip.id, Some(&oversized)).is_err());
    assert!(db.update_clip_name(clip.id, Some("line\nbreak")).is_err());
    assert_eq!(
        db.get_clip_by_id(clip.id).unwrap().name.as_deref(),
        Some("📌 Deploy token")
    );

    db.update_clip_name(clip.id, Some("   ")).unwrap();
    assert_eq!(db.get_clip_collection_summary().unwrap().named_count, 0);
    assert!(db.get_clip_by_id(clip.id).unwrap().name.is_none());

    db.delete_clip(clip.id).unwrap();
    assert!(db.update_clip_name(clip.id, Some("Nope")).is_err());
}

#[test]
fn test_trash_and_activity_logging() {
    let db = setup_test_db();
    let clip = save_plain_test_clip(&db, "text", "Trash Me", "thash1", "Notes");

    // Trash clip
    db.delete_clip(clip.id).unwrap();
    let trashed = db.get_trashed_clips().unwrap();
    assert_eq!(trashed.len(), 1);

    // Empty trash
    db.empty_trash().unwrap();
    assert_eq!(db.get_trashed_clips().unwrap().len(), 0);

    // Check activity logs
    let logs = db.get_activity_logs(None, None).unwrap();
    assert!(logs.len() >= 2); // clip_trashed, trash_emptied
    assert_eq!(logs[0].event_type, "trash_emptied");

    // Clear logs
    db.clear_activity_logs().unwrap();
    assert_eq!(db.get_activity_logs(None, None).unwrap().len(), 0);
}

#[test]
fn test_trashed_clips_are_read_only_and_leave_category_bins() {
    let db = setup_test_db();
    let category = db
        .create_bin("Projects", "Folder", "#3b82f6", None)
        .unwrap();
    let tag = db
        .create_bin_with_type("Keep", "Tag", "#f59e0b", None, "tag")
        .unwrap();
    let clip = save_plain_test_clip(
        &db,
        "text",
        "Original searchable text",
        "trash-policy-hash",
        "Tests",
    );

    db.update_clip_note(clip.id, Some("Original searchable note"))
        .unwrap();
    db.assign_to_bin(clip.id, Some(category.id)).unwrap();
    db.add_clip_to_bin(clip.id, tag.id).unwrap();
    db.delete_clip(clip.id).unwrap();

    let trashed = db.get_trashed_clips().unwrap();
    assert_eq!(trashed.len(), 1);
    assert_eq!(trashed[0].bin_id, None);
    assert_eq!(trashed[0].note.as_deref(), Some("Original searchable note"));
    let category_after_trash = db
        .get_bins()
        .unwrap()
        .into_iter()
        .find(|bin| bin.id == category.id)
        .unwrap();
    assert_eq!(category_after_trash.clip_count, Some(0));
    {
        let conn = db.conn.lock();
        let category_links: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM clip_bins WHERE clip_id = ?1 AND bin_id = ?2",
                params![clip.id, category.id],
                |row| row.get(0),
            )
            .unwrap();
        let tag_links: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM clip_bins WHERE clip_id = ?1 AND bin_id = ?2",
                params![clip.id, tag.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(category_links, 0);
        assert_eq!(tag_links, 1);
    }

    db.assign_to_bin(clip.id, Some(category.id)).unwrap();
    db.update_clip_note(clip.id, Some("Should be ignored"))
        .unwrap();
    db.update_clip_text(clip.id, "Should also be ignored")
        .unwrap();
    let unchanged = db.get_trashed_clips().unwrap();
    assert_eq!(unchanged[0].bin_id, None);
    assert_eq!(
        unchanged[0].note.as_deref(),
        Some("Original searchable note")
    );
    assert_eq!(
        unchanged[0].text_content.as_deref(),
        Some("Original searchable text")
    );

    db.restore_clip(clip.id).unwrap();
    let restored = db.get_clips(None, false).unwrap();
    assert_eq!(restored[0].bin_id, None);
    assert!(restored[0].bin_ids.as_ref().unwrap().contains(&tag.id));
    db.assign_to_bin(clip.id, Some(category.id)).unwrap();
    db.update_clip_note(clip.id, Some("Editable after restore"))
        .unwrap();
    let edited = db.get_clips(Some(category.id), false).unwrap();
    assert_eq!(edited[0].note.as_deref(), Some("Editable after restore"));
}
