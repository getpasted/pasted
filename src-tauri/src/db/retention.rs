use rusqlite::{params, Result};

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
                    ) WHERE revision_rank > ?1
                 )",
                params![keep_count],
            )?;
        }
        tx.commit()
    }
}
