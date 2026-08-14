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
    let preview: Value = serde_json::from_slice(&preview_output.stdout).expect("Extractor JSON");
    assert_eq!(preview["targetKind"], "extractor");
    assert_eq!(preview["targetRef"], stable_ref);
    assert_eq!(preview["outcome"], "failed");
    assert_eq!(preview["failure"]["code"], "engine_not_installed");
    assert_eq!(preview["appliedClipId"], Value::Null);
    assert_eq!(preview["ocrUpdated"], false);
    assert_eq!(preview["classificationUpdated"], false);
    assert_eq!(preview["participants"][0]["pass"], "extract");
    assert!(!preview.to_string().contains("private image bytes"));
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
