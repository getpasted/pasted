use super::*;

pub(super) fn migrate_apple_vision_labels_identity(conn: &Connection) -> Result<()> {
    let old_ref = crate::content_extraction::LEGACY_APPLE_VISION_LABELS_REF;
    let new_ref = crate::content_extraction::APPLE_VISION_LABELS_REF;
    let old_id = conn
        .query_row(
            "SELECT id FROM content_extractors WHERE stable_ref = ?1",
            params![old_ref],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    let Some(old_id) = old_id else {
        return Ok(());
    };
    let new_id = conn
        .query_row(
            "SELECT id FROM content_extractors WHERE stable_ref = ?1",
            params![new_ref],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;

    if let Some(new_id) = new_id {
        conn.execute(
            "INSERT OR IGNORE INTO extractor_recipe_revisions
                (extractor_id, revision, recipe_json, recipe_hash, authoring_session_id, created_at)
             SELECT ?1, revision, recipe_json, recipe_hash, NULL, created_at
             FROM extractor_recipe_revisions WHERE extractor_id = ?2",
            params![new_id, old_id],
        )?;
        conn.execute(
            "UPDATE extractor_authoring_sessions SET extractor_id = ?1 WHERE extractor_id = ?2",
            params![new_id, old_id],
        )?;
        conn.execute(
            "DELETE FROM extractor_recipe_revisions WHERE extractor_id = ?1",
            params![old_id],
        )?;
        conn.execute("DELETE FROM content_extractors WHERE id = ?1", [old_id])?;
    } else {
        conn.execute(
            "UPDATE content_extractors SET stable_ref = ?1 WHERE id = ?2",
            params![new_ref, old_id],
        )?;
    }

    for statement in [
        "UPDATE clips SET ocr_extractor_ref = ?1 WHERE ocr_extractor_ref = ?2",
        "UPDATE clip_analysis_results SET participant_ref = ?1 WHERE participant_ref = ?2",
        "UPDATE clip_extraction_attempts SET participant_ref = ?1 WHERE participant_ref = ?2",
        "UPDATE clip_searchable_text SET extractor_ref = ?1 WHERE extractor_ref = ?2",
    ] {
        conn.execute(statement, params![new_ref, old_ref])?;
    }
    for table in ["clip_analysis_results", "clip_extraction_attempts"] {
        conn.execute(
            &format!(
                "UPDATE {table}
                 SET result_json = REPLACE(result_json, ?1, ?2)
                 WHERE result_json LIKE '%' || ?1 || '%'"
            ),
            params![old_ref, new_ref],
        )?;
    }
    conn.execute(
        "UPDATE clip_versions
         SET context_json = REPLACE(context_json, ?1, ?2)
         WHERE context_json LIKE '%' || ?1 || '%'",
        params![old_ref, new_ref],
    )?;
    Ok(())
}
