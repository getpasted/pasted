use std::sync::Arc;
use tauri::State;

use crate::db::{DbState, SavedTransform, TransformClipApplication, TransformDefinition};
use crate::features::{self, Feature};

#[tauri::command]
pub fn get_bin_transform_ref(
    bin_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<String>, String> {
    db.get_bin_transform_ref(bin_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_bin_transform_ref(
    bin_id: i64,
    transform_ref: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    features::require(&db, Feature::Bins)?;
    features::require(&db, Feature::Transformations)?;
    db.set_bin_transform_ref(bin_id, transform_ref.as_deref())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_operations(db: State<'_, Arc<DbState>>) -> Result<Vec<crate::db::Operation>, String> {
    db.get_operations().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_intent_transforms(db: State<'_, Arc<DbState>>) -> Result<Vec<SavedTransform>, String> {
    db.get_intent_transforms()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_transforms(db: State<'_, Arc<DbState>>) -> Result<Vec<TransformDefinition>, String> {
    features::require(&db, Feature::Transformations)?;
    db.get_transform_definitions()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_saved_transform(
    name: String,
    plan: crate::transformation_intent::TransformationPlan,
    connection_id: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<SavedTransform, String> {
    features::require(&db, Feature::Transformations)?;
    let transform_name = if name.trim().is_empty() {
        plan.summary.trim()
    } else {
        name.trim()
    };
    let transform = db
        .create_saved_transform(transform_name, &plan, connection_id.as_deref())
        .map_err(|error| error.to_string())?;
    let _ = db.log_activity(
        "transform_saved",
        &format!("Saved Transform: {}", transform.name),
    );
    Ok(transform)
}

#[tauri::command]
pub fn update_saved_transform(
    transform_ref: String,
    name: String,
    plan: crate::transformation_intent::TransformationPlan,
    connection_id: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<SavedTransform, String> {
    features::require(&db, Feature::Transformations)?;
    let transform_name = if name.trim().is_empty() {
        plan.summary.trim()
    } else {
        name.trim()
    };
    let transform = db
        .update_saved_transform(
            &transform_ref,
            transform_name,
            &plan,
            connection_id.as_deref(),
        )
        .map_err(|error| error.to_string())?;
    let _ = db.log_activity(
        "transform_updated",
        &format!("Updated Transform: {}", transform.name),
    );
    Ok(transform)
}

#[tauri::command]
pub fn delete_saved_transform(
    transform_ref: String,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    features::require(&db, Feature::Transformations)?;
    db.delete_saved_transform(&transform_ref)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn apply_transform_preview_to_clip(
    clip_id: i64,
    transform_ref: String,
    expected_input: String,
    output: String,
    connection_id: Option<String>,
    duration_ms: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::db::ClipTransformationProvenance, String> {
    features::require(&db, Feature::Transformations)?;
    let provenance = db
        .apply_transform_output_to_clip(TransformClipApplication {
            clip_id,
            transform_ref: &transform_ref,
            expected_input: &expected_input,
            output: &output,
            connection_id: connection_id.as_deref(),
            duration_ms,
            bin_move: None,
        })
        .map_err(|error| error.to_string())?;
    let _ = db.log_activity(
        "clip_transformed",
        &format!(
            "Applied Transform {} to clip #{}",
            provenance.transform_name, clip_id
        ),
    );
    Ok(provenance)
}

#[tauri::command]
pub fn get_clip_transformation_provenance(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<crate::db::ClipTransformationProvenance>, String> {
    db.get_clip_transformation_provenance(clip_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_operation(
    name: String,
    op_type: String,
    config: Option<String>,
    category: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::db::Operation, String> {
    features::require(&db, Feature::Transformations)?;
    db.create_operation(&name, &op_type, config.as_deref(), category.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_operation(
    id: i64,
    name: String,
    op_type: String,
    config: Option<String>,
    category: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    features::require(&db, Feature::Transformations)?;
    db.update_operation(id, &name, &op_type, config.as_deref(), category.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn duplicate_operation(
    reference: String,
    name: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::db::Operation, String> {
    features::require(&db, Feature::Transformations)?;
    db.duplicate_operation(&reference, name.as_deref())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_operation(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    features::require(&db, Feature::Transformations)?;
    db.delete_operation(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn transform_text(
    input: String,
    filter_type: String,
    config: Option<String>,
) -> Result<String, String> {
    crate::transformation_service::execute_legacy_preview(&input, &filter_type, config.as_deref())
}

#[tauri::command]
pub async fn execute_transformation(
    request: crate::transformation_service::ExecutionRequest,
    db: State<'_, Arc<DbState>>,
) -> Result<
    crate::transformation_service::ExecutionOutcome,
    crate::transformation_service::ExecutionError,
> {
    if let Err(message) = features::require(&db, Feature::Transformations) {
        return Err(crate::transformation_service::ExecutionError {
            code: "feature_disabled",
            message,
            step: None,
            operation_ref: None,
        });
    }
    let cancellation = request
        .client_request_id
        .clone()
        .map(crate::transformation_service::CancellationRegistration::register);
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        crate::transformation_service::execute_with_cancellation(
            &db,
            request,
            cancellation
                .as_ref()
                .map(|registration| registration.flag()),
        )
    })
    .await
    .map_err(|error| crate::transformation_service::ExecutionError {
        code: "executor_join_failed",
        message: error.to_string(),
        step: None,
        operation_ref: None,
    })?
}

#[tauri::command]
pub fn cancel_transformation_execution(client_request_id: String) -> bool {
    crate::transformation_service::cancel_execution(&client_request_id)
}
