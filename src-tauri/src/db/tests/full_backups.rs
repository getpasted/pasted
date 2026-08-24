use super::*;

#[test]
fn full_backup_round_trip_covers_every_durable_table_and_interface_state() {
    let db = setup_test_db();
    let active_path = db.database_path();
    let backup_path = active_path.with_extension("pastedbackup");
    let clip = db
        .save_clip(
            "text",
            Some("complete backup marker"),
            None,
            None,
            "full-backup-marker",
            "Tests",
        )
        .unwrap();
    db.update_clip_text(clip.id, "updated backup marker")
        .unwrap();
    db.replace_analysis_classifications(
        clip.id,
        &clip.content_hash,
        &[crate::content_classification::ClassificationMatch {
            classifier_ref: "prose".into(),
            classifier_name: "Prose".into(),
            content_type: "prose".into(),
            priority: 180,
            start_offset: 0,
            end_offset: 10,
        }],
        "original_text",
    )
    .unwrap();
    let file_clip = db
        .save_clip(
            "file",
            Some(r#"["/tmp/backup-audio.wav"]"#),
            None,
            None,
            "full-backup-file-marker",
            "Tests",
        )
        .unwrap();
    let protected_bin = db
        .create_bin("Backup Protection", "🔐", "default", None)
        .unwrap();
    db.update_bin_protection(protected_bin.id, true).unwrap();
    db.assign_to_bin(clip.id, Some(protected_bin.id)).unwrap();
    db.update_clip_hotkey(clip.id, Some("Alt+Shift+9")).unwrap();
    let transcription_extractor = db
        .get_content_extractors()
        .unwrap()
        .into_iter()
        .find(|candidate| {
            candidate.stable_ref == crate::content_extraction::WHISPER_TRANSCRIPTION_REF
        })
        .unwrap();
    assert!(db
        .replace_clip_searchable_text(
            file_clip.id,
            &file_clip.content_hash,
            &transcription_extractor,
            Some("complete transcription backup marker"),
        )
        .unwrap());
    db.record_extraction_observations(
        file_clip.id,
        &file_clip.content_hash,
        &[crate::content_analysis::ExtractionObservation {
            extractor_ref: transcription_extractor.stable_ref.clone(),
            extractor_name: transcription_extractor.name.clone(),
            engine: transcription_extractor.engine.clone(),
            priority: transcription_extractor.priority,
            duplicate_of: None,
            outcome: crate::content_extraction::ExtractionOutcome::Produced {
                text: "complete transcription backup marker".into(),
            },
        }],
    )
    .unwrap();
    db.save_setting("fullBackupSetting", "preserved").unwrap();
    db.log_activity("app_started", "Complete backup test")
        .unwrap();
    db.create_intelligence_connection(
        "Backup Connection",
        "openai_compatible",
        Some("http://127.0.0.1:1234/v1"),
        Some("local-model"),
        Some("keychain:pasted:test"),
    )
    .unwrap();
    let extractor = db.get_content_extractors().unwrap().remove(0);
    db.update_content_extractor(
        extractor.id,
        &crate::content_extraction::ExtractorInput {
            name: "Backup Extractor Marker".into(),
            description: extractor.description,
            enabled: false,
            priority: 77,
        },
    )
    .unwrap();

    let client_state = r#"{"version":1,"localStorage":{"pasted_sidebar_width":"280"}}"#;
    let window_state = r#"{"main":{"width":1200,"height":800}}"#;
    let report = db
        .create_full_backup(&backup_path, Some(client_state), Some(window_state))
        .unwrap();
    assert!(report.size_bytes > 0);
    let inspection = db.inspect_full_backup(&backup_path).unwrap();
    assert_eq!(inspection.format_version, FULL_BACKUP_FORMAT_VERSION);
    assert_eq!(inspection.created_at, report.created_at);
    assert_eq!(inspection.size_bytes, report.size_bytes);

    let table_names = |connection: &Connection| -> Vec<String> {
        let mut statement = connection
            .prepare(
                "SELECT name FROM sqlite_master
                     WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
                     ORDER BY name",
            )
            .unwrap();
        statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap()
    };
    let source_tables = table_names(&db.conn.lock());
    let backup_connection = Connection::open(&backup_path).unwrap();
    let backup_tables = table_names(&backup_connection);
    for table in source_tables {
        assert!(
            backup_tables.contains(&table),
            "full backup omitted durable table {table}"
        );
    }
    assert!(backup_tables.contains(&"pasted_backup_manifest".to_string()));
    let external_state_notice: String = backup_connection
        .query_row(
            "SELECT external_state_notice FROM pasted_backup_manifest LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(external_state_notice.contains("paths to original files"));
    assert!(external_state_notice.contains("credential stores"));
    drop(backup_connection);

    db.save_setting("fullBackupSetting", "mutated").unwrap();
    db.save_clip(
        "text",
        Some("post-backup marker"),
        None,
        None,
        "post-backup-marker",
        "Tests",
    )
    .unwrap();
    let (restore_report, restored_client_state, restored_window_state) = db
        .restore_full_backup(&backup_path, Some("{}"), Some("{}"))
        .unwrap();

    assert_eq!(restored_client_state.as_deref(), Some(client_state));
    assert_eq!(restored_window_state.as_deref(), Some(window_state));
    assert_eq!(
        db.get_setting("fullBackupSetting").unwrap().as_deref(),
        Some("preserved")
    );
    assert_eq!(db.get_all_clips_for_backup().unwrap().len(), 2);
    let restored_clip = db.get_clip_by_id(clip.id).unwrap();
    assert_eq!(restored_clip.shortcut.as_deref(), Some("Alt+Shift+9"));
    assert!(restored_clip.is_protected);
    assert!(db.get_bin(protected_bin.id).unwrap().protect_clips);
    assert!(!db.get_clip_versions(clip.id).unwrap().is_empty());
    assert_eq!(
        db.get_analysis_classifications(clip.id).unwrap()[0].content_type,
        "prose"
    );
    assert!(db
        .get_activity_logs(None, None)
        .unwrap()
        .iter()
        .any(|entry| entry.event_type == "app_started"));
    assert_eq!(db.get_intelligence_connections().unwrap().len(), 1);
    assert_eq!(
        db.get_clip_searchable_text(file_clip.id)
            .unwrap()
            .unwrap()
            .searchable_text,
        "complete transcription backup marker"
    );
    assert_eq!(
        db.get_extraction_history(file_clip.id, 101, 0)
            .unwrap()
            .len(),
        1
    );
    let restored_extractor = db.get_content_extractor(&extractor.stable_ref).unwrap();
    assert_eq!(restored_extractor.name, "Backup Extractor Marker");
    assert!(!restored_extractor.enabled);
    assert_eq!(restored_extractor.priority, 77);
    assert!(Path::new(&restore_report.recovery_path).is_file());
    assert_eq!(
        db.consume_pending_full_restore_client_state()
            .unwrap()
            .as_deref(),
        Some(client_state)
    );
    assert!(db
        .consume_pending_full_restore_client_state()
        .unwrap()
        .is_none());

    let _ = fs::remove_file(backup_path);
    let _ = fs::remove_file(restore_report.recovery_path);
}

#[test]
fn full_restore_rejects_invalid_embedded_state_before_replacing_library() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!("pasted-invalid-backup-{unique}"));
    fs::create_dir_all(&directory).unwrap();
    let db = DbState::new(directory.join("library.db")).unwrap();
    let backup_path = db.database_path().with_extension("pastedbackup");
    db.save_setting("liveStateMarker", "untouched").unwrap();
    db.create_full_backup(&backup_path, Some("{}"), Some("{}"))
        .unwrap();
    let backup = Connection::open(&backup_path).unwrap();
    backup
        .execute(
            "UPDATE pasted_backup_manifest SET client_state_json = 'not-json'",
            [],
        )
        .unwrap();
    let _ = backup.pragma_update(None, "wal_checkpoint", "TRUNCATE");
    drop(backup);

    assert!(db.inspect_full_backup(&backup_path).is_err());
    assert!(db
        .restore_full_backup(&backup_path, Some("{}"), Some("{}"))
        .is_err());
    assert_eq!(
        db.get_setting("liveStateMarker").unwrap().as_deref(),
        Some("untouched")
    );
    let recovery_count = fs::read_dir(&directory)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("Pasted_Pre_Restore_")
        })
        .count();
    assert_eq!(recovery_count, 0);
    let _ = fs::remove_file(backup_path);
    drop(db);
    let _ = fs::remove_dir_all(directory);
}
