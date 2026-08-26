use super::*;

pub(super) fn initialize_search_history_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS search_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            canonical_request_json TEXT NOT NULL UNIQUE,
            request_json TEXT NOT NULL,
            result_count INTEGER NOT NULL CHECK(result_count >= 0),
            use_count INTEGER NOT NULL DEFAULT 1 CHECK(use_count > 0),
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
            last_used_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
        );
        CREATE INDEX IF NOT EXISTS idx_search_history_last_used
            ON search_history (last_used_at DESC, id DESC);",
    )?;
    Ok(())
}
