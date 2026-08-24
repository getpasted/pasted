use super::*;

#[derive(Clone, Copy)]
pub(crate) struct NamedMigration {
    pub(super) key: &'static str,
    pub(super) apply: fn(&Connection) -> Result<()>,
}

pub(crate) fn run_named_migrations(conn: &Connection, migrations: &[NamedMigration]) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            key TEXT PRIMARY KEY,
            applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    for migration in migrations {
        let applied: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE key = ?1)",
            [migration.key],
            |row| row.get(0),
        )?;
        if applied {
            continue;
        }
        let transaction = conn.unchecked_transaction()?;
        (migration.apply)(&transaction)?;
        transaction.execute(
            "INSERT INTO schema_migrations (key) VALUES (?1)",
            [migration.key],
        )?;
        transaction.commit()?;
    }
    Ok(())
}

pub(crate) fn run_registered_migrations(conn: &Connection) -> Result<()> {
    const MIGRATIONS: &[NamedMigration] = &[
        NamedMigration {
            key: "appExclusionHotkeysV1",
            apply: migrate_app_exclusion_hotkey_setting,
        },
        NamedMigration {
            key: "transformTerminologyV1",
            apply: migrate_transform_activity_terminology,
        },
        NamedMigration {
            key: "currentTransformationBackfillV1",
            apply: backfill_current_transformation,
        },
        NamedMigration {
            key: "analysisTransformCanonicalTimestampsV1",
            apply: migrate_analysis_transform_timestamps,
        },
    ];
    run_named_migrations(conn, MIGRATIONS)
}

#[cfg(test)]
mod tests;
