use rusqlite::OptionalExtension;
use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_path(label: &str, extension: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "pasted-cli-{label}-{}-{stamp}.{extension}",
        std::process::id()
    ))
}

fn run(database: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pasted"))
        .env("PASTED_DATABASE_PATH", database)
        .env("PASTED_CONFIG_DIR", database.with_extension("config"))
        .args(arguments)
        .output()
        .expect("run pasted CLI")
}

fn run_with_stdin(database: &Path, arguments: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_pasted"))
        .env("PASTED_DATABASE_PATH", database)
        .env("PASTED_CONFIG_DIR", database.with_extension("config"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run pasted CLI with stdin");
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(input.as_bytes())
        .expect("write CLI stdin");
    child.wait_with_output().expect("read pasted CLI output")
}

fn success_json(database: &Path, arguments: &[&str]) -> Value {
    let output = run(database, arguments);
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("valid JSON output")
}

fn success_json_with_stdin(database: &Path, arguments: &[&str], input: &str) -> Value {
    let output = run_with_stdin(database, arguments, input);
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("valid JSON output")
}

fn analysis_fixture(name: &str) -> Value {
    let contents = match name {
        "analyzer-interactive-text" => {
            include_str!("../../contracts/analysis/v1/analyzer-interactive-text.json")
        }
        "analyzer-capture-text" => {
            include_str!("../../contracts/analysis/v1/analyzer-capture-text.json")
        }
        "inspector-interactive-text" => {
            include_str!("../../contracts/analysis/v1/inspector-interactive-text.json")
        }
        "suggestion-interactive-empty" => {
            include_str!("../../contracts/analysis/v1/suggestion-interactive-empty.json")
        }
        "extractor-interactive-unavailable" => {
            include_str!("../../contracts/analysis/v1/extractor-interactive-unavailable.json")
        }
        "classifier-interactive-no-match" => {
            include_str!("../../contracts/analysis/v1/classifier-interactive-no-match.json")
        }
        _ => panic!("unknown Analysis fixture {name}"),
    };
    serde_json::from_str(contents).expect("valid Analysis contract fixture")
}

fn clean_database(path: &Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
}

#[test]
fn extractor_recipes_have_cli_authoring_and_execution_parity() {
    let database = temporary_path("extractor-recipe", "db");
    let recipe_path = temporary_path("extractor-recipe", "json");
    let input_path = temporary_path("extractor-input", "txt");
    let recipe = serde_json::json!({
        "definitionVersion": 1,
        "accepts": ["file_references"],
        "output": "searchable_text",
        "steps": [{
            "id": "extract",
            "executable": {
                "path": env!("CARGO_BIN_EXE_pasted"),
                "discover": [],
                "versionArguments": ["--version"]
            },
            "arguments": ["licenses"],
            "mode": "once",
            "capture": "stdout_text",
            "timeoutSeconds": 30
        }],
        "resources": []
    });
    std::fs::write(
        &recipe_path,
        serde_json::to_vec_pretty(&recipe).expect("serialize recipe"),
    )
    .expect("write recipe");
    std::fs::write(&input_path, "input").expect("write input");

    let created = success_json(
        &database,
        &[
            "extractor",
            "create",
            "--name",
            "Portable Test Extractor",
            "--recipe",
            recipe_path.to_str().expect("recipe path"),
            "--json",
        ],
    );
    assert_eq!(created["engine"], "recipe-v1");
    assert_eq!(created["recipe"]["accepts"][0], "file_references");

    let history = success_json(
        &database,
        &[
            "extractor",
            "history",
            created["stableRef"].as_str().expect("stable ref"),
            "--json",
        ],
    );
    assert_eq!(history[0]["source"], "manual");

    let run = success_json(
        &database,
        &[
            "extractor",
            "run",
            created["stableRef"].as_str().expect("stable ref"),
            "--file",
            input_path.to_str().expect("input path"),
            "--json",
        ],
    );
    assert_eq!(run["outcome"], "produced");
    assert!(run["output"]
        .as_str()
        .is_some_and(|output| !output.is_empty()));

    clean_database(&database);
    let _ = std::fs::remove_file(recipe_path);
    let _ = std::fs::remove_file(input_path);
}

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
fn structural_inspector_has_registry_preview_and_apply_parity() {
    let database = temporary_path("inspector", "db");
    let clip = success_json(&database, &["copy", "alpha beta\ngamma", "--json"]);
    let clip_id = clip["id"].as_i64().expect("clip ID");
    let clip_id_text = clip_id.to_string();

    let inspectors = success_json(&database, &["inspector", "list", "--json"]);
    assert_eq!(inspectors[0]["stableRef"], "inspector:structure-v1");
    assert_eq!(inspectors[0]["outputContract"], "structural_metadata");
    let media = inspectors
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|item| item["stableRef"] == "inspector:media-metadata-v1")
        })
        .expect("shipped Media Metadata Inspector");
    assert!(matches!(
        media["engine"].as_str(),
        Some("ffprobe-cli-v1" | "mediainfo-cli-v1")
    ));
    assert_eq!(media["inputContract"], "file_references");
    assert_eq!(media["outputContract"], "media_metadata");
    assert!(media["isAvailable"].is_boolean());
    let legacy_media = success_json(
        &database,
        &["inspector", "get", "inspector:ffprobe-media-v1", "--json"],
    );
    assert_eq!(legacy_media["stableRef"], "inspector:media-metadata-v1");

    let registry = success_json(
        &database,
        &["registry", "list", "--kind", "inspector", "--json"],
    );
    assert_eq!(registry[0]["analysisPass"], "inspect");
    assert_eq!(registry[0]["participantContract"]["pass"], "inspect");
    assert_eq!(
        registry[0]["participantContract"]["requires"][0],
        "clip_kind"
    );
    assert_eq!(registry[0]["capabilities"]["canDisable"], false);
    assert!(registry
        .as_array()
        .is_some_and(|items| items.iter().any(|item| {
            item["stableRef"] == "inspector:media-metadata-v1"
                && item["outputContract"] == "media_metadata"
                && item["typeRelations"][0]["kind"] == "accepts"
                && item["typeRelations"][0]["typeId"] == "file"
        })));

    let preview = success_json(
        &database,
        &["inspector", "run", "--clip", &clip_id_text, "--json"],
    );
    assert_eq!(preview["formatVersion"], 1);
    assert_eq!(preview["result"]["text"]["characterCount"], 16);
    assert_eq!(preview["result"]["text"]["wordCount"], 3);
    assert_eq!(preview["result"]["text"]["lineCount"], 2);
    assert_eq!(preview["appliedClipId"], Value::Null);
    assert!(!preview.to_string().contains("alpha beta"));
    assert_eq!(preview, analysis_fixture("inspector-interactive-text"));

    let unicode = success_json(
        &database,
        &["inspector", "run", "--text", "é 😀\n", "--json"],
    );
    assert_eq!(unicode["result"]["byteCount"], 8);
    assert_eq!(unicode["result"]["text"]["characterCount"], 4);
    assert_eq!(unicode["result"]["text"]["wordCount"], 2);
    assert_eq!(unicode["result"]["text"]["lineCount"], 1);

    let applied = success_json(
        &database,
        &[
            "inspector",
            "run",
            "--clip",
            &clip_id_text,
            "--apply",
            "--json",
        ],
    );
    assert_eq!(applied["appliedClipId"], clip_id);
    clean_database(&database);
}

