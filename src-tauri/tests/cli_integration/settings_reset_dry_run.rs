use super::support::*;

#[test]
fn settings_reset_dry_runs_report_changes_without_mutation() {
    let database = temporary_path("settings-reset-dry-run", "db");
    success_json(
        &database,
        &["settings", "set", "themeMode", "warm", "--json"],
    );

    let preview = success_json(
        &database,
        &["settings", "reset", "general", "--dry-run", "--json"],
    );
    assert_eq!(preview["page"], "general");
    assert_eq!(preview["reset"], false);
    assert_eq!(preview["dryRun"], true);
    assert!(preview["details"]["changes"]
        .as_array()
        .is_some_and(|changes| changes.iter().any(|change| change["key"] == "themeMode")));
    assert_eq!(
        success_json(&database, &["settings", "get", "themeMode", "--json"])["value"],
        "warm"
    );

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
    let preview = success_json(
        &database,
        &["settings", "reset", "intelligence", "--dry-run", "--json"],
    );
    assert_eq!(preview["details"]["connectionDetailsPreserved"], true);
    assert_eq!(
        success_json(&database, &["connection", "list", "--json"])[0]["enabled"],
        true
    );
    clean_database(&database);
}
