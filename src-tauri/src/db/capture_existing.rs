use rusqlite::{params, Connection, Result};

use super::{ClipItem, DbState};

impl DbState {
    pub(super) fn refresh_existing_clip(&self, conn: &Connection, id: i64) -> Result<ClipItem> {
        conn.execute(
            "UPDATE clips SET created_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), is_trashed = 0, trashed_at = NULL WHERE id = ?1",
            params![id],
        )?;
        let _ = self.enforce_history_limit_internal(conn, id);
        self.get_clip_by_id_internal(conn, id)
    }
}