#[test]
fn smart_actions_suggestion_has_registry_and_non_mutating_cli_parity() {
    let database = temporary_path("suggestion", "db");
    let transform = success_json(
        &database,
        &[
            "transform",
            "create",
            "--name",
            "Clean URL",
            "--steps-json",
            r#"[{"operationRef":"builtin:clean_url_tracking","configJson":null,"failurePolicy":"stop"}]"#,
            "--json",
        ],
    );
    let transform_ref = transform["stableRef"].as_str().expect("Transform ref");
    let secret_url = "https://example.com/private-token-0123456789?utm_source=test";
    let clip = success_json(&database, &["copy", secret_url, "--json"]);
    let clip_id = clip["id"].as_i64().expect("clip ID").to_string();

    let suggestions = success_json(&database, &["suggestion", "list", "--json"]);
    assert_eq!(suggestions[0]["stableRef"], "suggestion:smart-actions-v1");
    assert_eq!(suggestions[0]["outputContract"], "suggestions");

    let registry = success_json(
        &database,
        &["registry", "list", "--kind", "suggestion", "--json"],
    );
    assert_eq!(registry[0]["analysisPass"], "suggest");
    assert_eq!(
        registry[0]["participantContract"]["requires"],
        serde_json::json!(["analyzable_text", "structural_metadata"])
    );
    assert_eq!(
        registry[0]["inputContract"],
        "analyzable_text+structural_metadata"
    );
    assert_eq!(registry[0]["capabilities"]["canDisable"], false);

    let result = success_json(
        &database,
        &["suggestion", "run", "--clip", &clip_id, "--json"],
    );
    assert_eq!(result["formatVersion"], 1);
    assert_eq!(result["policy"], "interactive");
    assert_eq!(result["through"], "suggest");
    assert_eq!(result["result"]["signals"][0], "url");
    assert_eq!(
        result["result"]["actions"][0]["transformRef"],
        transform_ref
    );
    assert_eq!(result["appliedClipId"], Value::Null);
    assert!(!result.to_string().contains("private-token-0123456789"));

    let empty = success_json(
        &database,
        &["suggestion", "run", "--text", "ordinary words", "--json"],
    );
    assert_eq!(empty, analysis_fixture("suggestion-interactive-empty"));
    clean_database(&database);
}

