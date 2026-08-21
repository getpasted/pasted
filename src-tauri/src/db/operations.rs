use rusqlite::{params, OptionalExtension, Result};
use serde::{Deserialize, Serialize};

use super::{analysis_toggle_activity, DbState};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Operation {
    pub id: i64,
    #[serde(default)]
    pub stable_id: String,
    pub name: String,
    pub op_type: String,
    pub config: Option<String>,
    pub category: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedCustomOperation {
    pub executor_kind: String,
    pub config_json: String,
    pub enabled: bool,
    pub trusted: bool,
}

impl DbState {
    pub fn get_operations(&self) -> Result<Vec<Operation>> {
        let conn = self.conn.lock();
        let mut operations = crate::operation_registry::BUILTIN_OPERATIONS
            .iter()
            .enumerate()
            .map(|(index, definition)| Operation {
                id: -((index as i64) + 1),
                stable_id: format!("builtin:{}", definition.key),
                name: definition.name.to_string(),
                op_type: definition.key.to_string(),
                config: None,
                category: definition.category_label.to_string(),
                created_at: String::new(),
            })
            .collect::<Vec<_>>();
        let mut stmt = conn.prepare(
            "SELECT row_id, id, name, executor_kind, config_json, category, created_at
             FROM custom_operations ORDER BY row_id ASC",
        )?;
        let op_iter = stmt.query_map([], |row| {
            let operation_id = row.get::<_, String>(1)?;
            let executor_kind = row.get::<_, String>(3)?;
            let config_json = row.get::<_, String>(4)?;
            let (op_type, config) = Self::legacy_operation_fields(&executor_kind, &config_json);
            Ok(Operation {
                id: row.get(0)?,
                stable_id: format!("custom:{operation_id}"),
                name: row.get(2)?,
                op_type,
                config,
                category: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        for o in op_iter {
            operations.push(o?);
        }
        Ok(operations)
    }

    pub fn get_operation(&self, reference: &str) -> Result<Operation> {
        let numeric_id = reference.parse::<i64>().ok();
        self.get_operations()?
            .into_iter()
            .find(|operation| numeric_id == Some(operation.id) || operation.stable_id == reference)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn duplicate_operation(&self, reference: &str, name: Option<&str>) -> Result<Operation> {
        let source = self.get_operation(reference)?;
        let default_name = format!("{} Copy", source.name);
        self.create_operation(
            name.unwrap_or(&default_name),
            &source.op_type,
            source.config.as_deref(),
            Some(&source.category),
        )
    }

    pub fn get_library_items(
        &self,
        kind: Option<&str>,
        include_archived: bool,
    ) -> Result<Vec<crate::library_items::LibraryItemView>> {
        if let Some(kind) = kind {
            if !matches!(
                kind,
                "capture"
                    | "inspector"
                    | "extractor"
                    | "classifier"
                    | "suggestion"
                    | "operation"
                    | "transform"
            ) {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Unknown library item kind".into(),
                ));
            }
        }
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT stable_ref, kind, name, description, group_label, icon, enabled,
                    is_builtin, is_archived, sort_order, revision, input_contract,
                    output_contract, created_at, updated_at
             FROM library_items
             WHERE (?1 IS NULL OR kind = ?1) AND (?2 OR is_archived = 0)
             ORDER BY kind, COALESCE(sort_order, 10000), name COLLATE NOCASE",
        )?;
        let rows = statement.query_map(params![kind, include_archived], |row| {
            let item = crate::library_items::LibraryItem {
                stable_ref: row.get(0)?,
                kind: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                group_label: row.get(4)?,
                icon: row.get(5)?,
                enabled: row.get(6)?,
                is_builtin: row.get(7)?,
                is_archived: row.get(8)?,
                sort_order: row.get(9)?,
                revision: row.get(10)?,
                input_contract: row.get(11)?,
                output_contract: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            };
            let analysis_pass = item.analysis_pass();
            let participant_contract = item.participant_contract();
            let type_relations = item.type_relations();
            let capabilities = item.capabilities();
            Ok(crate::library_items::LibraryItemView {
                item,
                analysis_pass,
                participant_contract,
                type_relations,
                capabilities,
            })
        })?;
        rows.collect()
    }

    pub fn set_library_item_enabled(
        &self,
        kind: &str,
        stable_ref: &str,
        enabled: bool,
    ) -> Result<()> {
        let conn = self.conn.lock();
        let (changed, activity_event, activity_description, analysis_name) = match kind {
            "capture" | "inspector" | "suggestion" => {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Built-in lifecycle capabilities cannot be disabled".to_string(),
                ));
            }
            "extractor" => {
                let (name, previous): (String, bool) = conn
                    .query_row(
                        "SELECT name, enabled FROM content_extractors
                         WHERE stable_ref = ?1 AND is_deleted = 0",
                        params![stable_ref],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .optional()?
                    .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
                if previous == enabled {
                    return Ok(());
                }
                let changed = conn.execute(
                    "UPDATE content_extractors
                     SET enabled = ?1, updated_at = CURRENT_TIMESTAMP
                     WHERE stable_ref = ?2 AND is_deleted = 0",
                    params![enabled, stable_ref],
                )?;
                let (event_type, description) =
                    analysis_toggle_activity("extractor", &name, enabled)
                        .expect("Extractor toggles have Activity metadata");
                (changed, event_type, description, Some(name))
            }
            "classifier" => {
                let (name, previous): (String, bool) = conn
                    .query_row(
                        "SELECT name, enabled FROM content_classifiers
                         WHERE stable_ref = ?1 AND is_deleted = 0",
                        params![stable_ref],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .optional()?
                    .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
                if previous == enabled {
                    return Ok(());
                }
                let changed = conn.execute(
                    "UPDATE content_classifiers
                     SET enabled = ?1, updated_at = CURRENT_TIMESTAMP
                     WHERE stable_ref = ?2 AND is_deleted = 0",
                    params![enabled, stable_ref],
                )?;
                let (event_type, description) =
                    analysis_toggle_activity("classifier", &name, enabled)
                        .expect("Classifier toggles have Activity metadata");
                (changed, event_type, description, Some(name))
            }
            "operation" => {
                let Some(operation_id) = stable_ref.strip_prefix("custom:") else {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "Built-in Operations cannot be disabled".to_string(),
                    ));
                };
                let changed = conn.execute(
                    "UPDATE custom_operations
                     SET enabled = ?1, updated_at = CURRENT_TIMESTAMP
                     WHERE id = ?2",
                    params![enabled, operation_id],
                )?;
                (
                    changed,
                    "library_item_enabled_changed",
                    format!(
                        "{} operation {stable_ref}",
                        if enabled { "Enabled" } else { "Disabled" }
                    ),
                    None,
                )
            }
            "transform" => {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Transforms do not currently have an enabled state".to_string(),
                ));
            }
            _ => {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Unknown library item kind".to_string(),
                ));
            }
        };
        drop(conn);
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        if matches!(kind, "extractor" | "classifier") {
            self.log_analysis_participant_toggle(
                kind,
                stable_ref,
                analysis_name
                    .as_deref()
                    .expect("Analysis toggles retain the participant name"),
                enabled,
            );
        } else {
            let _ = self.log_activity(activity_event, &activity_description);
        }
        Ok(())
    }

    pub fn create_operation(
        &self,
        name: &str,
        op_type: &str,
        config: Option<&str>,
        category: Option<&str>,
    ) -> Result<Operation> {
        let conn = self.conn.lock();
        let cat = category.unwrap_or("Custom Operations");
        let (executor_kind, config_json) = Self::operation_storage_fields(op_type, config);
        conn.execute(
            "INSERT INTO custom_operations
                (name, executor_kind, config_json, category, trusted)
             VALUES (?1, ?2, ?3, ?4, 1)",
            params![name, executor_kind, config_json, cat],
        )?;
        let id = conn.last_insert_rowid();
        let stable_id: String = conn.query_row(
            "SELECT id FROM custom_operations WHERE row_id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        let operation = Operation {
            id,
            stable_id: format!("custom:{stable_id}"),
            name: name.to_string(),
            op_type: op_type.to_string(),
            config: config.map(str::to_string),
            category: cat.to_string(),
            created_at: conn.query_row(
                "SELECT created_at FROM custom_operations WHERE row_id = ?1",
                params![id],
                |row| row.get(0),
            )?,
        };
        drop(conn);
        let _ = self.log_activity(
            "operation_created",
            &format!("Created Operation \"{}\"", operation.name),
        );
        Ok(operation)
    }

    pub fn update_operation(
        &self,
        id: i64,
        name: &str,
        op_type: &str,
        config: Option<&str>,
        category: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock();
        let cat = category.unwrap_or("Custom Operations");
        let (executor_kind, config_json) = Self::operation_storage_fields(op_type, config);
        let changed = conn.execute(
            "UPDATE custom_operations
             SET name = ?1, executor_kind = ?2, config_json = ?3, category = ?4,
                 updated_at = CURRENT_TIMESTAMP
             WHERE row_id = ?5",
            params![name, executor_kind, config_json, cat, id],
        )?;
        drop(conn);
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let _ = self.log_activity(
            "operation_updated",
            &format!("Updated Operation \"{}\"", name),
        );
        Ok(())
    }

    pub fn delete_operation(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        let stable_id = conn
            .query_row(
                "SELECT id FROM custom_operations WHERE row_id = ?1",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(stable_id) = stable_id else {
            return Err(rusqlite::Error::InvalidParameterName(
                "Operation not found".to_string(),
            ));
        };
        let operation_ref = format!("custom:{stable_id}");
        let transform_name = conn
            .query_row(
                "SELECT saved_transforms.name
                 FROM saved_transforms, json_each(saved_transforms.plan_json, '$.steps') AS step
                 WHERE json_extract(step.value, '$.executor.kind') = 'deterministic'
                   AND json_extract(step.value, '$.executor.operation_ref') = ?1
                 ORDER BY saved_transforms.name ASC
                 LIMIT 1",
                params![operation_ref],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(transform_name) = transform_name {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "Operation is used by “{transform_name}”. Remove it from that Transform before deleting it."
            )));
        }
        conn.execute(
            "DELETE FROM custom_operations WHERE row_id = ?1",
            params![id],
        )?;
        drop(conn);
        let _ = self.log_activity(
            "operation_deleted",
            &format!("Deleted Operation {operation_ref}"),
        );
        Ok(())
    }
}
