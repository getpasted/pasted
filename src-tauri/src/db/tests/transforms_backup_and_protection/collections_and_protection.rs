use super::super::*;

#[test]
fn clip_collection_pages_and_summary_cover_active_and_trashed_clips() {
    let db = setup_test_db();
    let empty = db.get_clip_collection_summary().unwrap();
    assert_eq!(empty.active_count, 0);
    assert_eq!(empty.trash_count, 0);
    assert!(empty.clip_type_counts.is_empty());

    let clips = (0..6)
        .map(|index| {
            db.save_clip(
                if index % 2 == 0 { "text" } else { "link" },
                Some(&format!("clip {index}")),
                None,
                None,
                &format!("paged-clip-{index}"),
                if index < 4 { "Editor" } else { "Browser" },
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    db.toggle_pin(clips[0].id).unwrap();
    db.toggle_protected(clips[1].id).unwrap();
    db.toggle_concealed(clips[3].id).unwrap();
    db.update_clip_note(clips[2].id, Some("Remember this"))
        .unwrap();
    db.delete_clip(clips[5].id).unwrap();
    db.delete_clip(clips[4].id).unwrap();

    let first = db.get_clips_page(None, false, Some(2), Some(0)).unwrap();
    let second = db.get_clips_page(None, false, Some(2), Some(2)).unwrap();
    assert_eq!(first.len(), 2);
    assert_eq!(second.len(), 2);
    assert!(first
        .iter()
        .all(|left| second.iter().all(|right| left.id != right.id)));
    assert_eq!(
        db.get_trashed_clips_page(Some(1), Some(0)).unwrap().len(),
        1
    );
    assert_eq!(
        db.get_trashed_clips_page(Some(1), Some(1)).unwrap().len(),
        1
    );

    let summary = db.get_clip_collection_summary().unwrap();
    assert_eq!(summary.active_count, 4);
    assert_eq!(summary.trash_count, 2);
    assert_eq!(summary.pinned_count, 1);
    assert_eq!(summary.protected_count, 1);
    assert_eq!(summary.concealed_count, 1);
    assert_eq!(summary.noted_count, 1);
    assert_eq!(summary.clip_type_counts.len(), 1);
    assert_eq!(summary.clip_type_counts[0].clip_type, "text");
    assert_eq!(summary.clip_type_counts[0].count, 4);
    assert_eq!(summary.type_counts.len(), 1);
    assert_eq!(summary.type_counts[0].content_type, "link");
    assert_eq!(summary.type_counts[0].count, 2);
    assert_eq!(
        summary
            .source_counts
            .iter()
            .map(|item| item.count)
            .sum::<i64>(),
        4
    );
}

#[test]
fn clip_shortcuts_protect_assignments_and_keep_protection_when_cleared() {
    let db = setup_test_db();
    let clip = save_plain_test_clip(
        &db,
        "text",
        "durable shortcut",
        "clip-shortcut-protection",
        "Tests",
    );

    db.update_clip_hotkey(clip.id, Some("Alt+Shift+7")).unwrap();
    let assigned = db.get_clip_by_id(clip.id).unwrap();
    assert_eq!(assigned.shortcut.as_deref(), Some("Alt+Shift+7"));
    assert!(assigned.is_protected);
    assert_eq!(assigned.is_explicitly_protected, Some(true));
    assert_eq!(
        db.get_clip_hotkeys().unwrap(),
        vec![(clip.id, "Alt+Shift+7".to_string())]
    );
    assert!(db.batch_protect_clips(vec![clip.id], false).is_err());

    db.update_clip_hotkey(clip.id, None).unwrap();
    let cleared = db.get_clip_by_id(clip.id).unwrap();
    assert_eq!(cleared.shortcut, None);
    assert!(cleared.is_protected);
    assert_eq!(cleared.is_explicitly_protected, Some(true));
    assert!(db.get_clip_hotkeys().unwrap().is_empty());

    db.batch_protect_clips(vec![clip.id], false).unwrap();
    assert!(!db.get_clip_by_id(clip.id).unwrap().is_protected);
}

#[test]
fn protecting_bin_blocks_unprotect_after_clip_hotkey_is_removed() {
    let db = setup_test_db();
    let bin = db
        .create_bin("Protected Bin", "🛡️", "default", None)
        .unwrap();
    let clip = save_plain_test_clip(
        &db,
        "text",
        "hotkey and bin protection",
        "hotkey-bin-protection-precedence",
        "Tests",
    );

    db.update_bin_protection(bin.id, true).unwrap();
    db.update_clip_hotkey(clip.id, Some("Alt+Shift+8")).unwrap();
    db.assign_to_bin(clip.id, Some(bin.id)).unwrap();
    db.update_clip_hotkey(clip.id, None).unwrap();

    let protected = db.get_clip_by_id(clip.id).unwrap();
    assert!(protected.is_protected);
    assert_eq!(protected.is_explicitly_protected, Some(true));
    assert_eq!(protected.protecting_bin_ids, vec![bin.id]);
    assert!(db.batch_protect_clips(vec![clip.id], false).is_err());
    assert!(db
        .get_clip_by_id(clip.id)
        .unwrap()
        .is_explicitly_protected
        .unwrap());

    db.assign_to_bin(clip.id, None).unwrap();
    db.batch_protect_clips(vec![clip.id], false).unwrap();
    assert!(!db.get_clip_by_id(clip.id).unwrap().is_protected);
}

#[test]
fn manual_bin_protection_is_inherited_without_mutating_clips() {
    let db = setup_test_db();
    let bin = db
        .create_bin("Protected Bin", "🛡️", "default", None)
        .unwrap();
    let clip = save_plain_test_clip(
        &db,
        "text",
        "inherited",
        "bin-inherited-protection",
        "Tests",
    );
    db.update_bin_protection(bin.id, true).unwrap();
    db.assign_to_bin(clip.id, Some(bin.id)).unwrap();

    let protected = db.get_clip_by_id(clip.id).unwrap();
    assert!(protected.is_protected);
    assert_eq!(protected.is_explicitly_protected, Some(false));
    assert_eq!(protected.protecting_bin_ids, vec![bin.id]);
    let raw: i32 = db
        .conn
        .lock()
        .query_row(
            "SELECT is_protected FROM clips WHERE id = ?1",
            params![clip.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(raw, 0, "inherited protection must not mutate the clip flag");

    let trash = db.batch_trash_clips(vec![clip.id]).unwrap();
    assert_eq!(trash.changed_count, 0);
    db.purge_clip_permanently(clip.id).unwrap();
    assert!(db.get_clip_by_id(clip.id).is_ok());
    db.clear_history().unwrap();
    assert!(db.get_clip_by_id(clip.id).is_ok());

    db.assign_to_bin(clip.id, None).unwrap();
    assert!(!db.get_clip_by_id(clip.id).unwrap().is_protected);
    assert_eq!(
        db.batch_trash_clips(vec![clip.id]).unwrap().changed_count,
        1
    );
}

#[test]
fn smart_bins_cannot_apply_inherited_clip_policies() {
    let db = setup_test_db();
    let rule = serde_json::json!({
        "version": 1,
        "conditions": [{"type": "clip_type", "operator": "is", "value": "text"}],
        "match": "all"
    })
    .to_string();
    let bin = db
        .create_bin("Smart", "🧠", "default", Some(&rule))
        .unwrap();
    assert!(db.update_bin_protection(bin.id, true).is_err());
    assert!(!db.get_bin(bin.id).unwrap().protect_clips);
    assert!(db.update_bin_concealment(bin.id, true).is_err());
    assert!(!db.get_bin(bin.id).unwrap().conceal_clips);
}

#[test]
fn legacy_databases_migrate_clip_shortcuts_and_bin_protection() {
    let path = std::env::temp_dir().join(format!(
        "pasted-shortcut-protection-migration-{}.db",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE clips (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    content_type TEXT NOT NULL,
                    text_content TEXT,
                    html_content TEXT,
                    image_base64 TEXT,
                    content_hash TEXT UNIQUE NOT NULL,
                    source TEXT DEFAULT 'Unknown',
                    is_pinned INTEGER DEFAULT 0,
                    bin_id INTEGER,
                    note TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                 );
                 CREATE TABLE bins (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    icon TEXT DEFAULT 'Folder',
                    color TEXT DEFAULT 'default',
                    smart_rule TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                 );",
        )
        .unwrap();
    drop(connection);

    let db = DbState::new(path.clone()).unwrap();
    assert!(column_exists(&db.conn.lock(), "clips", "shortcut").unwrap());
    assert!(column_exists(&db.conn.lock(), "clips", "is_concealed").unwrap());
    assert!(column_exists(&db.conn.lock(), "clips", "is_revealed").unwrap());
    assert!(column_exists(&db.conn.lock(), "bins", "protect_clips").unwrap());
    assert!(column_exists(&db.conn.lock(), "bins", "conceal_clips").unwrap());
    assert!(column_exists(&db.conn.lock(), "content_types", "conceal_clips").unwrap());
    let view_exists: bool = db
        .conn
        .lock()
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'view' AND name = 'effective_clip_protection')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(view_exists);
    drop(db);
    let _ = fs::remove_file(path);
}
