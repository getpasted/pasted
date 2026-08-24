use rusqlite::{params, Result};
use serde::{Deserialize, Serialize};

use super::DbState;

mod reset;
#[cfg(test)]
mod reset_tests;

pub struct IntelligenceConnectionUpdate<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub provider_kind: &'a str,
    pub endpoint: Option<&'a str>,
    pub model: Option<&'a str>,
    pub credential_ref: Option<&'a str>,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IntelligenceConnection {
    pub id: String,
    pub name: String,
    pub provider_kind: String,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub credential_ref: Option<String>,
    pub enabled: bool,
    pub priority: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl DbState {
    pub fn get_intelligence_connections(&self) -> Result<Vec<IntelligenceConnection>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, name, provider_kind, endpoint, model, credential_ref,
                    enabled, priority, created_at, updated_at
             FROM intelligence_connections
             ORDER BY priority ASC, row_id ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(IntelligenceConnection {
                id: row.get(0)?,
                name: row.get(1)?,
                provider_kind: row.get(2)?,
                endpoint: row.get(3)?,
                model: row.get(4)?,
                credential_ref: row.get(5)?,
                enabled: row.get::<_, i64>(6)? != 0,
                priority: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_intelligence_connection(&self, id: &str) -> Result<IntelligenceConnection> {
        self.get_intelligence_connections()?
            .into_iter()
            .find(|connection| connection.id == id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn create_intelligence_connection(
        &self,
        name: &str,
        provider_kind: &str,
        endpoint: Option<&str>,
        model: Option<&str>,
        credential_ref: Option<&str>,
    ) -> Result<IntelligenceConnection> {
        if name.trim().is_empty() {
            return Err(rusqlite::Error::InvalidParameterName(
                "Connection name cannot be empty".into(),
            ));
        }
        crate::intelligence_connections::validate_credential_reference(credential_ref)
            .map_err(rusqlite::Error::InvalidParameterName)?;
        let conn = self.conn.lock();
        let priority: i64 = conn.query_row(
            "SELECT COALESCE(MAX(priority), -1) + 1 FROM intelligence_connections",
            [],
            |row| row.get(0),
        )?;
        conn.execute(
            "INSERT INTO intelligence_connections
                (name, provider_kind, endpoint, model, credential_ref, priority)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                name.trim(),
                provider_kind,
                endpoint,
                model,
                credential_ref,
                priority
            ],
        )?;
        let row_id = conn.last_insert_rowid();
        conn.query_row(
            "SELECT id, name, provider_kind, endpoint, model, credential_ref,
                    enabled, priority, created_at, updated_at
             FROM intelligence_connections WHERE row_id = ?1",
            params![row_id],
            |row| {
                Ok(IntelligenceConnection {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    provider_kind: row.get(2)?,
                    endpoint: row.get(3)?,
                    model: row.get(4)?,
                    credential_ref: row.get(5)?,
                    enabled: row.get::<_, i64>(6)? != 0,
                    priority: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )
    }

    pub fn ensure_intelligence_connection_candidate(
        &self,
        name: &str,
        provider_kind: &str,
        endpoint: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock();
        let exists = conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM intelligence_connections
                WHERE provider_kind = ?1
                  AND COALESCE(endpoint, '') = COALESCE(?2, '')
            )",
            params![provider_kind, endpoint],
            |row| row.get::<_, bool>(0),
        )?;
        if exists {
            return Ok(());
        }
        let priority: i64 = conn.query_row(
            "SELECT COALESCE(MAX(priority), -1) + 1 FROM intelligence_connections",
            [],
            |row| row.get(0),
        )?;
        conn.execute(
            "INSERT INTO intelligence_connections
                (name, provider_kind, endpoint, enabled, priority)
             VALUES (?1, ?2, ?3, 0, ?4)",
            params![name.trim(), provider_kind, endpoint, priority],
        )?;
        Ok(())
    }

    pub fn update_intelligence_connection(
        &self,
        request: IntelligenceConnectionUpdate<'_>,
    ) -> Result<()> {
        if request.name.trim().is_empty() {
            return Err(rusqlite::Error::InvalidParameterName(
                "Connection name cannot be empty".into(),
            ));
        }
        crate::intelligence_connections::validate_credential_reference(request.credential_ref)
            .map_err(rusqlite::Error::InvalidParameterName)?;
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE intelligence_connections
             SET name = ?1, provider_kind = ?2, endpoint = ?3, model = ?4,
                 credential_ref = ?5, enabled = ?6, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?7",
            params![
                request.name.trim(),
                request.provider_kind,
                request.endpoint,
                request.model,
                request.credential_ref,
                request.enabled as i64,
                request.id
            ],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    pub fn delete_intelligence_connection(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock();
        let changed = conn.execute(
            "DELETE FROM intelligence_connections WHERE id = ?1",
            params![id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }
}
