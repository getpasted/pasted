use rusqlite::{params, Connection, Result};
use std::collections::HashMap;

use super::{
    add_column_if_missing, column_exists, describe_clip_ids, ClipItem, ClipMutationSummary, DbState,
};

pub(super) fn append_clip_concealment(conn: &Connection, clips: &mut [ClipItem]) -> Result<()> {
    if clips.is_empty() {
        return Ok(());
    }
    let ids = clips.iter().map(|clip| clip.id).collect::<Vec<_>>();
    let ids_json = serde_json::to_string(&ids)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    let mut concealment = HashMap::<i64, (bool, bool, bool, Vec<i64>, Vec<String>)>::new();
    let mut statement = conn.prepare(
        "SELECT clip_id, is_concealed, is_explicitly_concealed, is_explicitly_revealed,
                concealing_bin_ids, concealing_content_types
         FROM effective_clip_concealment
         WHERE clip_id IN (SELECT CAST(value AS INTEGER) FROM json_each(?1))",
    )?;
    for row in statement.query_map(params![ids_json], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i32>(1)? != 0,
            row.get::<_, i32>(2)? != 0,
            row.get::<_, i32>(3)? != 0,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
        ))
    })? {
        let (clip_id, effective, explicit, revealed, bin_ids, content_types) = row?;
        concealment.insert(
            clip_id,
            (
                effective,
                explicit,
                revealed,
                bin_ids
                    .unwrap_or_default()
                    .split(',')
                    .filter_map(|value| value.parse::<i64>().ok())
                    .collect(),
                content_types
                    .unwrap_or_default()
                    .split(',')
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .collect(),
            ),
        );
    }
    for clip in clips {
        if let Some((effective, explicit, revealed, bin_ids, content_types)) =
            concealment.remove(&clip.id)
        {
            clip.is_concealed = effective;
            clip.is_explicitly_concealed = Some(explicit);
            clip.is_explicitly_revealed = revealed;
            clip.concealing_bin_ids = bin_ids;
            clip.concealing_content_types = content_types;
        }
    }
    Ok(())
}

