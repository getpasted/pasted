use super::super::*;

#[path = "../search_history.rs"]
mod search_history;

#[test]
fn test_wal_mode_and_indexing() {
    let db = setup_test_db();
    let conn = db.conn.lock();

    // Verify WAL mode is configured
    let mode: String = conn
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    assert!(
        mode.to_lowercase() == "wal" || mode.to_lowercase() == "memory",
        "journal_mode should be wal or memory (test db), got: {}",
        mode
    );

    // Verify indexes exist
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='index'")
        .unwrap();
    let index_names: Vec<String> = stmt
        .query_map([], |r| r.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    assert!(index_names.contains(&"idx_clips_pinned_created".to_string()));
    assert!(index_names.contains(&"idx_clips_bin_created".to_string()));
    assert!(index_names.contains(&"idx_clips_hash".to_string()));
    assert!(index_names.contains(&"idx_clips_active_timeline".to_string()));
}

#[test]
fn test_fts5_search_indexing() {
    let db = setup_test_db();

    let clip1 = save_plain_test_clip(
        &db,
        "text",
        "Supercalifragilisticexpialidocious secret token",
        "HashFTS1",
        "IntelliJ",
    );
    let _clip2 = save_plain_test_clip(
        &db,
        "text",
        "Unrelated standard content text",
        "HashFTS2",
        "Safari",
    );

    let search_res = search_test_clips(&db, "Supercalifragilisticexpialidocious");
    assert_eq!(search_res.len(), 1);
    assert_eq!(search_res[0].id, clip1.id);

    db.update_clip_name(clip1.id, Some("Celestial Archive"))
        .unwrap();
    let name_search = search_test_clips(&db, "celestial");
    assert_eq!(name_search.len(), 1);
    assert_eq!(name_search[0].id, clip1.id);

    let status = db.get_search_index_status().unwrap();
    assert_eq!(status.indexes.len(), 2);
    assert!(status.indexes.iter().all(|index| index.healthy));
    {
        let conn = db.conn.lock();
        conn.execute("INSERT INTO clips_fts(clips_fts) VALUES('delete-all')", [])
            .unwrap();
    }
    assert!(!db.get_search_index_status().unwrap().indexes[0].healthy);
    assert!(db
        .rebuild_search_index("all")
        .unwrap()
        .indexes
        .iter()
        .all(|index| index.healthy));

    db.delete_clip(clip1.id).unwrap();
    let search_after_delete = search_test_clips(&db, "Supercalifragilisticexpialidocious");
    assert_eq!(search_after_delete.len(), 0);
}

#[test]
fn file_extraction_is_hash_safe_searchable_and_non_destructive() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "file",
            Some(r#"["/tmp/interview.wav"]"#),
            None,
            None,
            "file-transcription-hash",
            "Tests",
        )
        .unwrap();
    let extractor = db
        .get_content_extractors()
        .unwrap()
        .into_iter()
        .find(|extractor| {
            extractor.stable_ref == crate::content_extraction::WHISPER_TRANSCRIPTION_REF
        })
        .unwrap();

    assert!(!db
        .replace_clip_searchable_text(
            clip.id,
            "stale-hash",
            &extractor,
            Some("quasar transcript marker"),
        )
        .unwrap());
    assert!(db
        .replace_clip_searchable_text(
            clip.id,
            &clip.content_hash,
            &extractor,
            Some("quasar transcript marker"),
        )
        .unwrap());
    let stored = db.get_clip_searchable_text(clip.id).unwrap().unwrap();
    assert_eq!(stored.searchable_text, "quasar transcript marker");
    assert_eq!(stored.extractor_ref, extractor.stable_ref);
    assert_eq!(
        db.get_clip_by_id(clip.id).unwrap().text_content.as_deref(),
        Some(r#"["/tmp/interview.wav"]"#)
    );
    let matches = search_test_clips(&db, "quasar");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].id, clip.id);
    assert_eq!(
        db.search_clips(&ClipSearchRequest {
            query: "quasar marker".into(),
            limit: 10,
            ..Default::default()
        })
        .unwrap()
        .items[0]
            .id,
        clip.id
    );
    assert_eq!(
        db.search_clips(&ClipSearchRequest {
            query: "quasar missing".into(),
            limit: 10,
            ..Default::default()
        })
        .unwrap()
        .total_count,
        0
    );

    assert!(db
        .replace_clip_searchable_text(clip.id, &clip.content_hash, &extractor, None)
        .unwrap());
    assert!(db.get_clip_searchable_text(clip.id).unwrap().is_none());
    assert!(search_test_clips(&db, "quasar").is_empty());

    assert!(db
        .replace_clip_searchable_text(
            clip.id,
            &clip.content_hash,
            &extractor,
            Some("stale quasar marker"),
        )
        .unwrap());
    db.conn
        .lock()
        .execute(
            "UPDATE clips SET content_hash = 'changed-file-hash' WHERE id = ?1",
            params![clip.id],
        )
        .unwrap();
    assert!(db.get_clip_searchable_text(clip.id).unwrap().is_none());
    assert!(search_test_clips(&db, "quasar").is_empty());
}

