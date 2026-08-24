use super::support::*;

#[test]
fn app_lock_restart_policy_has_a_stable_cli_contract() {
    let database = temporary_path("app-lock-restart", "db");

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
    let idle = success_json_with_stdin(
        &database,
        &["app-lock", "idle", "5m", "--stdin", "--json"],
        "y\n",
    );
    assert_eq!(idle["idleMinutes"], 5);

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
    assert_eq!(status["lockOnRestart"], false);
    assert_eq!(status["lockOnSleep"], false);
    assert_eq!(status["captureWhileLocked"], false);
    clean_database(&database);
}

#[test]
fn database_protection_has_a_conservative_cli_contract() {
    let database = temporary_path("storage-protection", "db");
    let protection = success_json(&database, &["database", "protection", "--json"]);
    assert!(matches!(
        protection["status"].as_str(),
        Some("protected" | "notDetected" | "unknown")
    ));
    assert!(protection["summary"].is_string());
    assert!(protection["detail"].is_string());
    assert!(protection.get("technology").is_some());
    clean_database(&database);
}

#[test]
fn clip_hotkeys_and_bin_policies_have_structured_cli_parity() {
    let database = temporary_path("clip-hotkeys-bin-protection", "db");
    let clip = success_json(&database, &["copy", "durable CLI clip", "--json"]);
    let clip_id = clip["id"].as_i64().expect("clip ID").to_string();
    let bin = success_json(
        &database,
        &["bin", "create", "--name", "Protected CLI Bin", "--json"],
    );
    let bin_id = bin["id"].as_i64().expect("Bin ID").to_string();

    let hotkey = success_json(
        &database,
        &["clip", "hotkey", &clip_id, "Alt+Shift+7", "--json"],
    );
    assert_eq!(hotkey["clipId"].to_string(), clip_id);
    assert_eq!(hotkey["hotkey"], "Alt+Shift+7");
    assert_eq!(hotkey["protected"], true);

    let protection = success_json(&database, &["bin", "protect", &bin_id, "on", "--json"]);
    assert_eq!(protection["protectClips"], true);
    let concealment = success_json(&database, &["bin", "conceal", &bin_id, "on", "--json"]);
    assert_eq!(concealment["concealClips"], true);
    let concealed_type = success_json(
        &database,
        &[
            "type",
            "update",
            "payment_card",
            "--conceal",
            "off",
            "--json",
        ],
    );
    assert_eq!(concealed_type["concealClips"], false);
    let clip_concealment = success_json(&database, &["clip", "conceal", &clip_id, "--json"]);
    assert_eq!(clip_concealment["action"], "conceal");
    assert_eq!(clip_concealment["changedCount"], 1);
    success_json(&database, &["clip", "assign", &bin_id, &clip_id, "--json"]);

    let fetched = success_json(&database, &["clip", "get", &clip_id, "--json"]);
    assert_eq!(fetched["hotkey"], "Alt+Shift+7");
    assert_eq!(fetched["is_protected"], true);
    assert_eq!(fetched["is_explicitly_protected"], true);
    assert_eq!(fetched["is_concealed"], true);
    assert_eq!(fetched["is_explicitly_concealed"], true);

    let revealed = success_json(&database, &["clip", "reveal", &clip_id, "--json"]);
    assert_eq!(revealed["action"], "reveal");
    assert_eq!(revealed["changedCount"], 1);
    let fetched = success_json(&database, &["clip", "get", &clip_id, "--json"]);
    assert_eq!(fetched["is_concealed"], false);
    assert_eq!(fetched["is_explicitly_revealed"], true);
    success_json(&database, &["clip", "conceal", &clip_id, "--json"]);

    success_json(
        &database,
        &["settings", "set", "enableHotkeys", "false", "--json"],
    );
    for arguments in [
        vec!["clip", "hotkey", clip_id.as_str(), "none", "--json"],
        vec!["bin", "hotkey", bin_id.as_str(), "Alt+Shift+8", "--json"],
    ] {
        let disabled = run(&database, &arguments);
        assert!(!disabled.status.success());
        assert!(String::from_utf8_lossy(&disabled.stderr)
            .contains("Hotkeys is disabled in Settings → Functionality"));
    }
    let preserved = success_json(&database, &["clip", "get", &clip_id, "--json"]);
    assert_eq!(preserved["hotkey"], "Alt+Shift+7");
    success_json(
        &database,
        &["settings", "set", "enableHotkeys", "true", "--json"],
    );

    success_json(
        &database,
        &["settings", "set", "enableConcealment", "false", "--json"],
    );
    for arguments in [
        vec!["clip", "reveal", clip_id.as_str(), "--json"],
        vec!["bin", "conceal", bin_id.as_str(), "off", "--json"],
        vec![
            "type",
            "update",
            "payment_card",
            "--conceal",
            "on",
            "--json",
        ],
    ] {
        let disabled = run(&database, &arguments);
        assert!(!disabled.status.success());
        assert!(String::from_utf8_lossy(&disabled.stderr)
            .contains("Concealment is disabled in Settings → Functionality"));
    }
    let preserved_bin = success_json(&database, &["bin", "get", &bin_id, "--json"]);
    assert_eq!(preserved_bin["bin"]["conceal_clips"], true);
    let preserved_type = success_json(&database, &["type", "list", "--json"]);
    assert_eq!(
        preserved_type
            .as_array()
            .and_then(|types| types.iter().find(|item| item["id"] == "payment_card"))
            .map(|item| item["concealClips"].clone()),
        Some(Value::Bool(false))
    );

    let cleared = success_json(&database, &["clip", "hotkey", &clip_id, "none", "--json"]);
    assert_eq!(cleared["hotkey"], Value::Null);
    let fetched = success_json(&database, &["clip", "get", &clip_id, "--json"]);
    assert_eq!(fetched["is_protected"], true);
    clean_database(&database);
}

#[test]
fn help_advertises_database_and_live_app_surfaces() {
    let database = temporary_path("help", "db");
    let output = run(&database, &["help"]);
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    for command in [
        "pasted analyzer",
        "pasted settings",
        "pasted recording",
        "pasted queue",
        "pasted backup inspect",
        "pasted transform test",
    ] {
        assert!(text.contains(command), "help omitted {command}");
    }
    clean_database(&database);
}