pub(super) fn configure_content_type_schema(conn: &Connection) -> Result<()> {
    let was_missing = !column_exists(conn, "content_types", "conceal_clips")?;
    add_column_if_missing(
        conn,
        "content_types",
        "conceal_clips",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    if was_missing {
        conn.execute(
            "UPDATE content_types SET conceal_clips = 1 WHERE id IN ('credential', 'payment_card', 'jwt')",
            [],
        )?;
    }
    Ok(())
}

pub(super) fn create_effective_view(conn: &Connection) -> Result<()> {
    // Concealment is presentation policy, not deletion protection. It can be
    // applied explicitly, by registered Content Types, or by durable manual Bin
    // membership. Smart-rule matches remain calculated and do not inherit it.
    conn.execute_batch(
        "DROP VIEW IF EXISTS effective_clip_concealment;
         CREATE VIEW effective_clip_concealment AS
         SELECT clips.id AS clip_id,
                CASE WHEN COALESCE(clips.is_revealed, 0) = 1 THEN 0
                     WHEN COALESCE(clips.is_concealed, 0) = 1
                          OR EXISTS (
                              SELECT 1 FROM bins
                              WHERE COALESCE(bins.conceal_clips, 0) = 1
                                AND (bins.id = clips.bin_id OR EXISTS (
                                    SELECT 1 FROM clip_bins
                                    WHERE clip_bins.clip_id = clips.id
                                      AND clip_bins.bin_id = bins.id
                                ))
                          )
                          OR EXISTS (
                              SELECT 1
                              FROM clip_analysis_classifications AS classifications
                              JOIN content_types ON content_types.id = classifications.content_type
                              WHERE classifications.clip_id = clips.id
                                AND classifications.input_hash = clips.content_hash
                                AND COALESCE(content_types.conceal_clips, 0) = 1
                          )
                          OR EXISTS (
                              SELECT 1 FROM content_types
                              WHERE content_types.id = clips.content_type
                                AND COALESCE(content_types.conceal_clips, 0) = 1
                          )
                     THEN 1 ELSE 0 END AS is_concealed,
                COALESCE(clips.is_concealed, 0) AS is_explicitly_concealed,
                COALESCE(clips.is_revealed, 0) AS is_explicitly_revealed,
                (SELECT GROUP_CONCAT(concealing.id)
                 FROM bins AS concealing
                 WHERE COALESCE(concealing.conceal_clips, 0) = 1
                   AND (concealing.id = clips.bin_id OR EXISTS (
                       SELECT 1 FROM clip_bins
                       WHERE clip_bins.clip_id = clips.id
                         AND clip_bins.bin_id = concealing.id
                   ))) AS concealing_bin_ids,
                (SELECT GROUP_CONCAT(content_type_id)
                 FROM (
                     SELECT DISTINCT classifications.content_type AS content_type_id
                     FROM clip_analysis_classifications AS classifications
                     JOIN content_types ON content_types.id = classifications.content_type
                     WHERE classifications.clip_id = clips.id
                       AND classifications.input_hash = clips.content_hash
                       AND COALESCE(content_types.conceal_clips, 0) = 1
                     UNION
                     SELECT content_types.id AS content_type_id
                     FROM content_types
                     WHERE content_types.id = clips.content_type
                       AND COALESCE(content_types.conceal_clips, 0) = 1
                 )) AS concealing_content_types
         FROM clips;",
    )?;
    Ok(())
}

impl DbState {
    pub fn toggle_concealed(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock();
        let concealed: bool = conn
            .query_row(
                "SELECT is_concealed FROM effective_clip_concealment WHERE clip_id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap_or(false);
        drop(conn);
        let next = !concealed;
        self.batch_conceal_clips(vec![id], next)?;
        Ok(next)
    }

    pub fn batch_conceal_clips(
        &self,
        ids: Vec<i64>,
        concealed_state: bool,
    ) -> Result<ClipMutationSummary> {
        let requested_count = ids.len();
        let conn = self.conn.lock();
        let mut changed_ids = Vec::new();
        for id in ids {
            let changed = if concealed_state {
                conn.execute(
                    "UPDATE clips SET is_concealed = 1, is_revealed = 0
                     WHERE id = ?1 AND (COALESCE(is_concealed, 0) != 1 OR COALESCE(is_revealed, 0) != 0)",
                    params![id],
                )?
            } else {
                conn.execute(
                    "UPDATE clips SET is_concealed = 0, is_revealed = 1
                     WHERE id = ?1 AND (COALESCE(is_concealed, 0) != 0 OR COALESCE(is_revealed, 0) != 1)",
                    params![id],
                )?
            };
            if changed > 0 {
                changed_ids.push(id);
            }
        }
        if !changed_ids.is_empty() {
            let event_type = if changed_ids.len() == 1 {
                "clip_concealment_toggled"
            } else {
                "clips_concealment_toggled"
            };
            let verb = if concealed_state {
                "Concealed"
            } else {
                "Revealed"
            };
            let _ = self.log_activity_internal(
                &conn,
                event_type,
                &format!("{} {}", verb, describe_clip_ids(&changed_ids)),
            );
        }
        Ok(ClipMutationSummary::new(
            if concealed_state { "conceal" } else { "reveal" },
            requested_count,
            changed_ids,
        ))
    }

    pub fn update_bin_concealment(&self, id: i64, conceal_clips: bool) -> Result<()> {
        let conn = self.conn.lock();
        let is_smart: bool = conn.query_row(
            "SELECT smart_rule IS NOT NULL AND TRIM(smart_rule) <> '' FROM bins WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        if conceal_clips && is_smart {
            return Err(rusqlite::Error::InvalidParameterName(
                "Smart Bins cannot apply inherited concealment".into(),
            ));
        }
        conn.execute(
            "UPDATE bins SET conceal_clips = ?1 WHERE id = ?2",
            params![conceal_clips, id],
        )?;
        drop(conn);
        let activity_description = if conceal_clips {
            format!("Enabled inherited concealment for Bin #{id}")
        } else {
            format!("Disabled inherited concealment for Bin #{id}")
        };
        let _ = self.log_activity("bin_concealment_changed", &activity_description);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::setup_test_db;

    #[test]
    fn concealment_is_effective_from_clips_content_types_and_manual_bins() {
        let db = setup_test_db();
        let explicit = db
            .save_clip(
                "text",
                Some("explicit"),
                None,
                None,
                "conceal-explicit",
                "Tests",
            )
            .unwrap();
        let typed = db
            .save_clip(
                "payment_card",
                Some("4111111111111111"),
                None,
                None,
                "conceal-typed",
                "Tests",
            )
            .unwrap();
        let inherited = db
            .save_clip(
                "text",
                Some("inherited"),
                None,
                None,
                "conceal-bin",
                "Tests",
            )
            .unwrap();
        let bin = db.create_bin("Private", "EyeOff", "default", None).unwrap();

        db.toggle_concealed(explicit.id).unwrap();
        db.update_bin_concealment(bin.id, true).unwrap();
        db.assign_to_bin(inherited.id, Some(bin.id)).unwrap();

        let explicit = db.get_clip_by_id(explicit.id).unwrap();
        assert!(explicit.is_concealed);
        assert_eq!(explicit.is_explicitly_concealed, Some(true));
        let typed = db.get_clip_by_id(typed.id).unwrap();
        assert!(typed.is_concealed);
        assert_eq!(typed.is_explicitly_concealed, Some(false));
        assert_eq!(typed.concealing_content_types, vec!["payment_card"]);
        let inherited = db.get_clip_by_id(inherited.id).unwrap();
        assert!(inherited.is_concealed);
        assert_eq!(inherited.is_explicitly_concealed, Some(false));
        assert_eq!(inherited.concealing_bin_ids, vec![bin.id]);
        assert_eq!(db.get_clip_collection_summary().unwrap().concealed_count, 3);

        assert!(!db.toggle_concealed(typed.id).unwrap());
        let revealed = db.get_clip_by_id(typed.id).unwrap();
        assert!(!revealed.is_concealed);
        assert_eq!(revealed.is_explicitly_concealed, Some(false));
        assert!(revealed.is_explicitly_revealed);
        assert_eq!(db.get_clip_collection_summary().unwrap().concealed_count, 2);
    }

    #[test]
    fn transfer_round_trip_preserves_clip_shortcuts_and_bin_policies() {
        let source = setup_test_db();
        let bin = source.create_bin("Durable", "🔐", "default", None).unwrap();
        source.update_bin_protection(bin.id, true).unwrap();
        source.update_bin_concealment(bin.id, true).unwrap();
        let clip = source
            .save_clip(
                "text",
                Some("portable shortcut"),
                None,
                None,
                "portable-clip-shortcut",
                "Tests",
            )
            .unwrap();
        source.toggle_concealed(clip.id).unwrap();
        source.assign_to_bin(clip.id, Some(bin.id)).unwrap();
        source
            .update_clip_hotkey(clip.id, Some("CmdOrCtrl+Shift+8"))
            .unwrap();

        assert_eq!(
            source
                .get_clip_by_id(clip.id)
                .unwrap()
                .is_explicitly_concealed,
            Some(true)
        );
        let revealed_clip = source
            .save_clip(
                "text",
                Some("portable reveal"),
                None,
                None,
                "portable-clip-reveal",
                "Tests",
            )
            .unwrap();
        source
            .assign_to_bin(revealed_clip.id, Some(bin.id))
            .unwrap();
        assert!(!source.toggle_concealed(revealed_clip.id).unwrap());
        let archive = source.export_backup_json().unwrap();
        let archive_value = serde_json::from_str::<serde_json::Value>(&archive).unwrap();
        let archived_clip = archive_value["clips"]
            .as_array()
            .unwrap()
            .iter()
            .find(|candidate| candidate["content_hash"] == "portable-clip-shortcut")
            .unwrap();
        assert_eq!(archived_clip["is_explicitly_concealed"], true);
        let archived_reveal = archive_value["clips"]
            .as_array()
            .unwrap()
            .iter()
            .find(|candidate| candidate["content_hash"] == "portable-clip-reveal")
            .unwrap();
        assert_eq!(archived_reveal["is_explicitly_revealed"], true);

        let destination = setup_test_db();
        destination.import_backup_json(&archive).unwrap();
        let restored_bin = destination
            .get_bins()
            .unwrap()
            .into_iter()
            .find(|candidate| candidate.name == "Durable")
            .unwrap();
        assert!(restored_bin.protect_clips);
        assert!(restored_bin.conceal_clips);
        let restored = destination
            .get_all_clips_for_backup()
            .unwrap()
            .into_iter()
            .find(|candidate| candidate.content_hash == "portable-clip-shortcut")
            .unwrap();
        assert_eq!(restored.shortcut.as_deref(), Some("CmdOrCtrl+Shift+8"));
        assert_eq!(restored.is_explicitly_protected, Some(true));
        assert_eq!(restored.is_explicitly_concealed, Some(true));
        let restored_reveal = destination
            .get_all_clips_for_backup()
            .unwrap()
            .into_iter()
            .find(|candidate| candidate.content_hash == "portable-clip-reveal")
            .unwrap();
        assert!(!restored_reveal.is_concealed);
        assert!(restored_reveal.is_explicitly_revealed);
        assert!(
            destination
                .get_clip_by_id(restored.id)
                .unwrap()
                .is_protected
        );
    }
}
