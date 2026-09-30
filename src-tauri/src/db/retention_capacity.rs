use rusqlite::{Connection, Result};

use super::DbState;

impl DbState {
    pub(super) fn effective_history_capacity(
        &self,
        conn: &Connection,
        requested: i64,
    ) -> Result<i64> {
        if requested == 0 {
            return Ok(0);
        }
        let (exempt, active): (i64, i64) = conn.query_row(
            "SELECT
                COALESCE(SUM(CASE WHEN is_pinned = 1 OR id IN (
                    SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1
                ) THEN 1 ELSE 0 END), 0),
                COUNT(*)
             FROM clips WHERE COALESCE(is_trashed, 0) = 0",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if exempt < requested {
            return Ok(requested);
        }
        // Keep one slot for a new ordinary clip when protected clips fill the limit.
        let expanded = exempt.saturating_add((active > exempt) as i64);
        if expanded <= requested {
            return Ok(requested);
        }
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('keepClipCount', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [expanded.to_string()],
        )?;
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('historyLimitAutoExpanded', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [expanded.to_string()],
        )?;
        let attributes = serde_json::json!({
            "previousLimit": requested,
            "newLimit": expanded,
            "pinnedOrProtectedCount": exempt,
        });
        let _ = self.log_activity_internal_with_attributes(
            conn,
            "setting_history_limit_expanded",
            &format!("Increased History limit from {requested} to {expanded} because pinned or protected clips filled it"),
            &attributes.to_string(),
        );
        Ok(expanded)
    }
}
