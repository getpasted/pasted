use super::*;

pub(crate) fn insert_default_bins(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT INTO bins (name, icon, color, smart_rule) VALUES ('Images', '🖼️', '#ec4899', '{\"version\":1,\"conditions\":[{\"type\":\"clip_type\",\"operator\":\"is\",\"value\":\"image\"}],\"match\":\"any\"}')",
        [],
    )?;
    conn.execute(
        "INSERT INTO bins (name, icon, color, smart_rule) VALUES ('Links and Web', 'Link', '#3b82f6', '{\"version\":1,\"conditions\":[{\"type\":\"content_type\",\"operator\":\"is\",\"value\":\"link\"}],\"match\":\"any\"}')",
        [],
    )?;
    conn.execute(
        "INSERT INTO bins (name, icon, color, smart_rule) VALUES ('Code Snippets', 'Code', '#10b981', '{\"version\":1,\"conditions\":[{\"type\":\"content_type\",\"operator\":\"is\",\"value\":\"code\"}],\"match\":\"any\"}')",
        [],
    )?;
    Ok(())
}

pub(crate) fn table_exists(conn: &Connection, name: &str) -> Result<bool> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        params![name],
        |row| row.get(0),
    )
}

pub(crate) fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let names = statement.query_map([], |row| row.get::<_, String>(1))?;
    for name in names {
        if name? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<()> {
    if !column_exists(conn, table, column)? {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
            [],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
