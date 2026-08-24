use std::collections::HashMap;

use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::Value;

const CONTRACT_JSON: &str = include_str!("../../shared/settings-contract.json");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsContract {
    version: u32,
    factory_reset: String,
    pages: Vec<SettingsPageDefinition>,
    settings: Vec<SettingDefinition>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsPageDefinition {
    id: String,
    reset_strategy: ResetStrategy,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ResetStrategy {
    Settings,
    Dedicated,
    None,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ResetBehavior {
    Default,
    Preserve,
    FactoryOnly,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Visibility {
    Public,
    Private,
    Internal,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Mutation {
    Direct,
    Dedicated,
    Internal,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingDefinition {
    key: String,
    owner: String,
    default: Option<Value>,
    reset: ResetBehavior,
    visibility: Visibility,
    mutation: Mutation,
    validation: Validation,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Validation {
    Boolean,
    Integer {
        minimum: i64,
        maximum: i64,
    },
    Choice {
        values: Vec<String>,
    },
    Language,
    String {
        #[serde(rename = "maximumBytes")]
        maximum_bytes: usize,
    },
    JsonArray,
}

static CONTRACT: Lazy<SettingsContract> = Lazy::new(|| {
    let contract: SettingsContract =
        serde_json::from_str(CONTRACT_JSON).expect("shared settings contract must be valid");
    assert_eq!(contract.factory_reset, "delete_all");
    contract
});

pub fn version() -> u32 {
    CONTRACT.version
}

pub fn default_value(key: &str) -> Option<String> {
    definition(key)
        .and_then(|definition| definition.default.as_ref())
        .map(persisted_value)
}

pub fn default_bool(key: &str) -> Option<bool> {
    definition(key)?.default.as_ref()?.as_bool()
}

pub fn default_u64(key: &str) -> Option<u64> {
    definition(key)?.default.as_ref()?.as_u64()
}

pub fn validate_direct_value(key: &str, value: &str) -> Result<(), String> {
    let definition = definition(key).ok_or_else(|| format!("Unknown setting: {key}"))?;
    if definition.mutation != Mutation::Direct {
        return Err("That setting must be changed through its dedicated controls".into());
    }
    validate(definition, value)
}

pub fn reset_defaults(page: &str) -> Result<HashMap<String, String>, String> {
    let page_definition = CONTRACT
        .pages
        .iter()
        .find(|definition| definition.id == page)
        .ok_or_else(|| format!("Unknown Settings page: {page}"))?;
    if page_definition.reset_strategy != ResetStrategy::Settings {
        return Err(format!("{page} uses a dedicated reset service"));
    }
    Ok(defaults_for_page(page))
}

pub fn dedicated_reset_defaults(page: &str) -> HashMap<String, String> {
    defaults_for_page(page)
}

pub fn is_private(key: &str) -> bool {
    definition(key).is_some_and(|definition| definition.visibility == Visibility::Private)
}

pub fn is_managed(key: &str) -> bool {
    definition(key).is_some_and(|definition| definition.mutation != Mutation::Direct)
}

pub fn is_cli_readable(key: &str) -> bool {
    definition(key).is_some_and(|definition| definition.visibility == Visibility::Public)
}

fn definition(key: &str) -> Option<&'static SettingDefinition> {
    CONTRACT
        .settings
        .iter()
        .find(|definition| definition.key == key)
}

fn defaults_for_page(page: &str) -> HashMap<String, String> {
    CONTRACT
        .settings
        .iter()
        .filter(|definition| definition.owner == page && definition.reset == ResetBehavior::Default)
        .map(|definition| {
            let value = definition
                .default
                .as_ref()
                .expect("resettable setting must declare a default");
            (definition.key.clone(), persisted_value(value))
        })
        .collect()
}

fn persisted_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        _ => serde_json::to_string(value).expect("setting default must serialize"),
    }
}

fn validate(definition: &SettingDefinition, value: &str) -> Result<(), String> {
    let invalid = |message: &str| Err(format!("{} {message}", definition.key));
    match &definition.validation {
        Validation::Boolean if matches!(value, "true" | "false") => Ok(()),
        Validation::Boolean => invalid("must be true or false"),
        Validation::Integer { minimum, maximum } => match value.parse::<i64>() {
            Ok(parsed) if (*minimum..=*maximum).contains(&parsed) => Ok(()),
            Ok(_) => invalid(&format!("must be between {minimum} and {maximum}")),
            Err(_) => invalid("must be a whole number"),
        },
        Validation::Choice { values } if values.iter().any(|choice| choice == value) => Ok(()),
        Validation::Choice { .. } => invalid("has an unsupported value"),
        Validation::Language => crate::localization::validate_configured_language(value),
        Validation::String { maximum_bytes } if value.len() <= *maximum_bytes => Ok(()),
        Validation::String { maximum_bytes } => {
            invalid(&format!("cannot exceed {maximum_bytes} bytes"))
        }
        Validation::JsonArray => match serde_json::from_str::<Value>(value) {
            Ok(Value::Array(_)) => Ok(()),
            _ => invalid("must be a JSON array"),
        },
    }
}

#[cfg(test)]
mod tests;
