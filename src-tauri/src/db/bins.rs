use rusqlite::{params, Result};

use super::{
    derived_origin_kind, push_smart_condition, smart_bin_feature_policy, Bin, DbState,
    SmartBinFeaturePolicy,
};

type BinRow = (
    i64,
    String,
    String,
    String,
    Option<String>,
    String,
    Option<String>,
    bool,
    bool,
    String,
);

impl DbState {
    pub fn get_bins(&self) -> Result<Vec<Bin>> {
        let conn = self.conn.lock();
        let features = smart_bin_feature_policy(&conn)?;
        let mut stmt = conn.prepare("SELECT id, name, icon, color, smart_rule, COALESCE(bin_type, 'category'), shortcut, COALESCE(protect_clips, 0), COALESCE(conceal_clips, 0), created_at FROM bins ORDER BY id ASC")?;
        let bin_rows: Vec<BinRow> = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut bins = Vec::new();
        for (
            id,
            name,
            icon,
            color,
            smart_rule,
            bin_type,
            shortcut,
            protect_clips,
            conceal_clips,
            created_at,
        ) in bin_rows
        {
            let count: i64 = if let Some(ref sr_json) = smart_rule {
                if let Ok(parsed) = crate::smart_bins::parse_rule_json(sr_json) {
                    let join_op = if parsed.match_mode == "all" {
                        " AND "
                    } else {
                        " OR "
                    };

                    let mut cond_sqls: Vec<String> = Vec::new();
                    let mut query_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

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
                        let sql = format!("SELECT COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) AND (({}) OR bin_id = ? OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?))", combined);
                        query_params.push(Box::new(id));
                        query_params.push(Box::new(id));
                        let param_refs: Vec<&dyn rusqlite::ToSql> =
                            query_params.iter().map(|p| p.as_ref()).collect();
                        conn.query_row(&sql, param_refs.as_slice(), |r| r.get(0))
                            .unwrap_or(0)
                    } else {
                        conn.query_row("SELECT COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) AND (bin_id = ?1 OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?1))", params![id], |r| r.get(0)).unwrap_or(0)
                    }
                } else {
                    conn.query_row("SELECT COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) AND (bin_id = ?1 OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?1))", params![id], |r| r.get(0)).unwrap_or(0)
                }
            } else {
                conn.query_row("SELECT COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) AND (bin_id = ?1 OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?1))", params![id], |r| r.get(0)).unwrap_or(0)
            };

            let clip_order = {
                let mut order_statement = conn.prepare(
                    "SELECT clip_id FROM bin_clip_order WHERE bin_id = ?1 ORDER BY position ASC",
                )?;
                let ordered_ids = order_statement
                    .query_map(params![id], |row| row.get::<_, i64>(0))?
                    .collect::<Result<Vec<_>, _>>()?;
                ordered_ids
            };

            bins.push(Bin {
                id,
                name,
                icon,
                color,
                smart_rule,
                bin_type,
                shortcut,
                protect_clips,
                conceal_clips,
                clip_count: Some(count),
                clip_order,
                created_at,
            });
        }
        Ok(bins)
    }

    pub fn get_bin(&self, id: i64) -> Result<Bin> {
        self.get_bins()?
            .into_iter()
            .find(|bin| bin.id == id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn update_bin_hotkey(&self, id: i64, hotkey: Option<&str>) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE bins SET shortcut = ?1 WHERE id = ?2",
            params![hotkey, id],
        )?;
        Ok(())
    }

    pub fn update_bin_protection(&self, id: i64, protect_clips: bool) -> Result<()> {
        let conn = self.conn.lock();
        let is_smart: bool = conn.query_row(
            "SELECT smart_rule IS NOT NULL AND TRIM(smart_rule) <> '' FROM bins WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        if protect_clips && is_smart {
            return Err(rusqlite::Error::InvalidParameterName(
                "Smart Bins cannot apply inherited protection".into(),
            ));
        }
        conn.execute(
            "UPDATE bins SET protect_clips = ?1 WHERE id = ?2",
            params![protect_clips, id],
        )?;
        drop(conn);
        let activity_description = if protect_clips {
            format!("Enabled inherited protection for Bin #{id}")
        } else {
            format!("Disabled inherited protection for Bin #{id}")
        };
        let _ = self.log_activity("bin_protection_changed", &activity_description);
        Ok(())
    }

    pub fn get_clip_hotkeys(&self) -> Result<Vec<(i64, String)>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, shortcut FROM clips
             WHERE COALESCE(is_trashed, 0) = 0
               AND NULLIF(TRIM(shortcut), '') IS NOT NULL
             ORDER BY id ASC",
        )?;
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect();
        rows
    }

    pub fn get_bin_hotkeys(&self) -> Result<Vec<(i64, String, String)>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, name, shortcut FROM bins
             WHERE NULLIF(TRIM(shortcut), '') IS NOT NULL
             ORDER BY id ASC",
        )?;
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect();
        rows
    }

    pub fn get_pipeline_hotkeys(&self) -> Result<Vec<(String, String, String)>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, name, shortcut FROM saved_transforms
             WHERE authoring_kind = 'manual'
               AND NULLIF(TRIM(shortcut), '') IS NOT NULL
             ORDER BY row_id ASC",
        )?;
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect();
        rows
    }

    pub fn update_clip_hotkey(&self, clip_id: i64, hotkey: Option<&str>) -> Result<()> {
        let hotkey = hotkey.map(str::trim).filter(|value| !value.is_empty());
        if hotkey.is_some_and(|value| value.len() > 256) {
            return Err(rusqlite::Error::InvalidParameterName(
                "Clip hotkey is too long".into(),
            ));
        }
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE clips
             SET shortcut = ?1,
                 is_protected = CASE WHEN ?1 IS NOT NULL THEN 1 ELSE is_protected END
             WHERE id = ?2 AND COALESCE(is_trashed, 0) = 0",
            params![hotkey, clip_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    pub fn restore_clip_hotkey_state(
        &self,
        clip_id: i64,
        hotkey: Option<&str>,
        explicitly_protected: bool,
    ) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE clips SET shortcut = ?1, is_protected = ?2 WHERE id = ?3",
            params![hotkey, explicitly_protected, clip_id],
        )?;
        Ok(())
    }

    pub fn get_bin_transform_ref(&self, bin_id: i64) -> Result<Option<String>> {
        let conn = self.conn.lock();
        let transform_id: Option<String> = conn.query_row(
            "SELECT default_transform_id FROM bins WHERE id = ?1",
            params![bin_id],
            |row| row.get(0),
        )?;
        Ok(transform_id.map(|id| format!("transform:{id}")))
    }

    pub fn set_bin_transform_ref(&self, bin_id: i64, transform_ref: Option<&str>) -> Result<()> {
        let transform_id =
            transform_ref.map(|value| value.strip_prefix("transform:").unwrap_or(value));
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE bins SET default_transform_id = ?1 WHERE id = ?2",
            params![transform_id, bin_id],
        )?;
        Ok(())
    }

    pub fn matching_smart_bin_transforms(
        &self,
        clip_type: &str,
        file_formats: &[String],
        content_types: &[String],
        text: &str,
        source: &str,
    ) -> Result<Vec<(i64, String)>> {
        let features = SmartBinFeaturePolicy {
            clip_types: crate::features::is_enabled(self, crate::features::Feature::ClipTypes),
            content_types: crate::features::is_enabled(
                self,
                crate::features::Feature::ContentTypes,
            ),
            file_formats: crate::features::is_enabled(self, crate::features::Feature::FileFormats),
            sources: crate::features::is_enabled(self, crate::features::Feature::Sources),
        };
        let file_paths = if clip_type.eq_ignore_ascii_case("file") {
            serde_json::from_str::<Vec<String>>(text).unwrap_or_default()
        } else {
            Vec::new()
        };
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, smart_rule, default_transform_id FROM bins
             WHERE smart_rule IS NOT NULL AND default_transform_id IS NOT NULL ORDER BY id ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        let mut matches = Vec::new();
        for row in rows {
            let (bin_id, rule_json, transform_id) = row?;
            let Ok(rule) = crate::smart_bins::parse_rule_json(&rule_json) else {
                continue;
            };
            let condition_matches = |kind: &str, operator: &str, value: &str| {
                let contains = operator == "contains"
                    || (operator.is_empty() && matches!(kind, "source" | "contains" | "file_path"));
                let text_matches = |actual: &str| {
                    if contains {
                        actual.to_lowercase().contains(&value.to_lowercase())
                    } else {
                        actual.eq_ignore_ascii_case(value)
                    }
                };
                match kind {
                    "clip_type" => features.clip_types && text_matches(clip_type),
                    "content_type" => {
                        features.content_types
                            && content_types
                                .iter()
                                .any(|content_type| text_matches(content_type))
                    }
                    "file_format" => {
                        features.file_formats
                            && file_formats
                                .iter()
                                .any(|file_format| text_matches(file_format))
                    }
                    "source" => features.sources && text_matches(source),
                    "contains" => text.to_lowercase().contains(&value.to_lowercase()),
                    "origin_kind" => {
                        derived_origin_kind(clip_type, source).eq_ignore_ascii_case(value.trim())
                    }
                    "file_extension" => {
                        let extension = value.trim().trim_start_matches('.').to_lowercase();
                        !extension.is_empty()
                            && file_paths
                                .iter()
                                .any(|path| path.to_lowercase().ends_with(&format!(".{extension}")))
                    }
                    "file_path" => {
                        let value = value.trim().to_lowercase();
                        !value.is_empty()
                            && file_paths
                                .iter()
                                .any(|path| path.to_lowercase().contains(&value))
                    }
                    _ => false,
                }
            };
            let values = rule
                .conditions
                .iter()
                .map(|condition| {
                    condition_matches(&condition.target, &condition.operator, &condition.value)
                })
                .collect::<Vec<_>>();
            let matched = if rule.match_mode == "all" {
                values.iter().all(|value| *value)
            } else {
                values.iter().any(|value| *value)
            };
            if matched {
                matches.push((bin_id, format!("transform:{transform_id}")));
            }
        }
        Ok(matches)
    }

    pub fn create_bin_with_type(
        &self,
        name: &str,
        icon: &str,
        color: &str,
        smart_rule: Option<&str>,
        bin_type: &str,
    ) -> Result<Bin> {
        let smart_rule = smart_rule
            .map(crate::smart_bins::normalize_rule_json)
            .transpose()
            .map_err(rusqlite::Error::InvalidParameterName)?;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO bins (name, icon, color, smart_rule, bin_type) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![name, icon, color, smart_rule, bin_type],
        )?;
        let id = conn.last_insert_rowid();
        conn.query_row(
            "SELECT id, name, icon, color, smart_rule, COALESCE(bin_type, 'category'), shortcut, COALESCE(protect_clips, 0), COALESCE(conceal_clips, 0), created_at FROM bins WHERE id = ?1",
            params![id],
            |row| {
                Ok(Bin {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    icon: row.get(2)?,
                    color: row.get(3)?,
                    smart_rule: row.get(4)?,
                    bin_type: row.get(5)?,
                    shortcut: row.get(6)?,
                    protect_clips: row.get(7)?,
                    conceal_clips: row.get(8)?,
                    clip_count: Some(0),
                    clip_order: Vec::new(),
                    created_at: row.get(9)?,
                })
            },
        )
    }

    pub fn create_bin(
        &self,
        name: &str,
        icon: &str,
        color: &str,
        smart_rule: Option<&str>,
    ) -> Result<Bin> {
        self.create_bin_with_type(name, icon, color, smart_rule, "category")
    }

    pub fn reorder_pinned_clips(&self, ids: Vec<i64>) -> Result<()> {
        if ids.len() > 100_000 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Pinned order exceeds Pasted's safety limit".to_string(),
            ));
        }
        let requested = ids
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        if requested.len() != ids.len() {
            return Err(rusqlite::Error::InvalidParameterName(
                "Pinned order contains duplicate clips".to_string(),
            ));
        }
        let current = self
            .get_clips(None, true)?
            .into_iter()
            .map(|clip| clip.id)
            .collect::<std::collections::HashSet<_>>();
        if requested != current {
            return Err(rusqlite::Error::InvalidParameterName(
                "Pinned order must contain every current pinned clip exactly once".to_string(),
            ));
        }
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        for (idx, id) in ids.iter().enumerate() {
            tx.execute(
                "UPDATE clips SET pin_order = ?1 WHERE id = ?2",
                params![idx as i32, id],
            )?;
        }
        tx.commit()
    }

    pub fn reorder_bin_clips(&self, bin_id: i64, ids: Vec<i64>) -> Result<()> {
        if ids.len() > 100_000 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Bin order exceeds Pasted's safety limit".to_string(),
            ));
        }
        let unique = ids
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        if unique.len() != ids.len() {
            return Err(rusqlite::Error::InvalidParameterName(
                "Bin order contains duplicate clips".to_string(),
            ));
        }
        let current_ids = self
            .get_clips(Some(bin_id), false)?
            .into_iter()
            .map(|clip| clip.id)
            .collect::<std::collections::HashSet<_>>();
        if current_ids != unique {
            return Err(rusqlite::Error::InvalidParameterName(
                "Bin order must contain every current clip exactly once".to_string(),
            ));
        }

        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM bins WHERE id = ?1)",
            params![bin_id],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        tx.execute(
            "DELETE FROM bin_clip_order WHERE bin_id = ?1",
            params![bin_id],
        )?;
        for (position, clip_id) in ids.iter().enumerate() {
            tx.execute(
                "INSERT INTO bin_clip_order (bin_id, clip_id, position) VALUES (?1, ?2, ?3)",
                params![bin_id, clip_id, position as i64],
            )?;
        }
        self.log_activity_internal(
            &tx,
            "bin_clips_reordered",
            &format!("Reordered {} clips in Bin #{bin_id}", ids.len()),
        )?;
        tx.commit()
    }

    pub fn update_bin(
        &self,
        id: i64,
        name: &str,
        icon: &str,
        color: &str,
        smart_rule: Option<&str>,
    ) -> Result<()> {
        let smart_rule = smart_rule
            .map(crate::smart_bins::normalize_rule_json)
            .transpose()
            .map_err(rusqlite::Error::InvalidParameterName)?;
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE bins
             SET name = ?1, icon = ?2, color = ?3, smart_rule = ?4,
                 protect_clips = CASE WHEN ?4 IS NOT NULL THEN 0 ELSE protect_clips END,
                 conceal_clips = CASE WHEN ?4 IS NOT NULL THEN 0 ELSE conceal_clips END
             WHERE id = ?5",
            params![name, icon, color, smart_rule, id],
        )?;
        Ok(())
    }

    pub fn delete_bin(
        &self,
        id: i64,
        disposition: &str,
        destination_bin_id: Option<i64>,
    ) -> Result<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;

        let bin_name: String =
            tx.query_row("SELECT name FROM bins WHERE id = ?1", params![id], |row| {
                row.get(0)
            })?;
        let clip_ids = {
            let mut stmt = tx.prepare(
                "SELECT id FROM clips
                 WHERE (is_trashed IS NULL OR is_trashed = 0)
                   AND (bin_id = ?1 OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?1))",
            )?;
            let ids = stmt
                .query_map(params![id], |row| row.get::<_, i64>(0))?
                .collect::<Result<Vec<_>>>()?;
            ids
        };

        match disposition {
            "keep" => {
                for clip_id in &clip_ids {
                    tx.execute(
                        "UPDATE clips SET bin_id = NULL WHERE id = ?1 AND bin_id = ?2",
                        params![clip_id, id],
                    )?;
                }
            }
            "trash" => {
                for clip_id in &clip_ids {
                    let changed = tx.execute(
                        "UPDATE clips
                         SET is_trashed = 1,
                             trashed_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                         WHERE id = ?1 AND clips.id NOT IN (SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1)",
                        params![clip_id],
                    )?;
                    if changed > 0 {
                        self.clear_category_bin_assignments_internal(&tx, *clip_id)?;
                    } else {
                        tx.execute(
                            "UPDATE clips SET bin_id = NULL WHERE id = ?1 AND bin_id = ?2",
                            params![clip_id, id],
                        )?;
                    }
                }
                self.enforce_trash_limit_internal(&tx)?;
            }
            "move" => {
                let destination_id = destination_bin_id.ok_or_else(|| {
                    rusqlite::Error::InvalidParameterName(
                        "A destination Bin is required when moving clips".to_string(),
                    )
                })?;
                if destination_id == id {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "The destination Bin must be different from the deleted Bin".to_string(),
                    ));
                }
                let destination_exists = tx.query_row(
                    "SELECT EXISTS(
                         SELECT 1 FROM bins
                         WHERE id = ?1
                           AND (smart_rule IS NULL OR TRIM(smart_rule) = '')
                           AND COALESCE(bin_type, 'category') != 'tag'
                     )",
                    params![destination_id],
                    |row| row.get::<_, bool>(0),
                )?;
                if !destination_exists {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "The destination must be another manual Bin".to_string(),
                    ));
                }
                for clip_id in &clip_ids {
                    self.clear_category_bin_assignments_internal(&tx, *clip_id)?;
                    tx.execute(
                        "INSERT OR REPLACE INTO clip_bins (clip_id, bin_id) VALUES (?1, ?2)",
                        params![clip_id, destination_id],
                    )?;
                    tx.execute(
                        "UPDATE clips SET bin_id = ?1 WHERE id = ?2",
                        params![destination_id, clip_id],
                    )?;
                }
            }
            _ => {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Unknown Bin deletion outcome".to_string(),
                ));
            }
        }

        tx.execute("DELETE FROM clip_bins WHERE bin_id = ?1", params![id])?;
        tx.execute("DELETE FROM bins WHERE id = ?1", params![id])?;
        let outcome = match disposition {
            "trash" => "moved its clips to Trash",
            "move" => "moved its clips to another Bin",
            _ => "kept its clips in No Bin",
        };
        self.log_activity_internal(
            &tx,
            "bin_deleted",
            &format!(
                "Deleted Bin \"{}\" and {} ({} clips)",
                bin_name,
                outcome,
                clip_ids.len()
            ),
        )?;
        tx.commit()
    }
}
