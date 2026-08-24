use std::collections::HashSet;

use rusqlite::{params, Result};

use super::{DbState, IntelligenceConnection};

impl DbState {
    pub fn reorder_intelligence_connections(&self, ids: &[String]) -> Result<()> {
        let unique = ids.iter().collect::<HashSet<_>>();
        if unique.len() != ids.len() {
            return Err(rusqlite::Error::InvalidParameterName(
                "Connection order contains duplicate IDs".into(),
            ));
        }
        let current = self
            .get_intelligence_connections()?
            .into_iter()
            .map(|connection| connection.id)
            .collect::<HashSet<_>>();
        if current != ids.iter().cloned().collect() {
            return Err(rusqlite::Error::InvalidParameterName(
                "Connection order must contain every current Connection exactly once".into(),
            ));
        }
        let mut conn = self.conn.lock();
        let transaction = conn.transaction()?;
        for (priority, id) in ids.iter().enumerate() {
            let changed = transaction.execute(
                "UPDATE intelligence_connections SET priority = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![priority as i64, id],
            )?;
            if changed == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
        }
        transaction.commit()
    }

    pub fn reset_intelligence_connection_policy(
        &self,
        detected: &[(String, Option<String>)],
    ) -> Result<Vec<IntelligenceConnection>> {
        let current = self.get_intelligence_connections()?;
        let mut ordered_ids = Vec::with_capacity(current.len());
        for (provider_kind, endpoint) in detected {
            if let Some(connection) = current.iter().find(|connection| {
                &connection.provider_kind == provider_kind && &connection.endpoint == endpoint
            }) {
                if !ordered_ids.contains(&connection.id) {
                    ordered_ids.push(connection.id.clone());
                }
            }
        }
        let remaining = current
            .iter()
            .filter(|connection| !ordered_ids.contains(&connection.id))
            .map(|connection| connection.id.clone())
            .collect::<Vec<_>>();
        ordered_ids.extend(remaining);

        let mut conn = self.conn.lock();
        let transaction = conn.transaction()?;
        for (priority, id) in ordered_ids.iter().enumerate() {
            transaction.execute(
                "UPDATE intelligence_connections SET enabled = 0, priority = ?1,
                 updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![priority as i64, id],
            )?;
        }
        transaction.commit()?;
        drop(conn);
        self.get_intelligence_connections()
    }
}
