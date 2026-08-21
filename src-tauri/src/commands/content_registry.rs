use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use crate::db::{ContentClassificationRescanReport, DbState, FileFormatRescanReport};
use crate::features::{self, Feature};

pub mod content_type_policy;

#[tauri::command]
pub fn get_content_classifiers(
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::content_classification::Classifier>, String> {
    db.get_content_classifiers()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_library_items(
    kind: Option<String>,
    include_archived: Option<bool>,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::library_items::LibraryItemView>, String> {
    db.get_library_items(kind.as_deref(), include_archived.unwrap_or(false))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_library_item_enabled(
    kind: String,
    stable_ref: String,
    enabled: bool,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    if matches!(kind.as_str(), "operation" | "transform") {
        features::require(&db, Feature::Transformations)?;
    }
    db.set_library_item_enabled(&kind, &stable_ref, enabled)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_content_types(
    include_archived: Option<bool>,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::content_types::ContentTypeDefinition>, String> {
    db.get_content_types(include_archived.unwrap_or(false))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_content_type_groups(
    include_archived: Option<bool>,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::content_types::ContentTypeGroupDefinition>, String> {
    db.get_content_type_groups(include_archived.unwrap_or(false))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_content_type_group(
    input: crate::content_types::ContentTypeGroupInput,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::content_types::ContentTypeGroupDefinition, String> {
    db.create_content_type_group(&input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_content_type_group(
    id: String,
    input: crate::content_types::ContentTypeGroupInput,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::content_types::ContentTypeGroupDefinition, String> {
    db.update_content_type_group(&id, &input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_content_type_group_archived(
    id: String,
    archived: bool,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.set_content_type_group_archived(&id, archived)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_content_type_group(id: String, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.delete_content_type_group(&id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn restore_default_content_type_groups(
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::content_types::ContentTypeGroupDefinition>, String> {
    db.restore_default_content_type_groups()
        .map_err(|error| error.to_string())?;
    db.get_content_type_groups(true)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_content_classifier(
    input: crate::content_classification::ClassifierInput,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::content_classification::Classifier, String> {
    db.create_content_classifier(&input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_content_classifier(
    id: i64,
    input: crate::content_classification::ClassifierInput,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::content_classification::Classifier, String> {
    db.update_content_classifier(id, &input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn duplicate_content_classifier(
    reference: String,
    name: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::content_classification::Classifier, String> {
    db.duplicate_content_classifier(&reference, name.as_deref())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_content_classifier(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.delete_content_classifier(id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn restore_default_content_classifiers(
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::content_classification::Classifier>, String> {
    db.restore_default_content_classifiers()
        .map_err(|error| error.to_string())?;
    db.get_content_classifiers()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_clip_content_matches(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::db::AnalysisClassification>, String> {
    db.get_analysis_classifications(clip_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn rescan_content_classification_history(
    confirmed: bool,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<ContentClassificationRescanReport, String> {
    features::require(&db, Feature::ContentClassification)?;
    if !confirmed {
        return Err("History rescans require explicit confirmation.".to_string());
    }
    let db = Arc::clone(&db);
    let report = tauri::async_runtime::spawn_blocking(move || db.rescan_content_classification())
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
    let _ = app.emit("app-menu-action", "refresh-data");
    Ok(report)
}

#[tauri::command]
pub async fn rescan_file_format_history(
    confirmed: bool,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<FileFormatRescanReport, String> {
    features::require(&db, Feature::FileFormats)?;
    if !confirmed {
        return Err("History rescans require explicit confirmation.".to_string());
    }
    let db = Arc::clone(&db);
    let report = tauri::async_runtime::spawn_blocking(move || db.rescan_file_formats())
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
    let _ = app.emit("app-menu-action", "refresh-data");
    Ok(report)
}

#[tauri::command]
pub fn test_content_classifier(
    input: crate::content_classification::ClassifierInput,
    sample: String,
) -> Result<crate::classification_execution::ClassificationResult, String> {
    crate::content_classification::validate_classifier_input(&input)?;
    let classifier = crate::content_classification::Classifier {
        id: 0,
        stable_ref: "preview".into(),
        name: input.name,
        content_type: input.content_type.clone(),
        description: input.description,
        patterns: input.patterns,
        validator: input.validator,
        enabled: true,
        priority: input.priority,
        is_builtin: false,
        defaults: None,
        is_deleted: false,
    };
    Ok(crate::classification_execution::analyze_classifier(
        &sample,
        &classifier,
    ))
}
