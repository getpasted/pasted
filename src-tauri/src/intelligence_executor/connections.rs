use super::*;

fn is_supported_connection(connection: &IntelligenceConnection) -> bool {
    crate::intelligence_provider::supports_connection(connection)
}

#[cfg(test)]
pub(super) fn select_connection(
    db: &DbState,
    requested_id: Option<&str>,
) -> Result<IntelligenceConnection, IntelligenceExecutionError> {
    select_connections(db, requested_id).map(|mut connections| connections.remove(0))
}

pub(super) fn select_connections(
    db: &DbState,
    requested_id: Option<&str>,
) -> Result<Vec<IntelligenceConnection>, IntelligenceExecutionError> {
    let connections = db
        .get_intelligence_connections()
        .map_err(|error| IntelligenceExecutionError::new("database_error", error.to_string()))?;
    if let Some(id) = requested_id {
        return connections
            .into_iter()
            .find(|connection| {
                connection.id == id && connection.enabled && is_supported_connection(connection)
            })
            .map(|connection| vec![connection])
            .ok_or_else(|| {
                IntelligenceExecutionError::new(
                    "connection_unavailable",
                    "The selected intelligence connection is not enabled or supported",
                )
            });
    }
    let candidates = connections
        .into_iter()
        .filter(|connection| connection.enabled && is_supported_connection(connection))
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        Err(IntelligenceExecutionError::new(
            "no_enabled_connection",
            "Power on a provider and try again.",
        ))
    } else {
        Ok(candidates)
    }
}

pub(super) fn is_retryable_provider_error(error: &IntelligenceExecutionError) -> bool {
    matches!(
        error.code,
        "connection_failed"
            | "connection_timeout"
            | "provider_failed"
            | "model_not_ready"
            | "apple_intelligence_not_enabled"
            | "device_not_eligible"
    )
}

pub(super) fn finish_scheduler_permit<T>(
    permit: &mut crate::intelligence_scheduler::SchedulerPermit,
    result: &Result<T, IntelligenceExecutionError>,
) {
    use crate::intelligence_scheduler::SchedulerCompletion;
    match result {
        Ok(_) => permit.finish(SchedulerCompletion::Succeeded, None),
        Err(error) if error.code == "execution_cancelled" => {
            permit.finish(SchedulerCompletion::Cancelled, Some(error.message.clone()))
        }
        Err(error) => permit.finish(
            SchedulerCompletion::Failed,
            Some(format!("{}: {}", error.code, error.message)),
        ),
    }
}
