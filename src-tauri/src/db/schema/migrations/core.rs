use super::super::*;

pub(crate) fn migrate_legacy_container_schema(conn: &Connection) -> Result<()> {
    // Pre-release databases used "board" for the same concept now consistently named "bin".
    if table_exists(conn, "boards")? && !table_exists(conn, "bins")? {
        conn.execute("ALTER TABLE boards RENAME TO bins", [])?;
    }
    if table_exists(conn, "clips")?
        && column_exists(conn, "clips", "board_id")?
        && !column_exists(conn, "clips", "bin_id")?
    {
        conn.execute("ALTER TABLE clips RENAME COLUMN board_id TO bin_id", [])?;
    }
    if table_exists(conn, "bins")?
        && column_exists(conn, "bins", "board_type")?
        && !column_exists(conn, "bins", "bin_type")?
    {
        conn.execute("ALTER TABLE bins RENAME COLUMN board_type TO bin_type", [])?;
    }
    if table_exists(conn, "clip_boards")? && !table_exists(conn, "clip_bins")? {
        conn.execute("ALTER TABLE clip_boards RENAME TO clip_bins", [])?;
    }
    if table_exists(conn, "clip_bins")?
        && column_exists(conn, "clip_bins", "board_id")?
        && !column_exists(conn, "clip_bins", "bin_id")?
    {
        conn.execute("ALTER TABLE clip_bins RENAME COLUMN board_id TO bin_id", [])?;
    }

    // SQLite cannot rename indexes; replace any pre-release names after their tables move.
    conn.execute("DROP INDEX IF EXISTS idx_clips_board_created", [])?;
    conn.execute("DROP INDEX IF EXISTS idx_clip_boards_board_id", [])?;
    conn.execute("DROP INDEX IF EXISTS idx_clip_boards_clip_id", [])?;
    Ok(())
}

pub(crate) fn migrate_clip_source_schema(conn: &Connection) -> Result<()> {
    if !table_exists(conn, "clips")? {
        return Ok(());
    }
    let has_legacy_column = column_exists(conn, "clips", "source_app")?;
    let has_source_column = column_exists(conn, "clips", "source")?;
    if has_legacy_column && has_source_column {
        return Err(rusqlite::Error::InvalidQuery);
    }
    if !has_legacy_column {
        return Ok(());
    }

    // The FTS table is a derived cache whose schema cannot be altered in place.
    // Remove it and its writers inside the same transaction as the canonical
    // column rename; the normal startup path recreates and rebuilds it below.
    let transaction = conn.unchecked_transaction()?;
    transaction.execute_batch(
        "DROP TRIGGER IF EXISTS clips_ai;
         DROP TRIGGER IF EXISTS clips_ad;
         DROP TRIGGER IF EXISTS clips_au;
         DROP TABLE IF EXISTS clips_fts;
         ALTER TABLE clips RENAME COLUMN source_app TO source;",
    )?;
    if table_exists(&transaction, "bins")? && column_exists(&transaction, "bins", "smart_rule")? {
        transaction.execute(
            "UPDATE bins
             SET smart_rule = replace(smart_rule, '\"source_app\"', '\"source\"')
             WHERE smart_rule LIKE '%\"source_app\"%'",
            [],
        )?;
    }
    transaction.commit()?;
    Ok(())
}
