use rusqlite::{params, Connection, Result};

use super::DbState;

impl DbState {
    pub fn configure_clip_retention(&self, keep_count: i64, keep_age_days: i64) -> Result<()> {
        let keep_count = keep_count.clamp(0, 100_000);
        let keep_age_days = keep_age_days.clamp(0, 36_500);
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES ('keepClipCount', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [keep_count.to_string()],
        )?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES ('keepClipAgeDays', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [keep_age_days.to_string()],
        )?;
        self.enforce_clip_retention_internal(&tx, keep_count, keep_age_days)?;
        tx.commit()
    }

    pub fn enforce_clip_retention(&self, keep_count: i64, keep_age_days: i64) -> Result<()> {
        let conn = self.conn.lock();
        self.enforce_clip_retention_internal(
            &conn,
            keep_count.clamp(0, 100_000),
            keep_age_days.clamp(0, 36_500),
        )
    }

    pub fn configure_trash_retention(&self, keep_count: i64, keep_age_days: i64) -> Result<()> {
        let keep_count = keep_count.clamp(0, 100_000);
        let keep_age_days = keep_age_days.clamp(0, 36_500);
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES ('trashCapacityCount', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [keep_count.to_string()],
        )?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES ('trashAgeDays', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [keep_age_days.to_string()],
        )?;
        self.enforce_trash_retention_internal(&tx, keep_count, keep_age_days)?;
        tx.commit()
    }

    pub fn enforce_trash_retention(&self, keep_count: i64, keep_age_days: i64) -> Result<()> {
        let conn = self.conn.lock();
        self.enforce_trash_retention_internal(
            &conn,
            keep_count.clamp(0, 100_000),
            keep_age_days.clamp(0, 36_500),
        )
    }

    pub fn configure_activity_retention(&self, keep_count: i64, keep_age_days: i64) -> Result<()> {
        let keep_count = keep_count.clamp(0, 100_000);
        let keep_age_days = keep_age_days.clamp(0, 36_500);
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES ('activityLogCapacity', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [keep_count.to_string()],
        )?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES ('activityLogAgeDays', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [keep_age_days.to_string()],
        )?;
        self.enforce_activity_retention_internal(&tx, keep_count, keep_age_days)?;
        tx.commit()
    }

    pub fn enforce_activity_retention(&self, keep_count: i64, keep_age_days: i64) -> Result<()> {
        let conn = self.conn.lock();
        self.enforce_activity_retention_internal(
            &conn,
            keep_count.clamp(0, 100_000),
            keep_age_days.clamp(0, 36_500),
        )
    }

    pub fn purge_old_clips(&self, keep_count: i64) -> Result<()> {
        self.enforce_clip_retention(keep_count, 0)
    }

    pub fn enforce_history_limit_internal(&self, conn: &Connection) -> Result<()> {
        let keep_count: i64 = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'keepClipCount'",
                [],
                |r| r.get(0),
            )
            .ok()
            .and_then(|v: String| v.parse().ok())
            .unwrap_or(1000);
        let keep_age_days: i64 = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'keepClipAgeDays'",
                [],
                |r| r.get(0),
            )
            .ok()
            .and_then(|v: String| v.parse().ok())
            .unwrap_or(0);

        self.enforce_clip_retention_internal(conn, keep_count, keep_age_days)
    }

    fn enforce_clip_retention_internal(
        &self,
        conn: &Connection,
        keep_count: i64,
        keep_age_days: i64,
    ) -> Result<()> {
        let keep_count = keep_count.max(0);
        let keep_age_days = keep_age_days.max(0);

        let enable_trash: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'enableTrash'",
                [],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "true".to_string());

        let mut ids = Vec::new();
        if keep_age_days > 0 {
            let age_modifier = format!("-{keep_age_days} days");
            let mut stmt = conn.prepare(
                "SELECT id FROM clips
                 WHERE is_pinned = 0
                   AND clips.id NOT IN (SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1)
                   AND (is_trashed IS NULL OR is_trashed = 0)
                   AND datetime(created_at) < datetime('now', ?1)
                 ORDER BY created_at ASC, id ASC",
            )?;
            ids.extend(
                stmt.query_map([age_modifier], |r| r.get::<_, i64>(0))?
                    .filter_map(|r| r.ok()),
            );
        }

        if keep_count > 0 {
            let active_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM clips
                     WHERE is_pinned = 0
                       AND clips.id NOT IN (SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1)
                       AND (is_trashed IS NULL OR is_trashed = 0)",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            let excess = active_count.saturating_sub(keep_count);
            if excess > 0 {
                let mut stmt = conn.prepare(
                    "SELECT id FROM clips
                     WHERE is_pinned = 0
                       AND clips.id NOT IN (SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1)
                       AND (is_trashed IS NULL OR is_trashed = 0)
                     ORDER BY created_at ASC, id ASC LIMIT ?1",
                )?;
                ids.extend(
                    stmt.query_map(params![excess], |r| r.get::<_, i64>(0))?
                        .filter_map(|r| r.ok()),
                );
            }
        }

        ids.sort_unstable();
        ids.dedup();
        for id in ids {
            if enable_trash == "true" {
                let changed = conn.execute(
                        "UPDATE clips SET is_trashed = 1, trashed_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?1",
                        params![id],
                    ).unwrap_or(0);
                if changed > 0 {
                    let _ = self.clear_category_bin_assignments_internal(conn, id);
                }
                let _ = self.log_activity_internal(
                    conn,
                    "clip_auto_trashed",
                    &format!(
                        "Auto-trashed clip #{} (history retention policy exceeded)",
                        id
                    ),
                );
            } else {
                let _ = conn.execute("DELETE FROM clips WHERE id = ?1", params![id]);
                let _ = self.log_activity_internal(
                    conn,
                    "clip_deleted",
                    &format!(
                        "Auto-purged clip #{} (history retention policy exceeded)",
                        id
                    ),
                );
            }
        }
        Ok(())
    }

    pub fn enforce_trash_limit_internal(&self, conn: &Connection) -> Result<()> {
        let capacity: i64 = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'trashCapacityCount'",
                [],
                |r| r.get(0),
            )
            .ok()
            .and_then(|v: String| v.parse().ok())
            .unwrap_or(500);
        let keep_age_days: i64 = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'trashAgeDays'",
                [],
                |r| r.get(0),
            )
            .ok()
            .and_then(|v: String| v.parse().ok())
            .unwrap_or(0);

        self.enforce_trash_retention_internal(conn, capacity, keep_age_days)
    }

    fn enforce_trash_retention_internal(
        &self,
        conn: &Connection,
        keep_count: i64,
        keep_age_days: i64,
    ) -> Result<()> {
        let keep_count = keep_count.max(0);
        let keep_age_days = keep_age_days.max(0);

        if keep_age_days > 0 {
            let age_modifier = format!("-{keep_age_days} days");
            conn.execute(
                "DELETE FROM clips
                 WHERE is_trashed = 1
                   AND clips.id NOT IN (SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1)
                   AND datetime(COALESCE(trashed_at, created_at)) < datetime('now', ?1)",
                [age_modifier],
            )?;
        }

        if keep_count > 0 {
            conn.execute(
                "DELETE FROM clips
                 WHERE is_trashed = 1
                   AND clips.id NOT IN (SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1)
                   AND id NOT IN (
                       SELECT id FROM clips
                       WHERE is_trashed = 1 AND clips.id NOT IN (SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1)
                       ORDER BY COALESCE(trashed_at, created_at) DESC, id DESC LIMIT ?1
                   )",
                params![keep_count],
            )?;
        }
        Ok(())
    }
}
