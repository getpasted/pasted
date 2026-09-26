use std::collections::HashMap;

use rusqlite::{params, Connection, Result};

use super::{append_smart_bin_memberships, clip_item_from_row, ClipItem, DbState};

const LIST_PREVIEW_QUERY_CHARS: usize = 1_025;

impl DbState {
    pub(super) fn get_clips_by_ids_internal(
        conn: &Connection,
        ids: &[i64],
    ) -> Result<Vec<ClipItem>> {
        Self::get_clips_by_ids_with_projection(conn, ids, false)
    }

    pub(super) fn get_clip_list_items_by_ids_internal(
        conn: &Connection,
        ids: &[i64],
    ) -> Result<Vec<ClipItem>> {
        Self::get_clips_by_ids_with_projection(conn, ids, true)
    }

    fn get_clips_by_ids_with_projection(
        conn: &Connection,
        ids: &[i64],
        bounded: bool,
    ) -> Result<Vec<ClipItem>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids_json = serde_json::to_string(ids)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let text = if bounded {
            format!("CASE WHEN content_type = 'file' THEN text_content ELSE SUBSTR(text_content, 1, {LIST_PREVIEW_QUERY_CHARS}) END")
        } else {
            "text_content".into()
        };
        let note = if bounded {
            format!("SUBSTR(note, 1, {LIST_PREVIEW_QUERY_CHARS})")
        } else {
            "note".into()
        };
        let mut statement = conn.prepare(&format!(
            "SELECT id, content_type, {text}, NULL AS html_content, NULL AS image_base64, image_path,
                    content_hash, source, is_pinned, is_protected, COALESCE(pin_order, 0),
                    bin_id, {note}, COALESCE(is_trashed, 0), trashed_at, created_at,
                    (SELECT GROUP_CONCAT(bin_id) FROM clip_bins WHERE clip_id = clips.id),
                    current_transformation_id IS NOT NULL,
                    ocr_extractor_ref, ocr_extractor_name, ocr_engine_version, shortcut
             FROM clips
             WHERE id IN (SELECT CAST(value AS INTEGER) FROM json_each(?1))"
        ))?;
        let clips = statement
            .query_map(params![ids_json], clip_item_from_row)?
            .collect::<Result<Vec<_>>>()?;
        let mut by_id = clips
            .into_iter()
            .map(|clip| (clip.id, clip))
            .collect::<HashMap<_, _>>();
        let mut ordered = ids
            .iter()
            .filter_map(|id| by_id.remove(id))
            .collect::<Vec<_>>();
        append_smart_bin_memberships(conn, &mut ordered)?;
        Ok(ordered)
    }
}
