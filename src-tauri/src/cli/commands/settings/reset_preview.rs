use pasted_lib::db::{DbState, IntelligenceConnection};
use rusqlite::Result;

pub(super) fn analysis_changes(db: &DbState) -> Result<Vec<String>> {
    let mut changes = Vec::new();
    for item in db.get_content_extractors()? {
        let Some(defaults) = &item.defaults else {
            continue;
        };
        let definition_changed = item.name != defaults.name
            || item.description != defaults.description
            || item.engine != defaults.engine
            || item.executable_path != defaults.executable_path
            || item.model_path != defaults.model_path
            || item.input_contract != defaults.input_contract
            || item.output_contract != defaults.output_contract
            || item.enabled != defaults.enabled
            || item.priority != defaults.priority;
        let recipe_changed = item
            .default_recipe
            .as_ref()
            .is_some_and(|recipe| item.recipe != *recipe);
        if definition_changed || recipe_changed {
            changes.push(item.name);
        }
    }
    for item in db.get_content_classifiers()? {
        let Some(defaults) = &item.defaults else {
            continue;
        };
        if item.name != defaults.name
            || item.content_type != defaults.content_type
            || item.description != defaults.description
            || item.patterns != defaults.patterns
            || item.validator != defaults.validator
            || item.enabled != defaults.enabled
            || item.priority != defaults.priority
        {
            changes.push(item.name);
        }
    }
    for item in db.get_content_types(true)? {
        let Some(defaults) = &item.defaults else {
            continue;
        };
        if item.is_archived
            || item.label != defaults.label
            || item.icon != defaults.icon
            || item.group != defaults.group
            || item.conceal_clips.unwrap_or(false) != defaults.conceal_clips
        {
            changes.push(item.label);
        }
    }
    for item in db.get_content_type_groups(true)? {
        let Some(defaults) = &item.defaults else {
            continue;
        };
        if item.is_archived
            || item.label != defaults.label
            || item.sort_order != defaults.sort_order
        {
            changes.push(item.label);
        }
    }
    changes.sort();
    Ok(changes)
}

pub(super) fn intelligence_changes(
    connections: &[IntelligenceConnection],
    detected: &[(String, Option<String>)],
) -> Vec<String> {
    let mut ordered = Vec::with_capacity(connections.len());
    for (provider_kind, endpoint) in detected {
        if let Some(connection) = connections.iter().find(|connection| {
            &connection.provider_kind == provider_kind && &connection.endpoint == endpoint
        }) {
            if !ordered.contains(&connection.id) {
                ordered.push(connection.id.clone());
            }
        }
    }
    let remaining = connections
        .iter()
        .filter_map(|connection| (!ordered.contains(&connection.id)).then(|| connection.id.clone()))
        .collect::<Vec<_>>();
    ordered.extend(remaining);
    connections
        .iter()
        .flat_map(|connection| {
            let priority = ordered
                .iter()
                .position(|id| id == &connection.id)
                .unwrap_or(0);
            [
                connection
                    .enabled
                    .then(|| format!("{}:enabled", connection.id)),
                (connection.priority != priority as i64)
                    .then(|| format!("{}:priority", connection.id)),
            ]
            .into_iter()
            .flatten()
        })
        .collect()
}
