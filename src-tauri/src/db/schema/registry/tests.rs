use super::*;

fn fail_after_write(conn: &Connection) -> Result<()> {
    conn.execute("CREATE TABLE should_roll_back (id INTEGER)", [])?;
    Err(rusqlite::Error::InvalidQuery)
}

fn succeed(conn: &Connection) -> Result<()> {
    conn.execute("CREATE TABLE migrated_table (id INTEGER)", [])?;
    Ok(())
}

#[test]
fn named_migrations_mark_only_successful_atomic_steps() {
    let conn = Connection::open_in_memory().unwrap();
    let failing = NamedMigration {
        key: "failingV1",
        apply: fail_after_write,
    };
    assert!(run_named_migrations(&conn, &[failing]).is_err());
    assert!(!table_exists(&conn, "should_roll_back").unwrap());
    let marked: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE key = 'failingV1')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!marked);

    let successful = NamedMigration {
        key: "successfulV1",
        apply: succeed,
    };
    run_named_migrations(&conn, &[successful]).unwrap();
    run_named_migrations(&conn, &[successful]).unwrap();
    assert!(table_exists(&conn, "migrated_table").unwrap());
}
