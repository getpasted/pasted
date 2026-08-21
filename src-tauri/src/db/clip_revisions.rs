use rusqlite::{params, Result};

use super::{ClipItem, ClipRevisionContext, ClipRevisionOrganization, ClipVersion, DbState};

impl DbState {
    #[cfg(test)]
    pub fn get_clip_versions(&self, clip_id: i64) -> Result<Vec<ClipVersion>> {
        self.get_clip_versions_page(clip_id, -1, 0)
    }

    pub fn get_clip_versions_page(
        &self,
        clip_id: i64,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ClipVersion>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, clip_id, text_content, context_json, created_at
             FROM clip_versions WHERE clip_id = ?1
             ORDER BY created_at DESC, id DESC LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(params![clip_id, limit, offset.max(0)], |row| {
            let context_json: Option<String> = row.get(3)?;
            let context = context_json
                .as_deref()
                .and_then(|value| serde_json::from_str::<ClipRevisionContext>(value).ok());
            Ok(ClipVersion {
                id: row.get(0)?,
                clip_id: row.get(1)?,
                text_content: row.get(2)?,
                action_kind: context.as_ref().map(|value| value.action_kind.clone()),
                action_label: context.as_ref().map(|value| value.action_label.clone()),
                restores_organization: context
                    .as_ref()
                    .and_then(|value| value.organization.as_ref())
                    .is_some(),
                created_at: row.get(4)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_clip_version_count(&self, clip_id: i64) -> Result<i64> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT COUNT(*) FROM clip_versions WHERE clip_id = ?1",
            params![clip_id],
            |row| row.get(0),
        )
    }

    pub fn restore_clip_version(&self, clip_id: i64, version_id: i64) -> Result<ClipItem> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let (target_text, context_json): (String, Option<String>) = tx.query_row(
            "SELECT text_content, context_json FROM clip_versions WHERE id = ?1 AND clip_id = ?2",
            params![version_id, clip_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let target_context = context_json
            .as_deref()
            .and_then(|value| serde_json::from_str::<ClipRevisionContext>(value).ok());
        let (current_text, current_bin_id, is_trashed, current_transformation_id): (
            Option<String>,
            Option<i64>,
            i32,
            Option<String>,
        ) = tx
            .query_row(
                "SELECT text_content, bin_id, COALESCE(is_trashed, 0), current_transformation_id FROM clips WHERE id = ?1",
                params![clip_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;
        if is_trashed != 0 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Restore this clip from Trash before restoring a revision".to_string(),
            ));
        }

        let target_bin_id = target_context
            .as_ref()
            .and_then(|context| context.organization.as_ref())
            .map(|organization| organization.category_bin_id);
        let organization_changes = target_bin_id
            .map(|target| target != current_bin_id)
            .unwrap_or(false);
        let target_transformation_id = target_context
            .as_ref()
            .and_then(|context| context.current_transformation_id.clone());
        if current_text.as_deref() == Some(target_text.as_str())
            && !organization_changes
            && current_transformation_id == target_transformation_id
        {
            tx.commit()?;
            return self.get_clip_by_id_internal(&conn, clip_id);
        }

        if let Some(current_text) = current_text {
            let inverse_context = target_bin_id.map(|_| ClipRevisionContext {
                schema_version: 1,
                action_kind: "restore".to_string(),
                action_label: "Before restoring an earlier revision".to_string(),
                organization: Some(ClipRevisionOrganization {
                    category_bin_id: current_bin_id,
                }),
                current_transformation_id: current_transformation_id.clone(),
            });
            let inverse_context = inverse_context.or_else(|| {
                Some(ClipRevisionContext {
                    schema_version: 1,
                    action_kind: "restore".to_string(),
                    action_label: "Before restoring an earlier revision".to_string(),
                    organization: None,
                    current_transformation_id: current_transformation_id.clone(),
                })
            });
            let inverse_json = inverse_context
                .map(|context| serde_json::to_string(&context))
                .transpose()
                .map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))?;
            tx.execute(
                "INSERT INTO clip_versions (clip_id, text_content, context_json) VALUES (?1, ?2, ?3)",
                params![clip_id, current_text, inverse_json],
            )?;
        }

        tx.execute(
            "UPDATE clips SET text_content = ?1, current_transformation_id = ?2 WHERE id = ?3",
            params![target_text, target_transformation_id, clip_id],
        )?;
        if let Some(target_bin_id) = target_bin_id {
            tx.execute(
                "DELETE FROM clip_bins
                 WHERE clip_id = ?1 AND bin_id IN (
                    SELECT id FROM bins WHERE COALESCE(bin_type, 'category') != 'tag'
                 )",
                params![clip_id],
            )?;
            let restored_bin_id = if let Some(bin_id) = target_bin_id {
                let changed = tx.execute(
                    "INSERT OR REPLACE INTO clip_bins (clip_id, bin_id)
                     SELECT ?1, id FROM bins
                     WHERE id = ?2 AND COALESCE(bin_type, 'category') != 'tag'",
                    params![clip_id, bin_id],
                )?;
                (changed > 0).then_some(bin_id)
            } else {
                None
            };
            tx.execute(
                "UPDATE clips SET bin_id = ?1 WHERE id = ?2",
                params![restored_bin_id, clip_id],
            )?;
        }
        Self::prune_clip_versions_internal(&tx, clip_id)?;
        tx.commit()?;
        let _ = self.log_activity_internal(
            &conn,
            "clip_revision_restored",
            &format!("Restored revision #{version_id} for clip #{clip_id}"),
        );
        self.get_clip_by_id_internal(&conn, clip_id)
    }
}