#[test]
fn whole_analyzer_has_one_versioned_privacy_safe_cli_contract() {
    let database = temporary_path("analyzer", "db");
    let secret = "agent@example.com private-token-0123456789";
    let interactive = success_json(&database, &["analyzer", "run", "--text", secret, "--json"]);
    assert_eq!(interactive["formatVersion"], 1);
    assert_eq!(interactive["policy"], "interactive");
    assert_eq!(interactive["through"], "suggest");
    assert_eq!(interactive["result"]["clipKind"], "text");
    assert_eq!(
        interactive["result"]["classificationMatches"][0]["contentType"],
        "email"
    );
    assert!(interactive["result"]["structure"].is_object());
    assert!(interactive["result"]["suggestions"].is_object());
    assert_eq!(interactive["participants"][0]["pass"], "inspect");
    assert_eq!(interactive["participants"][1]["pass"], "classify");
    assert_eq!(interactive["participants"][2]["pass"], "suggest");
    assert!(!interactive.to_string().contains("private-token-0123456789"));
    assert_eq!(
        success_json(
            &database,
            &["analyzer", "run", "--text", "ordinary words", "--json"],
        ),
        analysis_fixture("analyzer-interactive-text")
    );

    let capture = success_json(
        &database,
        &[
            "analyzer",
            "run",
            "--text",
            "ordinary words",
            "--policy",
            "capture",
            "--json",
        ],
    );
    assert_eq!(capture["through"], "classify");
    assert!(capture["result"].get("suggestions").is_none());
    assert_eq!(capture["participants"].as_array().map(Vec::len), Some(2));
    assert!(!capture.to_string().contains("ordinary words"));
    assert_eq!(capture, analysis_fixture("analyzer-capture-text"));
    clean_database(&database);
}

