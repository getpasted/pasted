use rusqlite::{params, Connection, Result};

use super::DbState;

impl DbState {
    pub fn enforce_revision_retention(&self, keep_count: i64) -> Result<()> {
        let keep_count = keep_count.max(0);
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES ('revisionHistoryLimit', ?1)
             ON CONFLICT(key) DO UPDATE SET value = ?1",
            params![keep_count.to_string()],
        )?;
        if keep_count > 0 {
            tx.execute(
                "DELETE FROM clip_versions WHERE id IN (
                    SELECT id FROM (
                        SELECT id,
                               ROW_NUMBER() OVER (PARTITION BY clip_id ORDER BY id DESC) AS revision_rank
                        FROM clip_versions
                        WHERE COALESCE(json_extract(context_json, '$.action_kind'), '') != 'original'
                          AND id != (SELECT MIN(original.id) FROM clip_versions original
                                    WHERE original.clip_id = clip_versions.clip_id)
                    ) WHERE revision_rank > ?1
                 )",
                params![keep_count],
            )?;
        }
        tx.commit()
    }

    pub(super) fn revision_history_limit_internal(conn: &Connection) -> i64 {
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'revisionHistoryLimit'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(10)
        .max(0)
    }

    pub(super) fn revision_history_enabled_internal(conn: &Connection) -> bool {
        let value = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'enableRevisions'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok();
        crate::features::setting_value_is_enabled(value.as_deref())
    }

    pub(super) fn prune_clip_versions_internal(conn: &Connection, clip_id: i64) -> Result<()> {
        let limit = Self::revision_history_limit_internal(conn);
        if limit == 0 {
            return Ok(());
        }
        conn.execute(
            "DELETE FROM clip_versions
             WHERE clip_id = ?1
               AND COALESCE(json_extract(context_json, '$.action_kind'), '') != 'original'
               AND id != (SELECT MIN(original.id) FROM clip_versions original
                          WHERE original.clip_id = ?1)
               AND id NOT IN (
                SELECT id FROM clip_versions
                WHERE clip_id = ?1
                  AND COALESCE(json_extract(context_json, '$.action_kind'), '') != 'original'
                  AND id != (SELECT MIN(original.id) FROM clip_versions original
                             WHERE original.clip_id = ?1)
                ORDER BY id DESC LIMIT ?2
             )",
            params![clip_id, limit],
        )?;
        Ok(())
    }
}
