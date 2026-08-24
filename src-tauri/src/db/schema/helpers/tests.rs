use super::*;

#[test]
fn additive_migrations_are_idempotent_without_swallowing_sqlite_failures() {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute("CREATE TABLE example (id INTEGER)", [])
        .unwrap();
    add_column_if_missing(&connection, "example", "label", "TEXT").unwrap();
    add_column_if_missing(&connection, "example", "label", "TEXT").unwrap();
    assert!(column_exists(&connection, "example", "label").unwrap());

    let error = add_column_if_missing(&connection, "missing_table", "label", "TEXT")
        .expect_err("a missing migration target must fail startup");
    assert!(error.to_string().contains("no such table"));
}
