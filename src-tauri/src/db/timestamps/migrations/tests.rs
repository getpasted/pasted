use super::*;

mod registry;

fn analysis_transform_timestamp_fixture() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE pipelines (created_at TEXT, updated_at TEXT);
         CREATE TABLE saved_transforms (id TEXT, created_at TEXT, updated_at TEXT);
         CREATE TABLE clip_transformations (created_at TEXT);
         CREATE TABLE transformation_executions (started_at TEXT, completed_at TEXT);
         CREATE TABLE clip_analysis_classifications (updated_at TEXT);
         CREATE TABLE clip_analysis_results (updated_at TEXT);
         CREATE TABLE clip_extraction_attempts (run_at TEXT);
         CREATE TABLE clip_searchable_text (updated_at TEXT);",
    )
    .unwrap();
    conn
}

#[test]
fn migration_normalizes_every_analysis_and_transform_timestamp() {
    let conn = analysis_transform_timestamp_fixture();
    conn.execute_batch(
        "INSERT INTO pipelines VALUES
            ('2026-08-16 23:45:00', '2026-08-16T18:45:00-05:00');
         INSERT INTO saved_transforms VALUES
            ('later', '2026-08-16 23:45:00', '2026-08-16T20:00:00-05:00'),
            ('earlier', '2026-08-16T23:45:00Z', '2026-08-17T00:30:00Z');
         INSERT INTO clip_transformations VALUES ('2026-08-17T00:45:00+01:00');
         INSERT INTO transformation_executions VALUES
            ('2026-08-16 23:45:00', '2026-08-16T18:45:00-05:00');
         INSERT INTO clip_analysis_classifications VALUES ('2026-08-16 23:45:00');
         INSERT INTO clip_analysis_results VALUES ('2026-08-16T18:45:00-05:00');
         INSERT INTO clip_extraction_attempts VALUES ('2026-08-17T00:45:00+01:00');
         INSERT INTO clip_searchable_text VALUES ('2026-08-16 23:45:00');",
    )
    .unwrap();

    migrate_analysis_transform_timestamps(&conn).unwrap();

    for (table, column) in [
        ("pipelines", "created_at"),
        ("pipelines", "updated_at"),
        ("saved_transforms", "created_at"),
        ("clip_transformations", "created_at"),
        ("transformation_executions", "started_at"),
        ("transformation_executions", "completed_at"),
        ("clip_analysis_classifications", "updated_at"),
        ("clip_analysis_results", "updated_at"),
        ("clip_extraction_attempts", "run_at"),
        ("clip_searchable_text", "updated_at"),
    ] {
        let values = conn
            .prepare(&format!("SELECT {column} FROM {table}"))
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap();
        assert!(values.iter().all(|value| {
            value.ends_with('Z') && canonical_utc_timestamp(value, "Test").unwrap() == *value
        }));
    }

    let ordered = conn
        .prepare("SELECT id FROM saved_transforms ORDER BY updated_at")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>>>()
        .unwrap();
    assert_eq!(ordered, ["earlier", "later"]);
}

#[test]
fn malformed_timestamp_rolls_back_the_entire_migration() {
    let conn = analysis_transform_timestamp_fixture();
    conn.execute_batch(
        "INSERT INTO saved_transforms VALUES
            ('transform', '2026-08-16 23:45:00', '2026-08-16 23:46:00');
         INSERT INTO clip_analysis_results VALUES ('not-a-timestamp');",
    )
    .unwrap();

    assert!(migrate_analysis_transform_timestamps(&conn).is_err());
    let timestamps: (String, String) = conn
        .query_row(
            "SELECT created_at, updated_at FROM saved_transforms",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        timestamps,
        ("2026-08-16 23:45:00".into(), "2026-08-16 23:46:00".into())
    );
}