#[test]
fn search_uses_the_shared_paginated_contract_and_exact_collection_filters() {
    let database = temporary_path("search", "db");
    success_json(&database, &["copy", "first@example.com", "--json"]);
    success_json(&database, &["copy", "second@example.com", "--json"]);

    let first_page = success_json(
        &database,
        &[
            "search",
            "example.com",
            "--clip",
            "text",
            "--content",
            "email",
            "--source",
            "CLI Terminal",
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
    assert_eq!(partial_source["totalCount"], 0);
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

#[test]
fn clip_shortcuts_and_bin_protection_have_structured_cli_parity() {
    let database = temporary_path("clip-shortcuts-bin-protection", "db");
    let clip = success_json(&database, &["copy", "durable CLI clip", "--json"]);
    let clip_id = clip["id"].as_i64().expect("clip ID").to_string();
    let bin = success_json(
        &database,
        &["bin", "create", "--name", "Protected CLI Bin", "--json"],
    );
    let bin_id = bin["id"].as_i64().expect("Bin ID").to_string();

    let shortcut = success_json(
        &database,
        &["clip", "shortcut", &clip_id, "Alt+Shift+7", "--json"],
    );
    assert_eq!(shortcut["clipId"].to_string(), clip_id);
    assert_eq!(shortcut["shortcut"], "Alt+Shift+7");
    assert_eq!(shortcut["protected"], true);

    let protection = success_json(&database, &["bin", "protect", &bin_id, "on", "--json"]);
    assert_eq!(protection["protectClips"], true);
    success_json(&database, &["clip", "assign", &bin_id, &clip_id, "--json"]);

    let fetched = success_json(&database, &["clip", "get", &clip_id, "--json"]);
    assert_eq!(fetched["shortcut"], "Alt+Shift+7");
    assert_eq!(fetched["is_protected"], true);
    assert_eq!(fetched["is_explicitly_protected"], true);

    let cleared = success_json(&database, &["clip", "shortcut", &clip_id, "none", "--json"]);
    assert_eq!(cleared["shortcut"], Value::Null);
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

#[test]
fn extractor_lifecycle_and_registry_capabilities_run_end_to_end() {
    let database = temporary_path("extractors", "db");
    let shipped = success_json(&database, &["extractor", "list", "--json"]);
    let tesseract = shipped
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|item| item["stableRef"] == "extractor:tesseract-ocr")
        })
        .expect("shipped Tesseract Extractor");
    assert_eq!(tesseract["engine"], "recipe-v1");
    assert_eq!(
        tesseract["recipe"]["steps"][0]["executable"]["discover"],
        serde_json::json!(["tesseract"])
    );
    assert_eq!(tesseract["inputContract"], "image");
    assert_eq!(tesseract["outputContract"], "searchable_text");
    assert!(tesseract["isAvailable"].is_boolean());
    assert_eq!(tesseract["runtime"]["method"], "recipe");
    assert!(tesseract["runtime"]["usesAutomaticDiscovery"].is_boolean());
    let whisper = shipped
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|item| item["stableRef"] == "extractor:whisper-transcription")
        })
        .expect("shipped Whisper Extractor");
    assert_eq!(whisper["engine"], "recipe-v1");
    assert_eq!(
        whisper["recipe"]["steps"][0]["executable"]["discover"],
        serde_json::json!(["ffmpeg"])
    );
    assert_eq!(
        whisper["recipe"]["steps"][1]["executable"]["discover"],
        serde_json::json!(["whisper-cli"])
    );
    assert_eq!(whisper["inputContract"], "file_references");
    assert_eq!(whisper["outputContract"], "searchable_text");
    assert_eq!(whisper["modelPath"], Value::Null);
    let configured_whisper = success_json(
        &database,
        &[
            "extractor",
            "update",
            "extractor:whisper-transcription",
            "--model",
            "/tmp/pasted-cli-missing-whisper-model.bin",
            "--json",
        ],
    );
    assert_eq!(
        configured_whisper["modelPath"],
        "/tmp/pasted-cli-missing-whisper-model.bin"
    );
    assert_eq!(configured_whisper["isAvailable"], false);

    let missing_executable = temporary_path("missing-custom-extractor", "bin");
    let executable = missing_executable.to_str().expect("custom executable path");
    let created = success_json(
        &database,
        &[
            "extractor",
            "create",
            "--name",
            "CLI Extractor",
            "--method",
            "custom-command",
            "--executable",
            executable,
            "--enabled",
            "--json",
        ],
    );
    let stable_ref = created["stableRef"].as_str().expect("Extractor stable ref");
    assert_eq!(created["engine"], "recipe-v1");
    assert_eq!(
        created["recipe"]["steps"][0]["executable"]["path"],
        executable
    );
    assert_eq!(created["executablePath"], executable);
    assert_eq!(created["isAvailable"], false);
    assert_eq!(created["enabled"], true);
    assert_eq!(created["revision"], 1);

    let fetched = success_json(&database, &["extractor", "get", stable_ref, "--json"]);
    assert_eq!(fetched["name"], "CLI Extractor");

    let registry = success_json(
        &database,
        &["registry", "list", "--kind", "extractor", "--json"],
    );
    let registry_item = registry
        .as_array()
        .and_then(|items| items.iter().find(|item| item["stableRef"] == stable_ref))
        .expect("Extractor registry item");
    assert_eq!(registry_item["analysisPass"], "extract");
    assert_eq!(
        registry_item["participantContract"]["provides"],
        serde_json::json!(["searchable_text", "analyzable_text"])
    );
    assert_eq!(registry_item["typeRelations"][0]["typeId"], "image");
    assert_eq!(registry_item["capabilities"]["canDuplicate"], true);
    assert_eq!(registry_item["capabilities"]["canDelete"], true);

    let duplicate = success_json(
        &database,
        &[
            "extractor",
            "duplicate",
            stable_ref,
            "--name",
            "CLI Extractor Copy",
            "--json",
        ],
    );
    assert_eq!(duplicate["name"], "CLI Extractor Copy");

    let image = temporary_path("extractor-input", "png");
    std::fs::write(&image, b"private image bytes").expect("write Extractor input");
    let preview_output = run(
        &database,
        &[
            "extractor",
            "run",
            stable_ref,
            "--file",
            image.to_str().expect("image path"),
            "--json",
        ],
    );
    assert_eq!(preview_output.status.code(), Some(1));
    let preview: Value = serde_json::from_slice(&preview_output.stdout).expect("Extractor JSON");
    assert_eq!(preview["formatVersion"], 1);
    assert_eq!(preview["policy"], "interactive");
    assert_eq!(preview["through"], "suggest");
    assert_eq!(preview["targetKind"], "extractor");
    assert_eq!(preview["targetRef"], stable_ref);
    assert_eq!(preview["outcome"], "failed");
    assert_eq!(preview["failure"]["code"], "engine_unavailable");
    assert_eq!(preview["appliedClipId"], Value::Null);
    assert_eq!(preview["ocrUpdated"], false);
    assert_eq!(preview["searchableTextUpdated"], false);
    assert_eq!(preview["classificationUpdated"], false);
    assert_eq!(preview["participants"][0]["pass"], "extract");
    assert_eq!(preview["participants"][0]["stableRef"], stable_ref);
    assert!(!preview.to_string().contains("private image bytes"));
    assert_eq!(
        analysis_fixture("extractor-interactive-unavailable")["failure"]["code"],
        "engine_not_installed"
    );
    let _ = std::fs::remove_file(image);

    success_json(
        &database,
        &[
            "registry",
            "disable",
            "--kind",
            "extractor",
            "--ref",
            stable_ref,
            "--json",
        ],
    );
    let disabled = success_json(&database, &["extractor", "get", stable_ref, "--json"]);
    assert_eq!(disabled["enabled"], false);
    let activity = success_json(&database, &["activity", "list", "--all", "--json"]);
    assert!(activity
        .as_array()
        .is_some_and(|logs| logs.iter().any(|log| {
            log["event_type"] == "content_extractor_disabled"
                && log["description"] == "Disabled Extractor \"CLI Extractor\""
        })));

    let deleted = success_json(&database, &["extractor", "delete", stable_ref, "--json"]);
    assert_eq!(deleted["deleted"], true);
    clean_database(&database);
}

