use rusqlite::{params, Result};

use super::super::{DbState, ResolvedCustomOperation};

impl DbState {
    fn canonical_executor_kind(operation_type: &str) -> &str {
        match operation_type {
            "shell_script" => "shell",
            "regex" | "cli" | "shell" | "http" | "ai" => operation_type,
            _ => "cli",
        }
    }

    pub(in crate::db) fn operation_storage_fields(
        op_type: &str,
        config: Option<&str>,
    ) -> (String, String) {
        if crate::operation_registry::is_builtin_operation(op_type) {
            (
                "builtin".to_string(),
                serde_json::json!({
                    "key": op_type,
                    "legacy_config": config.map(|value| Self::normalize_json_config(Some(value))),
                })
                .to_string(),
            )
        } else {
            (
                Self::canonical_executor_kind(op_type).to_string(),
                Self::normalize_json_config(config),
            )
        }
    }

    pub(in crate::db) fn legacy_operation_fields(
        executor_kind: &str,
        config_json: &str,
    ) -> (String, Option<String>) {
        if executor_kind == "builtin" {
            let value = serde_json::from_str::<serde_json::Value>(config_json).unwrap_or_default();
            let operation_type = value["key"].as_str().unwrap_or("unknown").to_string();
            let config = value.get("legacy_config").and_then(|config| {
                if config.is_null() {
                    None
                } else if let Some(text) = config.as_str() {
                    Some(text.to_string())
                } else {
                    Some(config.to_string())
                }
            });
            (operation_type, config)
        } else {
            let operation_type = if executor_kind == "shell" {
                "shell_script"
            } else {
                executor_kind
            };
            let value = serde_json::from_str::<serde_json::Value>(config_json).ok();
            let config = value.map(|value| {
                value
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| value.to_string())
            });
            (operation_type.to_string(), config)
        }
    }

    pub fn resolve_custom_operation(
        &self,
        operation_ref: &str,
    ) -> Result<Option<ResolvedCustomOperation>> {
        let Some(operation_id) = operation_ref.strip_prefix("custom:") else {
            return Ok(None);
        };
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT executor_kind, config_json, enabled, trusted
             FROM custom_operations WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![operation_id])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        Ok(Some(ResolvedCustomOperation {
            executor_kind: row.get(0)?,
            config_json: row.get(1)?,
            enabled: row.get::<_, i64>(2)? != 0,
            trusted: row.get::<_, i64>(3)? != 0,
        }))
    }
}