#[test]
fn authoritative_search_combines_axes_pagination_trash_extraction_and_feature_gates() {
    let db = setup_test_db();
    assert!(db
        .search_clips(&ClipSearchRequest {
            limit: MAX_CLIP_SEARCH_PAGE_SIZE + 1,
            ..Default::default()
        })
        .is_err());
    let matching = db
        .save_clip(
            "file",
            Some(r#"["/tmp/report.pdf"]"#),
            None,
            None,
            "authoritative-search-match",
            "Finder",
        )
        .unwrap();
    let other = save_plain_test_clip(
        &db,
        "text",
        "ordinary shared marker",
        "authoritative-search-other",
        "Terminal",
    );
    let extractor = db
        .get_content_extractors()
        .unwrap()
        .into_iter()
        .find(|extractor| {
            extractor.stable_ref == crate::content_extraction::WHISPER_TRANSCRIPTION_REF
        })
        .unwrap();
    db.replace_clip_searchable_text(
        matching.id,
        &matching.content_hash,
        &extractor,
        Some("extracted shared marker"),
    )
    .unwrap();
    {
        let conn = db.conn.lock();
        conn.execute(
            "INSERT INTO clip_analysis_classifications
                    (clip_id, content_type, classifier_ref, source_representation, input_hash,
                     start_offset, end_offset)
                 VALUES (?1, 'document', 'test:document', 'searchable_text', ?2, 0, 9)",
            params![matching.id, matching.content_hash],
        )
        .unwrap();
        conn.execute(
            "UPDATE clip_analysis_results
                 SET content_hash = ?3, input_hash = ?3, format_version = ?4,
                     result_json = '{\"formats\":[{\"format\":\"pdf\"}]}'
                 WHERE clip_id = ?1 AND participant_ref = ?2",
            params![
                matching.id,
                crate::content_inspection::FILE_FORMAT_INSPECTOR_REF,
                matching.content_hash,
                crate::analysis_contract::ANALYSIS_CONTRACT_VERSION,
            ],
        )
        .unwrap();
    }

    let combined = db
        .search_clips(&ClipSearchRequest {
            query: "extracted clip:fi content:doc format:pd source:find".into(),
            limit: 10,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(combined.schema_version, 1);
    assert_eq!(combined.total_count, 1);
    assert_eq!(combined.items[0].id, matching.id);
    assert_eq!(combined.items[0].content_types, vec!["document"]);
    assert_eq!(combined.items[0].file_formats, vec!["pdf"]);
    assert_eq!(combined.items[0].html_content, None);
    assert_eq!(combined.items[0].image_base64, None);
    assert_eq!(
        db.search_clips(&ClipSearchRequest {
            sources: vec!["find".into()],
            limit: 10,
            ..Default::default()
        })
        .unwrap()
        .total_count,
        1,
        "explicit Search filters use partial matching"
    );

    let first_page = db
        .search_clips(&ClipSearchRequest {
            query: "shared marker".into(),
            limit: 1,
            ..Default::default()
        })
        .unwrap();
    let second_page = db
        .search_clips(&ClipSearchRequest {
            query: "shared marker".into(),
            limit: 1,
            offset: 1,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(first_page.total_count, 2);
    assert_eq!(second_page.total_count, 2);
    assert_ne!(first_page.items[0].id, second_page.items[0].id);
    assert!(first_page
        .items
        .iter()
        .chain(&second_page.items)
        .any(|clip| clip.id == other.id));

    db.delete_clip(matching.id).unwrap();
    let trashed = db
        .search_clips(&ClipSearchRequest {
            query: "extracted is:trashed".into(),
            limit: 10,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(trashed.total_count, 1);
    assert_eq!(trashed.items[0].id, matching.id);
    assert!(trashed.items[0].is_trashed);

    for (setting, filter) in [
        ("enableClipTypes", "clip:file"),
        ("enableTypes", "content:document"),
        ("enableFileFormats", "format:pdf"),
        ("enableSources", "source:finder"),
    ] {
        db.save_setting(setting, "false").unwrap();
        assert_eq!(
            db.search_clips(&ClipSearchRequest {
                query: format!("{filter} is:trashed"),
                limit: 10,
                ..Default::default()
            })
            .unwrap()
            .total_count,
            0,
            "{setting} must suspend its Search filter"
        );
        db.save_setting(setting, "true").unwrap();
    }
    assert_eq!(
        db.search_clips(&ClipSearchRequest {
            query: "format:pd is:trashed".into(),
            limit: 10,
            ..Default::default()
        })
        .unwrap()
        .total_count,
        1,
        "collection-axis filters use case-insensitive partial matching"
    );
}

#[test]
fn test_startup_rebuilds_fts_before_clip_updates() {
    let db = setup_test_db();
    let clip = save_plain_test_clip(
        &db,
        "text",
        "Recoverable noted clip",
        "HashFTSRecovery",
        "Notes",
    );
    db.update_clip_note(clip.id, Some("Keep this note"))
        .unwrap();

    {
        let conn = db.conn.lock();
        conn.execute("INSERT INTO clips_fts(clips_fts) VALUES('delete-all')", [])
            .unwrap();
    }

    db.init_tables().unwrap();
    let search_results = search_test_clips(&db, "Recoverable");
    assert_eq!(search_results.len(), 1);
    assert_eq!(search_results[0].id, clip.id);

    assert!(db.toggle_pin(clip.id).unwrap());
    db.update_clip_note(clip.id, Some("Updated note")).unwrap();
    db.delete_clip(clip.id).unwrap();
    assert!(db.get_clips(None, false).unwrap().is_empty());
}
