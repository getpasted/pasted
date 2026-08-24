use rusqlite::{params, Result};

use super::super::DbState;
use super::repository::saved_transform_by_id;
use super::{PipelineStepInput, SavedTransform, TransformAuthoringKind, TransformDefinition};

impl DbState {
    pub fn get_saved_transforms(&self) -> Result<Vec<SavedTransform>> {
        let conn = self.conn.lock();
        let ids = {
            let mut statement = conn
                .prepare("SELECT id FROM saved_transforms ORDER BY updated_at DESC, row_id DESC")?;
            let ids = statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>>>()?;
            ids
        };
        ids.into_iter()
            .map(|id| saved_transform_by_id(&conn, &id))
            .collect()
    }

    pub fn get_intent_transforms(&self) -> Result<Vec<SavedTransform>> {
        Ok(self
            .get_saved_transforms()?
            .into_iter()
            .filter(|transform| transform.authoring_kind == "intent")
            .collect())
    }

    pub fn get_transform_definitions(&self) -> Result<Vec<TransformDefinition>> {
        let mut definitions = self
            .get_saved_transforms()?
            .into_iter()
            .map(TransformDefinition::from)
            .collect::<Vec<_>>();
        definitions.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.name.cmp(&right.name))
        });
        Ok(definitions)
    }

    pub fn resolve_transform_definition(
        &self,
        transform_ref: &str,
    ) -> Result<Option<TransformDefinition>> {
        if transform_ref.starts_with("pipeline:") {
            return self
                .resolve_saved_transform(transform_ref.trim_start_matches("pipeline:"))
                .map(|transform| transform.map(TransformDefinition::from));
        }
        self.resolve_saved_transform(transform_ref)
            .map(|transform| transform.map(TransformDefinition::from))
    }

    pub fn duplicate_transform_definition(
        &self,
        transform_ref: &str,
        name: Option<&str>,
    ) -> Result<TransformDefinition> {
        let definition = self
            .resolve_transform_definition(transform_ref)?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let duplicate_name = name
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("{} Copy", definition.name));
        match definition.authoring_kind {
            TransformAuthoringKind::Intent => {
                let plan = definition.plan.ok_or_else(|| {
                    rusqlite::Error::InvalidParameterName(
                        "Saved Transform has no execution plan".to_string(),
                    )
                })?;
                self.create_saved_transform(
                    &duplicate_name,
                    &plan,
                    definition.connection_id.as_deref(),
                )
                .map(TransformDefinition::from)
            }
            TransformAuthoringKind::Manual => {
                let steps = definition
                    .steps
                    .into_iter()
                    .map(|step| PipelineStepInput {
                        operation_ref: step.operation_ref,
                        config_json: step.config_json,
                        failure_policy: step.failure_policy,
                    })
                    .collect::<Vec<_>>();
                self.create_pipeline(&duplicate_name, &steps, None)
                    .map(TransformDefinition::from)
            }
        }
    }

    pub fn delete_transform_definition(&self, transform_ref: &str) -> Result<()> {
        if transform_ref.starts_with("pipeline:") {
            self.delete_pipeline(transform_ref)
        } else {
            self.delete_saved_transform(transform_ref)
        }
    }

    pub fn resolve_saved_transform(&self, transform_ref: &str) -> Result<Option<SavedTransform>> {
        let transform_id = transform_ref
            .strip_prefix("transform:")
            .unwrap_or(transform_ref);
        let conn = self.conn.lock();
        match saved_transform_by_id(&conn, transform_id) {
            Ok(transform) => Ok(Some(transform)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn create_saved_transform(
        &self,
        name: &str,
        plan: &crate::transformation_intent::TransformationPlan,
        connection_id: Option<&str>,
    ) -> Result<SavedTransform> {
        plan.validate()
            .map_err(rusqlite::Error::InvalidParameterName)?;
        let plan_json = serde_json::to_string(plan).map_err(|error| {
            rusqlite::Error::InvalidParameterName(format!("invalid Transform: {error}"))
        })?;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO saved_transforms (name, plan_json, connection_id, authoring_kind)
             VALUES (?1, ?2, ?3, 'intent')",
            params![name.trim(), plan_json, connection_id],
        )?;
        let row_id = conn.last_insert_rowid();
        let stable_id: String = conn.query_row(
            "SELECT id FROM saved_transforms WHERE row_id = ?1",
            params![row_id],
            |row| row.get(0),
        )?;
        saved_transform_by_id(&conn, &stable_id)
    }

    pub fn update_saved_transform(
        &self,
        transform_ref: &str,
        name: &str,
        plan: &crate::transformation_intent::TransformationPlan,
        connection_id: Option<&str>,
    ) -> Result<SavedTransform> {
        plan.validate()
            .map_err(rusqlite::Error::InvalidParameterName)?;
        let plan_json = serde_json::to_string(plan).map_err(|error| {
            rusqlite::Error::InvalidParameterName(format!("invalid Transform: {error}"))
        })?;
        let transform_id = transform_ref
            .strip_prefix("transform:")
            .unwrap_or(transform_ref);
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE saved_transforms
             SET name = ?1,
                 plan_json = ?2,
                 connection_id = ?3,
                 revision = revision + 1,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?4",
            params![name.trim(), plan_json, connection_id, transform_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        saved_transform_by_id(&conn, transform_id)
    }

    pub fn delete_saved_transform(&self, transform_ref: &str) -> Result<()> {
        let transform_id = transform_ref
            .strip_prefix("transform:")
            .unwrap_or(transform_ref);
        let conn = self.conn.lock();
        let changed = conn.execute(
            "DELETE FROM saved_transforms WHERE id = ?1",
            params![transform_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }
}
