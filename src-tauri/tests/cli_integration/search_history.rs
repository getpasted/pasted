use super::super::support::{clean_database, run, success_json, temporary_path};
use rusqlite::Connection;

#[test]
fn search_history_records_deduplicates_pages_and_supports_management() {
    let database = temporary_path("search-history", "db");
    success_json(&database, &["copy", "alpha example", "--json"]);
    success_json(&database, &["copy", "beta example", "--json"]);

    success_json(&database, &["search", "example", "--limit", "1", "--json"]);
    success_json(
        &database,
        &[
            "search", "example", "--limit", "1", "--offset", "1", "--json",
        ],
    );
    success_json(&database, &["search", "example", "--limit", "1", "--json"]);

    let page = success_json(
        &database,
        &["search-history", "list", "--limit", "10", "--json"],
    );
    assert_eq!(page["totalCount"], 1);
    assert_eq!(page["items"][0]["request"]["query"], "example");
    assert_eq!(page["items"][0]["resultCount"], 2);
    assert_eq!(page["items"][0]["useCount"], 2);

    let id = page["items"][0]["id"].as_i64().expect("history ID");
    let deleted = success_json(
        &database,
        &["search-history", "delete", &id.to_string(), "--json"],
    );
    assert_eq!(deleted["id"], id);
    assert_eq!(deleted["deleted"], true);
    assert_eq!(
        success_json(&database, &["search-history", "list", "--json"])["totalCount"],
        0
    );

    success_json(&database, &["search", "alpha", "--json"]);
    success_json(&database, &["search", "beta", "--json"]);
    let missing_confirmation = run(&database, &["search-history", "clear", "--json"]);
    assert_eq!(missing_confirmation.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&missing_confirmation.stderr).contains("permanent"));
    let cleared = success_json(&database, &["search-history", "clear", "--yes", "--json"]);
    assert_eq!(cleared["clearedCount"], 2);
    clean_database(&database);
}

#[test]
fn retention_configures_and_prunes_search_history() {
    let database = temporary_path("search-history-retention", "db");
    success_json(&database, &["copy", "alpha beta", "--json"]);
    success_json(&database, &["search", "alpha", "--json"]);
    success_json(&database, &["search", "beta", "--json"]);

    let connection = Connection::open(&database).expect("open Search history database");
    connection
        .execute(
            "UPDATE search_history SET last_used_at = '2000-01-01T00:00:00Z' WHERE request_json LIKE '%alpha%'",
            [],
        )
        .expect("age one Search history entry");
    drop(connection);

    let retention = success_json(
        &database,
        &[
            "retention",
            "--search-count",
            "10",
            "--search-days",
            "1",
            "--json",
        ],
    );
    assert_eq!(retention["searchHistoryMaximumEntries"], 10);
    assert_eq!(retention["searchHistoryMaximumEntriesUnlimited"], false);
    assert_eq!(retention["searchHistoryMaximumAgeDays"], 1);
    assert_eq!(retention["searchHistoryMaximumAgeForever"], false);
    let page = success_json(&database, &["search-history", "list", "--json"]);
    assert_eq!(page["totalCount"], 1);
    assert_eq!(page["items"][0]["request"]["query"], "beta");

    let invalid_age = run(
        &database,
        &["retention", "--search-days", "36501", "--json"],
    );
    assert_eq!(invalid_age.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&invalid_age.stderr)
        .contains("--search-days must be forever or a number from 0 to 36500"));
    clean_database(&database);
}
