use super::support::*;

#[test]
fn history_and_settings_commands_have_executable_json_contracts() {
    let database = temporary_path("history", "db");
    let saved = success_json(&database, &["copy", "person@example.com", "--json"]);
    assert_eq!(saved["contentType"], "text");
    assert_eq!(saved["contentTypes"][0], "email");

    let listed = success_json(&database, &["list", "--limit", "5", "--json"]);
    assert_eq!(listed.as_array().map(Vec::len), Some(1));
    assert_eq!(listed[0]["content_type"], "text");
    assert_eq!(listed[0]["content_types"][0], "email");

    let insights = success_json(&database, &["insights", "summary", "--json"]);
    assert_eq!(insights["clip_types"][0]["clip_type"], "text");
    assert!(insights["clip_types"][0].get("content_type").is_none());
    assert_eq!(insights["content_types"][0]["content_type"], "email");
    assert_eq!(
        insights["daily_activity"].as_array().map(Vec::len),
        Some(14)
    );
    let plain_insights = run(&database, &["insights", "summary"]);
    assert!(plain_insights.status.success());
    let plain_insights = String::from_utf8_lossy(&plain_insights.stdout);
    assert!(plain_insights.contains("Daily activity (local time):"));
    assert!(plain_insights.contains(
        insights["daily_activity"][0]["date"]
            .as_str()
            .expect("daily date")
    ));

    let setting = success_json(
        &database,
        &["settings", "set", "revisionHistoryLimit", "7", "--json"],
    );
    assert_eq!(setting["value"], "7");
    let fetched = success_json(
        &database,
        &["settings", "get", "revisionHistoryLimit", "--json"],
    );
    assert_eq!(fetched["value"], "7");

    let language = success_json(&database, &["settings", "set", "language", "en", "--json"]);
    assert_eq!(language["value"], "en");
    let invalid_language = run(
        &database,
        &["settings", "set", "language", "not-a-locale", "--json"],
    );
    assert_eq!(invalid_language.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&invalid_language.stderr).contains("Unsupported language"));
    assert_eq!(
        success_json(&database, &["settings", "get", "language", "--json"],)["value"],
        "en"
    );
    let refused = run(&database, &["clear"]);
    assert_eq!(refused.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&refused.stderr).contains("--yes"));
    clean_database(&database);
}

#[test]
fn extracted_management_adapters_preserve_structured_cli_contracts() {
    let database = temporary_path("management-adapters", "db");
    let activity_path = temporary_path("activity-export", "json");
    let transfer_path = temporary_path("transfer-export", "json");

    let retention = success_json(
        &database,
        &[
            "retention",
            "--count",
            "25",
            "--days",
            "30",
            "--trash-count",
            "10",
            "--trash-days",
            "7",
            "--log-count",
            "50",
            "--log-days",
            "14",
            "--revision-count",
            "5",
            "--analysis-count",
            "25",
            "--json",
        ],
    );
    assert_eq!(retention["maximumClips"], 25);
    assert_eq!(retention["maximumAgeDays"], 30);
    assert_eq!(retention["trashMaximumClips"], 10);
    assert_eq!(retention["activityMaximumEntries"], 50);
    assert_eq!(retention["revisionsPerClip"], 5);
    assert_eq!(retention["analyzationsPerClip"], 25);

    success_json(&database, &["copy", "portable adapter clip", "--json"]);
    success_json(
        &database,
        &["settings", "set", "enableHotkeys", "false", "--json"],
    );
    let activity = success_json(&database, &["activity", "list", "--all", "--json"]);
    assert!(activity
        .as_array()
        .is_some_and(|entries| !entries.is_empty()));

    let exported_activity = success_json(
        &database,
        &[
            "activity",
            "export",
            activity_path.to_str().expect("activity export path"),
            "--json",
        ],
    );
    assert_eq!(exported_activity["format"], "json");
    assert_eq!(
        success_json(&database, &["activity", "clear", "--yes", "--json"])["cleared"],
        true
    );
    assert_eq!(
        success_json(&database, &["activity", "list", "--all", "--json"])
            .as_array()
            .map(Vec::len),
        Some(0)
    );
    let imported_activity = success_json(
        &database,
        &[
            "activity",
            "import",
            activity_path.to_str().expect("activity import path"),
            "--json",
        ],
    );
    assert!(imported_activity["importedCount"]
        .as_u64()
        .is_some_and(|count| count > 0));

    let exported_transfer = success_json(
        &database,
        &[
            "transfer",
            "export",
            transfer_path.to_str().expect("transfer export path"),
            "--json",
        ],
    );
    assert_eq!(exported_transfer["inspection"]["clipCount"], 1);
    let inspected_transfer = success_json(
        &database,
        &[
            "transfer",
            "inspect",
            transfer_path.to_str().expect("transfer inspect path"),
            "--json",
        ],
    );
    assert_eq!(inspected_transfer["clipCount"], 1);
    let imported_transfer = success_json(
        &database,
        &[
            "transfer",
            "import",
            transfer_path.to_str().expect("transfer import path"),
            "--json",
        ],
    );
    assert_eq!(imported_transfer["processedClipCount"], 1);

    clean_database(&database);
    let _ = std::fs::remove_file(activity_path);
    let _ = std::fs::remove_file(transfer_path);
}