#[test]
fn classifier_preview_and_apply_share_the_safe_execution_contract() {
    let database = temporary_path("classifiers", "db");
    let clip = success_json(&database, &["copy", "ticket-123", "--json"]);
    let clip_id = clip["id"].as_i64().expect("clip ID");
    let clip_id_text = clip_id.to_string();
    let classifier = success_json(
        &database,
        &[
            "classifier",
            "create",
            "--name",
            "Ticket IDs",
            "--type",
            "code",
            "--regex",
            "^ticket-[0-9]+$",
            "--json",
        ],
    );
    let stable_ref = classifier["stable_ref"]
        .as_str()
        .expect("Classifier stable ref");
    let fetched = success_json(&database, &["classifier", "get", stable_ref, "--json"]);
    assert_eq!(fetched["name"], "Ticket IDs");
    let duplicate = success_json(
        &database,
        &[
            "classifier",
            "duplicate",
            stable_ref,
            "--name",
            "Ticket IDs Copy",
            "--json",
        ],
    );
    assert_eq!(duplicate["name"], "Ticket IDs Copy");

    let registry = success_json(
        &database,
        &["registry", "list", "--kind", "classifier", "--json"],
    );
    let registry_item = registry
        .as_array()
        .and_then(|items| items.iter().find(|item| item["stableRef"] == stable_ref))
        .expect("Classifier registry item");
    assert_eq!(registry_item["analysisPass"], "classify");
    assert_eq!(
        registry_item["participantContract"]["requires"],
        serde_json::json!(["analyzable_text"])
    );
    assert_eq!(registry_item["typeRelations"][0]["kind"], "classifies_as");
    assert_eq!(registry_item["typeRelations"][0]["typeId"], "code");
    assert_eq!(registry_item["capabilities"]["canDuplicate"], true);
    assert_eq!(registry_item["capabilities"]["canDelete"], true);

    success_json(
        &database,
        &[
            "registry",
            "disable",
            "--kind",
            "classifier",
            "--ref",
            stable_ref,
            "--json",
        ],
    );
    success_json(
        &database,
        &[
            "registry",
            "enable",
            "--kind",
            "classifier",
            "--ref",
            stable_ref,
            "--json",
        ],
    );
    let activity = success_json(&database, &["activity", "list", "--all", "--json"]);
    assert!(activity.as_array().is_some_and(|logs| {
        logs.iter()
            .any(|log| log["event_type"] == "content_classifier_disabled")
            && logs
                .iter()
                .any(|log| log["event_type"] == "content_classifier_enabled")
    }));

    let mut no_match = success_json(
        &database,
        &[
            "classifier",
            "run",
            stable_ref,
            "--text",
            "ordinary words",
            "--json",
        ],
    );
    assert_eq!(no_match["targetRef"], stable_ref);
    no_match["targetRef"] = Value::String("classifier:email".into());
    assert_eq!(
        no_match,
        analysis_fixture("classifier-interactive-no-match")
    );

    let preview = success_json(
        &database,
        &[
            "classifier",
            "run",
            stable_ref,
            "--text",
            "ticket-123",
            "--json",
        ],
    );
    assert_eq!(preview["formatVersion"], 1);
    assert_eq!(preview["policy"], "interactive");
    assert_eq!(preview["through"], "suggest");
    assert_eq!(preview["targetKind"], "classifier");
    assert_eq!(preview["targetRef"], stable_ref);
    assert_eq!(preview["outcome"], "matched");
    assert_eq!(preview["matched"], true);
    assert_eq!(preview["contentTypes"][0], "code");
    assert_eq!(preview["matches"][0]["classifierRef"], stable_ref);
    assert_eq!(preview["appliedClipId"], Value::Null);
    assert_eq!(preview["participants"][0]["pass"], "classify");
    assert!(!preview.to_string().contains("ticket-123"));

    let applied = success_json(
        &database,
        &[
            "classifier",
            "run",
            stable_ref,
            "--clip",
            &clip_id_text,
            "--apply",
            "--json",
        ],
    );
    assert_eq!(applied["outcome"], "matched");
    assert_eq!(applied["appliedClipId"], clip_id);

    let clips = success_json(&database, &["list", "--limit", "5", "--json"]);
    let updated = clips
        .as_array()
        .and_then(|items| items.iter().find(|item| item["id"] == clip_id))
        .expect("updated clip");
    assert_eq!(updated["content_type"], "text");
    assert_eq!(updated["content_types"][0], "code");

    let deleted = success_json(&database, &["classifier", "delete", stable_ref, "--json"]);
    assert_eq!(deleted["deleted"], true);
    clean_database(&database);
}
