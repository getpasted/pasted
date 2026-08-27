use super::super::support::*;

#[test]
fn app_lock_policy_has_a_stable_cli_contract() {
    let database = temporary_path("app-lock-policy", "db");

    let initial = success_json(&database, &["app-lock", "status", "--json"]);
    assert_eq!(initial["lockOnRestart"], true);
    assert!(initial["systemAuthAvailable"].is_boolean());
    assert!(initial["appleWatchAvailable"].is_boolean());

    let changed = success_json(&database, &["app-lock", "lock-on-restart", "off", "--json"]);
    assert_eq!(changed["lockOnRestart"], false);
    assert_eq!(
        success_json(&database, &["app-lock", "idle", "1h", "--json"])["idleMinutes"],
        60
    );
    assert_eq!(
        success_json(&database, &["app-lock", "lock-on-sleep", "off", "--json"])["lockOnSleep"],
        false
    );
    assert_eq!(
        success_json(
            &database,
            &["app-lock", "capture-while-locked", "off", "--json"],
        )["captureWhileLocked"],
        false
    );
    assert_eq!(
        success_json(&database, &["app-lock", "system-auth", "off", "--json"])["systemAuthEnabled"],
        false
    );
    assert_eq!(
        success_json(&database, &["app-lock", "apple-watch", "off", "--json"])["appleWatchEnabled"],
        false
    );

    let enabled = success_json_with_stdin(
        &database,
        &["app-lock", "enable", "--stdin", "--json"],
        "x\n",
    );
    assert_eq!(enabled["enabled"], true);
    let changed_passphrase = success_json_with_stdin(
        &database,
        &["app-lock", "change-passphrase", "--stdin", "--json"],
        "x\ny\n",
    );
    assert_eq!(changed_passphrase["changed"], true);
    let reset_policy = success_json_with_stdin(
        &database,
        &["app-lock", "reset-policy", "--stdin", "--json"],
        "y\n",
    );
    assert_eq!(reset_policy["reset"], true);
    assert_eq!(reset_policy["credentialsPreserved"], true);
    assert_eq!(reset_policy["lockOnRestart"], true);
    assert_eq!(reset_policy["lockOnSleep"], true);
    assert_eq!(reset_policy["captureWhileLocked"], true);

    let disabled = success_json_with_stdin(
        &database,
        &["app-lock", "disable", "--stdin", "--json"],
        "y\n",
    );
    assert_eq!(disabled["enabled"], false);
    assert_eq!(disabled["credentialsCleared"], true);

    let connection = rusqlite::Connection::open(&database).expect("open CLI database");
    let verifier = connection
        .query_row(
            "SELECT value FROM settings WHERE key = 'appLockVerifier'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .expect("query app-lock verifier");
    assert_eq!(verifier, None);
    let status = success_json(&database, &["app-lock", "status", "--json"]);
    assert_eq!(status["lockOnRestart"], true);
    assert_eq!(status["lockOnSleep"], true);
    assert_eq!(status["captureWhileLocked"], true);
    clean_database(&database);
}
