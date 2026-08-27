use std::collections::HashMap;

use pasted_lib::db::DbState;

use super::super::support::{clean_database, run, temporary_path};

#[test]
fn update_check_stops_at_the_functionality_gate_when_disabled() {
    let database = temporary_path("update-policy", "db");
    let db = DbState::new(database.clone()).expect("create test database");
    db.save_settings(&HashMap::from([(
        "enableUpdateChecks".to_string(),
        "false".to_string(),
    )]))
    .expect("disable update checks");
    drop(db);

    let output = run(&database, &["update", "check", "--json"]);

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "Software Updates is disabled in Settings → Functionality."
    );
    assert!(output.stdout.is_empty());

    clean_database(&database);
}
