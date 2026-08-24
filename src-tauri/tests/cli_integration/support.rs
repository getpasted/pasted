pub(super) use rusqlite::OptionalExtension;
pub(super) use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn temporary_path(label: &str, extension: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "pasted-cli-{label}-{}-{stamp}.{extension}",
        std::process::id()
    ))
}

pub(super) fn run(database: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pasted"))
        .env("PASTED_DATABASE_PATH", database)
        .env("PASTED_CONFIG_DIR", database.with_extension("config"))
        .args(arguments)
        .output()
        .expect("run pasted CLI")
}

pub(super) fn run_with_stdin(database: &Path, arguments: &[&str], input: &str) -> Output {
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

pub(super) fn success_json(database: &Path, arguments: &[&str]) -> Value {
    let output = run(database, arguments);
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("valid JSON output")
}

pub(super) fn success_json_with_stdin(database: &Path, arguments: &[&str], input: &str) -> Value {
    let output = run_with_stdin(database, arguments, input);
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("valid JSON output")
}

pub(super) fn analysis_fixture(name: &str) -> Value {
    let contents = match name {
        "analyzer-interactive-text" => {
            include_str!("../../../contracts/analysis/v1/analyzer-interactive-text.json")
        }
        "analyzer-capture-text" => {
            include_str!("../../../contracts/analysis/v1/analyzer-capture-text.json")
        }
        "inspector-interactive-text" => {
            include_str!("../../../contracts/analysis/v1/inspector-interactive-text.json")
        }
        "suggestion-interactive-empty" => {
            include_str!("../../../contracts/analysis/v1/suggestion-interactive-empty.json")
        }
        "extractor-interactive-unavailable" => {
            include_str!("../../../contracts/analysis/v1/extractor-interactive-unavailable.json")
        }
        "classifier-interactive-no-match" => {
            include_str!("../../../contracts/analysis/v1/classifier-interactive-no-match.json")
        }
        _ => panic!("unknown Analysis fixture {name}"),
    };
    serde_json::from_str(contents).expect("valid Analysis contract fixture")
}

pub(super) fn clean_database(path: &Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
}
