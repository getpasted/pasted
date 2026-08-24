use super::*;

pub(super) fn initialize_extraction_attempt_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS clip_extraction_attempts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
            run_id TEXT NOT NULL,
            participant_ref TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            priority INTEGER NOT NULL,
            result_json TEXT NOT NULL,
            run_at TEXT NOT NULL,
            input_fingerprint TEXT NOT NULL DEFAULT '',
            failure_class TEXT CHECK (
                failure_class IS NULL OR failure_class IN ('terminal', 'dependency', 'transient')
            ),
            retry_after TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_clip_extraction_attempts_history
            ON clip_extraction_attempts (clip_id, run_at DESC, id DESC, priority, participant_ref);",
    )?;
    conn.execute(
        "INSERT INTO clip_extraction_attempts
            (clip_id, run_id, participant_ref, content_hash, priority, result_json, run_at)
         SELECT results.clip_id,
                'migrated-' || results.clip_id,
                results.participant_ref,
                results.content_hash,
                CAST(json_extract(results.result_json, '$.priority') AS INTEGER),
                results.result_json,
                COALESCE(
                    strftime('%Y-%m-%dT%H:%M:%SZ', results.updated_at),
                    strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                )
         FROM clip_analysis_results AS results
         WHERE results.participant_ref LIKE 'extractor:%'
           AND NOT EXISTS (
                SELECT 1 FROM clip_extraction_attempts AS attempts
                WHERE attempts.clip_id = results.clip_id
           )",
        [],
    )?;
    add_column_if_missing(
        conn,
        "clip_extraction_attempts",
        "input_fingerprint",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    add_column_if_missing(conn, "clip_extraction_attempts", "failure_class", "TEXT")?;
    add_column_if_missing(conn, "clip_extraction_attempts", "retry_after", "TEXT")?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_clip_extraction_attempts_reuse
         ON clip_extraction_attempts
            (clip_id, participant_ref, input_fingerprint, id DESC)",
        [],
    )?;
    Ok(())
}
