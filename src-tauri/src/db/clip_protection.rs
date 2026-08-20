use rusqlite::{params, Result};

use super::{describe_clip_ids, ClipMutationSummary, DbState};

impl DbState {
    pub fn toggle_protected(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock();
        let current_protected: i32 = conn
            .query_row(
                "SELECT is_protected FROM clips WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        drop(conn);
        let new_protected = current_protected == 0;
        self.batch_protect_clips(vec![id], new_protected)?;
        Ok(new_protected)
    }

    pub fn batch_protect_clips(
        &self,
        ids: Vec<i64>,
        protected_state: bool,
    ) -> Result<ClipMutationSummary> {
        let requested_count = ids.len();
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        if !protected_state && !ids.is_empty() {
            let ids_json = serde_json::to_string(&ids)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            let shortcut_count: i64 = tx.query_row(
                "SELECT COUNT(*) FROM clips
                 WHERE id IN (SELECT CAST(value AS INTEGER) FROM json_each(?1))
                   AND NULLIF(TRIM(shortcut), '') IS NOT NULL",
                params![ids_json],
                |row| row.get(0),
            )?;
            if shortcut_count > 0 {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Remove the clip hotkey before removing explicit protection".into(),
                ));
            }
            let protecting_bin_count: i64 = tx.query_row(
                "SELECT COUNT(*) FROM effective_clip_protection
                 WHERE clip_id IN (SELECT CAST(value AS INTEGER) FROM json_each(?1))
                   AND NULLIF(TRIM(protecting_bin_ids), '') IS NOT NULL",
                params![ids_json],
                |row| row.get(0),
            )?;
            if protecting_bin_count > 0 {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Remove the clip from protecting Bins before removing explicit protection"
                        .into(),
                ));
            }
        }
        let mut changed_ids = Vec::new();
        for id in ids {
            let changed = tx.execute(
                "UPDATE clips SET is_protected = ?1
                 WHERE id = ?2 AND COALESCE(is_protected, 0) != ?1",
                params![if protected_state { 1 } else { 0 }, id],
            )?;
            if changed > 0 {
                changed_ids.push(id);
            }
        }
        tx.commit()?;
        if !changed_ids.is_empty() {
            let event_type = if changed_ids.len() == 1 {
                "clip_protected_toggled"
            } else {
                "clips_protected_toggled"
            };
            let verb = if protected_state {
                "Protected"
            } else {
                "Unprotected"
            };
            let _ = self.log_activity_internal(
                &conn,
                event_type,
                &format!("{} {}", verb, describe_clip_ids(&changed_ids)),
            );
        }
        Ok(ClipMutationSummary::new(
            if protected_state {
                "protect"
            } else {
                "unprotect"
            },
            requested_count,
            changed_ids,
        ))
    }
}
