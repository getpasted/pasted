use super::*;

const DEFAULT_BROWSER_SMART_RULE: &str = concat!(
    r#"{"version":1,"conditions":["#,
    r#"{"type":"source","operator":"contains","value":"Safari"},"#,
    r#"{"type":"source","operator":"contains","value":"Chrome"},"#,
    r#"{"type":"source","operator":"contains","value":"Chromium"},"#,
    r#"{"type":"source","operator":"contains","value":"Firefox"},"#,
    r#"{"type":"source","operator":"contains","value":"Edge"},"#,
    r#"{"type":"source","operator":"is","value":"Arc"},"#,
    r#"{"type":"source","operator":"contains","value":"Brave"},"#,
    r#"{"type":"source","operator":"contains","value":"Vivaldi"},"#,
    r#"{"type":"source","operator":"contains","value":"Opera"},"#,
    r#"{"type":"source","operator":"contains","value":"Orion"},"#,
    r#"{"type":"source","operator":"is","value":"Zen Browser"}],"match":"any"}"#,
);

pub(crate) fn insert_default_bins(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT INTO bins (name, icon, color, smart_rule) VALUES ('Projects', '🗂️', '#6b7280', NULL)",
        [],
    )?;
    conn.execute(
        "INSERT INTO bins (name, icon, color, smart_rule) VALUES ('From Browsers', '🌐', '#6b7280', ?1)",
        params![DEFAULT_BROWSER_SMART_RULE],
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
