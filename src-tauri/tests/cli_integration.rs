use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
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

fn success_json(database: &Path, arguments: &[&str]) -> Value {
    let output = run(database, arguments);
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
        "enricher-interactive-empty" => {
            include_str!("../../contracts/analysis/v1/enricher-interactive-empty.json")
        }
        "extractor-interactive-unavailable" => {
            include_str!("../../contracts/analysis/v1/extractor-interactive-unavailable.json")
        }
        "detector-interactive-no-match" => {
            include_str!("../../contracts/analysis/v1/detector-interactive-no-match.json")
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
fn history_and_settings_commands_have_executable_json_contracts() {
    let database = temporary_path("history", "db");
    let saved = success_json(&database, &["copy", "person@example.com", "--json"]);
    assert_eq!(saved["contentType"], "email");

    let listed = success_json(&database, &["list", "--limit", "5", "--json"]);
    assert_eq!(listed.as_array().map(Vec::len), Some(1));
    assert_eq!(listed[0]["content_type"], "email");

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

    let refused = run(&database, &["clear"]);
    assert_eq!(refused.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&refused.stderr).contains("--yes"));
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

    let registry = success_json(
        &database,
        &["registry", "list", "--kind", "inspector", "--json"],
    );
    assert_eq!(registry[0]["analysisPass"], "inspect");
    assert_eq!(registry[0]["capabilities"]["canDisable"], false);

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
fn smart_actions_enricher_has_registry_and_non_mutating_cli_parity() {
    let database = temporary_path("enricher", "db");
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

    let enrichers = success_json(&database, &["enricher", "list", "--json"]);
    assert_eq!(enrichers[0]["stableRef"], "enricher:smart-actions-v1");
    assert_eq!(enrichers[0]["outputContract"], "recommendations");

    let registry = success_json(
        &database,
        &["registry", "list", "--kind", "enricher", "--json"],
    );
    assert_eq!(registry[0]["analysisPass"], "enrich");
    assert_eq!(
        registry[0]["inputContract"],
        "analyzable_text+classification+structural_metadata"
    );
    assert_eq!(registry[0]["capabilities"]["canDisable"], false);

    let result = success_json(
        &database,
        &["enricher", "run", "--clip", &clip_id, "--json"],
    );
    assert_eq!(result["formatVersion"], 1);
    assert_eq!(result["policy"], "interactive");
    assert_eq!(result["through"], "enrich");
    assert_eq!(result["result"]["signals"][0], "url");
    assert_eq!(
        result["result"]["actions"][0]["transformRef"],
        transform_ref
    );
    assert_eq!(result["appliedClipId"], Value::Null);
    assert!(!result.to_string().contains("private-token-0123456789"));

    let empty = success_json(
        &database,
        &["enricher", "run", "--text", "ordinary words", "--json"],
    );
    assert_eq!(empty, analysis_fixture("enricher-interactive-empty"));
    clean_database(&database);
}

#[test]
fn whole_analyzer_has_one_versioned_privacy_safe_cli_contract() {
    let database = temporary_path("analyzer", "db");
    let secret = "agent@example.com private-token-0123456789";
    let interactive = success_json(&database, &["analyzer", "run", "--text", secret, "--json"]);
    assert_eq!(interactive["formatVersion"], 1);
    assert_eq!(interactive["policy"], "interactive");
    assert_eq!(interactive["through"], "enrich");
    assert_eq!(interactive["result"]["clipKind"], "text");
    assert_eq!(interactive["result"]["detectedType"], "text");
    assert!(interactive["result"]["structure"].is_object());
    assert!(interactive["result"]["recommendations"].is_object());
    assert_eq!(interactive["participants"][0]["pass"], "inspect");
    assert_eq!(interactive["participants"][1]["pass"], "classify");
    assert_eq!(interactive["participants"][2]["pass"], "enrich");
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
    assert!(capture["result"].get("recommendations").is_none());
    assert_eq!(capture["participants"].as_array().map(Vec::len), Some(2));
    assert!(!capture.to_string().contains("ordinary words"));
    assert_eq!(capture, analysis_fixture("analyzer-capture-text"));
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
    let created = success_json(
        &database,
        &[
            "extractor",
            "create",
            "--name",
            "CLI Extractor",
            "--engine",
            "test-unavailable-v1",
            "--json",
        ],
    );
    let stable_ref = created["stableRef"].as_str().expect("Extractor stable ref");
    assert_eq!(created["isAvailable"], false);

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
    let mut preview: Value =
        serde_json::from_slice(&preview_output.stdout).expect("Extractor JSON");
    assert_eq!(preview["formatVersion"], 1);
    assert_eq!(preview["policy"], "interactive");
    assert_eq!(preview["through"], "enrich");
    assert_eq!(preview["targetKind"], "extractor");
    assert_eq!(preview["targetRef"], stable_ref);
    assert_eq!(preview["outcome"], "failed");
    assert_eq!(preview["failure"]["code"], "engine_not_installed");
    assert_eq!(preview["appliedClipId"], Value::Null);
    assert_eq!(preview["ocrUpdated"], false);
    assert_eq!(preview["classificationUpdated"], false);
    assert_eq!(preview["participants"][0]["pass"], "extract");
    assert!(!preview.to_string().contains("private image bytes"));
    preview["targetRef"] = Value::String("extractor:test".into());
    preview["participants"][0]["stableRef"] = Value::String("extractor:test".into());
    assert_eq!(
        preview,
        analysis_fixture("extractor-interactive-unavailable")
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

    let deleted = success_json(&database, &["extractor", "delete", stable_ref, "--json"]);
    assert_eq!(deleted["deleted"], true);
    clean_database(&database);
}

#[test]
fn detector_preview_and_apply_share_the_safe_execution_contract() {
    let database = temporary_path("detectors", "db");
    let clip = success_json(&database, &["copy", "ticket-123", "--json"]);
    let clip_id = clip["id"].as_i64().expect("clip ID");
    let clip_id_text = clip_id.to_string();
    let detector = success_json(
        &database,
        &[
            "detector",
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
    let stable_ref = detector["stable_ref"]
        .as_str()
        .expect("Detector stable ref");
    let fetched = success_json(&database, &["detector", "get", stable_ref, "--json"]);
    assert_eq!(fetched["name"], "Ticket IDs");
    let duplicate = success_json(
        &database,
        &[
            "detector",
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
        &["registry", "list", "--kind", "detector", "--json"],
    );
    let registry_item = registry
        .as_array()
        .and_then(|items| items.iter().find(|item| item["stableRef"] == stable_ref))
        .expect("Detector registry item");
    assert_eq!(registry_item["analysisPass"], "classify");
    assert_eq!(registry_item["capabilities"]["canDuplicate"], true);
    assert_eq!(registry_item["capabilities"]["canDelete"], true);

    let mut no_match = success_json(
        &database,
        &[
            "detector",
            "run",
            stable_ref,
            "--text",
            "ordinary words",
            "--json",
        ],
    );
    assert_eq!(no_match["targetRef"], stable_ref);
    no_match["targetRef"] = Value::String("detector:email".into());
    assert_eq!(no_match, analysis_fixture("detector-interactive-no-match"));

    let preview = success_json(
        &database,
        &[
            "detector",
            "run",
            stable_ref,
            "--text",
            "ticket-123",
            "--json",
        ],
    );
    assert_eq!(preview["formatVersion"], 1);
    assert_eq!(preview["policy"], "interactive");
    assert_eq!(preview["through"], "enrich");
    assert_eq!(preview["targetKind"], "detector");
    assert_eq!(preview["targetRef"], stable_ref);
    assert_eq!(preview["outcome"], "matched");
    assert_eq!(preview["matched"], true);
    assert_eq!(preview["detectedType"], "code");
    assert_eq!(preview["appliedClipId"], Value::Null);
    assert_eq!(preview["participants"][0]["pass"], "classify");
    assert!(!preview.to_string().contains("ticket-123"));

    let applied = success_json(
        &database,
        &[
            "detector",
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
    assert_eq!(updated["content_type"], "code");

    let deleted = success_json(&database, &["detector", "delete", stable_ref, "--json"]);
    assert_eq!(deleted["deleted"], true);
    clean_database(&database);
}
