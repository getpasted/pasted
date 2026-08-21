use std::sync::Arc;
use tauri::{AppHandle, State};

use super::{register_all_app_shortcuts, register_changed_hotkeys};
use crate::db::DbState;
use crate::features::{self, Feature};

#[tauri::command]
pub fn get_manual_transforms(
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::manual_transform_service::ManualTransform>, String> {
    crate::manual_transform_service::list(&db).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_manual_transform(
    name: String,
    steps: Vec<crate::manual_transform_service::ManualTransformStepInput>,
    hotkey: Option<String>,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<crate::manual_transform_service::ManualTransform, String> {
    features::require(&db, Feature::Transformations)?;
    let has_hotkey = hotkey
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    if has_hotkey {
        features::require(&db, Feature::Hotkeys)?;
    }
    let manual_transform =
        crate::manual_transform_service::create(&db, &name, &steps, hotkey.as_deref())
            .map_err(|error| error.to_string())?;
    if has_hotkey {
        let _ = register_all_app_shortcuts(&app);
    }
    Ok(manual_transform)
}

#[tauri::command]
pub fn update_manual_transform(
    transform_ref: String,
    name: String,
    steps: Vec<crate::manual_transform_service::ManualTransformStepInput>,
    hotkey: Option<String>,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<crate::manual_transform_service::ManualTransform, String> {
    features::require(&db, Feature::Transformations)?;
    let previous_shortcut = db
        .resolve_transform_definition(&transform_ref)
        .map_err(|error| error.to_string())?
        .and_then(|definition| definition.shortcut);
    let hotkey_changed =
        hotkey.as_deref().map(str::trim) != previous_shortcut.as_deref().map(str::trim);
    if hotkey_changed {
        features::require(&db, Feature::Hotkeys)?;
    }
    let manual_transform = crate::manual_transform_service::update(
        &db,
        &transform_ref,
        &name,
        &steps,
        hotkey.as_deref(),
    )
    .map_err(|error| error.to_string())?;
    if hotkey_changed {
        let _ = register_all_app_shortcuts(&app);
    }
    Ok(manual_transform)
}

#[tauri::command]
pub fn update_manual_transform_hotkey(
    transform_ref: String,
    hotkey: Option<String>,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<(), String> {
    features::require(&db, Feature::Transformations)?;
    features::require(&db, Feature::Hotkeys)?;
    let previous = crate::manual_transform_service::list(&db)
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|manual_transform| {
            manual_transform.stable_ref == transform_ref
                || manual_transform.stable_ref.strip_prefix("transform:")
                    == transform_ref
                        .strip_prefix("pipeline:")
                        .or_else(|| transform_ref.strip_prefix("transform:"))
        })
        .ok_or_else(|| "Transform not found.".to_string())?
        .shortcut;
    crate::manual_transform_service::update_shortcut(&db, &transform_ref, hotkey.as_deref())
        .map_err(|error| error.to_string())?;
    let changed_hotkeys: Vec<String> = hotkey.clone().into_iter().collect();
    if let Err(error) = register_changed_hotkeys(&app, &changed_hotkeys) {
        crate::manual_transform_service::update_shortcut(&db, &transform_ref, previous.as_deref())
            .map_err(|rollback| {
                format!("{error}; restoring the previous Transform hotkey failed: {rollback}")
            })?;
        let _ = register_all_app_shortcuts(&app);
        return Err(error);
    }
    Ok(())
}

#[tauri::command]
pub fn delete_manual_transform(
    transform_ref: String,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<(), String> {
    features::require(&db, Feature::Transformations)?;
    let had_hotkey = db
        .resolve_transform_definition(&transform_ref)
        .map_err(|error| error.to_string())?
        .and_then(|definition| definition.shortcut)
        .is_some_and(|value| !value.trim().is_empty());
    crate::manual_transform_service::delete(&db, &transform_ref)
        .map_err(|error| error.to_string())?;
    if had_hotkey {
        let _ = register_all_app_shortcuts(&app);
    }
    Ok(())
}

#[tauri::command]
pub async fn preview_manual_transform_steps(
    input: String,
    steps: Vec<crate::manual_transform_service::ManualTransformStepInput>,
    client_request_id: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<String, crate::transformation_service::ExecutionError> {
    if let Err(message) = features::require(&db, Feature::Transformations) {
        return Err(crate::transformation_service::ExecutionError {
            code: "feature_disabled",
            message,
            step: None,
            operation_ref: None,
        });
    }
    let cancellation = client_request_id
        .clone()
        .map(crate::transformation_service::CancellationRegistration::register);
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        crate::transformation_service::preview_manual_transform_steps(
            &db,
            &input,
            &steps,
            client_request_id.as_deref(),
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
