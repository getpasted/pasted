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
pub async fn propose_extractor_recipe(
    request: crate::intelligence_executor::ProposeExtractorRecipeRequest,
    client_request_id: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<
    crate::intelligence_executor::ExtractorRecipeProposal,
    crate::intelligence_executor::IntelligenceExecutionError,
> {
    let cancellation =
        client_request_id.map(crate::transformation_service::CancellationRegistration::register);
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        crate::intelligence_executor::propose_extractor_recipe(
            &db,
            request,
            cancellation
                .as_ref()
                .map(|registration| registration.flag()),
        )
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
pub async fn plan_transformation_intent(
    request: crate::intelligence_executor::PlanIntentRequest,
    client_request_id: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<
    crate::intelligence_executor::PlanIntentOutcome,
    crate::intelligence_executor::IntelligenceExecutionError,
> {
    if let Err(message) = features::require(&db, Feature::Transformations) {
        return Err(crate::intelligence_executor::IntelligenceExecutionError {
            code: "feature_disabled",
            message,
        });
    }
    let cancellation = client_request_id
        .clone()
        .map(crate::transformation_service::CancellationRegistration::register);
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        let result = crate::intelligence_executor::plan_intent_with_cancellation(
            &db,
            request,
            client_request_id.as_deref(),
            cancellation
                .as_ref()
                .map(|registration| registration.flag()),
        );
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
                if error.code == "execution_cancelled" {
                    let _ =
                        db.log_activity("transform_draft_cancelled", "Cancelled Transform draft");
                } else {
                    let _ = db.log_activity(
                        "transform_draft_failed",
                        &format!("Transform draft failed ({})", error.code),
                    );
                }
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
    client_request_id: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<
    crate::intelligence_executor::ExecutePlanOutcome,
    crate::intelligence_executor::IntelligenceExecutionError,
> {
    if let Err(message) = features::require(&db, Feature::Transformations) {
        return Err(crate::intelligence_executor::IntelligenceExecutionError {
            code: "feature_disabled",
            message,
        });
    }
    let cancellation = client_request_id
        .clone()
        .map(crate::transformation_service::CancellationRegistration::register);
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        let result = crate::intelligence_executor::execute_plan_with_cancellation(
            &db,
            request,
            client_request_id.as_deref(),
            cancellation
                .as_ref()
                .map(|registration| registration.flag()),
        );
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
                if error.code == "execution_cancelled" {
                    let _ = db.log_activity("transform_test_cancelled", "Cancelled Transform test");
                } else {
                    let _ = db.log_activity(
                        "transform_test_failed",
                        &format!("Transform test failed ({})", error.code),
                    );
                }
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
pub fn get_intelligence_scheduler_snapshot() -> crate::intelligence_scheduler::SchedulerSnapshot {
    crate::intelligence_scheduler::snapshot()
}
