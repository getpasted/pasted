use super::*;

#[cfg(test)]
mod tests;

const MIGRATION_BATCH_SIZE: i64 = 500;
const ANALYSIS_TRANSFORM_SAVEPOINT: &str = "analysis_transform_timestamp_migration";

fn canonicalize_timestamp_column(
    conn: &Connection,
    table: &str,
    column: &str,
    label: &str,
) -> Result<()> {
    if !table_exists(conn, table)? || !column_exists(conn, table, column)? {
        return Ok(());
    }
    let mut last_rowid = 0;
    loop {
        let rows = {
            let mut statement = conn.prepare(&format!(
                "SELECT rowid, {column} FROM {table}
                 WHERE rowid > ?1 AND {column} IS NOT NULL
                 ORDER BY rowid LIMIT ?2"
            ))?;
            let rows = statement
                .query_map(params![last_rowid, MIGRATION_BATCH_SIZE], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>>>()?;
            rows
        };
        if rows.is_empty() {
            break;
        }
        for (rowid, value) in &rows {
            let canonical = canonical_utc_timestamp(value, label)?;
            if canonical != *value {
                conn.execute(
                    &format!("UPDATE {table} SET {column} = ?1 WHERE rowid = ?2"),
                    params![canonical, rowid],
                )?;
            }
        }
        last_rowid = rows.last().expect("non-empty timestamp batch").0;
    }
    Ok(())
}

fn migrate_analysis_transform_timestamp_columns(conn: &Connection) -> Result<()> {
    for (table, columns, label) in [
        ("pipelines", &["created_at", "updated_at"][..], "Transform"),
        (
            "saved_transforms",
            &["created_at", "updated_at"][..],
            "Transform",
        ),
        (
            "clip_transformations",
            &["created_at"][..],
            "Transform provenance",
        ),
        (
            "transformation_executions",
            &["started_at", "completed_at"][..],
            "Transform execution",
        ),
        (
            "clip_analysis_classifications",
            &["updated_at"][..],
            "Analysis classification",
        ),
        (
            "clip_analysis_results",
            &["updated_at"][..],
            "Analysis result",
        ),
        (
            "clip_extraction_attempts",
            &["run_at"][..],
            "Extraction attempt",
        ),
        (
            "clip_searchable_text",
            &["updated_at"][..],
            "Searchable text",
        ),
    ] {
        for column in columns {
            canonicalize_timestamp_column(conn, table, column, label)?;
        }
    }
    Ok(())
}

pub(in crate::db) fn migrate_analysis_transform_timestamps(conn: &Connection) -> Result<()> {
    conn.execute_batch(&format!("SAVEPOINT {ANALYSIS_TRANSFORM_SAVEPOINT}"))?;
    match migrate_analysis_transform_timestamp_columns(conn) {
        Ok(()) => conn.execute_batch(&format!("RELEASE {ANALYSIS_TRANSFORM_SAVEPOINT}")),
        Err(error) => {
            conn.execute_batch(&format!(
                "ROLLBACK TO {ANALYSIS_TRANSFORM_SAVEPOINT};
                 RELEASE {ANALYSIS_TRANSFORM_SAVEPOINT}"
            ))?;
            Err(error)
        }
    }
}

pub(in crate::db) fn migrate_canonical_timestamps(conn: &Connection) -> Result<()> {
    const MIGRATION_KEY: &str = "canonicalUtcTimestampsV1";
    let applied: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE key = ?1)",
        [MIGRATION_KEY],
        |row| row.get(0),
    )?;
    if applied {
        return Ok(());
    }

    let transaction = conn.unchecked_transaction()?;
    for (table, columns) in [
        (
            "clips",
            &["created_at", "trashed_at", "ocr_attempted_at"][..],
        ),
        ("activity_logs", &["created_at", "observed_at"][..]),
    ] {
        if !table_exists(&transaction, table)? {
            continue;
        }
        for column in columns {
            if !column_exists(&transaction, table, column)? {
                continue;
            }
            transaction.execute(
                &format!(
                    "UPDATE {table}
                     SET {column} = strftime('%Y-%m-%dT%H:%M:%SZ', {column})
                     WHERE {column} IS NOT NULL AND datetime({column}) IS NOT NULL"
                ),
                [],
            )?;
        }
    }
    transaction.execute(
        "INSERT INTO schema_migrations (key, applied_at)
         VALUES (?1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
        [MIGRATION_KEY],
    )?;
    transaction.commit()
}
