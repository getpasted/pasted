use super::*;

pub(super) fn initialize_clip_schema(conn: &Connection) -> Result<()> {
    // High-performance SQLite configuration
    conn.pragma_update(None, "foreign_keys", "ON")?;
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    let _ = conn.pragma_update(None, "synchronous", "NORMAL");
    let _ = conn.pragma_update(None, "temp_store", "MEMORY");
    let _ = conn.pragma_update(None, "wal_autocheckpoint", "500");
    let _ = conn.pragma_update(None, "auto_vacuum", "INCREMENTAL");
    let _ = conn.pragma_update(None, "optimize", "");

    migrate_legacy_container_schema(conn)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS clips (
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
            created_at DATETIME DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
        )",
        [],
    )?;
    crate::file_reference_health::create_file_reference_health_table(conn)?;
    // High-speed composite indexes
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_clips_pinned_created ON clips (is_pinned, created_at DESC)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_clips_bin_created ON clips (bin_id, created_at DESC)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_clips_hash ON clips (content_hash)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS bins (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            icon TEXT DEFAULT 'Folder',
            color TEXT DEFAULT 'default',
            smart_rule TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Every additive migration distinguishes an existing column from a real
    // SQLite failure. Never discard ALTER TABLE errors during startup.
    add_column_if_missing(conn, "clips", "note", "TEXT")?;
    add_column_if_missing(conn, "clips", "name", "TEXT")?;
    add_column_if_missing(conn, "clips", "is_trashed", "INTEGER DEFAULT 0")?;
    add_column_if_missing(conn, "clips", "trashed_at", "DATETIME")?;
    add_column_if_missing(conn, "clips", "is_protected", "INTEGER DEFAULT 0")?;
    add_column_if_missing(conn, "clips", "is_concealed", "INTEGER NOT NULL DEFAULT 0")?;
    add_column_if_missing(conn, "clips", "is_revealed", "INTEGER NOT NULL DEFAULT 0")?;
    add_column_if_missing(conn, "clips", "shortcut", "TEXT")?;
    add_column_if_missing(conn, "clips", "image_path", "TEXT")?;
    add_column_if_missing(conn, "clips", "pin_order", "INTEGER DEFAULT 0")?;
    add_column_if_missing(conn, "clips", "current_transformation_id", "TEXT")?;
    add_column_if_missing(
        conn,
        "clips",
        "ocr_status",
        "TEXT NOT NULL DEFAULT 'not_applicable'",
    )?;
    add_column_if_missing(conn, "clips", "ocr_input_hash", "TEXT")?;
    add_column_if_missing(conn, "clips", "ocr_engine_version", "TEXT")?;
    add_column_if_missing(conn, "clips", "ocr_extractor_ref", "TEXT")?;
    add_column_if_missing(conn, "clips", "ocr_extractor_name", "TEXT")?;
    add_column_if_missing(conn, "clips", "ocr_attempted_at", "DATETIME")?;
    add_column_if_missing(conn, "clips", "ocr_error", "TEXT")?;
    conn.execute(
        "UPDATE clips
         SET ocr_status = CASE
                WHEN content_type = 'image' AND COALESCE(text_content, '') != '' THEN 'complete'
                WHEN content_type = 'image' THEN 'never'
                ELSE 'not_applicable'
             END,
             ocr_input_hash = CASE WHEN content_type = 'image' THEN content_hash ELSE NULL END,
             ocr_engine_version = CASE
                WHEN content_type = 'image' AND COALESCE(text_content, '') != '' THEN COALESCE(ocr_engine_version, 'legacy')
                ELSE ocr_engine_version
             END
         WHERE content_type = 'image' AND ocr_input_hash IS NULL",
        [],
    )?;
    conn.execute(
        "UPDATE clips
         SET ocr_extractor_ref = CASE ocr_engine_version
                WHEN 'macos-vision-v1' THEN 'extractor:apple-vision-ocr'
                ELSE ocr_extractor_ref
             END,
             ocr_extractor_name = CASE ocr_engine_version
                WHEN 'macos-vision-v1' THEN 'Apple Vision OCR'
                WHEN 'legacy' THEN 'Legacy OCR'
                ELSE ocr_extractor_name
             END
         WHERE ocr_status = 'complete' AND ocr_extractor_name IS NULL",
        [],
    )?;
    conn.execute(
        "UPDATE clips SET ocr_status = 'never', ocr_error = NULL
         WHERE content_type = 'image' AND ocr_status IN ('queued', 'running')",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_clips_ocr_backfill
         ON clips (content_type, ocr_status, is_trashed, id)",
        [],
    )?;
    add_column_if_missing(conn, "bins", "smart_rule", "TEXT")?;
    add_column_if_missing(conn, "bins", "bin_type", "TEXT DEFAULT 'category'")?;
    add_column_if_missing(conn, "bins", "shortcut", "TEXT")?;
    add_column_if_missing(conn, "bins", "protect_clips", "INTEGER NOT NULL DEFAULT 0")?;
    add_column_if_missing(conn, "bins", "conceal_clips", "INTEGER NOT NULL DEFAULT 0")?;

    migrate_clip_source_schema(conn)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS clip_versions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
            text_content TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_clip_versions_clip_id ON clip_versions(clip_id, created_at DESC)",
        [],
    )?;
    add_column_if_missing(conn, "clip_versions", "context_json", "TEXT")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS clip_analysis_classifications (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
            content_type TEXT NOT NULL,
            classifier_ref TEXT NOT NULL,
            source_representation TEXT NOT NULL
                CHECK (source_representation IN ('original_text', 'searchable_text')),
            input_hash TEXT NOT NULL,
            start_offset INTEGER,
            end_offset INTEGER,
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
            CHECK (
                (start_offset IS NULL AND end_offset IS NULL)
                OR (start_offset >= 0 AND end_offset > start_offset)
            )
        )",
        [],
    )?;
    migrate_multi_type_classifications(conn)?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_clip_analysis_classification_type
         ON clip_analysis_classifications(content_type, clip_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_clip_analysis_classification_clip
         ON clip_analysis_classifications(clip_id, input_hash, classifier_ref, start_offset)",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS clip_analysis_results (
            clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
            participant_ref TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            input_hash TEXT NOT NULL,
            format_version INTEGER NOT NULL CHECK(format_version > 0),
            result_json TEXT NOT NULL,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (clip_id, participant_ref)
        )",
        [],
    )?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS clip_extraction_attempts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
            run_id TEXT NOT NULL,
            participant_ref TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            priority INTEGER NOT NULL,
            result_json TEXT NOT NULL,
            run_at TEXT NOT NULL
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
    conn.execute(
        "CREATE TABLE IF NOT EXISTS clip_searchable_text (
            clip_id INTEGER PRIMARY KEY REFERENCES clips(id) ON DELETE CASCADE,
            extractor_ref TEXT NOT NULL,
            extractor_name TEXT NOT NULL,
            engine TEXT NOT NULL,
            input_hash TEXT NOT NULL,
            searchable_text TEXT NOT NULL,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    let _ = conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_clips_trashed ON clips (is_trashed, created_at DESC)",
        [],
    );
    let _ = conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_clips_protected ON clips (is_protected, created_at DESC)",
        [],
    );
    let _ = conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_clips_named_created ON clips (created_at DESC)
         WHERE name IS NOT NULL AND TRIM(name) != ''",
        [],
    );
    let _ = conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_clips_shortcut ON clips (shortcut)",
        [],
    );
    let _ = conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_clips_active_timeline ON clips (is_trashed, is_pinned DESC, created_at DESC)",
        [],
    );

    search_indexes::ensure_search_indexes(conn);

    Ok(())
}
