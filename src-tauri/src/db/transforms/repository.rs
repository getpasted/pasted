use rusqlite::{params, Connection, Result};

use super::SavedTransform;

pub(super) fn saved_transform_by_id(
    conn: &Connection,
    transform_id: &str,
) -> Result<SavedTransform> {
    conn.query_row(
        "SELECT row_id, id, name, plan_json, connection_id, shortcut, authoring_kind, revision, created_at, updated_at
         FROM saved_transforms WHERE id = ?1",
        params![transform_id],
        |row| {
            let stable_id: String = row.get(1)?;
            let plan_json: String = row.get(3)?;
            let plan = serde_json::from_str(&plan_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            Ok(SavedTransform {
                id: row.get(0)?,
                stable_ref: format!("transform:{stable_id}"),
                name: row.get(2)?,
                plan,
                connection_id: row.get(4)?,
                shortcut: row.get(5)?,
                authoring_kind: row.get(6)?,
                revision: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        },
    )
}
