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
