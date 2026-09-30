use rusqlite::Result;

use super::DbState;

impl DbState {
    pub fn configure_clip_retention(&self, keep_count: i64, keep_age_days: i64) -> Result<()> {
        let keep_count = keep_count.clamp(0, 100_000);
        let keep_age_days = keep_age_days.clamp(0, 36_500);
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let requested = keep_count.to_string();
        let previous: Option<String> = tx
            .query_row(
                "SELECT value FROM settings WHERE key = 'keepClipCount'",
                [],
                |row| row.get(0),
            )
            .ok();
        let previous_age: Option<String> = tx
            .query_row(
                "SELECT value FROM settings WHERE key = 'keepClipAgeDays'",
                [],
                |row| row.get(0),
            )
            .ok();
        if previous.as_deref() != Some(requested.as_str()) {
            tx.execute(
                "DELETE FROM settings WHERE key = 'historyLimitAutoExpanded'",
                [],
            )?;
        }
        tx.execute(
            "INSERT INTO settings (key, value) VALUES ('keepClipCount', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [requested],
        )?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES ('keepClipAgeDays', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [keep_age_days.to_string()],
        )?;
        self.enforce_clip_retention_internal(&tx, keep_count, keep_age_days, None)?;
        let applied: String = tx.query_row(
            "SELECT value FROM settings WHERE key = 'keepClipCount'",
            [],
            |row| row.get(0),
        )?;
        tx.commit()?;
        drop(conn);
        for (key, before, after) in [
            ("keepClipCount", previous.as_deref(), applied),
            (
                "keepClipAgeDays",
                previous_age.as_deref(),
                keep_age_days.to_string(),
            ),
        ] {
            if before == Some(after.as_str()) {
                continue;
            }
            if let Some(activity) =
                crate::settings_activity::describe_setting_change(key, before, &after)
            {
                let _ = self.log_activity(activity.event_type, &activity.description);
            }
        }
        Ok(())
    }
}
