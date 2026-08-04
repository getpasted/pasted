use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationPluginManifest {
    pub manifest_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub description: String,
    #[serde(default)]
    pub permissions: Vec<PluginPermission>,
    pub contributions: PluginContributions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PluginPermission {
    Network { hosts: Vec<String> },
    Process { executables: Vec<String> },
    Secrets { providers: Vec<String> },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginContributions {
    #[serde(default)]
    pub operations: Vec<PluginOperation>,
    #[serde(default)]
    pub credential_providers: Vec<CredentialProviderContribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginOperation {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub executor: PluginExecutor,
    #[serde(default)]
    pub default_config: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PluginExecutor {
    OpenaiResponses {
        connection: String,
        model: String,
        instructions: String,
    },
    Http {
        connection: String,
        method: String,
        url: String,
    },
    Process {
        executable: String,
        args: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialProviderContribution {
    pub id: String,
    pub name: String,
    pub provider_kind: String,
    pub executable: Option<String>,
    pub reference_scheme: String,
    pub user_presence_required: bool,
}

fn value_contains_secret(value: &Value) -> bool {
    match value {
        Value::Object(values) => values.iter().any(|(key, value)| {
            let key = key.to_ascii_lowercase();
            key.contains("api_key")
                || key.contains("secret")
                || key.contains("token")
                || value_contains_secret(value)
        }),
        Value::Array(values) => values.iter().any(value_contains_secret),
        _ => false,
    }
}

impl OperationPluginManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.manifest_version != 1 {
            return Err(format!(
                "Unsupported operation plugin manifest version: {}",
                self.manifest_version
            ));
        }
        if !self.id.contains('.') || self.id.chars().any(char::is_whitespace) {
            return Err("Plugin IDs must be reverse-domain-style identifiers".to_string());
        }

        let mut contribution_ids = HashSet::new();
        for operation in &self.contributions.operations {
            if !contribution_ids.insert(operation.id.as_str()) {
                return Err(format!("Duplicate contribution ID: {}", operation.id));
            }
            if value_contains_secret(&operation.default_config) {
                return Err(format!(
                    "Operation {} contains an inline secret; use a connection reference",
                    operation.id
                ));
            }
            match &operation.executor {
                PluginExecutor::OpenaiResponses { .. } => {
                    let allowed = self.permissions.iter().any(|permission| {
                        matches!(permission, PluginPermission::Network { hosts } if hosts.iter().any(|host| host == "api.openai.com"))
                    });
                    if !allowed {
                        return Err(
                            "OpenAI Operations require api.openai.com permission".to_string()
                        );
                    }
                }
                PluginExecutor::Http { url, .. } => {
                    let host = url
                        .split("//")
                        .nth(1)
                        .and_then(|value| value.split('/').next())
                        .unwrap_or_default();
                    let allowed = self.permissions.iter().any(|permission| {
                        matches!(permission, PluginPermission::Network { hosts } if hosts.iter().any(|allowed| allowed == host))
                    });
                    if !allowed {
                        return Err(format!("HTTP Operation host is not permitted: {host}"));
                    }
                }
                PluginExecutor::Process { executable, .. } => {
                    let allowed = self.permissions.iter().any(|permission| {
                        matches!(permission, PluginPermission::Process { executables } if executables.iter().any(|allowed| allowed == executable))
                    });
                    if !allowed {
                        return Err(format!("Process executable is not permitted: {executable}"));
                    }
                }
            }
        }

        for provider in &self.contributions.credential_providers {
            if !contribution_ids.insert(provider.id.as_str()) {
                return Err(format!("Duplicate contribution ID: {}", provider.id));
            }
            if let Some(executable) = &provider.executable {
                let allowed = self.permissions.iter().any(|permission| {
                    matches!(permission, PluginPermission::Process { executables } if executables.iter().any(|allowed| allowed == executable))
                });
                if !allowed {
                    return Err(format!(
                        "Credential provider executable is not permitted: {executable}"
                    ));
                }
            }
        }
        Ok(())
    }
}

pub fn bundled_example_plugins() -> Vec<OperationPluginManifest> {
    [
        include_str!("../operation-plugins/openai.json"),
        include_str!("../operation-plugins/onepassword.json"),
    ]
    .into_iter()
    .map(|manifest| {
        let plugin: OperationPluginManifest =
            serde_json::from_str(manifest).expect("bundled plugin manifest must be valid JSON");
        plugin
            .validate()
            .expect("bundled plugin manifest must pass capability validation");
        plugin
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_examples_are_valid_and_have_unique_ids() {
        let plugins = bundled_example_plugins();
        let mut ids = HashSet::new();
        for plugin in plugins {
            plugin.validate().unwrap();
            assert!(ids.insert(plugin.id));
        }
    }

    #[test]
    fn manifests_cannot_smuggle_inline_credentials() {
        let mut plugin = bundled_example_plugins().remove(0);
        plugin.contributions.operations[0].default_config =
            serde_json::json!({ "api_key": "do-not-store-this" });
        assert!(plugin.validate().unwrap_err().contains("inline secret"));
    }
}
