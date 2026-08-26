use super::*;

fn request(query: &str) -> ClipSearchRequest {
    ClipSearchRequest {
        query: query.into(),
        limit: 25,
        offset: 50,
        ..Default::default()
    }
}

#[test]
fn records_canonical_requests_without_pagination_and_deduplicates() {
    let db = setup_test_db();
    let mut first = request("  Important Search  ");
    first.clip_ids = vec![9, 3, 9];
    first.sources = vec![" Safari ".into(), "safari".into()];

    let recorded = db.record_search_history(&first, 7).unwrap();
    assert_eq!(recorded.request.query, "Important Search");
    assert_eq!(recorded.request.clip_ids, vec![3, 9]);
    assert_eq!(recorded.request.sources, vec!["safari"]);
    assert_eq!(recorded.request.limit, 0);
    assert_eq!(recorded.request.offset, 0);
    assert_eq!(recorded.result_count, 7);
    assert_eq!(recorded.use_count, 1);
    assert!(recorded.last_used_at.ends_with('Z'));
    assert_eq!(
        chrono::DateTime::parse_from_rfc3339(&recorded.last_used_at)
            .unwrap()
            .offset()
            .local_minus_utc(),
        0
    );

    let mut same = request("Important Search");
    same.clip_ids = vec![3, 9];
    same.sources = vec!["SAFARI".into()];
    let updated = db.record_search_history(&same, 2).unwrap();
    assert_eq!(updated.id, recorded.id);
    assert_eq!(updated.result_count, 2);
    assert_eq!(updated.use_count, 2);

    let page = db.list_search_history(20, 0).unwrap();
    assert_eq!(page.total_count, 1);
    assert_eq!(page.items, vec![updated]);
    assert_eq!(page.limit, 20);
    assert_eq!(page.offset, 0);
}

#[test]
fn retention_is_bounded_immediately_and_zero_is_unlimited() {
    let db = setup_test_db();
    assert_eq!(db.configure_search_history_retention(2, 0).unwrap(), 0);
    for query in ["one", "two", "three"] {
        db.record_search_history(&request(query), 1).unwrap();
    }
    let retained = db.list_search_history(10, 0).unwrap();
    assert_eq!(retained.total_count, 2);
    assert_eq!(
        retained
            .items
            .iter()
            .map(|entry| entry.request.query.as_str())
            .collect::<Vec<_>>(),
        vec!["three", "two"]
    );

    assert_eq!(db.configure_search_history_retention(1, 0).unwrap(), 1);
    assert_eq!(db.list_search_history(10, 0).unwrap().total_count, 1);

    db.configure_search_history_retention(0, 0).unwrap();
    for query in ["four", "five", "six"] {
        db.record_search_history(&request(query), 1).unwrap();
    }
    assert_eq!(db.list_search_history(10, 0).unwrap().total_count, 4);
}

#[test]
fn missing_retention_setting_uses_the_bounded_default() {
    let db = setup_test_db();
    for index in 0..=DEFAULT_SEARCH_HISTORY_LIMIT {
        db.record_search_history(&request(&format!("default limit {index}")), 0)
            .unwrap();
    }
    assert_eq!(
        db.list_search_history(MAX_SEARCH_HISTORY_PAGE_SIZE, 0)
            .unwrap()
            .total_count,
        DEFAULT_SEARCH_HISTORY_LIMIT
    );
}

#[test]
fn management_and_safety_bounds_are_enforced() {
    let db = setup_test_db();
    let recorded = db.record_search_history(&request("delete me"), 0).unwrap();
    assert!(db.delete_search_history(recorded.id).unwrap());
    assert!(!db.delete_search_history(recorded.id).unwrap());
    assert!(db.delete_search_history(0).is_err());

    db.record_search_history(&request("one"), 0).unwrap();
    db.record_search_history(&request("two"), 0).unwrap();
    assert_eq!(db.clear_search_history().unwrap(), 2);
    assert_eq!(db.list_search_history(10, 0).unwrap().total_count, 0);

    assert!(db.list_search_history(0, 0).is_err());
    assert!(db
        .list_search_history(MAX_SEARCH_HISTORY_PAGE_SIZE + 1, 0)
        .is_err());
    assert!(db
        .list_search_history(1, MAX_CLIP_SEARCH_OFFSET + 1)
        .is_err());
    assert!(db.configure_search_history_retention(-1, 0).is_err());
    assert!(db
        .configure_search_history_retention(MAX_SEARCH_HISTORY_LIMIT as i64 + 1, 0)
        .is_err());
    assert!(db.configure_search_history_retention(100, -1).is_err());
    assert!(db
        .configure_search_history_retention(100, MAX_SEARCH_HISTORY_AGE_DAYS + 1)
        .is_err());
}

