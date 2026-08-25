use super::support::*;

mod version_support;
mod versions;

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
fn clip_visual_labels_have_stable_cli_mutation_and_reset_contracts() {
    let database = temporary_path("clip-visual-labels", "db");
    let clip = success_json(&database, &["copy", "labelled CLI clip", "--json"]);
    let clip_id = clip["id"].as_i64().expect("clip ID").to_string();

    let added = success_json(
        &database,
        &["clip", "labels", "add", &clip_id, "favorite", "--json"],
    );
    assert_eq!(added["clipId"].to_string(), clip_id);
    assert_eq!(added["labels"][0]["value"], "favorite");
    assert_eq!(added["labels"][0]["source"], "manual");
    assert_eq!(added["hasOverrides"], true);

    let listed = success_json(&database, &["clip", "labels", "list", &clip_id, "--json"]);
    assert_eq!(listed, added);

    let refused = run(&database, &["clip", "labels", "reset", &clip_id, "--json"]);
    assert!(!refused.status.success());
    let preserved = success_json(&database, &["clip", "labels", "list", &clip_id, "--json"]);
    assert_eq!(preserved["labels"][0]["value"], "favorite");

    let reset = success_json(
        &database,
        &["clip", "labels", "reset", &clip_id, "--yes", "--json"],
    );
    assert_eq!(reset["labels"], serde_json::json!([]));
    assert_eq!(reset["hasOverrides"], false);
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
        "pasted clip labels",
        "pasted clip versions",
        "pasted clip restore-version",
        "pasted clip delete-version",
        "pasted recording",
        "pasted queue",
        "pasted backup inspect",
        "pasted transform test",
    ] {
        assert!(text.contains(command), "help omitted {command}");
    }
    clean_database(&database);
}
