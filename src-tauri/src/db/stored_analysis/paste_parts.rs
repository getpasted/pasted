use rusqlite::{params, Result};

use super::super::DbState;

impl DbState {
    pub fn replace_capture_classifications(
        &self,
        clip_id: i64,
        content_hash: &str,
        source: &str,
        matches: &[crate::content_classification::ClassificationMatch],
    ) -> Result<()> {
        self.replace_analysis_classifications(clip_id, content_hash, matches, "original_text")?;
        self.replace_paste_parts(
            clip_id,
            content_hash,
            source,
            &crate::smart_paste::parts::from_classifications(matches),
        )?;
        Ok(())
    }

    pub fn replace_paste_parts(
        &self,
        clip_id: i64,
        content_hash: &str,
        source: &str,
        analysis: &crate::smart_paste::parts::PastePartsAnalysis,
    ) -> Result<bool> {
        validate(source, analysis)?;
        let encoded = serde_json::to_string(analysis)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let mut conn = self.conn.lock();
        let transaction = conn.transaction()?;
        let clip_matches: bool = transaction.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM clips
                WHERE id = ?1 AND content_hash = ?2 AND COALESCE(is_trashed, 0) = 0
            )",
            params![clip_id, content_hash],
            |row| row.get(0),
        )?;
        if !clip_matches {
            return Ok(false);
        }
        transaction.execute(
            "INSERT INTO clip_analysis_results
                (clip_id, participant_ref, content_hash, input_hash, format_version,
                 result_json, updated_at)
             VALUES (?1, ?2, ?3, ?3, ?4, ?5, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
             ON CONFLICT(clip_id, participant_ref) DO UPDATE SET
                content_hash = excluded.content_hash,
                input_hash = excluded.input_hash,
                format_version = excluded.format_version,
                result_json = excluded.result_json,
                updated_at = excluded.updated_at",
            params![
                clip_id,
                crate::smart_paste::parts::PARTICIPANT_REF,
                content_hash,
                crate::smart_paste::parts::FORMAT_VERSION,
                encoded,
            ],
        )?;
        transaction.commit()?;
        Ok(true)
    }

    pub fn get_paste_parts(
        &self,
        clip_id: i64,
    ) -> Result<Option<crate::smart_paste::parts::PastePartsAnalysis>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT results.result_json
             FROM clip_analysis_results AS results
             JOIN clips ON clips.id = results.clip_id
             WHERE results.clip_id = ?1
               AND results.participant_ref = ?2
               AND results.content_hash = clips.content_hash
               AND results.input_hash = clips.content_hash
               AND results.format_version = ?3
               AND COALESCE(clips.is_trashed, 0) = 0",
        )?;
        let mut rows = statement.query(params![
            clip_id,
            crate::smart_paste::parts::PARTICIPANT_REF,
            crate::smart_paste::parts::FORMAT_VERSION,
        ])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        let encoded: String = row.get(0)?;
        serde_json::from_str(&encoded).map(Some).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })
    }

    pub fn get_paste_parts_by_hash(
        &self,
        content_hash: &str,
    ) -> Result<Option<crate::smart_paste::parts::PastePartsAnalysis>> {
        let conn = self.conn.lock();
        let encoded = conn.query_row(
            "SELECT results.result_json
             FROM clip_analysis_results AS results
             JOIN clips ON clips.id = results.clip_id
             WHERE clips.content_hash = ?1
               AND results.participant_ref = ?2
               AND results.content_hash = clips.content_hash
               AND results.input_hash = clips.content_hash
               AND results.format_version = ?3
               AND COALESCE(clips.is_trashed, 0) = 0
             ORDER BY clips.id DESC LIMIT 1",
            params![
                content_hash,
                crate::smart_paste::parts::PARTICIPANT_REF,
                crate::smart_paste::parts::FORMAT_VERSION,
            ],
            |row| row.get::<_, String>(0),
        );
        match encoded {
            Ok(encoded) => serde_json::from_str(&encoded).map(Some).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            }),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

fn validate(source: &str, analysis: &crate::smart_paste::parts::PastePartsAnalysis) -> Result<()> {
    let source_chars = source.chars().count();
    let invalid = analysis.format_version != crate::smart_paste::parts::FORMAT_VERSION
        || analysis.parts.len() > crate::smart_paste::parts::MAX_PARTS_PER_CLIP
        || analysis.parts.iter().any(|part| {
            part.kind.is_empty()
                || part.kind.len() > 80
                || part.role.as_ref().is_some_and(|role| role.len() > 160)
                || part.aliases.len() > crate::smart_paste::parts::MAX_ALIASES_PER_PART
                || part
                    .aliases
                    .iter()
                    .any(|alias| alias.is_empty() || alias.len() > 160)
                || part.analyzer_ref.is_empty()
                || part.analyzer_ref.len() > 160
                || !part.confidence.is_finite()
                || !(0.0..=1.0).contains(&part.confidence)
                || part.start_offset >= part.end_offset
                || part.end_offset > source_chars
                || crate::smart_paste::parts::value_at_offsets(source, part).is_none()
        });
    if invalid {
        return Err(rusqlite::Error::InvalidParameterName(
            "Paste-part analysis exceeds its safety contract".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn paste_parts_are_hash_safe_and_preserve_unicode_offsets() {
        let db = crate::db::tests::setup_test_db();
        let source = "A😀 user@example.com";
        let clip = db.save_text_clip(source, "Tests").unwrap();
        let analysis = crate::smart_paste::parts::PastePartsAnalysis {
            format_version: crate::smart_paste::parts::FORMAT_VERSION,
            parts: vec![crate::smart_paste::parts::PastePart {
                kind: "email".into(),
                role: Some("work".into()),
                aliases: vec!["work email".into()],
                start_offset: 3,
                end_offset: 19,
                confidence: 1.0,
                analyzer_ref: "test:email".into(),
            }],
        };
        assert!(db
            .replace_paste_parts(clip.id, &clip.content_hash, source, &analysis)
            .unwrap());
        assert_eq!(db.get_paste_parts(clip.id).unwrap(), Some(analysis));
        assert!(!db
            .replace_paste_parts(
                clip.id,
                "stale",
                source,
                &crate::smart_paste::parts::PastePartsAnalysis::empty(),
            )
            .unwrap());
    }
}
