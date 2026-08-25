use rusqlite::{params, OptionalExtension, Result};

use super::DbState;

impl DbState {
    pub(super) fn delete_clip_version_internal(&self, clip_id: i64, version_id: i64) -> Result<()> {
        if version_id <= 0 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Current cannot be deleted from Version History".into(),
            ));
        }

        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let (action_kind, is_oldest) = tx
            .query_row(
                "SELECT json_extract(context_json, '$.action_kind'),
                        id = (SELECT MIN(original.id) FROM clip_versions original
                              WHERE original.clip_id = ?2)
                 FROM clip_versions WHERE id = ?1 AND clip_id = ?2",
                params![version_id, clip_id],
                |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, bool>(1)?)),
            )
            .optional()?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        if is_oldest || action_kind.as_deref() == Some("original") {
            return Err(rusqlite::Error::InvalidParameterName(
                "Original cannot be deleted from Version History".into(),
            ));
        }

        tx.execute(
            "DELETE FROM clip_versions WHERE id = ?1 AND clip_id = ?2",
            params![version_id, clip_id],
        )?;
        tx.commit()?;
        let _ = self.log_activity_internal(
            &conn,
            "clip_version_deleted",
            &format!("Deleted version #{version_id} from clip #{clip_id}"),
        );
        Ok(())
    }
}
