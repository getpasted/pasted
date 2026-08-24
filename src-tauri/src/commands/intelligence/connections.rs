use std::sync::Arc;
use tauri::State;

use crate::db::{DbState, IntelligenceConnection, IntelligenceConnectionUpdate};
use crate::features::{self, Feature};

#[tauri::command]
pub fn get_intelligence_connections(
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<IntelligenceConnection>, String> {
    db.get_intelligence_connections()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn detect_intelligence_connections(
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::intelligence_connections::DetectedIntelligenceConnection>, String> {
    let detected = tauri::async_runtime::spawn_blocking(
        crate::intelligence_connections::detect_intelligence_connections,
    )
    .await
    .map_err(|error| error.to_string())?;
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
    crate::intelligence_connections::validate_credential_reference(credential_ref.as_deref())?;
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
    crate::intelligence_connections::validate_credential_reference(credential_ref.as_deref())?;
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
    features::require(&db, Feature::Transformations)?;
    db.delete_intelligence_connection(&id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn reorder_intelligence_connections(
    ids: Vec<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    features::require(&db, Feature::Transformations)?;
    db.reorder_intelligence_connections(&ids)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn reset_intelligence_connections(
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<IntelligenceConnection>, String> {
    let detected = tauri::async_runtime::spawn_blocking(
        crate::intelligence_connections::detect_intelligence_connections,
    )
    .await
    .map_err(|error| error.to_string())?;
    let identities = detected
        .into_iter()
        .map(|candidate| {
            let endpoint = if candidate.provider_kind == "cli" {
                candidate.executable_path
            } else {
                candidate.default_endpoint.map(str::to_string)
            };
            (candidate.provider_kind.to_string(), endpoint)
        })
        .collect::<Vec<_>>();
    db.reset_intelligence_connection_policy(&identities)
        .map_err(|error| error.to_string())
}
