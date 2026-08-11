use std::collections::HashMap;

use serde::Serialize;

use crate::db::{ClipItem, ClipMutationSummary, DbState, TransformClipApplication};
use crate::features::{self, Feature};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BinAssignmentOutcome {
    pub mutation: ClipMutationSummary,
    pub updated_clips: Vec<ClipItem>,
}

/// Assign clips to one category Bin and run that Bin's saved Transform through
/// the same workflow for GUI single-drop, GUI batch-drop, and CLI callers.
pub fn assign_clips_to_bin(
    db: &DbState,
    clip_ids: Vec<i64>,
    bin_id: Option<i64>,
) -> Result<BinAssignmentOutcome, String> {
    let previous_bins = clip_ids
        .iter()
        .filter_map(|clip_id| {
            db.get_clip_by_id(*clip_id)
                .ok()
                .map(|clip| (*clip_id, clip.bin_id))
        })
        .collect::<HashMap<_, _>>();
    let mutation = db
        .batch_assign_bin_clips(clip_ids, bin_id)
        .map_err(|error| error.to_string())?;
    let Some(bin_id) = bin_id else {
        return Ok(BinAssignmentOutcome {
            updated_clips: mutation
                .clip_ids
                .iter()
                .filter_map(|clip_id| db.get_clip_by_id(*clip_id).ok())
                .collect(),
            mutation,
        });
    };
    if !features::is_enabled(db, Feature::Transformations) {
        return Ok(BinAssignmentOutcome {
            updated_clips: mutation
                .clip_ids
                .iter()
                .filter_map(|clip_id| db.get_clip_by_id(*clip_id).ok())
                .collect(),
            mutation,
        });
    }
    let Some(transform_ref) = db
        .get_bin_transform_ref(bin_id)
        .map_err(|error| error.to_string())?
    else {
        return Ok(BinAssignmentOutcome {
            updated_clips: mutation
                .clip_ids
                .iter()
                .filter_map(|clip_id| db.get_clip_by_id(*clip_id).ok())
                .collect(),
            mutation,
        });
    };

    let mut updated_clips = Vec::new();
    for clip_id in mutation.clip_ids.iter().copied() {
        let clip = db
            .get_clip_by_id(clip_id)
            .map_err(|error| error.to_string())?;
        if clip.content_type == "file" {
            updated_clips.push(clip);
            continue;
        }
        let Some(input) = db
            .get_active_clip_text(clip_id)
            .map_err(|error| error.to_string())?
        else {
            continue;
        };
        let (transform_name, _execution_id, outcome) =
            crate::intelligence_executor::execute_saved_transform(
                db,
                &transform_ref,
                input.clone(),
                crate::intelligence_executor::SavedTransformExecutionContext {
                    source_clip_id: Some(clip_id),
                    trigger_kind: "bin",
                    destination_kind: "replace",
                    client_request_id: None,
                },
                None,
            )
            .map_err(|error| error.message)?;
        if outcome.output == input {
            let _ = db.log_activity(
                "bin_transform_no_change",
                &format!(
                    "Transform {} made no changes when clip #{} entered bin #{}",
                    transform_name, clip_id, bin_id
                ),
            );
            updated_clips.push(
                db.get_clip_by_id(clip_id)
                    .map_err(|error| error.to_string())?,
            );
            continue;
        }
        db.apply_transform_output_to_clip(TransformClipApplication {
            clip_id,
            transform_ref: &transform_ref,
            expected_input: &input,
            output: &outcome.output,
            connection_id: outcome.connection_id.as_deref(),
            duration_ms: outcome.duration_ms,
            bin_move: Some((previous_bins.get(&clip_id).copied().flatten(), bin_id)),
        })
        .map_err(|error| error.to_string())?;
        let _ = db.log_activity(
            "bin_transform_executed",
            &format!(
                "Applied Transform {} when clip #{} entered bin #{}",
                transform_name, clip_id, bin_id
            ),
        );
        updated_clips.push(
            db.get_clip_by_id(clip_id)
                .map_err(|error| error.to_string())?,
        );
    }

    Ok(BinAssignmentOutcome {
        mutation,
        updated_clips,
    })
}

pub fn remove_clips_from_bin(
    db: &DbState,
    clip_ids: Vec<i64>,
    bin_id: i64,
) -> Result<BinAssignmentOutcome, String> {
    let mutation = db
        .batch_remove_bin_clips(clip_ids, bin_id)
        .map_err(|error| error.to_string())?;
    let updated_clips = mutation
        .clip_ids
        .iter()
        .filter_map(|clip_id| db.get_clip_by_id(*clip_id).ok())
        .collect();
    Ok(BinAssignmentOutcome {
        mutation,
        updated_clips,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn assignment_without_a_transform_uses_the_shared_mutation_contract() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("pasted_bin_assignment_{nonce}.db"));
        let db = Arc::new(DbState::new(path.clone()).unwrap());
        let clip = db
            .save_clip("text", Some("Hello"), None, None, "assignment", "Test")
            .unwrap();
        let bin = db.create_bin("Manual", "Folder", "#3b82f6", None).unwrap();

        let outcome = assign_clips_to_bin(&db, vec![clip.id], Some(bin.id)).unwrap();
        assert_eq!(outcome.mutation.action, "assign_bin");
        assert_eq!(outcome.mutation.changed_count, 1);
        assert_eq!(outcome.updated_clips.len(), 1);
        assert!(outcome.updated_clips[0]
            .bin_ids
            .as_ref()
            .unwrap()
            .contains(&bin.id));
        assert_eq!(db.get_clip_by_id(clip.id).unwrap().bin_id, Some(bin.id));

        drop(db);
        let _ = std::fs::remove_file(path);
    }
}
