use std::sync::Arc;

use tauri::State;

use crate::db::DbState;

#[tauri::command]
pub async fn repair_extractor_recipe(
    request: crate::intelligence_executor::RepairExtractorRecipeRequest,
    client_request_id: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<
    crate::intelligence_executor::ExtractorRepairOutcome,
    crate::intelligence_executor::IntelligenceExecutionError,
> {
    let cancellation =
        client_request_id.map(crate::transformation_service::CancellationRegistration::register);
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        crate::intelligence_executor::repair_extractor_recipe(
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
