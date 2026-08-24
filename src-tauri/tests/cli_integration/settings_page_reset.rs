use super::support::*;

#[test]
fn settings_pages_reset_through_stable_structured_cli_contracts() {
    let database = temporary_path("settings-page-reset", "db");

    for (key, value, page, expected) in [
        ("revisionHistoryLimit", "7", "general", "10"),
        ("captureFeedback", "false", "notifications", "true"),
        ("hudHotkey", "Ctrl+Shift+9", "hotkeys", "Alt+Shift+V"),
        (
            "excludePrivateBrowserWindows",
            "true",
            "app-exclusions",
            "false",
        ),
    ] {
        success_json(&database, &["settings", "set", key, value, "--json"]);
        let reset = success_json(&database, &["settings", "reset", page, "--json"]);
        assert_eq!(reset["page"], page);
        assert_eq!(reset["reset"], true);
        assert!(reset["details"]["changeCount"]
            .as_u64()
            .is_some_and(|count| count > 0));
        assert_eq!(
            success_json(&database, &["settings", "get", key, "--json"])["value"],
            expected
        );
    }

    assert_eq!(
        success_json(&database, &["settings", "reset", "analysis", "--json"])["details"]
            ["customDefinitionsPreserved"],
        true
    );
    assert_eq!(
        success_json(&database, &["settings", "reset", "security", "--json"])["details"]
            ["credentialsPreserved"],
        true
    );
    for (page, preservation) in [
        ("analysis", "customDefinitionsPreserved"),
        ("security", "credentialsPreserved"),
    ] {
        let preview = success_json(
            &database,
            &["settings", "reset", page, "--dry-run", "--json"],
        );
        assert_eq!(preview["reset"], false);
        assert_eq!(preview["details"][preservation], true);
    }

    success_json(
        &database,
        &[
            "connection",
            "create",
            "--name",
            "Reset Me",
            "--provider",
            "ollama",
            "--json",
        ],
    );
    let intelligence = success_json(&database, &["settings", "reset", "intelligence", "--json"]);
    assert_eq!(intelligence["details"]["connectionDetailsPreserved"], true);
    assert_eq!(
        success_json(&database, &["connection", "list", "--json"])[0]["enabled"],
        false
    );

    for (key, value) in [
        ("unregisteredSetting", "value"),
        ("backedUpClientState", "{}"),
    ] {
        let output = run(&database, &["settings", "set", key, value, "--json"]);
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&output.stderr).contains("internal"));
    }

    clean_database(&database);
}
