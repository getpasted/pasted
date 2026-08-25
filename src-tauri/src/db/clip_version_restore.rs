use rusqlite::{params, Result};

use super::{
    clip_revision_state::ClipRevisionDerivedState, ClipItem, ClipRevisionContext,
    ClipRevisionOrganization, DbState,
};

impl DbState {
    pub(super) fn restore_clip_version_internal(
        &self,
        clip_id: i64,
        version_id: i64,
    ) -> Result<ClipItem> {
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
        let (
            current_text,
            current_bin_id,
            is_trashed,
            current_transformation_id,
            content_type,
            content_hash,
        ): (
            Option<String>,
            Option<i64>,
            i32,
            Option<String>,
            String,
            String,
        ) = tx.query_row(
            "SELECT text_content, bin_id, COALESCE(is_trashed, 0), current_transformation_id,
                    content_type, content_hash FROM clips WHERE id = ?1",
            params![clip_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
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
        let current_derived_state = (content_type == "image")
            .then(|| ClipRevisionDerivedState::capture(&tx, clip_id))
            .transpose()?;
        let target_derived_state = (content_type == "image").then(|| {
            target_context
                .as_ref()
                .and_then(|context| context.derived_state.clone())
                .unwrap_or_else(|| {
                    if target_text.is_empty() {
                        ClipRevisionDerivedState::original(&content_hash)
                    } else {
                        ClipRevisionDerivedState::legacy_text(&content_hash)
                    }
                })
        });
        if current_text.as_deref() == Some(target_text.as_str())
            && !organization_changes
            && current_transformation_id == target_transformation_id
            && current_derived_state == target_derived_state
        {
            tx.commit()?;
            return self.get_clip_by_id_internal(&conn, clip_id);
        }

        if current_text.is_some() || content_type == "image" {
            let inverse_text = current_text.clone().unwrap_or_default();
            let inverse_context = ClipRevisionContext {
                schema_version: 2,
                action_kind: "restore".to_string(),
                action_label: "Before restoring an earlier revision".to_string(),
                organization: target_bin_id.map(|_| ClipRevisionOrganization {
                    category_bin_id: current_bin_id,
                }),
                current_transformation_id: current_transformation_id.clone(),
                derived_state: current_derived_state.clone(),
            };
            let inverse_json = serde_json::to_string(&inverse_context)
                .map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))?;
            tx.execute(
                "INSERT INTO clip_versions (clip_id, text_content, context_json) VALUES (?1, ?2, ?3)",
                params![clip_id, inverse_text, inverse_json],
            )?;
        }

        tx.execute(
            "UPDATE clips SET text_content = ?1, current_transformation_id = ?2 WHERE id = ?3",
            params![target_text, target_transformation_id, clip_id],
        )?;
        if let Some(target_derived_state) = target_derived_state.as_ref() {
            target_derived_state.restore(&tx, clip_id)?;
        }
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
            &format!("Restored version #{version_id} for clip #{clip_id}"),
        );
        self.get_clip_by_id_internal(&conn, clip_id)
    }
}
