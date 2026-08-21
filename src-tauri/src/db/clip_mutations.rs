use std::collections::HashSet;

use rusqlite::{params, Connection, OptionalExtension, Result};

use super::{
    describe_clip_ids, ensure_resource_size, ClipMutationSummary, ClipRevisionContext, DbState,
};

impl DbState {
    pub fn update_clip_note(&self, clip_id: i64, note: Option<&str>) -> Result<()> {
        if let Some(note) = note {
            ensure_resource_size(
                note,
                crate::resource_limits::MAX_CLIP_NOTE_BYTES,
                "Clip note",
            )?;
        }
        let conn = self.conn.lock();
        let mut stmt = conn.prepare_cached(
            "UPDATE clips SET note = ?1
             WHERE id = ?2 AND (is_trashed IS NULL OR is_trashed = 0)",
        )?;
        let changed = stmt.execute(params![note, clip_id])?;
        if changed == 0 {
            let exists = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM clips WHERE id = ?1)",
                [clip_id],
                |row| row.get::<_, bool>(0),
            )?;
            return if exists {
                Ok(())
            } else {
                Err(rusqlite::Error::QueryReturnedNoRows)
            };
        }
        let _ = self.log_activity_internal(
            &conn,
            "note_updated",
            &format!("Updated note for clip #{}", clip_id),
        );
        Ok(())
    }

    pub(super) fn revision_history_limit_internal(conn: &Connection) -> i64 {
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'revisionHistoryLimit'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(50)
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
             WHERE clip_id = ?1 AND id NOT IN (
                SELECT id FROM clip_versions
                WHERE clip_id = ?1 ORDER BY id DESC LIMIT ?2
             )",
            params![clip_id, limit],
        )?;
        Ok(())
    }

    pub fn update_clip_text(&self, clip_id: i64, text: &str) -> Result<()> {
        ensure_resource_size(
            text,
            crate::resource_limits::MAX_CLIP_TEXT_BYTES,
            "Clip text",
        )?;
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let (previous_text, is_trashed, current_transformation_id): (
            Option<String>,
            i32,
            Option<String>,
        ) = tx.query_row(
            "SELECT text_content, COALESCE(is_trashed, 0), current_transformation_id FROM clips WHERE id = ?1",
            params![clip_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;

        if is_trashed != 0 {
            return tx.commit();
        }

        if previous_text.as_deref() == Some(text) {
            return tx.commit();
        }

        if Self::revision_history_enabled_internal(&tx) {
            if let Some(previous_text) = previous_text {
                let context_json = serde_json::to_string(&ClipRevisionContext {
                    schema_version: 1,
                    action_kind: "edit".to_string(),
                    action_label: "Edited clip content".to_string(),
                    organization: None,
                    current_transformation_id,
                })
                .map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))?;
                tx.execute(
                "INSERT INTO clip_versions (clip_id, text_content, context_json) VALUES (?1, ?2, ?3)",
                params![clip_id, previous_text, context_json],
            )?;
                Self::prune_clip_versions_internal(&tx, clip_id)?;
            }
        }
        tx.execute(
            "UPDATE clips SET text_content = ?1, current_transformation_id = NULL WHERE id = ?2",
            params![text, clip_id],
        )?;
        tx.commit()
    }

    pub(super) fn clear_category_bin_assignments_internal(
        &self,
        conn: &Connection,
        clip_id: i64,
    ) -> Result<()> {
        conn.execute(
            "DELETE FROM clip_bins
             WHERE clip_id = ?1
               AND bin_id IN (
                   SELECT id FROM bins WHERE COALESCE(bin_type, 'category') != 'tag'
               )",
            params![clip_id],
        )?;
        conn.execute(
            "UPDATE clips SET bin_id = NULL WHERE id = ?1",
            params![clip_id],
        )?;
        Ok(())
    }

    pub fn delete_clip(&self, id: i64) -> Result<ClipMutationSummary> {
        self.batch_trash_clips(vec![id])
    }

    pub fn batch_trash_clips(&self, ids: Vec<i64>) -> Result<ClipMutationSummary> {
        let requested_count = ids.len();
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let mut changed_ids = Vec::new();
        for id in ids {
            let changed = tx.execute(
                "UPDATE clips SET is_trashed = 1, trashed_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 WHERE id = ?1
                   AND clips.id NOT IN (SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1)
                   AND (is_trashed IS NULL OR is_trashed = 0)",
                params![id],
            )?;
            if changed > 0 {
                self.clear_category_bin_assignments_internal(&tx, id)?;
                changed_ids.push(id);
            }
        }
        if !changed_ids.is_empty() {
            self.enforce_trash_limit_internal(&tx)?;
        }
        tx.commit()?;
        if !changed_ids.is_empty() {
            let event_type = if changed_ids.len() == 1 {
                "clip_trashed"
            } else {
                "clips_trashed"
            };
            let _ = self.log_activity_internal(
                &conn,
                event_type,
                &format!("Moved {} to Trash", describe_clip_ids(&changed_ids)),
            );
        }
        Ok(ClipMutationSummary::new(
            "trash",
            requested_count,
            changed_ids,
        ))
    }

    pub fn restore_clip(&self, id: i64) -> Result<ClipMutationSummary> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare_cached(
            "UPDATE clips SET is_trashed = 0, trashed_at = NULL WHERE id = ?1 AND is_trashed = 1",
        )?;
        let changed = stmt.execute(params![id])?;
        if changed > 0 {
            let _ = self.log_activity_internal(
                &conn,
                "clip_restored",
                &format!("Restored clip #{} from Trash", id),
            );
        }
        Ok(ClipMutationSummary::new(
            "restore",
            1,
            if changed > 0 { vec![id] } else { Vec::new() },
        ))
    }

    pub fn restore_all_trashed_clips(&self) -> Result<ClipMutationSummary> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let clip_ids = {
            let mut stmt =
                tx.prepare_cached("SELECT id FROM clips WHERE is_trashed = 1 ORDER BY id ASC")?;
            let rows = stmt
                .query_map([], |row| row.get::<_, i64>(0))?
                .collect::<Result<Vec<_>>>()?;
            rows
        };
        let requested_count = clip_ids.len();
        if !clip_ids.is_empty() {
            tx.execute(
                "UPDATE clips SET is_trashed = 0, trashed_at = NULL WHERE is_trashed = 1",
                [],
            )?;
        }
        tx.commit()?;
        if !clip_ids.is_empty() {
            let _ = self.log_activity_internal(
                &conn,
                "clips_restored_all",
                &format!("Restored all clips from Trash ({} items)", clip_ids.len()),
            );
        }
        Ok(ClipMutationSummary::new(
            "restore_all",
            requested_count,
            clip_ids,
        ))
    }

    pub fn purge_clip_permanently(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        let is_protected: i32 = conn
            .query_row(
                "SELECT is_protected FROM effective_clip_protection WHERE clip_id = ?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if is_protected != 0 {
            return Ok(());
        }
        let mut stmt = conn.prepare_cached(
            "DELETE FROM clips WHERE id = ?1 AND clips.id NOT IN (SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1)",
        )?;
        stmt.execute(params![id])?;
        let _ = self.log_activity_internal(
            &conn,
            "clip_deleted",
            &format!("Permanently deleted clip #{}", id),
        );
        Ok(())
    }

    pub fn empty_trash(&self) -> Result<()> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM clips WHERE is_trashed = 1 AND clips.id NOT IN (SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1)",
            [],
            |r| r.get(0),
        ).unwrap_or(0);
        let mut stmt = conn.prepare_cached(
            "DELETE FROM clips WHERE is_trashed = 1 AND clips.id NOT IN (SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1)",
        )?;
        stmt.execute([])?;
        let _ = self.log_activity_internal(
            &conn,
            "trash_emptied",
            &format!("Emptied Trash (permanently deleted {} items)", count),
        );
        Ok(())
    }

    pub fn batch_pin_clips(&self, ids: Vec<i64>, pin_state: bool) -> Result<ClipMutationSummary> {
        let requested_count = ids.len();
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let mut changed_ids = Vec::new();
        let mut seen_ids = HashSet::new();
        for id in ids {
            if !seen_ids.insert(id) {
                continue;
            }
            let current = tx
                .query_row(
                    "SELECT is_pinned FROM clips WHERE id = ?1",
                    params![id],
                    |row| row.get::<_, i32>(0),
                )
                .optional()?;
            if current.is_some_and(|value| (value != 0) != pin_state) {
                changed_ids.push(id);
            }
        }
        if pin_state && !changed_ids.is_empty() {
            tx.execute(
                "UPDATE clips SET pin_order = COALESCE(pin_order, 0) + ?1 WHERE is_pinned = 1",
                params![changed_ids.len() as i32],
            )?;
        }
        for (index, id) in changed_ids.iter().enumerate() {
            tx.execute(
                "UPDATE clips SET is_pinned = ?1, pin_order = ?2 WHERE id = ?3",
                params![
                    if pin_state { 1 } else { 0 },
                    if pin_state { index as i32 } else { 0 },
                    id
                ],
            )?;
        }
        tx.commit()?;
        if !changed_ids.is_empty() {
            let event_type = match (pin_state, changed_ids.len()) {
                (true, 1) => "clip_pinned",
                (true, _) => "clips_pinned",
                (false, 1) => "clip_unpinned",
                (false, _) => "clips_unpinned",
            };
            let verb = if pin_state { "Pinned" } else { "Unpinned" };
            let _ = self.log_activity_internal(
                &conn,
                event_type,
                &format!("{} {}", verb, describe_clip_ids(&changed_ids)),
            );
        }
        Ok(ClipMutationSummary::new(
            if pin_state { "pin" } else { "unpin" },
            requested_count,
            changed_ids,
        ))
    }

    pub fn batch_assign_bin_clips(
        &self,
        ids: Vec<i64>,
        bin_id: Option<i64>,
    ) -> Result<ClipMutationSummary> {
        let requested_count = ids.len();
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        if let Some(bin_id) = bin_id {
            let is_manual = tx
                .query_row(
                    "SELECT smart_rule IS NULL FROM bins WHERE id = ?1",
                    params![bin_id],
                    |row| row.get::<_, bool>(0),
                )
                .optional()?
                .unwrap_or(false);
            if !is_manual {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Clips can only be added directly to manual Bins".to_string(),
                ));
            }
        }
        let mut changed_ids = Vec::new();
        for clip_id in ids {
            let is_active = tx
                .query_row(
                    "SELECT 1 FROM clips WHERE id = ?1 AND (is_trashed IS NULL OR is_trashed = 0)",
                    params![clip_id],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?
                .is_some();
            if !is_active {
                continue;
            }
            if let Some(bid) = bin_id {
                let already_assigned = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM clip_bins WHERE clip_id = ?1 AND bin_id = ?2)",
                    params![clip_id, bid],
                    |row| row.get::<_, bool>(0),
                )?;
                if already_assigned {
                    continue;
                }
                tx.execute(
                    "INSERT OR REPLACE INTO clip_bins (clip_id, bin_id) VALUES (?1, ?2)",
                    params![clip_id, bid],
                )?;
                tx.execute(
                    "UPDATE clips SET bin_id = ?1 WHERE id = ?2",
                    params![bid, clip_id],
                )?;
            } else {
                let has_manual_bins = tx.query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM clip_bins membership
                        JOIN bins ON bins.id = membership.bin_id
                        WHERE membership.clip_id = ?1 AND bins.smart_rule IS NULL
                    )",
                    params![clip_id],
                    |row| row.get::<_, bool>(0),
                )?;
                if !has_manual_bins {
                    continue;
                }
                tx.execute(
                    "DELETE FROM clip_bins
                     WHERE clip_id = ?1 AND bin_id IN (
                        SELECT id FROM bins WHERE smart_rule IS NULL
                     )",
                    params![clip_id],
                )?;
                tx.execute(
                    "UPDATE clips SET bin_id = NULL WHERE id = ?1",
                    params![clip_id],
                )?;
            }
            changed_ids.push(clip_id);
        }
        tx.commit()?;
        if !changed_ids.is_empty() {
            let assigned = bin_id.is_some();
            let event_type = match (assigned, changed_ids.len()) {
                (true, 1) => "clip_bin_assigned",
                (true, _) => "clips_bin_assigned",
                (false, 1) => "clip_bin_unassigned",
                (false, _) => "clips_bin_unassigned",
            };
            let _ = self.log_activity_internal(
                &conn,
                event_type,
                &bin_id.map_or_else(
                    || {
                        format!(
                            "Removed {} from all manual Bins",
                            describe_clip_ids(&changed_ids)
                        )
                    },
                    |id| format!("Added {} to Bin #{id}", describe_clip_ids(&changed_ids)),
                ),
            );
        }
        Ok(ClipMutationSummary::new(
            if bin_id.is_some() {
                "assign_bin"
            } else {
                "unassign_bin"
            },
            requested_count,
            changed_ids,
        ))
    }

    pub fn batch_remove_bin_clips(
        &self,
        ids: Vec<i64>,
        bin_id: i64,
    ) -> Result<ClipMutationSummary> {
        let requested_count = ids.len();
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let is_manual = tx
            .query_row(
                "SELECT smart_rule IS NULL FROM bins WHERE id = ?1",
                params![bin_id],
                |row| row.get::<_, bool>(0),
            )
            .optional()?
            .unwrap_or(false);
        if !is_manual {
            return Err(rusqlite::Error::InvalidParameterName(
                "Clips can only be removed directly from manual Bins".to_string(),
            ));
        }
        let mut changed_ids = Vec::new();
        for clip_id in ids {
            let removed = tx.execute(
                "DELETE FROM clip_bins
                 WHERE clip_id = ?1 AND bin_id = ?2
                   AND EXISTS(
                       SELECT 1 FROM clips
                       WHERE id = ?1 AND (is_trashed IS NULL OR is_trashed = 0)
                   )",
                params![clip_id, bin_id],
            )?;
            if removed == 0 {
                continue;
            }
            tx.execute(
                "UPDATE clips
                 SET bin_id = (
                     SELECT membership.bin_id FROM clip_bins membership
                     JOIN bins ON bins.id = membership.bin_id
                     WHERE membership.clip_id = clips.id AND bins.smart_rule IS NULL
                     ORDER BY membership.bin_id ASC LIMIT 1
                 )
                 WHERE id = ?1 AND bin_id = ?2",
                params![clip_id, bin_id],
            )?;
            changed_ids.push(clip_id);
        }
        tx.commit()?;
        if !changed_ids.is_empty() {
            let event_type = if changed_ids.len() == 1 {
                "clip_bin_removed"
            } else {
                "clips_bin_removed"
            };
            let _ = self.log_activity_internal(
                &conn,
                event_type,
                &format!(
                    "Removed {} from Bin #{bin_id}",
                    describe_clip_ids(&changed_ids)
                ),
            );
        }
        Ok(ClipMutationSummary::new(
            "remove_bin",
            requested_count,
            changed_ids,
        ))
    }

    pub fn trash_unpinned_clips(&self) -> Result<()> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM clips WHERE is_pinned = 0 AND clips.id NOT IN (SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1) AND (is_trashed IS NULL OR is_trashed = 0)", [], |r| r.get(0)).unwrap_or(0);
        conn.execute(
            "UPDATE clips SET is_trashed = 1, trashed_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE is_pinned = 0 AND clips.id NOT IN (SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1) AND (is_trashed IS NULL OR is_trashed = 0)",
            [],
        )?;
        conn.execute(
            "DELETE FROM clip_bins
             WHERE clip_id IN (SELECT id FROM clips WHERE is_trashed = 1)
               AND bin_id IN (
                   SELECT id FROM bins WHERE COALESCE(bin_type, 'category') != 'tag'
               )",
            [],
        )?;
        conn.execute("UPDATE clips SET bin_id = NULL WHERE is_trashed = 1", [])?;
        let _ = self.log_activity_internal(
            &conn,
            "clips_trashed_all",
            &format!(
                "Moved all unpinned and unprotected clips to Trash ({} items)",
                count
            ),
        );
        let _ = self.enforce_trash_limit_internal(&conn);
        Ok(())
    }

    pub fn purge_unpinned_clips(&self) -> Result<()> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM clips WHERE is_pinned = 0 AND clips.id NOT IN (SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1)", [], |r| r.get(0)).unwrap_or(0);
        conn.execute(
            "DELETE FROM clips WHERE is_pinned = 0 AND clips.id NOT IN (SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1)",
            [],
        )?;
        let _ = self.log_activity_internal(
            &conn,
            "clips_purged_all",
            &format!(
                "Permanently deleted all unpinned and unprotected clips ({} items)",
                count
            ),
        );
        Ok(())
    }

    pub fn clear_all_clips(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM clips WHERE clips.id NOT IN (SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1)",
            [],
        )?;
        Ok(())
    }

    pub fn toggle_pin(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock();
        let current_pinned: i32 = conn.query_row(
            "SELECT is_pinned FROM clips WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )?;
        drop(conn);
        let new_pinned = current_pinned == 0;
        self.batch_pin_clips(vec![id], new_pinned)?;
        Ok(new_pinned)
    }

    pub fn assign_to_bin(&self, clip_id: i64, bin_id: Option<i64>) -> Result<ClipMutationSummary> {
        self.batch_assign_bin_clips(vec![clip_id], bin_id)
    }

    pub fn add_clip_to_bin(&self, clip_id: i64, bin_id: i64) -> Result<()> {
        let conn = self.conn.lock();
        let is_active = conn
            .query_row(
                "SELECT CASE WHEN is_trashed IS NULL OR is_trashed = 0 THEN 1 ELSE 0 END
             FROM clips WHERE id = ?1",
                params![clip_id],
                |row| row.get::<_, i32>(0),
            )
            .unwrap_or(0)
            != 0;
        if !is_active {
            return Ok(());
        }
        conn.execute(
            "INSERT OR REPLACE INTO clip_bins (clip_id, bin_id) VALUES (?1, ?2)",
            params![clip_id, bin_id],
        )?;
        conn.execute(
            "UPDATE clips SET bin_id = ?1 WHERE id = ?2",
            params![bin_id, clip_id],
        )?;
        Ok(())
    }

    pub fn remove_clip_from_bin(&self, clip_id: i64, bin_id: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM clip_bins WHERE clip_id = ?1 AND bin_id = ?2",
            params![clip_id, bin_id],
        )?;
        Ok(())
    }
}