#[test]
fn search_uses_the_shared_paginated_contract_and_fuzzy_collection_filters() {
    let database = temporary_path("search", "db");
    success_json(&database, &["copy", "first@example.com", "--json"]);
    success_json(&database, &["copy", "second@example.com", "--json"]);

    let first_page = success_json(
        &database,
        &[
            "search",
            "example.com",
            "--clip",
            "te",
            "--content",
            "mai",
            "--source",
            "Terminal",
            "--limit",
            "1",
            "--json",
        ],
    );
    assert_eq!(first_page["schemaVersion"], 1);
    assert_eq!(first_page["totalCount"], 2);
    assert_eq!(first_page["limit"], 1);
    assert_eq!(first_page["offset"], 0);
    assert_eq!(first_page["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(first_page["items"][0]["content_type"], "text");
    assert_eq!(first_page["items"][0]["content_types"][0], "email");
    assert!(first_page["items"][0].get("file_formats").is_some());
    assert!(first_page["items"][0].get("source").is_some());

    let second_page = success_json(
        &database,
        &[
            "search",
            "example.com",
            "--limit",
            "1",
            "--offset",
            "1",
            "--json",
        ],
    );
    assert_eq!(second_page["totalCount"], 2);
    assert_ne!(first_page["items"][0]["id"], second_page["items"][0]["id"]);

    let partial_source = success_json(&database, &["search", "--source", "CLI", "--json"]);
    assert_eq!(partial_source["totalCount"], 2);
    let oversized_page = run(&database, &["search", "--limit", "501", "--json"]);
    assert!(!oversized_page.status.success());
    assert!(String::from_utf8_lossy(&oversized_page.stderr)
        .contains("--limit must be between 1 and 500"));
    clean_database(&database);
}

#[test]
fn bin_lifecycle_and_full_backup_inspection_run_end_to_end() {
    let database = temporary_path("management", "db");
    let created = success_json(&database, &["bin", "create", "--name", "CLI Bin", "--json"]);
    let bin_id = created["id"].as_i64().expect("Bin ID");
    let bin_id_text = bin_id.to_string();
    let fetched = success_json(&database, &["bin", "get", &bin_id_text, "--json"]);
    assert_eq!(fetched["bin"]["name"], "CLI Bin");
    let duplicate = success_json(
        &database,
        &[
            "bin",
            "duplicate",
            &bin_id_text,
            "--name",
            "CLI Bin Copy",
            "--json",
        ],
    );
    assert_eq!(duplicate["name"], "CLI Bin Copy");

    let backup = temporary_path("inspection", "pastedbackup");
    success_json(
        &database,
        &[
            "backup",
            "create",
            backup.to_str().expect("backup path"),
            "--json",
        ],
    );
    let inspection = success_json(
        &database,
        &[
            "backup",
            "inspect",
            backup.to_str().expect("backup path"),
            "--json",
        ],
    );
    assert_eq!(inspection["formatVersion"], 1);

    let _ = std::fs::remove_file(backup);
    clean_database(&database);
}
