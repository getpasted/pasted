use std::collections::HashMap;

use rusqlite::{params, Connection, Result};

use super::{
    append_smart_bin_memberships, clip_item_from_row, push_smart_condition,
    smart_bin_feature_policy, ClipItem, DbState,
};

impl DbState {
    pub fn get_clip_image(&self, id: i64) -> Result<Option<String>> {
        let conn = self.conn.lock();
        let image: Option<String> = conn.query_row(
            "SELECT image_base64 FROM clips WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        Ok(image.filter(|value| crate::resource_limits::validate_raster_data_url(value).is_ok()))
    }

    pub fn get_active_clip_text(&self, id: i64) -> Result<Option<String>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT text_content FROM clips
             WHERE id = ?1 AND (is_trashed IS NULL OR is_trashed = 0)",
            params![id],
            |row| row.get(0),
        )
    }

    pub fn get_clip_by_id(&self, id: i64) -> Result<ClipItem> {
        let conn = self.conn.lock();
        self.get_clip_by_id_internal(&conn, id)
    }

    pub(super) fn get_clip_by_id_internal(&self, conn: &Connection, id: i64) -> Result<ClipItem> {
        let mut clip = conn.query_row(
            "SELECT id, content_type, text_content, html_content, image_base64, image_path, content_hash, source, is_pinned, is_protected, COALESCE(pin_order, 0), bin_id, note, is_trashed, trashed_at, created_at,
                    (SELECT GROUP_CONCAT(bin_id) FROM clip_bins WHERE clip_id = clips.id),
                    current_transformation_id IS NOT NULL,
                    ocr_extractor_ref, ocr_extractor_name, ocr_engine_version, shortcut
             FROM clips WHERE id = ?1",
            params![id],
            |row| {
                let bid: Option<i64> = row.get(11)?;
                let bin_ids_str: Option<String> = row.get(16)?;
                let mut bin_ids = bid.into_iter().collect::<Vec<_>>();
                if let Some(value) = bin_ids_str {
                    for value in value.split(',').filter_map(|part| part.parse::<i64>().ok()) {
                        if !bin_ids.contains(&value) {
                            bin_ids.push(value);
                        }
                    }
                }
                Ok(ClipItem {
                    id: row.get(0)?,
                    name: None,
                    content_type: row.get(1)?,
                    content_types: Vec::new(),
                    file_formats: Vec::new(),
                    text_content: row.get(2)?,
                    html_content: row.get(3)?,
                    image_base64: row.get(4)?,
                    image_path: row.get(5)?,
                    content_hash: row.get(6)?,
                    source: row.get(7)?,
                    is_pinned: row.get::<_, i32>(8)? != 0,
                    is_protected: row.get::<_, i32>(9)? != 0,
                    is_explicitly_protected: Some(row.get::<_, i32>(9)? != 0),
                    protecting_bin_ids: Vec::new(),
                    is_concealed: false,
                    is_explicitly_concealed: None,
                    is_explicitly_revealed: false,
                    concealing_bin_ids: Vec::new(),
                    concealing_content_types: Vec::new(),
                    shortcut: row.get(21)?,
                    is_transformed: row.get::<_, i32>(17)? != 0,
                    pin_order: row.get(10)?,
                    bin_id: bid,
                    bin_ids: Some(bin_ids),
                    note: row.get(12)?,
                    is_trashed: row.get::<_, i32>(13)? != 0,
                    trashed_at: row.get(14)?,
                    created_at: row.get(15)?,
                    ocr_extractor_ref: row.get(18)?,
                    ocr_extractor_name: row.get(19)?,
                    ocr_engine_version: row.get(20)?,
                })
            },
        )?;
        append_smart_bin_memberships(conn, std::slice::from_mut(&mut clip))?;
        Ok(clip)
    }

    pub(super) fn get_clips_by_ids_internal(
        conn: &Connection,
        ids: &[i64],
    ) -> Result<Vec<ClipItem>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids_json = serde_json::to_string(ids)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let mut statement = conn.prepare(
            "SELECT id, content_type, text_content, html_content, image_base64, image_path,
                    content_hash, source, is_pinned, is_protected, COALESCE(pin_order, 0),
                    bin_id, note, COALESCE(is_trashed, 0), trashed_at, created_at,
                    (SELECT GROUP_CONCAT(bin_id) FROM clip_bins WHERE clip_id = clips.id),
                    current_transformation_id IS NOT NULL,
                    ocr_extractor_ref, ocr_extractor_name, ocr_engine_version, shortcut
             FROM clips
             WHERE id IN (SELECT CAST(value AS INTEGER) FROM json_each(?1))",
        )?;
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

    pub fn get_clips(&self, bin_id: Option<i64>, only_pinned: bool) -> Result<Vec<ClipItem>> {
        self.get_clips_page(bin_id, only_pinned, None, None)
    }

    pub fn get_clips_page(
        &self,
        bin_id: Option<i64>,
        only_pinned: bool,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<ClipItem>> {
        let conn = self.conn.lock();
        let features = smart_bin_feature_policy(&conn)?;

        // Check if target bin has smart_rule
        let mut smart_rule_str: Option<String> = None;
        if let Some(bid) = bin_id {
            let res: Result<Option<String>> = conn.query_row(
                "SELECT smart_rule FROM bins WHERE id = ?1",
                params![bid],
                |r| r.get(0),
            );
            if let Ok(sr) = res {
                smart_rule_str = sr;
            }
        }

        let mut sql = String::from(
            "SELECT id, content_type, text_content, NULL as html_content, NULL as image_base64, image_path, content_hash, source, is_pinned, is_protected, COALESCE(pin_order, 0), bin_id, note, is_trashed, trashed_at, created_at,
             (SELECT GROUP_CONCAT(bin_id) FROM clip_bins WHERE clip_id = clips.id) as bin_ids_str,
             current_transformation_id IS NOT NULL,
             ocr_extractor_ref, ocr_extractor_name, ocr_engine_version, shortcut
             FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0)"
        );

        let mut query_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if only_pinned {
            sql.push_str(" AND is_pinned = 1");
        }

        if let Some(ref sr_json) = smart_rule_str {
            if let Ok(parsed) = crate::smart_bins::parse_rule_json(sr_json) {
                let join_op = if parsed.match_mode == "all" {
                    " AND "
                } else {
                    " OR "
                };

                let mut cond_sqls: Vec<String> = Vec::new();

                for condition in &parsed.conditions {
                    push_smart_condition(
                        &condition.target,
                        &condition.operator,
                        &condition.value,
                        features,
                        &mut cond_sqls,
                        &mut query_params,
                    );
                }

                if !cond_sqls.is_empty() {
                    let combined = cond_sqls.join(join_op);
                    if let Some(bid) = bin_id {
                        sql.push_str(&format!(" AND (({}) OR bin_id = ? OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?))", combined));
                        query_params.push(Box::new(bid));
                        query_params.push(Box::new(bid));
                    } else {
                        sql.push_str(&format!(" AND ({})", combined));
                    }
                } else if let Some(bid) = bin_id {
                    sql.push_str(" AND (bin_id = ? OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?))");
                    query_params.push(Box::new(bid));
                    query_params.push(Box::new(bid));
                }
            } else if let Some(bid) = bin_id {
                sql.push_str(
                    " AND (bin_id = ? OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?))",
                );
                query_params.push(Box::new(bid));
                query_params.push(Box::new(bid));
            }
        } else if let Some(bid) = bin_id {
            sql.push_str(
                " AND (bin_id = ? OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?))",
            );
            query_params.push(Box::new(bid));
            query_params.push(Box::new(bid));
        }

        if let Some(bid) = bin_id {
            sql.push_str(
                " ORDER BY
                    CASE WHEN EXISTS(
                        SELECT 1 FROM bin_clip_order ordered
                        WHERE ordered.bin_id = ? AND ordered.clip_id = clips.id
                    ) THEN 0 ELSE 1 END,
                    (SELECT position FROM bin_clip_order ordered
                     WHERE ordered.bin_id = ? AND ordered.clip_id = clips.id),
                    created_at DESC,
                    id DESC",
            );
            query_params.push(Box::new(bid));
            query_params.push(Box::new(bid));
        } else {
            sql.push_str(" ORDER BY is_pinned DESC, pin_order ASC, created_at DESC, id DESC");
        }

        if let Some(limit) = limit {
            sql.push_str(" LIMIT ? OFFSET ?");
            query_params.push(Box::new(limit.clamp(1, 10_000)));
            query_params.push(Box::new(offset.unwrap_or(0).max(0)));
        }

        let param_refs: Vec<&dyn rusqlite::ToSql> =
            query_params.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let clip_iter = stmt.query_map(param_refs.as_slice(), |row| {
            let primary_bid: Option<i64> = row.get(11)?;
            let bin_ids_str: Option<String> = row.get(16)?;
            let mut b_ids = Vec::new();
            if let Some(b) = primary_bid {
                b_ids.push(b);
            }
            if let Some(ref s) = bin_ids_str {
                for part in s.split(',') {
                    if let Ok(parsed_id) = part.parse::<i64>() {
                        if !b_ids.contains(&parsed_id) {
                            b_ids.push(parsed_id);
                        }
                    }
                }
            }

            Ok(ClipItem {
                id: row.get(0)?,
                name: None,
                content_type: row.get(1)?,
                content_types: Vec::new(),
                file_formats: Vec::new(),
                text_content: row.get(2)?,
                html_content: row.get(3)?,
                image_base64: row.get(4)?,
                image_path: row.get(5)?,
                content_hash: row.get(6)?,
                source: row.get(7)?,
                is_pinned: row.get::<_, i32>(8)? != 0,
                is_protected: row.get::<_, i32>(9)? != 0,
                is_explicitly_protected: Some(row.get::<_, i32>(9)? != 0),
                protecting_bin_ids: Vec::new(),
                is_concealed: false,
                is_explicitly_concealed: None,
                is_explicitly_revealed: false,
                concealing_bin_ids: Vec::new(),
                concealing_content_types: Vec::new(),
                shortcut: row.get(21)?,
                is_transformed: row.get::<_, i32>(17)? != 0,
                pin_order: row.get(10)?,
                bin_id: primary_bid,
                bin_ids: Some(b_ids),
                note: row.get(12)?,
                is_trashed: row.get::<_, i32>(13)? != 0,
                trashed_at: row.get(14)?,
                created_at: row.get(15)?,
                ocr_extractor_ref: row.get(18)?,
                ocr_extractor_name: row.get(19)?,
                ocr_engine_version: row.get(20)?,
            })
        })?;

        let mut clips = Vec::new();
        for clip in clip_iter {
            clips.push(clip?);
        }
        append_smart_bin_memberships(&conn, &mut clips)?;
        Ok(clips)
    }

    pub fn get_trashed_clips(&self) -> Result<Vec<ClipItem>> {
        self.get_trashed_clips_page(None, None)
    }

    pub fn get_trashed_clip_count(&self) -> Result<i64> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT COUNT(*) FROM clips WHERE is_trashed = 1",
            [],
            |row| row.get(0),
        )
    }

    pub fn get_trashed_clips_page(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<ClipItem>> {
        let conn = self.conn.lock();
        let mut sql = String::from(
            "SELECT id, content_type, text_content, NULL as html_content, NULL as image_base64, image_path, content_hash, source, is_pinned, is_protected, COALESCE(pin_order, 0), bin_id, note, is_trashed, trashed_at, created_at,
                    current_transformation_id IS NOT NULL,
                    ocr_extractor_ref, ocr_extractor_name, ocr_engine_version, shortcut
             FROM clips WHERE is_trashed = 1 ORDER BY COALESCE(trashed_at, created_at) DESC, id DESC"
        );
        let mut query_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(limit) = limit {
            sql.push_str(" LIMIT ? OFFSET ?");
            query_params.push(Box::new(limit.clamp(1, 10_000)));
            query_params.push(Box::new(offset.unwrap_or(0).max(0)));
        }
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            query_params.iter().map(|value| value.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let clip_iter = stmt.query_map(param_refs.as_slice(), |row| {
            let bid: Option<i64> = row.get(11)?;
            Ok(ClipItem {
                id: row.get(0)?,
                name: None,
                content_type: row.get(1)?,
                content_types: Vec::new(),
                file_formats: Vec::new(),
                text_content: row.get(2)?,
                html_content: row.get(3)?,
                image_base64: row.get(4)?,
                image_path: row.get(5)?,
                content_hash: row.get(6)?,
                source: row.get(7)?,
                is_pinned: row.get::<_, i32>(8)? != 0,
                is_protected: row.get::<_, i32>(9)? != 0,
                is_explicitly_protected: Some(row.get::<_, i32>(9)? != 0),
                protecting_bin_ids: Vec::new(),
                is_concealed: false,
                is_explicitly_concealed: None,
                is_explicitly_revealed: false,
                concealing_bin_ids: Vec::new(),
                concealing_content_types: Vec::new(),
                shortcut: row.get(20)?,
                is_transformed: row.get::<_, i32>(16)? != 0,
                pin_order: row.get(10)?,
                bin_id: bid,
                bin_ids: bid.map(|b| vec![b]),
                note: row.get(12)?,
                is_trashed: row.get::<_, i32>(13)? != 0,
                trashed_at: row.get(14)?,
                created_at: row.get(15)?,
                ocr_extractor_ref: row.get(17)?,
                ocr_extractor_name: row.get(18)?,
                ocr_engine_version: row.get(19)?,
            })
        })?;
        let mut clips = Vec::new();
        for clip in clip_iter {
            clips.push(clip?);
        }
        append_smart_bin_memberships(&conn, &mut clips)?;
        Ok(clips)
    }

    pub fn get_protected_clips(&self) -> Result<Vec<ClipItem>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare_cached(
            "SELECT id, content_type, text_content, NULL as html_content, NULL as image_base64, image_path, content_hash, source, is_pinned, is_protected, COALESCE(pin_order, 0), bin_id, note, is_trashed, trashed_at, created_at,
                    current_transformation_id IS NOT NULL,
                    ocr_extractor_ref, ocr_extractor_name, ocr_engine_version, shortcut
             FROM clips WHERE id IN (
                 SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1
             ) AND (is_trashed IS NULL OR is_trashed = 0) ORDER BY created_at DESC"
        )?;
        let clip_iter = stmt.query_map([], |row| {
            let bid: Option<i64> = row.get(11)?;
            Ok(ClipItem {
                id: row.get(0)?,
                name: None,
                content_type: row.get(1)?,
                content_types: Vec::new(),
                file_formats: Vec::new(),
                text_content: row.get(2)?,
                html_content: row.get(3)?,
                image_base64: row.get(4)?,
                image_path: row.get(5)?,
                content_hash: row.get(6)?,
                source: row.get(7)?,
                is_pinned: row.get::<_, i32>(8)? != 0,
                is_protected: row.get::<_, i32>(9)? != 0,
                is_explicitly_protected: Some(row.get::<_, i32>(9)? != 0),
                protecting_bin_ids: Vec::new(),
                is_concealed: false,
                is_explicitly_concealed: None,
                is_explicitly_revealed: false,
                concealing_bin_ids: Vec::new(),
                concealing_content_types: Vec::new(),
                shortcut: row.get(20)?,
                is_transformed: row.get::<_, i32>(16)? != 0,
                pin_order: row.get(10)?,
                bin_id: bid,
                bin_ids: bid.map(|b| vec![b]),
                note: row.get(12)?,
                is_trashed: row.get::<_, i32>(13)? != 0,
                trashed_at: row.get(14)?,
                created_at: row.get(15)?,
                ocr_extractor_ref: row.get(17)?,
                ocr_extractor_name: row.get(18)?,
                ocr_engine_version: row.get(19)?,
            })
        })?;
        let mut clips = Vec::new();
        for clip in clip_iter {
            clips.push(clip?);
        }
        append_smart_bin_memberships(&conn, &mut clips)?;
        Ok(clips)
    }
}
