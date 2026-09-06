use super::super::support::temporary_path;
use pasted_lib::library_storage::open_library;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn run(app_data: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pasted"))
        .env_remove("PASTED_DATABASE_PATH")
        .env("PASTED_DATA_DIR", app_data)
        .env("PASTED_CONFIG_DIR", app_data.join("test-config"))
        .args(args)
        .output()
        .unwrap()
}
fn json(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn cli_and_gui_service_share_move_exclusion_and_recovery_contract() {
    let root = temporary_path("library-moves", "fixture");
    let app_data = root.join("app");
    let destination = root.join("destination");
    fs::create_dir_all(&destination).unwrap();
    let (session, db) = open_library(&app_data).unwrap();
    // Use the canonical feature key instead of assuming the current registry spelling.
    db.save_setting(pasted_lib::features::Feature::Cli.setting_key(), "true")
        .unwrap();
    db.save_setting("move_fixture", "before").unwrap();
    let backup = root.join("before.pastedbackup");
    json(run(
        &app_data,
        &["backup", "create", backup.to_str().unwrap(), "--json"],
    ));
    db.save_setting("restore_guard", "preserved").unwrap();
    assert!(!run(
        &app_data,
        &["backup", "restore", backup.to_str().unwrap(), "--yes"]
    )
    .status
    .success());
    assert_eq!(
        db.get_setting("restore_guard").unwrap().as_deref(),
        Some("preserved")
    );
    let args = ["database", "move", destination.to_str().unwrap(), "--json"];
    let blocked = run(&app_data, &args);
    assert!(!blocked.status.success());
    assert!(!destination.join("pasted.db").exists());
    drop(db);
    drop(session);
    let moved = json(run(&app_data, &args));
    assert_eq!(moved["location"]["isDefault"], false);
    assert!(Path::new(moved["recoveryPath"].as_str().unwrap()).is_file());
    let (session, db) = open_library(&app_data).unwrap();
    assert_eq!(
        db.database_path(),
        fs::canonicalize(&destination).unwrap().join("pasted.db")
    );
    db.save_setting("move_fixture", "after").unwrap();
    drop(db);
    drop(session);
    fs::rename(&destination, root.join("disconnected")).unwrap();
    let status = json(run(&app_data, &["database", "status", "--json"]));
    assert_eq!(status["ready"], true);
    assert_eq!(status["notice"]["outcome"], "recovered");
    assert!(status["notice"]["recoveryCreatedAt"].is_string());
    assert!(run(&app_data, &["list", "--json"]).status.success());
    let (session, db) = open_library(&app_data).unwrap();
    assert_eq!(
        db.get_setting("move_fixture").unwrap().as_deref(),
        Some("before")
    );
    assert!(root.join("disconnected/pasted.db").is_file());
    drop(db);
    drop(session);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn snapshot_settings_listing_and_restore_match_the_shared_service() {
    let root = temporary_path("snapshot-cli", "fixture");
    let (session, db) = open_library(&root).unwrap();
    db.save_setting(pasted_lib::features::Feature::Cli.setting_key(), "true")
        .unwrap();
    db.save_clip("text", Some("first"), None, None, "first", "Fixture")
        .unwrap();
    assert!(
        run(&root, &["settings", "set", "snapshotIntervalMinutes", "15"])
            .status
            .success()
    );
    assert!(
        !run(&root, &["settings", "set", "snapshotIntervalMinutes", "-1"])
            .status
            .success()
    );
    assert_eq!(
        db.get_setting("snapshotIntervalMinutes")
            .unwrap()
            .as_deref(),
        Some("15")
    );
    let saved = json(run(&root, &["snapshots", "check", "--json"]));
    assert_eq!(saved["clipCount"], 1);
    let listed = json(run(&root, &["snapshots", "list", "--json"]));
    assert_eq!(listed[0], saved);
    assert!(json(run(&root, &["snapshots", "check", "--json"])).is_null());
    let id = saved["id"].as_str().unwrap();
    assert!(run(&root, &["settings", "set", "enableSnapshots", "false"])
        .status
        .success());
    for args in [
        vec!["snapshots", "list", "--json"],
        vec!["snapshots", "check", "--json"],
        vec!["snapshots", "restore", id, "--yes"],
    ] {
        assert!(!run(&root, &args).status.success());
    }
    assert_eq!(
        pasted_lib::library_storage::snapshots::list(&root)
            .unwrap()
            .len(),
        1
    );
    assert!(run(&root, &["settings", "set", "enableSnapshots", "true"])
        .status
        .success());
    db.save_clip("text", Some("later"), None, None, "later", "Fixture")
        .unwrap();
    assert!(
        !run(&root, &["snapshots", "restore", id, "--yes", "--json"])
            .status
            .success()
    );
    drop(db);
    drop(session);
    assert!(!run(&root, &["snapshots", "restore", id]).status.success());
    let restored = json(run(&root, &["snapshots", "restore", id, "--yes", "--json"]));
    assert!(Path::new(restored["recoveryPath"].as_str().unwrap()).is_file());
    let (session, db) = open_library(&root).unwrap();
    assert_eq!(db.get_clips(None, false).unwrap().len(), 1);
    drop(db);
    drop(session);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn storage_gates_block_mutations_and_keep_snapshots_independent_of_backups() {
    let root = temporary_path("storage-gates", "fixture");
    let (session, db) = open_library(&root).unwrap();
    db.save_setting(pasted_lib::features::Feature::Cli.setting_key(), "true")
        .unwrap();
    db.save_clip(
        "text",
        Some("preserved"),
        None,
        None,
        "preserved",
        "Fixture",
    )
    .unwrap();
    for key in [
        "enableLibraryMove",
        "enableBackups",
        "enableFactoryReset",
        "enableSnapshots",
    ] {
        db.save_setting(key, "false").unwrap();
    }
    let destination = root.join("destination");
    fs::create_dir_all(&destination).unwrap();
    assert!(db.move_library(&session, &destination, false).is_err());
    assert!(db.factory_reset().is_err());
    drop(db);
    drop(session);
    let backup = root.join("blocked.pastedbackup");
    let transfer = root.join("blocked.json");
    for args in [
        vec!["database", "move", destination.to_str().unwrap(), "--json"],
        vec!["backup", "create", backup.to_str().unwrap(), "--json"],
        vec!["transfer", "export", transfer.to_str().unwrap(), "--json"],
        vec!["import", "sources", "--json"],
        vec!["reset", "--yes", "--json"],
        vec!["snapshots", "check", "--json"],
    ] {
        let output = run(&root, &args);
        assert!(!output.status.success(), "{args:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("disabled"));
    }
    assert!(!backup.exists());
    assert!(!transfer.exists());
    assert!(!destination.join("pasted.db").exists());
    assert!(run(&root, &["settings", "set", "enableSnapshots", "true"])
        .status
        .success());
    let snapshot = json(run(&root, &["snapshots", "check", "--json"]));
    let id = snapshot["id"].as_str().unwrap();
    let blocked_export = root.join("blocked-snapshot.pastedbackup");
    let output = run(
        &root,
        &[
            "snapshots",
            "export",
            id,
            blocked_export.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("disabled"));
    assert!(!blocked_export.exists());
    json(run(&root, &["snapshots", "restore", id, "--yes", "--json"]));
    let (session, db) = open_library(&root).unwrap();
    assert_eq!(db.get_clips(None, false).unwrap().len(), 1);
    assert_eq!(
        db.get_setting("enableBackups").unwrap().as_deref(),
        Some("false")
    );
    drop(db);
    drop(session);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn database_status_honors_cli_and_lock_gates_including_standalone_paths() {
    let root = temporary_path("storage-status-policy", "fixture");
    let (session, db) = open_library(&root).unwrap();
    db.save_setting("enableCli", "false").unwrap();
    for name in ["database", "library"] {
        let denied = run(&root, &[name, "status", "--json"]);
        assert!(!denied.status.success());
        assert!(String::from_utf8_lossy(&denied.stderr).contains("CLI is disabled"));
        assert!(denied.stdout.is_empty());
    }
    db.save_setting("enableCli", "true").unwrap();
    pasted_lib::app_lock::configure(&db, "fixture-passphrase").unwrap();
    let denied = run(&root, &["database", "status", "--json"]);
    assert!(!denied.status.success());
    assert!(String::from_utf8_lossy(&denied.stderr).contains("locked"));
    let allowed = Command::new(env!("CARGO_BIN_EXE_pasted"))
        .env("PASTED_DATABASE_PATH", db.database_path())
        .env("PASTED_CONFIG_DIR", root.join("test-config"))
        .env("PASTED_APP_LOCK_PASSPHRASE", "fixture-passphrase")
        .args(["database", "status", "--json"])
        .output()
        .unwrap();
    assert_eq!(json(allowed)["ready"], true);
    drop(db);
    drop(session);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn zero_count_and_snapshot_deletion_use_the_shared_cli_contract() {
    let root = temporary_path("snapshot-delete-cli", "fixture");
    let (session, db) = open_library(&root).unwrap();
    db.save_setting("enableCli", "true").unwrap();
    db.save_clip(
        "text",
        Some("preserved"),
        None,
        None,
        "preserved",
        "Fixture",
    )
    .unwrap();
    let snapshot = json(run(&root, &["snapshots", "check", "--json"]));
    let id = snapshot["id"].as_str().unwrap();
    assert!(run(&root, &["settings", "set", "snapshotKeepCount", "0"])
        .status
        .success());
    db.save_setting("snapshotIntervalMinutes", "1").unwrap();
    assert!(json(run(&root, &["snapshots", "check", "--json"])).is_null());
    let created = json(run(&root, &["snapshots", "create", "--json"]));
    assert_eq!(created["clipCount"], 1);
    let exported = root.join("Snapshot.pastedbackup");
    let export = json(run(
        &root,
        &[
            "snapshots",
            "export",
            created["id"].as_str().unwrap(),
            exported.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(export["path"], exported.to_string_lossy().as_ref());
    assert!(exported.is_file());
    assert!(run(
        &root,
        &[
            "snapshots",
            "delete",
            created["id"].as_str().unwrap(),
            "--yes",
        ],
    )
    .status
    .success());
    assert!(!run(&root, &["snapshots", "delete", id]).status.success());
    assert_eq!(
        json(run(&root, &["snapshots", "list", "--json"]))[0],
        snapshot
    );
    db.save_setting("enableSnapshots", "false").unwrap();
    assert!(!run(&root, &["snapshots", "delete", id, "--yes"])
        .status
        .success());
    db.save_setting("enableSnapshots", "true").unwrap();
    assert_eq!(
        json(run(&root, &["snapshots", "delete", id, "--yes", "--json"])),
        serde_json::json!({ "id": id, "deleted": true })
    );
    assert!(json(run(&root, &["snapshots", "list", "--json"]))
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(db.get_clips(None, false).unwrap().len(), 1);
    drop(db);
    drop(session);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn factory_reset_removes_automatic_snapshots_and_preserves_manual_backups() {
    let root = temporary_path("snapshot-reset-cli", "fixture");
    let (session, db) = open_library(&root).unwrap();
    db.save_setting("enableCli", "true").unwrap();
    db.save_clip("text", Some("clip"), None, None, "clip", "Fixture")
        .unwrap();
    let snapshot =
        pasted_lib::library_storage::snapshots::create(&db, &root, chrono::Utc::now(), None)
            .unwrap();
    let manual = root.join("Manual.pastedbackup");
    db.create_full_backup(&manual, None, None).unwrap();
    drop(db);
    drop(session);

    let report = json(run(&root, &["reset", "--yes", "--json"]));
    assert_eq!(report["snapshotsDeleted"], 1);
    assert!(manual.is_file());
    assert!(pasted_lib::library_storage::snapshots::list(&root)
        .unwrap()
        .is_empty());
    assert!(!root
        .join("snapshots")
        .join(format!("{}.pastedbackup", snapshot.id))
        .exists());
    let (session, db) = open_library(&root).unwrap();
    assert!(db.get_clips(None, false).unwrap().is_empty());
    drop(db);
    drop(session);
    fs::remove_dir_all(root).unwrap();
}
