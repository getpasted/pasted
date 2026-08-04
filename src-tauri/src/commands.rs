use arboard::Clipboard;
use base64::Engine;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::{
    Bin, ClipItem, DbState, IntelligenceConnection, IntelligenceConnectionUpdate, Pipeline,
    PipelineStepInput, SavedTransform, TransformClipApplication, TransformationExecution,
};
use crate::sequential_paste::{SequentialQueueState, SequentialStatus};

#[tauri::command]
pub fn get_clips(
    search_query: Option<String>,
    bin_id: Option<i64>,
    only_pinned: bool,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<ClipItem>, String> {
    db.get_clips(search_query.as_deref(), bin_id, only_pinned)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_total_clip_count(db: State<'_, Arc<DbState>>) -> Result<i64, String> {
    db.get_total_clip_count().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_clip_image(db: State<'_, Arc<DbState>>, id: i64) -> Result<Option<String>, String> {
    db.get_clip_image(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_trashed_clips(db: State<'_, Arc<DbState>>) -> Result<Vec<ClipItem>, String> {
    db.get_trashed_clips().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn restore_clip(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.restore_clip(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn purge_clip_permanently(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.purge_clip_permanently(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn empty_trash(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.empty_trash().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_activity_logs(
    limit: Option<i64>,
    offset: Option<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::db::ActivityLog>, String> {
    db.get_activity_logs(limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_activity_logs(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.clear_activity_logs().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_app_setting(
    key: String,
    value: String,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.save_setting(&key, &value).map_err(|e| e.to_string())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub fn play_system_sound(sound_id: Option<u32>) {
    let id = sound_id.unwrap_or(1057);
    unsafe {
        #[link(name = "AudioToolbox", kind = "framework")]
        extern "C" {
            fn AudioServicesPlaySystemSound(sound_id: u32);
        }
        AudioServicesPlaySystemSound(id);
    }
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn play_system_sound(_sound_id: Option<u32>) {}

#[tauri::command]
pub fn get_app_setting(key: String, db: State<'_, Arc<DbState>>) -> Result<Option<String>, String> {
    db.get_setting(&key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_all_app_settings(
    db: State<'_, Arc<DbState>>,
) -> Result<std::collections::HashMap<String, String>, String> {
    db.get_all_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn enforce_clip_retention(keep_count: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.purge_old_clips(keep_count).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn enforce_revision_retention(
    keep_count: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.enforce_revision_retention(keep_count)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_clip_note(
    clip_id: i64,
    note: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.update_clip_note(clip_id, note.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_clip_text(
    clip_id: i64,
    text: String,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.update_clip_text(clip_id, &text)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_clip(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.delete_clip(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_pin_clip(id: i64, db: State<'_, Arc<DbState>>) -> Result<bool, String> {
    db.toggle_pin(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn assign_clip_bin(
    clip_id: i64,
    bin_id: Option<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<ClipItem>, String> {
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        let previous_category_bin_id = db
            .get_clip_by_id(clip_id)
            .map_err(|error| error.to_string())?
            .bin_id;
        db.assign_to_bin(clip_id, bin_id)
            .map_err(|error| error.to_string())?;
        let Some(bin_id) = bin_id else {
            return Ok(None);
        };
        let Some(transform_ref) = db
            .get_bin_transform_ref(bin_id)
            .map_err(|error| error.to_string())?
        else {
            return Ok(None);
        };
        let Some(input) = db
            .get_active_clip_text(clip_id)
            .map_err(|error| error.to_string())?
        else {
            return Ok(None);
        };
        let (transform_name, outcome) = crate::intelligence_executor::execute_saved_transform(
            &db,
            &transform_ref,
            input.clone(),
            Some(clip_id),
            "bin",
            "replace",
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
            return db
                .get_clip_by_id(clip_id)
                .map(Some)
                .map_err(|error| error.to_string());
        }
        db.apply_transform_output_to_clip(TransformClipApplication {
            clip_id,
            transform_ref: &transform_ref,
            expected_input: &input,
            output: &outcome.output,
            connection_id: outcome.connection_id.as_deref(),
            duration_ms: outcome.duration_ms,
            bin_move: Some((previous_category_bin_id, bin_id)),
        })
        .map_err(|error| error.to_string())?;
        let _ = db.log_activity(
            "bin_transform_executed",
            &format!(
                "Applied Transform {} when clip #{} entered bin #{}",
                transform_name, clip_id, bin_id
            ),
        );
        db.get_clip_by_id(clip_id)
            .map(Some)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn add_clip_to_bin(
    clip_id: i64,
    bin_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.add_clip_to_bin(clip_id, bin_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_clip_from_bin(
    clip_id: i64,
    bin_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.remove_clip_from_bin(clip_id, bin_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_pinned_clips(ids: Vec<i64>, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.reorder_pinned_clips(ids).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_clip_versions(
    clip_id: i64,
    limit: Option<i64>,
    offset: Option<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::db::ClipVersion>, String> {
    db.get_clip_versions_page(
        clip_id,
        limit.unwrap_or(50).clamp(1, 100),
        offset.unwrap_or(0).max(0),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_clip_version_count(clip_id: i64, db: State<'_, Arc<DbState>>) -> Result<i64, String> {
    db.get_clip_version_count(clip_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn restore_clip_version(
    clip_id: i64,
    version_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<ClipItem, String> {
    db.restore_clip_version(clip_id, version_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_tag(
    name: String,
    color: String,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::db::Bin, String> {
    db.create_bin_with_type(&name, "Tag", &color, None, "tag")
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn batch_pin_clips(
    ids: Vec<i64>,
    pin_state: bool,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.batch_pin_clips(ids, pin_state)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn batch_trash_clips(ids: Vec<i64>, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.batch_trash_clips(ids).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn batch_assign_bin_clips(
    ids: Vec<i64>,
    bin_id: Option<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.batch_assign_bin_clips(ids, bin_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_backup_json(db: State<'_, Arc<DbState>>) -> Result<String, String> {
    db.export_backup_json().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_backup_json(json_str: String, db: State<'_, Arc<DbState>>) -> Result<usize, String> {
    db.import_backup_json(&json_str).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_vault_passcode(passcode: String, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.set_vault_passcode(&passcode).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn verify_vault_passcode(
    passcode: String,
    db: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    db.verify_vault_passcode(&passcode)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn copy_clip_to_system(
    text: Option<String>,
    image_base64: Option<String>,
) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;

    if let Some(t) = text {
        clipboard.set_text(t).map_err(|e| e.to_string())?;
    } else if let Some(img_b64) = image_base64 {
        // Strip data:image/png;base64,
        let clean = img_b64.split(',').next_back().unwrap_or(&img_b64);
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, clean)
            .map_err(|e| e.to_string())?;

        let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
        let rgba = img.to_rgba8();
        let img_data = arboard::ImageData {
            width: rgba.width() as usize,
            height: rgba.height() as usize,
            bytes: std::borrow::Cow::Owned(rgba.into_raw()),
        };
        clipboard.set_image(img_data).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn paste_text_to_frontmost(text: String, app: AppHandle) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|error| error.to_string())?;
    clipboard
        .set_text(text)
        .map_err(|error| error.to_string())?;

    if let Some(hud) = app.get_webview_window("hud") {
        let _ = hud.hide();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.hide();
    }

    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(50));
        simulate_cmd_v_paste();
    });

    Ok(())
}

pub(crate) fn execute_clipboard_pipeline(
    db: &DbState,
    pipeline_ref: Option<&str>,
    paste_result: bool,
) -> Result<crate::transformation_service::ExecutionOutcome, String> {
    let mut clipboard = Clipboard::new().map_err(|error| error.to_string())?;
    let input = clipboard.get_text().map_err(|error| error.to_string())?;
    let outcome = crate::transformation_service::execute_shortcut_pipeline(db, input, pipeline_ref)
        .map_err(|error| error.to_string())?;
    clipboard
        .set_text(&outcome.output)
        .map_err(|error| error.to_string())?;
    if paste_result {
        thread::spawn(|| {
            thread::sleep(Duration::from_millis(50));
            simulate_cmd_v_paste();
        });
    }
    Ok(outcome)
}

#[tauri::command]
pub fn copy_with_last_pipeline(
    db: State<'_, Arc<DbState>>,
) -> Result<crate::transformation_service::ExecutionOutcome, String> {
    execute_clipboard_pipeline(&db, None, false)
}

#[tauri::command]
pub fn paste_with_last_pipeline(
    db: State<'_, Arc<DbState>>,
) -> Result<crate::transformation_service::ExecutionOutcome, String> {
    execute_clipboard_pipeline(&db, None, true)
}

#[tauri::command]
pub fn paste_with_pipeline(
    pipeline_ref: String,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::transformation_service::ExecutionOutcome, String> {
    execute_clipboard_pipeline(&db, Some(&pipeline_ref), true)
}

#[tauri::command]
pub fn get_last_pipeline_ref(db: State<'_, Arc<DbState>>) -> Result<Option<String>, String> {
    crate::transformation_service::get_last_pipeline_ref(&db).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_bins(db: State<'_, Arc<DbState>>) -> Result<Vec<Bin>, String> {
    db.get_bins().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_bin(
    name: String,
    icon: String,
    color: String,
    smart_rule: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<Bin, String> {
    db.create_bin(&name, &icon, &color, smart_rule.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_bin(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.delete_bin(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_bin(
    id: i64,
    name: String,
    icon: String,
    color: String,
    smart_rule: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.update_bin(id, &name, &icon, &color, smart_rule.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_pipelines(db: State<'_, Arc<DbState>>) -> Result<Vec<Pipeline>, String> {
    db.get_pipelines().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_pipeline(
    name: String,
    steps: Vec<PipelineStepInput>,
    shortcut: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<Pipeline, String> {
    db.create_pipeline(&name, &steps, shortcut.as_deref())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_pipeline(
    pipeline_ref: String,
    name: String,
    steps: Vec<PipelineStepInput>,
    shortcut: Option<String>,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<Pipeline, String> {
    let pipeline = db
        .update_pipeline(&pipeline_ref, &name, &steps, shortcut.as_deref())
        .map_err(|error| error.to_string())?;
    let _ = register_all_app_shortcuts(&app);
    Ok(pipeline)
}

#[tauri::command]
pub fn update_pipeline_shortcut(
    pipeline_ref: String,
    shortcut: Option<String>,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<(), String> {
    db.update_pipeline_shortcut(&pipeline_ref, shortcut.as_deref())
        .map_err(|error| error.to_string())?;
    let _ = register_all_app_shortcuts(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_pipeline(pipeline_ref: String, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.delete_pipeline(&pipeline_ref)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_bin_shortcut(
    id: i64,
    shortcut: Option<String>,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<(), String> {
    db.update_bin_shortcut(id, shortcut.as_deref())
        .map_err(|e| e.to_string())?;
    let _ = register_all_app_shortcuts(&app);
    Ok(())
}

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
    db.set_bin_transform_ref(bin_id, transform_ref.as_deref())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_operations(db: State<'_, Arc<DbState>>) -> Result<Vec<crate::db::Operation>, String> {
    db.get_operations().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_builtin_operations() -> Vec<crate::operation_registry::OperationDefinition> {
    crate::operation_registry::builtin_operations()
}

#[tauri::command]
pub fn get_intelligence_connections(
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<IntelligenceConnection>, String> {
    db.get_intelligence_connections()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn detect_intelligence_connections(
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::intelligence_connections::DetectedIntelligenceConnection>, String> {
    let detected = crate::intelligence_connections::detect_intelligence_connections();
    for candidate in &detected {
        let endpoint = if candidate.provider_kind == "cli" {
            candidate.executable_path.as_deref()
        } else {
            candidate.default_endpoint
        };
        db.ensure_intelligence_connection_candidate(
            candidate.name,
            candidate.provider_kind,
            endpoint,
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(detected)
}

#[tauri::command]
pub fn create_intelligence_connection(
    name: String,
    provider_kind: String,
    endpoint: Option<String>,
    model: Option<String>,
    credential_ref: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<IntelligenceConnection, String> {
    if name.trim().is_empty() {
        return Err("Connection name cannot be empty".to_string());
    }
    db.create_intelligence_connection(
        &name,
        &provider_kind,
        endpoint.as_deref(),
        model.as_deref(),
        credential_ref.as_deref(),
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Preserve the established flat Tauri IPC contract.
pub fn update_intelligence_connection(
    id: String,
    name: String,
    provider_kind: String,
    endpoint: Option<String>,
    model: Option<String>,
    credential_ref: Option<String>,
    enabled: bool,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Connection name cannot be empty".to_string());
    }
    db.update_intelligence_connection(IntelligenceConnectionUpdate {
        id: &id,
        name: &name,
        provider_kind: &provider_kind,
        endpoint: endpoint.as_deref(),
        model: model.as_deref(),
        credential_ref: credential_ref.as_deref(),
        enabled,
    })
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_intelligence_connection(
    id: String,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.delete_intelligence_connection(&id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn reorder_intelligence_connections(
    ids: Vec<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.reorder_intelligence_connections(&ids)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn validate_transformation_plan(
    plan: crate::transformation_intent::TransformationPlan,
) -> Result<crate::transformation_intent::ExecutionCharacter, String> {
    plan.validate()?;
    Ok(plan.execution_character())
}

#[tauri::command]
pub async fn plan_transformation_intent(
    request: crate::intelligence_executor::PlanIntentRequest,
    db: State<'_, Arc<DbState>>,
) -> Result<
    crate::intelligence_executor::PlanIntentOutcome,
    crate::intelligence_executor::IntelligenceExecutionError,
> {
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        let result = crate::intelligence_executor::plan_intent(&db, request);
        match &result {
            Ok(outcome) => {
                let _ = db.log_activity(
                    "transform_drafted",
                    &format!(
                        "Drafted a {}-step Transform with {} in {} ms",
                        outcome.plan.steps.len(),
                        outcome.connection_name,
                        outcome.duration_ms
                    ),
                );
            }
            Err(error) => {
                let _ = db.log_activity(
                    "transform_draft_failed",
                    &format!("Transform draft failed ({})", error.code),
                );
            }
        }
        result
    })
    .await
    .map_err(
        |error| crate::intelligence_executor::IntelligenceExecutionError {
            code: "executor_join_failed",
            message: error.to_string(),
        },
    )?
}

#[tauri::command]
pub async fn test_transformation_plan(
    request: crate::intelligence_executor::ExecutePlanRequest,
    db: State<'_, Arc<DbState>>,
) -> Result<
    crate::intelligence_executor::ExecutePlanOutcome,
    crate::intelligence_executor::IntelligenceExecutionError,
> {
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        let result = crate::intelligence_executor::execute_plan(&db, request);
        match &result {
            Ok(outcome) => {
                let provider = outcome
                    .connection_name
                    .as_deref()
                    .unwrap_or("local Operations");
                let _ = db.log_activity(
                    "transform_tested",
                    &format!(
                        "Tested a Transform with {provider} in {} ms",
                        outcome.duration_ms
                    ),
                );
            }
            Err(error) => {
                let _ = db.log_activity(
                    "transform_test_failed",
                    &format!("Transform test failed ({})", error.code),
                );
            }
        }
        result
    })
    .await
    .map_err(
        |error| crate::intelligence_executor::IntelligenceExecutionError {
            code: "executor_join_failed",
            message: error.to_string(),
        },
    )?
}

#[tauri::command]
pub fn get_saved_transforms(db: State<'_, Arc<DbState>>) -> Result<Vec<SavedTransform>, String> {
    db.get_saved_transforms().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_saved_transform(
    name: String,
    plan: crate::transformation_intent::TransformationPlan,
    connection_id: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<SavedTransform, String> {
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
    db.delete_saved_transform(&transform_ref)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn execute_saved_transform(
    transform_ref: String,
    input: String,
    db: State<'_, Arc<DbState>>,
) -> Result<
    crate::intelligence_executor::ExecutePlanOutcome,
    crate::intelligence_executor::IntelligenceExecutionError,
> {
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        let result = crate::intelligence_executor::execute_saved_transform(
            &db,
            &transform_ref,
            input,
            None,
            "manual",
            "preview",
        );
        match &result {
            Ok((transform_name, outcome)) => {
                let _ = db.log_activity(
                    "transform_executed",
                    &format!(
                        "Ran Transform: {} in {} ms",
                        transform_name, outcome.duration_ms
                    ),
                );
            }
            Err(error) => {
                let _ = db.log_activity(
                    "transform_execution_failed",
                    &format!("Transform failed: {} ({})", transform_ref, error.code),
                );
            }
        }
        result.map(|(_, outcome)| outcome)
    })
    .await
    .map_err(
        |error| crate::intelligence_executor::IntelligenceExecutionError {
            code: "executor_join_failed",
            message: error.to_string(),
        },
    )?
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
pub fn get_clip_transformation_executions(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<TransformationExecution>, String> {
    db.get_clip_transformation_executions(clip_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_operation_plugin_examples() -> Vec<crate::operation_plugins::OperationPluginManifest> {
    crate::operation_plugins::bundled_example_plugins()
}

#[tauri::command]
pub fn create_operation(
    name: String,
    op_type: String,
    config: Option<String>,
    category: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::db::Operation, String> {
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
    db.update_operation(id, &name, &op_type, config.as_deref(), category.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_operation(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
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
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        crate::transformation_service::execute(&db, request)
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
pub fn clear_history(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.clear_history().map_err(|e| e.to_string())
}

// Sequential Paste Commands
#[tauri::command]
pub fn start_sequential_paste(
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<SequentialStatus, String> {
    seq.start_queue();
    let status = seq.get_status();
    let _ = app.emit("sequential-updated", status.clone());
    Ok(status)
}

#[tauri::command]
pub fn push_sequential_item(
    item: String,
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<SequentialStatus, String> {
    seq.push_item(item);
    let status = seq.get_status();
    let _ = app.emit("sequential-updated", status.clone());
    Ok(status)
}

#[cfg(target_os = "macos")]
pub fn simulate_cmd_v_paste() {
    use std::process::Command;
    let _ = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to keystroke \"v\" using command down")
        .spawn();
}

#[cfg(target_os = "windows")]
pub fn simulate_cmd_v_paste() {
    use std::process::Command;
    let _ = Command::new("powershell")
        .arg("-Command")
        .arg("$wshell = New-Object -ComObject wscript.shell; $wshell.SendKeys('^v')")
        .spawn();
}

#[cfg(target_os = "linux")]
pub fn simulate_cmd_v_paste() {
    use std::process::Command;
    let _ = Command::new("xdotool").arg("key").arg("ctrl+v").spawn();
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub fn simulate_cmd_v_paste() {}

#[tauri::command]
pub fn pop_sequential_paste(
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<Option<String>, String> {
    let item = seq.pop_next();
    if let Some(ref text) = item {
        if let Ok(mut cb) = Clipboard::new() {
            let _ = cb.set_text(text);
        }

        // Hide main window if visible so focus returns to target app
        if let Some(main_win) = app.get_webview_window("main") {
            if main_win.is_visible().unwrap_or(false) {
                let _ = main_win.hide();
            }
        }

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            simulate_cmd_v_paste();
        });
    }
    let status = seq.get_status();
    let _ = app.emit("sequential-updated", status);
    Ok(item)
}

#[tauri::command]
pub fn remove_sequential_item_by_index(
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
    index: usize,
) -> Result<SequentialStatus, String> {
    let _ = seq.remove_item_by_index(index);
    let status = seq.get_status();
    let _ = app.emit("sequential-updated", status.clone());
    Ok(status)
}

#[tauri::command]
pub fn stop_sequential_paste(
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<SequentialStatus, String> {
    seq.stop_queue();
    let status = seq.get_status();
    let _ = app.emit("sequential-updated", status.clone());
    Ok(status)
}

#[tauri::command]
pub fn paste_all_sequential(
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<Option<String>, String> {
    let status = seq.get_status();
    if status.queue.is_empty() {
        return Ok(None);
    }
    let combined = status.queue.join("\n\n");
    if let Ok(mut cb) = Clipboard::new() {
        let _ = cb.set_text(&combined);
    }
    seq.clear_queue();
    let updated = seq.get_status();
    let _ = app.emit("sequential-updated", updated);

    // Hide main window if visible so focus returns to target app
    if let Some(main_win) = app.get_webview_window("main") {
        if main_win.is_visible().unwrap_or(false) {
            let _ = main_win.hide();
        }
    }

    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        simulate_cmd_v_paste();
    });

    Ok(Some(combined))
}

#[tauri::command]
pub fn get_sequential_status(
    seq: State<'_, Arc<SequentialQueueState>>,
) -> Result<SequentialStatus, String> {
    Ok(seq.get_status())
}

// Window & Activation Policy Commands
#[tauri::command]
pub fn toggle_quick_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
    Ok(())
}

#[tauri::command]
pub fn toggle_hud_window(app: AppHandle) -> Result<(), String> {
    println!("[Pasted HUD] toggle_hud_window invoked!");
    if let Some(window) = app.get_webview_window("hud") {
        let is_vis = window.is_visible().unwrap_or(false);
        println!(
            "[Pasted HUD] Window 'hud' found! Currently visible: {}",
            is_vis
        );
        if is_vis {
            let _ = window.hide();
            println!("[Pasted HUD] Hidden HUD window.");
        } else {
            let mut pos_payload = None;

            #[cfg(target_os = "macos")]
            {
                #[repr(C)]
                #[derive(Copy, Clone, Debug)]
                struct LocalPoint {
                    x: f64,
                    y: f64,
                }

                #[repr(C)]
                #[derive(Copy, Clone, Debug)]
                struct LocalSize {
                    width: f64,
                    height: f64,
                }

                #[repr(C)]
                #[derive(Copy, Clone, Debug)]
                struct LocalRect {
                    origin: LocalPoint,
                    size: LocalSize,
                }

                use objc::runtime::{Class, Object};
                use objc::{msg_send, sel, sel_impl};

                unsafe {
                    if let Some(event_class) = Class::get("NSEvent") {
                        let loc: LocalPoint = msg_send![event_class, mouseLocation];

                        let screens_class = Class::get("NSScreen");
                        if let Some(screens_cls) = screens_class {
                            let screens_array: *mut Object = msg_send![screens_cls, screens];
                            let screen_count: usize = msg_send![screens_array, count];

                            let mut target_screen: Option<*mut Object> = None;
                            let mut primary_height = 1080.0;

                            if screen_count > 0 {
                                let first_screen: *mut Object =
                                    msg_send![screens_array, objectAtIndex: 0usize];
                                let first_frame: LocalRect = msg_send![first_screen, frame];
                                primary_height = first_frame.size.height;
                            }

                            for i in 0..screen_count {
                                let screen: *mut Object =
                                    msg_send![screens_array, objectAtIndex: i];
                                let frame: LocalRect = msg_send![screen, frame];
                                if loc.x >= frame.origin.x
                                    && loc.x <= frame.origin.x + frame.size.width
                                    && loc.y >= frame.origin.y
                                    && loc.y <= frame.origin.y + frame.size.height
                                {
                                    target_screen = Some(screen);
                                    break;
                                }
                            }

                            let active_screen =
                                target_screen.unwrap_or_else(|| msg_send![screens_cls, mainScreen]);

                            if !active_screen.is_null() {
                                let vis_frame: LocalRect = msg_send![active_screen, visibleFrame];

                                let mouse_top_y = primary_height - loc.y;
                                let vis_top =
                                    primary_height - (vis_frame.origin.y + vis_frame.size.height);
                                let vis_bottom = primary_height - vis_frame.origin.y;
                                let vis_left = vis_frame.origin.x;
                                let vis_right = vis_frame.origin.x + vis_frame.size.width;

                                let hud_width = 360.0;
                                let hud_height = 440.0;

                                // Horizontal positioning (centered on cursor) & clamping
                                let mut target_x = loc.x - (hud_width / 2.0);
                                target_x = target_x.clamp(
                                    vis_left + 8.0,
                                    (vis_right - hud_width - 8.0).max(vis_left + 8.0),
                                );

                                // Vertical positioning & dynamic flip if near bottom edge
                                let mut target_y = mouse_top_y + 8.0;
                                if target_y + hud_height > vis_bottom - 8.0 {
                                    target_y = mouse_top_y - hud_height - 8.0;
                                }
                                target_y = target_y.clamp(
                                    vis_top + 8.0,
                                    (vis_bottom - hud_height - 8.0).max(vis_top + 8.0),
                                );

                                let is_flipped = target_y < mouse_top_y;
                                let payload = serde_json::json!({
                                    "flipped": is_flipped,
                                    "cursorX": loc.x,
                                    "cursorY": mouse_top_y,
                                    "targetX": target_x,
                                    "targetY": target_y
                                });
                                let _ = window.emit("hud_position_updated", payload.clone());
                                pos_payload = Some(payload);

                                println!(
                                    "[Pasted HUD] Smart positioning: target_x={}, target_y={} (Flipped: {})",
                                    target_x, target_y, is_flipped
                                );

                                if let Ok(ns_win_ptr) = window.ns_window() {
                                    let ns_win = ns_win_ptr as *mut Object;
                                    let _: () = msg_send![ns_win, setHasShadow: 0i8];
                                    let _: () = msg_send![ns_win, setAlphaValue: 0.0f64];
                                    let cocoa_y = primary_height - target_y - hud_height;
                                    let origin = LocalPoint {
                                        x: target_x,
                                        y: cocoa_y,
                                    };
                                    let _: () = msg_send![ns_win, setFrameOrigin: origin];
                                }

                                let _ = window.set_position(tauri::Position::Logical(
                                    tauri::LogicalPosition {
                                        x: target_x,
                                        y: target_y,
                                    },
                                ));
                            }
                        }
                    }
                }
            }

            let _ = window.show();
            let _ = window.set_focus();
            if let Ok(ns_win_ptr) = window.ns_window() {
                use objc::runtime::Object;
                use objc::{msg_send, sel, sel_impl};
                unsafe {
                    let ns_win = ns_win_ptr as *mut Object;
                    let _: () = msg_send![ns_win, setAlphaValue: 1.0f64];
                }
            }
            if let Some(payload) = pos_payload {
                let _ = window.emit("hud_position_updated", payload);
            }
            println!("[Pasted HUD] Successfully showed and focused HUD window!");
        }
    } else {
        println!("[Pasted HUD] Could not find window 'hud'");
    }
    Ok(())
}

#[tauri::command]
pub fn paste_clip_by_id(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<(), String> {
    let clips = db.get_clips(None, None, false).map_err(|e| e.to_string())?;
    if let Some(clip) = clips.into_iter().find(|c| c.id == clip_id) {
        let mut cb = Clipboard::new().map_err(|e| e.to_string())?;
        if let Some(txt) = &clip.text_content {
            let _ = cb.set_text(txt);
        } else if let Some(b64) = &clip.image_base64 {
            let _ = cb.set_text(b64);
        }

        if let Some(hud) = app.get_webview_window("hud") {
            let _ = hud.hide();
        }
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.hide();
        }

        thread::sleep(Duration::from_millis(50));
        simulate_cmd_v_paste();
    }
    Ok(())
}

#[tauri::command]
pub fn get_protected_clips(db: State<'_, Arc<DbState>>) -> Result<Vec<ClipItem>, String> {
    db.get_protected_clips().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_clip_protected(clip_id: i64, db: State<'_, Arc<DbState>>) -> Result<bool, String> {
    db.toggle_protected(clip_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn trash_unpinned_clips(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.trash_unpinned_clips().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn purge_unpinned_clips(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.purge_unpinned_clips().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_all_clips(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.clear_all_clips().map_err(|e| e.to_string())
}

fn get_dvorak_code_for_char(ch: char) -> Option<tauri_plugin_global_shortcut::Code> {
    use tauri_plugin_global_shortcut::Code;

    match ch.to_ascii_uppercase() {
        'A' => Some(Code::KeyA),
        'B' => Some(Code::KeyN),
        'C' => Some(Code::KeyI),
        'D' => Some(Code::KeyH),
        'E' => Some(Code::KeyD),
        'F' => Some(Code::KeyW),
        'G' => Some(Code::KeyE),
        'H' => Some(Code::KeyJ),
        'I' => Some(Code::KeyG),
        'J' => Some(Code::KeyP),
        'K' => Some(Code::BracketLeft),
        'L' => Some(Code::KeyU),
        'M' => Some(Code::KeyM),
        'N' => Some(Code::KeyL),
        'O' => Some(Code::KeyS),
        'P' => Some(Code::KeyR),
        'Q' => Some(Code::KeyO),
        'R' => Some(Code::KeyY),
        'S' => Some(Code::Semicolon),
        'T' => Some(Code::KeyK),
        'U' => Some(Code::KeyF),
        'V' => Some(Code::Period),
        'W' => Some(Code::Comma),
        'X' => Some(Code::KeyQ),
        'Y' => Some(Code::KeyT),
        'Z' => Some(Code::Slash),
        '1' => Some(Code::Digit1),
        '2' => Some(Code::Digit2),
        '3' => Some(Code::Digit3),
        '4' => Some(Code::Digit4),
        '5' => Some(Code::Digit5),
        '6' => Some(Code::Digit6),
        '7' => Some(Code::Digit7),
        '8' => Some(Code::Digit8),
        '9' => Some(Code::Digit9),
        '0' => Some(Code::Digit0),
        '`' => Some(Code::Backquote),
        _ => None,
    }
}

fn normalize_shortcut_aliases(shortcut: &str) -> String {
    shortcut
        .replace("CmdOrCtrl", "Super")
        .replace("Command", "Super")
        .replace("Cmd", "Super")
        .replace("Option", "Alt")
        .replace("Control", "Ctrl")
        .replace(['ç', 'Ç'], "C")
        .replace(['√', '◊'], "V")
        .replace(['µ', 'Â'], "M")
        .replace('≈', "X")
        .replace('ß', "S")
        .replace('∂', "D")
        .replace('ƒ', "F")
        .replace('©', "G")
        .replace('®', "R")
        .replace('†', "T")
        .replace('¥', "Y")
        .replace(['ø', 'Ø'], "O")
        .replace(['π', '∏'], "P")
        .replace(['å', 'Å'], "A")
        .replace('∫', "B")
        .replace('∆', "J")
        .replace('˚', "K")
        .replace('¬', "L")
        .replace('Ω', "Z")
        .replace('œ', "Q")
        .replace('∑', "W")
}

pub fn parse_shortcut_str(sc_str: &str) -> Option<tauri_plugin_global_shortcut::Shortcut> {
    use std::str::FromStr;
    use tauri_plugin_global_shortcut::Shortcut;

    let s = sc_str.trim();
    if s.is_empty() {
        return None;
    }

    if let Ok(sc) = Shortcut::from_str(s) {
        return Some(sc);
    }

    let clean = normalize_shortcut_aliases(s);

    if let Ok(sc) = Shortcut::from_str(&clean) {
        return Some(sc);
    }

    let parts: Vec<&str> = clean.split('+').collect();
    if let Some(last) = parts.last() {
        let last_trim = last.trim();
        if last_trim.len() == 1 && last_trim.chars().next().unwrap().is_ascii_alphabetic() {
            let key_str = format!("Key{}", last_trim.to_ascii_uppercase());
            let converted = format!("{}+{}", parts[..parts.len() - 1].join("+"), key_str);
            if let Ok(sc) = Shortcut::from_str(&converted) {
                return Some(sc);
            }
        }
        if last_trim.len() == 1 && last_trim.chars().next().unwrap().is_ascii_digit() {
            let key_str = format!("Digit{}", last_trim);
            let converted = format!("{}+{}", parts[..parts.len() - 1].join("+"), key_str);
            if let Ok(sc) = Shortcut::from_str(&converted) {
                return Some(sc);
            }
        }
    }

    None
}

pub fn parse_shortcut_str_all_layouts(
    sc_str: &str,
) -> Option<Vec<tauri_plugin_global_shortcut::Shortcut>> {
    use tauri_plugin_global_shortcut::{Modifiers, Shortcut};

    let s = sc_str.trim();
    if s.is_empty() {
        return None;
    }

    let clean = normalize_shortcut_aliases(s);

    let mut shortcuts = Vec::new();

    if let Some(sc) = parse_shortcut_str(&clean) {
        shortcuts.push(sc);
    }

    let parts: Vec<&str> = clean.split('+').collect();
    if let Some(last) = parts.last() {
        let last_trim = last.trim();
        if last_trim.len() == 1 {
            let ch = last_trim.chars().next().unwrap();
            let mut mods = Modifiers::empty();
            for m in &parts[..parts.len() - 1] {
                match m.trim() {
                    "Super" => mods |= Modifiers::SUPER,
                    "Alt" => mods |= Modifiers::ALT,
                    "Ctrl" => mods |= Modifiers::CONTROL,
                    "Shift" => mods |= Modifiers::SHIFT,
                    _ => {}
                }
            }

            if let Some(dvorak_code) = get_dvorak_code_for_char(ch) {
                let dvorak_sc = Shortcut::new(Some(mods), dvorak_code);
                if !shortcuts.contains(&dvorak_sc) {
                    shortcuts.push(dvorak_sc);
                }
            }
        }
    }

    if shortcuts.is_empty() {
        None
    } else {
        Some(shortcuts)
    }
}

#[allow(dead_code)]
fn try_register_shortcut(app: &AppHandle, sc_str: &str) {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    if let Some(shortcut) = parse_shortcut_str(sc_str) {
        match app.global_shortcut().register(shortcut) {
            Ok(_) => println!(
                "[Pasted Shortcut Register Success] Registered '{}' -> {:?}",
                sc_str, shortcut
            ),
            Err(e) => eprintln!(
                "[Pasted Shortcut Register Error] Failed to register '{}' -> {:?}",
                sc_str, e
            ),
        }
    } else {
        eprintln!(
            "[Pasted Shortcut Parse Error] Could not parse shortcut string: '{}'",
            sc_str
        );
    }
}

pub fn register_all_app_shortcuts(app: &AppHandle) -> Result<(), String> {
    if let Some(mgr) = app.try_state::<Arc<crate::hotkey_manager::HotkeyManager>>() {
        mgr.register_all(app)
    } else {
        Err("HotkeyManager state not initialized".to_string())
    }
}

#[derive(serde::Serialize)]
pub struct AccessibilityStatus {
    pub is_trusted: bool,
    pub is_dev_mode: bool,
}

#[tauri::command]
pub fn check_accessibility_permission() -> AccessibilityStatus {
    let is_trusted = {
        #[cfg(target_os = "macos")]
        {
            use std::ptr;
            #[link(name = "ApplicationServices", kind = "framework")]
            extern "C" {
                fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
            }
            unsafe { AXIsProcessTrustedWithOptions(ptr::null()) }
        }
        #[cfg(not(target_os = "macos"))]
        true
    };

    let is_dev_mode = cfg!(debug_assertions);

    AccessibilityStatus {
        is_trusted,
        is_dev_mode,
    }
}

#[tauri::command]
pub fn request_accessibility_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let _ = Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();
        let _ = Command::new("open")
            .arg("x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_Accessibility")
            .spawn();

        let status = check_accessibility_permission();
        status.is_trusted
    }
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let _ = Command::new("cmd")
            .arg("/c")
            .arg("start ms-settings:privacy-accessibility")
            .spawn();
        true
    }
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        let _ = Command::new("gnome-control-center").spawn();
        true
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    true
}

#[tauri::command]
pub fn register_app_setting_hotkey(
    key: String,
    value: String,
    app: AppHandle,
) -> Result<(), String> {
    let db = app.state::<Arc<DbState>>();
    db.save_setting(&key, &value)
        .map_err(|error| error.to_string())?;
    register_all_app_shortcuts(&app)
}

#[tauri::command]
pub fn register_hud_shortcut(shortcut_str: String, app: AppHandle) -> Result<(), String> {
    let db = app.state::<Arc<DbState>>();
    db.save_setting("hudHotkey", &shortcut_str)
        .map_err(|error| error.to_string())?;
    register_all_app_shortcuts(&app)
}

#[tauri::command]
pub fn set_dock_visibility(show_dock: bool, app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use tauri::ActivationPolicy;
        if show_dock {
            let _ = app.set_activation_policy(ActivationPolicy::Regular);
        } else {
            let _ = app.set_activation_policy(ActivationPolicy::Accessory);
        }
    }
    let _ = show_dock;
    let _ = app;
    Ok(())
}

#[tauri::command]
pub fn open_emoji_picker() {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg("tell application \"System Events\" to keystroke \" \" using {control down, command down}")
            .spawn();
    }
}

#[tauri::command]
pub fn get_installed_applications(db: State<'_, Arc<DbState>>) -> Result<Vec<String>, String> {
    let mut apps = std::collections::BTreeSet::new();

    if let Ok(history_apps) = db.get_distinct_source_apps() {
        for app in history_apps {
            if !app.trim().is_empty() {
                apps.insert(app);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let dirs = [
            "/Applications",
            "/System/Applications",
            "/System/Applications/Utilities",
        ];
        for dir in &dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "app") {
                        if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                            apps.insert(name.to_string());
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let dirs = ["/usr/share/applications", "/usr/local/share/applications"];
        for dir in &dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |ext| ext == "desktop") {
                        if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                            let clean_name = name.trim_end_matches(".desktop");
                            apps.insert(clean_name.to_string());
                        }
                    }
                }
            }
        }
    }

    let common = [
        "1Password",
        "Bitwarden",
        "Safari",
        "Google Chrome",
        "Firefox",
        "Slack",
        "Signal",
        "Telegram",
        "VS Code",
        "Terminal",
        "Warp",
        "Xcode",
        "Discord",
        "Keychain Access",
        "Passwords",
    ];
    for c in &common {
        apps.insert(c.to_string());
    }

    Ok(apps.into_iter().collect())
}

#[tauri::command]
pub fn extract_ocr_from_clip(clip_id: i64, db: State<'_, Arc<DbState>>) -> Result<String, String> {
    let clips = db.get_clips(None, None, false).map_err(|e| e.to_string())?;
    let clip = clips
        .into_iter()
        .find(|c| c.id == clip_id)
        .ok_or("Clip not found")?;

    if let Some(b64) = clip.image_base64 {
        let clean_b64 = if let Some(idx) = b64.find(',') {
            &b64[idx + 1..]
        } else {
            &b64
        };

        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(clean_b64) {
            if let Some(ocr_text) = crate::ocr::perform_ocr_on_image_bytes(&bytes) {
                db.update_clip_text(clip_id, &ocr_text)
                    .map_err(|error| error.to_string())?;
                return Ok(ocr_text);
            }
        }
    }
    Err("No text recognized in image".to_string())
}

#[tauri::command]
pub fn toggle_clipboard_pause(
    monitor_state: State<'_, Arc<crate::clipboard_monitor::ClipboardMonitorState>>,
    db: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let current = monitor_state
        .is_manually_paused
        .load(std::sync::atomic::Ordering::Relaxed);
    let new_val = !current;
    monitor_state
        .is_manually_paused
        .store(new_val, std::sync::atomic::Ordering::Relaxed);

    if new_val {
        let _ = db.log_activity(
            "recording_manually_paused",
            "Clipboard recording manually paused",
        );
    } else {
        let _ = db.log_activity(
            "recording_manually_resumed",
            "Clipboard recording manually resumed",
        );
    }

    Ok(monitor_state.is_paused())
}

#[tauri::command]
pub fn is_clipboard_paused(
    monitor_state: State<'_, Arc<crate::clipboard_monitor::ClipboardMonitorState>>,
) -> Result<bool, String> {
    Ok(monitor_state.is_paused())
}

#[tauri::command]
pub fn export_clips_json(db: State<'_, Arc<DbState>>) -> Result<String, String> {
    let clips = db.get_clips(None, None, false).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&clips).map_err(|e| e.to_string())
}

fn csv_cell(value: &str) -> String {
    let escaped = value.replace('"', "\"\"");
    let neutralized = if matches!(
        value.chars().next(),
        Some('=' | '+' | '-' | '@' | '\t' | '\r')
    ) {
        format!("'{escaped}")
    } else {
        escaped
    };
    format!("\"{neutralized}\"")
}

#[tauri::command]
pub fn export_clips_csv(db: State<'_, Arc<DbState>>) -> Result<String, String> {
    let clips = db.get_clips(None, None, false).map_err(|e| e.to_string())?;
    let mut csv = String::from("id,content_type,source_app,is_pinned,created_at,text_content\n");
    for c in clips {
        let line = format!(
            "{},{},{},{},{},{}\n",
            c.id,
            csv_cell(&c.content_type),
            csv_cell(&c.source_app),
            c.is_pinned,
            csv_cell(&c.created_at),
            csv_cell(c.text_content.as_deref().unwrap_or_default()),
        );
        csv.push_str(&line);
    }
    Ok(csv)
}

#[tauri::command]
pub fn import_clips_json(json_str: String, db: State<'_, Arc<DbState>>) -> Result<usize, String> {
    let items: Vec<ClipItem> = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;
    let mut count = 0;
    for item in items {
        if db
            .save_clip(
                &item.content_type,
                item.text_content.as_deref(),
                item.html_content.as_deref(),
                item.image_base64.as_deref(),
                &item.content_hash,
                &item.source_app,
            )
            .is_ok()
        {
            count += 1;
        }
    }
    Ok(count)
}

#[tauri::command]
pub fn get_analytics_summary(
    db: State<'_, Arc<DbState>>,
) -> Result<crate::db::AnalyticsSummary, String> {
    db.get_analytics_summary().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn install_cli_to_path() -> Result<String, String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let bin_dir = exe_path.parent().ok_or("Cannot locate binary directory")?;
    let cli_exe = bin_dir.join("pasted-cli");

    if !cli_exe.exists() {
        return Err(format!(
            "pasted-cli binary not found at '{:?}'. Run 'cargo build --bin pasted-cli' first.",
            cli_exe
        ));
    }

    let target_dir = dirs::home_dir()
        .map(|home| home.join(".local/bin"))
        .ok_or("Cannot locate your home directory")?;

    #[cfg(unix)]
    {
        let symlink_path = install_cli_symlink(&cli_exe, &target_dir)?;
        Ok(format!(
            "Successfully linked pasted-cli to '{}'. Make sure that directory is in your PATH.",
            symlink_path.display()
        ))
    }

    #[cfg(not(unix))]
    {
        Err("Automatic CLI installation is not supported on this platform yet".to_string())
    }
}

#[cfg(unix)]
fn install_cli_symlink(
    cli_exe: &std::path::Path,
    target_dir: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    use std::fs;
    use std::os::unix::fs::symlink;

    fs::create_dir_all(target_dir).map_err(|error| {
        format!(
            "Failed to create CLI directory '{}': {error}",
            target_dir.display()
        )
    })?;
    let symlink_path = target_dir.join("pasted-cli");
    match fs::symlink_metadata(&symlink_path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            let existing_target = fs::read_link(&symlink_path).map_err(|error| {
                format!(
                    "Failed to inspect existing CLI link '{}': {error}",
                    symlink_path.display()
                )
            })?;
            if existing_target == cli_exe {
                return Ok(symlink_path);
            }
            return Err(format!(
                "Refusing to replace existing CLI link '{}' (currently points to '{}')",
                symlink_path.display(),
                existing_target.display()
            ));
        }
        Ok(_) => {
            return Err(format!(
                "Refusing to replace existing file '{}'",
                symlink_path.display()
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "Failed to inspect CLI destination '{}': {error}",
                symlink_path.display()
            ));
        }
    }

    symlink(cli_exe, &symlink_path).map_err(|error| {
        format!(
            "Failed to create CLI link '{}': {error}",
            symlink_path.display()
        )
    })?;
    Ok(symlink_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn unique_test_directory(label: &str) -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("pasted-{label}-{}-{nonce}", std::process::id()))
    }

    #[test]
    #[cfg(unix)]
    fn cli_install_never_overwrites_an_existing_file() {
        let root = unique_test_directory("cli-preserve");
        let bin_dir = root.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let destination = bin_dir.join("pasted-cli");
        std::fs::write(&destination, "user-owned").unwrap();

        let error = install_cli_symlink(&root.join("source"), &bin_dir).unwrap_err();
        assert!(error.contains("Refusing to replace existing file"));
        assert_eq!(std::fs::read_to_string(&destination).unwrap(), "user-owned");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn cli_install_is_idempotent_for_its_existing_link() {
        let root = unique_test_directory("cli-idempotent");
        let source = root.join("pasted-cli-source");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(&source, "binary").unwrap();
        let bin_dir = root.join("bin");

        let first = install_cli_symlink(&source, &bin_dir).unwrap();
        let second = install_cli_symlink(&source, &bin_dir).unwrap();
        assert_eq!(first, second);
        assert_eq!(std::fs::read_link(second).unwrap(), source);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn test_parse_shortcut_str_variations() {
        assert!(parse_shortcut_str("CmdOrCtrl+Shift+V").is_some());
        assert!(parse_shortcut_str("Control+Alt+C").is_some());
        assert!(parse_shortcut_str("Ctrl+Alt+KeyC").is_some());
        assert!(parse_shortcut_str("Alt+Super+KeyV").is_some());
        assert!(parse_shortcut_str("Option+Cmd+C").is_some());
        assert!(parse_shortcut_str("Command+Shift+V").is_some());
        assert!(parse_shortcut_str("Control+Option+C").is_some());
        assert!(parse_shortcut_str("Control+Option+V").is_some());
        assert!(parse_shortcut_str("Super+Alt+KeyC").is_some());
        assert!(parse_shortcut_str("").is_none());
        assert!(parse_shortcut_str("   ").is_none());

        // Equivalence checks for key representations
        let sc1 = parse_shortcut_str("Option+Command+C").unwrap();
        let sc2 = parse_shortcut_str("Alt+Super+KeyC").unwrap();
        assert_eq!(
            sc1, sc2,
            "Option+Command+C should resolve to identical Shortcut struct as Alt+Super+KeyC"
        );

        // Option unicode character resolution tests
        let sc_unicode_c = parse_shortcut_str("Alt+ç").unwrap();
        let sc_ascii_c = parse_shortcut_str("Alt+KeyC").unwrap();
        assert_eq!(sc_unicode_c, sc_ascii_c, "Alt+ç should map to Alt+KeyC");
    }

    #[test]
    fn test_print_parsed_shortcuts() {
        let strings = vec![
            "Command+1",
            "Command+Digit1",
            "Super+Digit1",
            "Command+C",
            "Command+KeyC",
            "Super+KeyC",
            "Alt+Shift+V",
            "Alt+Shift+KeyV",
            "Control+Alt+C",
            "Control+Alt+KeyC",
        ];
        for s in strings {
            let parsed = parse_shortcut_str(s);
            println!("parse_shortcut_str('{s}') = {:?}", parsed);
        }
    }

    #[test]
    fn test_accessibility_status_check() {
        let status = check_accessibility_permission();
        println!(
            "Accessibility test status: trusted={}, dev_mode={}",
            status.is_trusted, status.is_dev_mode
        );
        assert_eq!(status.is_dev_mode, cfg!(debug_assertions));
    }

    #[test]
    fn csv_cells_escape_structure_and_neutralize_formulas() {
        assert_eq!(csv_cell("plain text"), "\"plain text\"");
        assert_eq!(
            csv_cell("commas, quotes \" and\nlines"),
            "\"commas, quotes \"\" and\nlines\""
        );
        assert_eq!(csv_cell("=2+2"), "\"'=2+2\"");
        assert_eq!(csv_cell("+SUM(A1:A2)"), "\"'+SUM(A1:A2)\"");
        assert_eq!(csv_cell("-1+2"), "\"'-1+2\"");
        assert_eq!(csv_cell("@SUM(A1:A2)"), "\"'@SUM(A1:A2)\"");
        assert_eq!(csv_cell("\t=2+2"), "\"'\t=2+2\"");
        assert_eq!(csv_cell("\r=2+2"), "\"'\r=2+2\"");
    }
}
