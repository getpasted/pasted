use super::*;

pub(super) fn initialize_organization_schema(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS clip_bins (
            clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
            bin_id INTEGER NOT NULL REFERENCES bins(id) ON DELETE CASCADE,
            PRIMARY KEY (clip_id, bin_id)
        )",
        [],
    )?;
    let _ = conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_clip_bins_bin_id ON clip_bins (bin_id)",
        [],
    );
    let _ = conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_clip_bins_clip_id ON clip_bins (clip_id)",
        [],
    );

    // One shared contract protects clips from every cleanup and destructive path.
    // Smart-rule matches are intentionally excluded: only durable manual membership
    // can apply inherited protection.
    conn.execute_batch(
        "DROP VIEW IF EXISTS effective_clip_protection;
         CREATE VIEW effective_clip_protection AS
         SELECT clips.id AS clip_id,
                CASE WHEN COALESCE(clips.is_protected, 0) = 1
                          OR NULLIF(TRIM(clips.shortcut), '') IS NOT NULL
                          OR EXISTS (
                              SELECT 1 FROM bins
                              WHERE COALESCE(bins.protect_clips, 0) = 1
                                AND (bins.id = clips.bin_id OR EXISTS (
                                    SELECT 1 FROM clip_bins
                                    WHERE clip_bins.clip_id = clips.id
                                      AND clip_bins.bin_id = bins.id
                                ))
                          )
                     THEN 1 ELSE 0 END AS is_protected,
                (SELECT GROUP_CONCAT(protecting.id)
                 FROM bins AS protecting
                 WHERE COALESCE(protecting.protect_clips, 0) = 1
                   AND (protecting.id = clips.bin_id OR EXISTS (
                       SELECT 1 FROM clip_bins
                       WHERE clip_bins.clip_id = clips.id
                         AND clip_bins.bin_id = protecting.id
                   ))) AS protecting_bin_ids
         FROM clips;",
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS bin_clip_order (
            bin_id INTEGER NOT NULL REFERENCES bins(id) ON DELETE CASCADE,
            clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
            position INTEGER NOT NULL CHECK(position >= 0),
            PRIMARY KEY (bin_id, clip_id),
            UNIQUE (bin_id, position)
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_bin_clip_order_position
         ON bin_clip_order (bin_id, position)",
        [],
    )?;

    let _ = conn.execute(
        "INSERT OR IGNORE INTO clip_bins (clip_id, bin_id)
         SELECT id, bin_id FROM clips WHERE bin_id IS NOT NULL",
        [],
    );

    // Trash is deliberately outside the organizational hierarchy. Clean up
    // legacy rows so restored clips never silently reappear in an old Bin.
    let _ = conn.execute(
        "DELETE FROM clip_bins
         WHERE clip_id IN (SELECT id FROM clips WHERE is_trashed = 1)
           AND bin_id IN (
               SELECT id FROM bins WHERE COALESCE(bin_type, 'category') != 'tag'
           )",
        [],
    );
    let _ = conn.execute("UPDATE clips SET bin_id = NULL WHERE is_trashed = 1", []);

    let _ = conn.execute(
        "CREATE TABLE IF NOT EXISTS activity_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_type TEXT NOT NULL,
            description TEXT NOT NULL,
            created_at DATETIME DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
            observed_at DATETIME DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
            severity_text TEXT NOT NULL DEFAULT 'info',
            category TEXT NOT NULL DEFAULT 'general',
            outcome TEXT NOT NULL DEFAULT 'unknown',
            attributes_json TEXT NOT NULL DEFAULT '{}'
        )",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE activity_logs ADD COLUMN observed_at DATETIME",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE activity_logs ADD COLUMN severity_text TEXT NOT NULL DEFAULT 'info'",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE activity_logs ADD COLUMN category TEXT NOT NULL DEFAULT 'general'",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE activity_logs ADD COLUMN outcome TEXT NOT NULL DEFAULT 'unknown'",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE activity_logs ADD COLUMN attributes_json TEXT NOT NULL DEFAULT '{}'",
        [],
    );
    let _ = conn.execute(
        "UPDATE activity_logs SET observed_at = created_at WHERE observed_at IS NULL",
        [],
    );
    let _ = conn.execute(
        "UPDATE activity_logs
         SET severity_text = CASE
                WHEN event_type LIKE '%failed%' OR event_type LIKE '%error%' THEN 'error'
                WHEN event_type LIKE '%ignored%' OR event_type LIKE '%skipped%'
                  OR event_type LIKE '%cancelled%' OR event_type LIKE '%auto_paused%' THEN 'warn'
                ELSE severity_text
             END,
             category = CASE
                WHEN event_type LIKE 'clip_%' OR event_type LIKE 'clips_%'
                  OR event_type LIKE 'trash_%' OR event_type LIKE 'note_%' THEN 'clip'
                WHEN event_type LIKE 'recording_%' OR event_type LIKE 'clipboard_%' THEN 'capture'
                WHEN event_type LIKE 'bin_%' OR event_type LIKE 'type_%'
                  OR event_type LIKE 'classifier_%' OR event_type LIKE 'content_%' THEN 'organization'
                WHEN event_type LIKE 'transform%' OR event_type LIKE 'operation_%'
                  OR event_type LIKE 'intelligence_%' THEN 'transformation'
                WHEN event_type LIKE 'setting_%' OR event_type = 'settings_changed' THEN 'settings'
                WHEN event_type LIKE 'queue_%' OR event_type LIKE 'hud_%' THEN 'workflow'
                WHEN event_type LIKE 'app_%' OR event_type LIKE 'library_%'
                  OR event_type LIKE 'backup_%' OR event_type LIKE 'external_%' THEN 'system'
                ELSE category
             END,
             outcome = CASE
                WHEN event_type LIKE '%failed%' OR event_type LIKE '%error%' THEN 'failure'
                WHEN event_type LIKE '%succeeded%' OR event_type LIKE '%_completed' THEN 'success'
                ELSE outcome
             END",
        [],
    );
    let _ = conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_logs_created ON activity_logs (created_at DESC)",
        [],
    );

    Ok(())
}
