use super::*;
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
    assert_eq!(active.iter().filter(|clip| !clip.is_pinned).count(), 2);
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

#[test]
fn activity_age_retention_removes_old_entries_with_unlimited_count() {
    let db = setup_test_db();
    db.log_activity("app_started", "Old activity").unwrap();
    db.log_activity("app_exit_requested", "Recent activity")
        .unwrap();
    {
        let conn = db.conn.lock();
        conn.execute(
            "UPDATE activity_logs SET created_at = datetime('now', '-31 days')
                 WHERE description = 'Old activity'",
            [],
        )
        .unwrap();
    }

    db.configure_activity_retention(0, 30).unwrap();

    let logs = db.get_activity_logs(None, None).unwrap();
    assert!(!logs.iter().any(|log| log.description == "Old activity"));
    assert!(logs.iter().any(|log| log.description == "Recent activity"));
}

#[test]
fn activity_archive_roundtrip_is_structured_inert_and_deduplicated() {
    let source = setup_test_db();
    source
        .log_activity("transformation_execution_failed", "Transform failed safely")
        .unwrap();
    source
        .log_activity("clip_restored", "Restored one clip")
        .unwrap();

    let json = source.export_activity_json().unwrap();
    let archive: ActivityArchive = serde_json::from_str(&json).unwrap();
    assert_eq!(archive.schema_version, 1);
    assert_eq!(archive.resource["service.name"], "Pasted");
    let failure = archive
        .entries
        .iter()
        .find(|entry| entry.event_name == "transformation_execution_failed")
        .unwrap();
    assert_eq!(failure.severity_text, "error");
    assert_eq!(failure.attributes["pasted.category"], "transformation");
    assert_eq!(failure.attributes["pasted.outcome"], "failure");
    assert!(!json.contains("text_content"));

    let destination = setup_test_db();
    destination.configure_activity_retention(0, 0).unwrap();
    let preview = destination.inspect_activity_json(&json).unwrap();
    assert_eq!(preview.scanned_count, 2);
    assert_eq!(preview.imported_count, 2);
    assert!(destination
        .get_activity_logs(None, None)
        .unwrap()
        .is_empty());
    let first = destination.import_activity_json(&json).unwrap();
    assert_eq!(first.scanned_count, 2);
    assert_eq!(first.imported_count, 2);
    assert_eq!(first.duplicate_count, 0);
    let second = destination.import_activity_json(&json).unwrap();
    assert_eq!(second.imported_count, 0);
    assert_eq!(second.duplicate_count, 2);
    assert_eq!(destination.get_activity_logs(None, None).unwrap().len(), 2);
}

#[test]
fn activity_import_rejects_invalid_records_without_partial_writes() {
    let db = setup_test_db();
    let archive = serde_json::json!({
        "schemaVersion": 1,
        "exportedAt": "2026-08-13T00:00:00Z",
        "resource": { "service.name": "Pasted" },
        "entries": [
            {
                "timestamp": "2026-08-13T00:00:00Z",
                "observedTimestamp": "2026-08-13T00:00:00Z",
                "eventName": "clip_restored",
                "severityText": "info",
                "body": "Valid record",
                "attributes": {}
            },
            {
                "timestamp": "not-a-time",
                "observedTimestamp": "2026-08-13T00:00:00Z",
                "eventName": "clip_restored",
                "severityText": "info",
                "body": "Invalid record",
                "attributes": {}
            }
        ]
    });
    assert!(db.import_activity_json(&archive.to_string()).is_err());
    assert!(db.get_activity_logs(None, None).unwrap().is_empty());
}

#[test]
fn activity_csv_export_has_a_stable_safe_content_contract() {
    let db = setup_test_db();
    {
        let conn = db.conn.lock();
        conn.execute(
            "INSERT INTO activity_logs
                    (event_type, description, created_at, observed_at, severity_text,
                     category, outcome, attributes_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "transformation_execution_failed",
                "=SUM(A1:A2), \"unsafe\"",
                "2026-08-13 12:34:56",
                "2026-08-13T12:35:00Z",
                "error",
                "transformation",
                "failure",
                r#"{"attempt":1}"#,
            ],
        )
        .unwrap();
    }

    let csv = db.export_activity_csv().unwrap();
    let mut lines = csv.lines();
    assert_eq!(
            lines.next(),
            Some("timestamp,observed_timestamp,event_name,severity_text,body,category,outcome,attributes_json")
        );
    let row = lines.next().unwrap();
    assert!(row.contains("\"2026-08-13T12:34:56Z\""));
    assert!(row.contains("\"transformation_execution_failed\""));
    assert!(row.contains("\"'=SUM(A1:A2), \"\"unsafe\"\"\""));
    assert!(row.contains("\"error\""));
    assert!(row.contains("\"transformation\",\"failure\""));
    assert!(lines.next().is_none());
    let records = DbState::parse_csv(&csv).unwrap();
    let exported_attributes: serde_json::Value = serde_json::from_str(&records[1][7]).unwrap();
    assert_eq!(exported_attributes["attempt"], 1);
    assert_eq!(exported_attributes["pasted.category"], "transformation");
    assert_eq!(exported_attributes["pasted.outcome"], "failure");
    assert!(exported_attributes["event.sequence"].is_number());

    let destination = setup_test_db();
    destination.configure_activity_retention(0, 0).unwrap();
    let preview = destination.inspect_activity_csv(&csv).unwrap();
    assert_eq!(preview.imported_count, 1);
    assert!(destination
        .get_activity_logs(None, None)
        .unwrap()
        .is_empty());
    let first = destination.import_activity_csv(&csv).unwrap();
    assert_eq!(first.scanned_count, 1);
    assert_eq!(first.imported_count, 1);
    assert_eq!(first.duplicate_count, 0);
    let second = destination.import_activity_csv(&csv).unwrap();
    assert_eq!(second.imported_count, 0);
    assert_eq!(second.duplicate_count, 1);
    let imported = destination.get_activity_logs(None, None).unwrap().remove(0);
    assert_eq!(imported.description, "=SUM(A1:A2), \"unsafe\"");
    assert_eq!(imported.category, "transformation");
    assert_eq!(imported.outcome, "failure");
    assert_eq!(imported.attributes["attempt"], 1);
    assert!(imported.attributes["event.sequence"].is_number());

    let invalid_target = setup_test_db();
    let invalid_csv = format!("{csv}\"broken\",\"row\"");
    assert!(invalid_target.import_activity_csv(&invalid_csv).is_err());
    assert!(invalid_target
        .get_activity_logs(None, None)
        .unwrap()
        .is_empty());
}
