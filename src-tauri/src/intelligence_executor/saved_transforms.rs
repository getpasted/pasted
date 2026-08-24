use super::*;

fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    crate::hashing::finalize_sha256_hex(hasher)
}

pub struct SavedTransformExecutionContext<'a> {
    pub source_clip_id: Option<i64>,
    pub trigger_kind: &'a str,
    pub destination_kind: &'a str,
    pub client_request_id: Option<&'a str>,
}

pub fn execute_saved_transform(
    db: &DbState,
    transform_ref: &str,
    input: String,
    context: SavedTransformExecutionContext<'_>,
    cancellation: Option<&AtomicBool>,
) -> Result<(String, String, ExecutePlanOutcome), IntelligenceExecutionError> {
    let transform = db
        .resolve_saved_transform(transform_ref)
        .map_err(|error| IntelligenceExecutionError::new("database_error", error.to_string()))?
        .ok_or_else(|| {
            IntelligenceExecutionError::new(
                "unknown_transform",
                format!("Unknown Transform: {transform_ref}"),
            )
        })?;
    let transform_name = transform.name.clone();
    let execution_id = db
        .begin_transformation_execution(TransformationExecutionStart {
            target_kind: "transform",
            target_ref: &transform.stable_ref,
            target_revision: Some(transform.revision),
            source_clip_id: context.source_clip_id,
            trigger_kind: context.trigger_kind,
            destination_kind: context.destination_kind,
            input_hash: &content_hash(&input),
        })
        .map_err(|error| IntelligenceExecutionError::new("database_error", error.to_string()))?;
    db.start_transformation_execution(&execution_id)
        .map_err(|error| IntelligenceExecutionError::new("database_error", error.to_string()))?;
    let started = Instant::now();
    let result = execute_plan_with_cancellation(
        db,
        ExecutePlanRequest {
            plan: transform.plan,
            input,
            connection_id: transform.connection_id,
        },
        context.client_request_id,
        cancellation,
    );
    match result {
        Ok(outcome) => {
            db.finish_transformation_execution(
                &execution_id,
                outcome.duration_ms,
                Some(&content_hash(&outcome.output)),
                None,
            )
            .map_err(|error| {
                IntelligenceExecutionError::new("database_error", error.to_string())
            })?;
            Ok((transform_name, execution_id, outcome))
        }
        Err(error) => {
            let duration_ms = started.elapsed().as_millis().min(i64::MAX as u128) as i64;
            if error.code == "execution_cancelled" {
                let _ = db.cancel_transformation_execution(&execution_id, duration_ms);
            } else {
                let _ = db.finish_transformation_execution(
                    &execution_id,
                    duration_ms,
                    None,
                    Some(&format!("{}: {}", error.code, error.message)),
                );
            }
            Err(error)
        }
    }
}

pub fn apply_smart_bin_transforms_for_clip(
    db: &DbState,
    clip_id: i64,
    clip_type: &str,
    content_types: &[String],
    initial_text: &str,
    source: &str,
) {
    let file_formats = db
        .get_clip_by_id(clip_id)
        .map(|clip| clip.file_formats)
        .unwrap_or_default();
    let Ok(matches) = db.matching_smart_bin_transforms(
        clip_type,
        &file_formats,
        content_types,
        initial_text,
        source,
    ) else {
        return;
    };
    let mut current = initial_text.to_string();
    for (bin_id, transform_ref) in matches {
        let transform_name = db
            .resolve_saved_transform(&transform_ref)
            .ok()
            .flatten()
            .map(|transform| transform.name)
            .unwrap_or_else(|| transform_ref.clone());
        let result = execute_saved_transform(
            db,
            &transform_ref,
            current.clone(),
            SavedTransformExecutionContext {
                source_clip_id: Some(clip_id),
                trigger_kind: "bin",
                destination_kind: "replace",
                client_request_id: None,
            },
            None,
        );
        match result {
            Ok((transform_name, _execution_id, outcome)) if outcome.output != current => {
                if db
                    .apply_transform_output_to_clip(TransformClipApplication {
                        clip_id,
                        transform_ref: &transform_ref,
                        expected_input: &current,
                        output: &outcome.output,
                        connection_id: outcome.connection_id.as_deref(),
                        duration_ms: outcome.duration_ms,
                        bin_move: None,
                    })
                    .is_ok()
                {
                    current = outcome.output;
                    let _ = db.log_activity("bin_transform_executed", &format!("Applied Transform {transform_name} when clip #{clip_id} matched Smart Bin #{bin_id}"));
                }
            }
            Ok(_) => {}
            Err(error) => {
                let _ = db.log_activity(
                    "bin_transform_failed",
                    &format!(
                        "Transform {transform_name} failed for Smart Bin #{bin_id} ({})",
                        error.code
                    ),
                );
            }
        }
    }
}
