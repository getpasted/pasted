use rusqlite::{params, Result};

use super::super::{ensure_resource_size, ClipRevisionContext, ClipRevisionOrganization, DbState};
use super::{ClipTransformationProvenance, TransformClipApplication};

impl DbState {
    pub fn apply_transform_output_to_clip(
        &self,
        request: TransformClipApplication<'_>,
    ) -> Result<ClipTransformationProvenance> {
        let TransformClipApplication {
            clip_id,
            transform_ref,
            expected_input,
            output,
            connection_id,
            duration_ms,
            bin_move,
        } = request;
        ensure_resource_size(
            expected_input,
            crate::resource_limits::MAX_TRANSFORM_TEXT_BYTES,
            "Transform input",
        )?;
        ensure_resource_size(
            output,
            crate::resource_limits::MAX_TRANSFORM_TEXT_BYTES,
            "Transform output",
        )?;
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let transform_id = transform_ref
            .strip_prefix("transform:")
            .or_else(|| transform_ref.strip_prefix("pipeline:"))
            .unwrap_or(transform_ref)
            .to_string();
        let (transform_name, transform_revision): (String, i64) = tx.query_row(
            "SELECT name, revision FROM saved_transforms WHERE id = ?1",
            params![transform_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let canonical_transform_ref = format!("transform:{transform_id}");
        let transform_id = Some(transform_id);
        let (current_text, is_trashed, current_transformation_id): (
            Option<String>,
            i32,
            Option<String>,
        ) = tx.query_row(
            "SELECT text_content, COALESCE(is_trashed, 0), current_transformation_id FROM clips WHERE id = ?1",
            params![clip_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        if is_trashed != 0 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Restore this clip before transforming it".to_string(),
            ));
        }
        if current_text.as_deref() != Some(expected_input) {
            return Err(rusqlite::Error::InvalidParameterName(
                "The clip changed after this preview was generated; preview it again".to_string(),
            ));
        }
        if expected_input == output {
            return Err(rusqlite::Error::InvalidParameterName(
                "The Transform did not change the clip".to_string(),
            ));
        }
        let (action_label, organization) =
            if let Some((previous_bin_id, destination_bin_id)) = bin_move {
                let destination_name = tx
                    .query_row(
                        "SELECT name FROM bins WHERE id = ?1",
                        params![destination_bin_id],
                        |row| row.get::<_, String>(0),
                    )
                    .unwrap_or_else(|_| format!("Bin #{destination_bin_id}"));
                (
                    format!("Moved to {destination_name} · Applied {transform_name}"),
                    Some(ClipRevisionOrganization {
                        category_bin_id: previous_bin_id,
                    }),
                )
            } else {
                (format!("Applied {transform_name}"), None)
            };
        if Self::revision_history_enabled_internal(&tx) {
            let context_json = serde_json::to_string(&ClipRevisionContext {
                schema_version: 1,
                action_kind: if organization.is_some() {
                    "transform_bin_drop".to_string()
                } else {
                    "transform".to_string()
                },
                action_label,
                organization,
                current_transformation_id,
                derived_state: None,
            })
            .map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))?;
            tx.execute(
                "INSERT INTO clip_versions (clip_id, text_content, context_json) VALUES (?1, ?2, ?3)",
                params![clip_id, expected_input, context_json],
            )?;
            Self::prune_clip_versions_internal(&tx, clip_id)?;
        }
        let transformation_id: String =
            tx.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))?;
        tx.execute(
            "INSERT INTO clip_transformations
                (id, clip_id, transform_id, transform_ref, transform_name, transform_revision,
                 connection_id, duration_ms, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
                     strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
            params![
                transformation_id,
                clip_id,
                transform_id,
                canonical_transform_ref,
                transform_name,
                transform_revision,
                connection_id,
                duration_ms.max(0)
            ],
        )?;
        tx.execute(
            "UPDATE clips SET text_content = ?1, current_transformation_id = ?2 WHERE id = ?3",
            params![output, transformation_id, clip_id],
        )?;
        let created_at: String = tx.query_row(
            "SELECT created_at FROM clip_transformations WHERE rowid = last_insert_rowid()",
            [],
            |row| row.get(0),
        )?;
        tx.commit()?;
        Ok(ClipTransformationProvenance {
            transform_ref: canonical_transform_ref,
            transform_name,
            transform_revision,
            connection_id: connection_id.map(str::to_string),
            duration_ms: duration_ms.max(0),
            created_at,
        })
    }

    pub fn get_clip_transformation_provenance(
        &self,
        clip_id: i64,
    ) -> Result<Option<ClipTransformationProvenance>> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT transformation.transform_ref, transformation.transform_id,
                    transformation.transform_name,
                    transformation.transform_revision, transformation.connection_id,
                    transformation.duration_ms, transformation.created_at
             FROM clips
             JOIN clip_transformations transformation
               ON transformation.id = clips.current_transformation_id
             WHERE clips.id = ?1",
            params![clip_id],
            |row| {
                let transform_ref: Option<String> = row.get(0)?;
                let transform_id: Option<String> = row.get(1)?;
                Ok(ClipTransformationProvenance {
                    transform_ref: transform_ref
                        .or_else(|| transform_id.map(|id| format!("transform:{id}")))
                        .unwrap_or_else(|| "transform:deleted".to_string()),
                    transform_name: row.get(2)?,
                    transform_revision: row.get(3)?,
                    connection_id: row.get(4)?,
                    duration_ms: row.get(5)?,
                    created_at: row.get(6)?,
                })
            },
        );
        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error),
        }
    }
}
