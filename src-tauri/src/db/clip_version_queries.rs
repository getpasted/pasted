use rusqlite::{params, Result};

use super::{
    clip_revision_state::ClipRevisionDerivedState, ClipRevisionContext, ClipVersion, DbState,
};

impl DbState {
    pub(super) fn get_clip_versions_page_internal(
        &self,
        clip_id: i64,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ClipVersion>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, clip_id, text_content, context_json, created_at,
                    id = (SELECT MIN(original.id) FROM clip_versions original
                          WHERE original.clip_id = ?1)
             FROM clip_versions WHERE clip_id = ?1
             ORDER BY created_at DESC, id DESC LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(params![clip_id, limit, offset.max(0)], |row| {
            let row_clip_id = row.get(1)?;
            let context_json: Option<String> = row.get(3)?;
            let context = context_json
                .as_deref()
                .and_then(|value| serde_json::from_str::<ClipRevisionContext>(value).ok());
            let is_original: bool = row.get(5)?;
            Ok(ClipVersion {
                id: row.get(0)?,
                clip_id: row_clip_id,
                text_content: row.get(2)?,
                action_kind: context.as_ref().map(|value| value.action_kind.clone()),
                action_label: context.as_ref().map(|value| value.action_label.clone()),
                restores_organization: context
                    .as_ref()
                    .and_then(|value| value.organization.as_ref())
                    .is_some(),
                visual_labels: context
                    .as_ref()
                    .and_then(|value| value.derived_state.as_ref())
                    .map(|state| state.effective_visual_labels(row_clip_id)),
                is_current: false,
                is_original,
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

    pub fn get_clip_version_timeline_page(
        &self,
        clip_id: i64,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ClipVersion>> {
        let limit = limit.max(1);
        let offset = offset.max(0);
        if offset > 0 {
            return self.get_clip_versions_page_internal(clip_id, limit, offset - 1);
        }
        let mut versions = vec![self.get_current_clip_version(clip_id)?];
        if limit > 1 {
            versions.extend(self.get_clip_versions_page_internal(clip_id, limit - 1, 0)?);
        }
        Ok(versions)
    }

    pub fn get_clip_version_timeline_count(&self, clip_id: i64) -> Result<i64> {
        self.get_current_clip_version(clip_id)?;
        Ok(self.get_clip_version_count(clip_id)? + 1)
    }

    fn get_current_clip_version(&self, clip_id: i64) -> Result<ClipVersion> {
        let conn = self.conn.lock();
        let (text_content, content_type, created_at): (Option<String>, String, String) = conn
            .query_row(
                "SELECT text_content, content_type, created_at FROM clips WHERE id = ?1",
                [clip_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;
        let visual_labels = if content_type == "image" {
            Some(
                ClipRevisionDerivedState::capture(&conn, clip_id)?.effective_visual_labels(clip_id),
            )
        } else {
            None
        };
        Ok(ClipVersion {
            id: 0,
            clip_id,
            text_content: text_content.unwrap_or_default(),
            action_kind: Some("current".into()),
            action_label: Some("Current".into()),
            restores_organization: false,
            visual_labels,
            is_current: true,
            is_original: false,
            created_at,
        })
    }
}