#[test]
fn age_and_count_retention_apply_together_and_persist_atomically() {
    let db = setup_test_db();
    for query in ["old", "newer", "newest"] {
        db.record_search_history(&request(query), 1).unwrap();
    }
    db.conn
        .lock()
        .execute(
            "UPDATE search_history SET last_used_at = '2000-01-01T00:00:00.000Z'
             WHERE request_json LIKE '%old%'",
            [],
        )
        .unwrap();

    assert_eq!(db.configure_search_history_retention(1, 30).unwrap(), 2);
    assert_eq!(db.list_search_history(10, 0).unwrap().total_count, 1);
    assert_eq!(
        db.get_setting("searchHistoryLimit").unwrap().as_deref(),
        Some("1")
    );
    assert_eq!(
        db.get_setting("searchHistoryAgeDays").unwrap().as_deref(),
        Some("30")
    );
}

#[test]
fn age_cutoff_is_deterministic_and_keeps_entries_at_the_boundary() {
    let db = setup_test_db();
    let boundary = db.record_search_history(&request("boundary"), 1).unwrap();
    let expired = db.record_search_history(&request("expired"), 1).unwrap();
    let conn = db.conn.lock();
    conn.execute(
        "UPDATE search_history SET last_used_at = ?1 WHERE id = ?2",
        params!["2026-01-02T00:00:00.000Z", boundary.id],
    )
    .unwrap();
    conn.execute(
        "UPDATE search_history SET last_used_at = ?1 WHERE id = ?2",
        params!["2026-01-01T23:59:59.999Z", expired.id],
    )
    .unwrap();
    let reference = chrono::DateTime::parse_from_rfc3339("2026-02-01T00:00:00.000Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    assert_eq!(
        crate::db::search_history::prune_search_history(&conn, 0, 30, reference).unwrap(),
        1
    );
    drop(conn);
    let retained = db.list_search_history(10, 0).unwrap();
    assert_eq!(retained.total_count, 1);
    assert_eq!(retained.items[0].request.query, "boundary");
}

#[test]
fn recording_enforces_the_persisted_age_policy() {
    let db = setup_test_db();
    let stale = db.record_search_history(&request("stale"), 1).unwrap();
    db.conn
        .lock()
        .execute(
            "UPDATE search_history SET last_used_at = '2000-01-01T00:00:00.000Z'
             WHERE id = ?1",
            [stale.id],
        )
        .unwrap();
    db.save_setting("searchHistoryLimit", "0").unwrap();
    db.save_setting("searchHistoryAgeDays", "30").unwrap();
    db.record_search_history(&request("fresh"), 1).unwrap();
    let retained = db.list_search_history(10, 0).unwrap();
    assert_eq!(retained.total_count, 1);
    assert_eq!(retained.items[0].request.query, "fresh");
}

#[test]
fn record_reuses_search_request_safety_validation() {
    let db = setup_test_db();
    assert!(db
        .record_search_history(&ClipSearchRequest::default(), 0)
        .is_err());
    db.record_search_history(
        &ClipSearchRequest {
            sources: vec!["Safari".into()],
            ..Default::default()
        },
        0,
    )
    .unwrap();
    let oversized = ClipSearchRequest {
        query: "x".repeat(MAX_CLIP_SEARCH_QUERY_BYTES + 1),
        ..Default::default()
    };
    assert!(db.record_search_history(&oversized, 0).is_err());
    assert_eq!(db.list_search_history(10, 0).unwrap().total_count, 1);
}

#[test]
fn factory_reset_removes_search_history() {
    let db = setup_test_db();
    db.record_search_history(&request("private search"), 3)
        .unwrap();
    db.factory_reset().unwrap();
    assert_eq!(db.list_search_history(10, 0).unwrap().total_count, 0);
}

#[test]
fn full_backup_round_trips_search_history() {
    let db = setup_test_db();
    db.record_search_history(&request("backup search"), 4)
        .unwrap();
    let backup_path = db.database_path().with_extension("pastedbackup");
    db.create_full_backup(&backup_path, None, None).unwrap();
    db.clear_search_history().unwrap();
    let (report, _, _) = db.restore_full_backup(&backup_path, None, None).unwrap();
    let restored = db.list_search_history(10, 0).unwrap();
    assert_eq!(restored.total_count, 1);
    assert_eq!(restored.items[0].request.query, "backup search");
    let _ = fs::remove_file(backup_path);
    let _ = fs::remove_file(report.recovery_path);
}
