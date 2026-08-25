use rusqlite::{Connection, Result};

pub(super) fn initialize(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS clip_visual_label_overrides (
            clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
            label TEXT NOT NULL COLLATE NOCASE,
            operation TEXT NOT NULL CHECK(operation IN ('add', 'suppress')),
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
            PRIMARY KEY (clip_id, label)
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_clip_visual_label_overrides_operation
         ON clip_visual_label_overrides(clip_id, operation)",
        [],
    )?;
    Ok(())
}
