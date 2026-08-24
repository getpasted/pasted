use rusqlite::{params, OptionalExtension, Result};

use super::super::{ClipSearchableText, DbState};

impl DbState {
    pub fn replace_clip_searchable_text(
        &self,
        clip_id: i64,
        input_hash: &str,
        extractor: &crate::content_extraction::Extractor,
        searchable_text: Option<&str>,
    ) -> Result<bool> {
        if input_hash.len() > 128
            || extractor.stable_ref.len() > 160
            || extractor.name.len() > 80
            || extractor.engine.len() > 80
            || searchable_text
                .is_some_and(|text| text.len() > crate::resource_limits::MAX_OCR_TEXT_BYTES)
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "Searchable extraction exceeds its safety limit".into(),
            ));
        }
        let mut conn = self.conn.lock();
        let transaction = conn.transaction()?;
        let clip_matches: bool = transaction.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM clips
                WHERE id = ?1 AND content_hash = ?2 AND content_type = 'file'
                  AND COALESCE(is_trashed, 0) = 0
            )",
            params![clip_id, input_hash],
            |row| row.get(0),
        )?;
        if !clip_matches {
            transaction.rollback()?;
            return Ok(false);
        }
        if let Some(searchable_text) = searchable_text {
            transaction.execute(
                "INSERT INTO clip_searchable_text
                    (clip_id, extractor_ref, extractor_name, engine, input_hash, searchable_text)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(clip_id) DO UPDATE SET
                    extractor_ref = excluded.extractor_ref,
                    extractor_name = excluded.extractor_name,
                    engine = excluded.engine,
                    input_hash = excluded.input_hash,
                    searchable_text = excluded.searchable_text,
                    updated_at = CURRENT_TIMESTAMP",
                params![
                    clip_id,
                    extractor.stable_ref,
                    extractor.name,
                    extractor.engine,
                    input_hash,
                    searchable_text,
                ],
            )?;
        } else {
            transaction.execute(
                "DELETE FROM clip_searchable_text WHERE clip_id = ?1",
                params![clip_id],
            )?;
        }
        transaction.commit()?;
        Ok(true)
    }

    pub fn get_clip_searchable_text(&self, clip_id: i64) -> Result<Option<ClipSearchableText>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT extracted.clip_id, extracted.extractor_ref, extracted.extractor_name,
                    extracted.engine, extracted.input_hash, extracted.searchable_text,
                    extracted.updated_at
             FROM clip_searchable_text AS extracted
             JOIN clips ON clips.id = extracted.clip_id
             WHERE extracted.clip_id = ?1
               AND extracted.input_hash = clips.content_hash
               AND clips.content_type = 'file'
               AND COALESCE(clips.is_trashed, 0) = 0",
            params![clip_id],
            |row| {
                Ok(ClipSearchableText {
                    clip_id: row.get(0)?,
                    extractor_ref: row.get(1)?,
                    extractor_name: row.get(2)?,
                    engine: row.get(3)?,
                    input_hash: row.get(4)?,
                    searchable_text: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )
        .optional()
    }
}
