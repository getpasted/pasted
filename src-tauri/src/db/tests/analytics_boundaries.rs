use super::*;

#[test]
fn insights_summary_is_strictly_read_only() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "text",
            Some("Read-only insight"),
            None,
            None,
            "insights-read-only",
            "",
        )
        .unwrap();
    let changes_before = db.conn.lock().total_changes();
    let before = db.get_clip_by_id(clip.id).unwrap();
    let summary = db.get_analytics_summary().unwrap();
    let after = db.get_clip_by_id(clip.id).unwrap();

    assert_eq!(summary.total_clips, 1);
    assert_eq!(db.conn.lock().total_changes(), changes_before);
    assert_eq!(after.source, before.source);
    assert_eq!(after.content_hash, before.content_hash);
}

#[test]
fn insights_groups_daily_activity_by_the_requested_local_calendar() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "text",
            Some("Boundary clip"),
            None,
            None,
            "boundary-clip",
            "Tests",
        )
        .unwrap();
    db.conn
        .lock()
        .execute(
            "UPDATE clips SET created_at = '2026-08-17T00:15:00Z' WHERE id = ?1",
            [clip.id],
        )
        .unwrap();

    let conn = db.conn.lock();
    let west =
        DbState::get_daily_activity_for_calendar(&conn, "2026-08-17T00:30:00Z", "-05:00").unwrap();
    assert_eq!(west[0].date, "2026-08-16");
    assert_eq!(west[0].count, 1);

    let east =
        DbState::get_daily_activity_for_calendar(&conn, "2026-08-17T00:30:00Z", "+05:30").unwrap();
    assert_eq!(east[0].date, "2026-08-17");
    assert_eq!(east[0].count, 1);
}
