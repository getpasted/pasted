use super::*;

pub(super) fn canonical_utc_timestamp(value: &str, label: &str) -> Result<String> {
    if let Ok(timestamp) = chrono::DateTime::parse_from_rfc3339(value) {
        return Ok(timestamp
            .with_timezone(&chrono::Utc)
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    }
    for format in ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S%.f"] {
        if let Ok(timestamp) = chrono::NaiveDateTime::parse_from_str(value, format) {
            return Ok(timestamp
                .and_utc()
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
        }
    }
    Err(rusqlite::Error::InvalidParameterName(format!(
        "{label} contains an invalid timestamp"
    )))
}

pub(super) fn canonicalize_optional_timestamp(
    value: &mut Option<String>,
    label: &str,
) -> Result<()> {
    if let Some(timestamp) = value.as_deref() {
        *value = Some(canonical_utc_timestamp(timestamp, label)?);
    }
    Ok(())
}

pub(super) fn migrate_canonical_timestamps(conn: &Connection) -> Result<()> {
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

pub(super) fn migrate_analysis_classification_timestamps(conn: &Connection) -> Result<()> {
    if !table_exists(conn, "clip_analysis_classifications")? {
        return Ok(());
    }
    let transaction = conn.unchecked_transaction()?;
    transaction.execute(
        "UPDATE clip_analysis_classifications
         SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', updated_at)
         WHERE updated_at NOT GLOB '????-??-??T??:??:??Z'
           AND datetime(updated_at) IS NOT NULL",
        [],
    )?;
    let invalid: bool = transaction.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM clip_analysis_classifications
            WHERE updated_at NOT GLOB '????-??-??T??:??:??Z'
        )",
        [],
        |row| row.get(0),
    )?;
    if invalid {
        return Err(rusqlite::Error::InvalidParameterName(
            "Analysis classification contains an invalid timestamp".into(),
        ));
    }
    transaction.commit()
}
pub(super) fn normalize_library_archive_timestamps(payload: &mut BackupPayload) -> Result<()> {
    payload.timestamp = canonical_utc_timestamp(&payload.timestamp, "Transfer file")?;
    for clip in &mut payload.clips {
        clip.created_at = canonical_utc_timestamp(&clip.created_at, "Transfer clip")?;
        canonicalize_optional_timestamp(&mut clip.trashed_at, "Transfer clip")?;
    }
    for bin in &mut payload.bins {
        bin.created_at = canonical_utc_timestamp(&bin.created_at, "Transfer Bin")?;
    }
    for operation in &mut payload.operations {
        if operation.id >= 0 {
            operation.created_at =
                canonical_utc_timestamp(&operation.created_at, "Transfer Operation")?;
        }
    }
    for pipeline in &mut payload.pipelines {
        pipeline.created_at = canonical_utc_timestamp(&pipeline.created_at, "Transfer Transform")?;
        pipeline.updated_at = canonical_utc_timestamp(&pipeline.updated_at, "Transfer Transform")?;
    }
    for transform in &mut payload.saved_transforms {
        transform.created_at =
            canonical_utc_timestamp(&transform.created_at, "Transfer Transform")?;
        transform.updated_at =
            canonical_utc_timestamp(&transform.updated_at, "Transfer Transform")?;
    }
    for metadata in &mut payload.ocr_metadata {
        canonicalize_optional_timestamp(&mut metadata.attempted_at, "Transfer OCR metadata")?;
    }
    Ok(())
}
