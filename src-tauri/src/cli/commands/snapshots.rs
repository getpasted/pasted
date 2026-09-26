use super::json_error;
use pasted_lib::db::DbState;
use pasted_lib::library_storage::{snapshots, LibrarySession};
use rusqlite::{Connection, Error, Result};
use std::path::PathBuf;

pub(crate) fn run(
    args: &[String],
    db_path: PathBuf,
    conn: Connection,
    session: &LibrarySession,
) -> Result<()> {
    let policy = DbState::open_existing(db_path.clone())?;
    super::require_feature(&policy, pasted_lib::features::Feature::Snapshots);
    drop(policy);
    let json = args.iter().any(|argument| argument == "--json");
    match args.get(2).map(String::as_str).unwrap_or("list") {
        "list" => {
            let snapshots =
                snapshots::list(&session.app_data).map_err(Error::InvalidParameterName)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&snapshots).map_err(json_error)?
                );
            } else {
                for snapshot in snapshots {
                    println!(
                        "{}  {}  {} clips  {} bytes",
                        snapshot.id, snapshot.created_at, snapshot.clip_count, snapshot.size_bytes
                    );
                }
            }
        }
        "check" => {
            drop(conn);
            let db = DbState::open_existing(db_path)?;
            let window = std::fs::read_to_string(
                super::super::get_app_config_dir().join(".window-state.json"),
            )
            .ok();
            let snapshot = snapshots::check(
                &db,
                &session.app_data,
                chrono::Utc::now(),
                window.as_deref(),
            )
            .map_err(Error::InvalidParameterName)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&snapshot).map_err(json_error)?
                );
            } else if let Some(snapshot) = snapshot {
                println!("Saved snapshot {}.", snapshot.id);
            } else {
                println!("No snapshot was due.");
            }
        }
        "create" => {
            drop(conn);
            let db = DbState::open_existing(db_path)?;
            let window = std::fs::read_to_string(
                super::super::get_app_config_dir().join(".window-state.json"),
            )
            .ok();
            let snapshot = snapshots::create(
                &db,
                &session.app_data,
                chrono::Utc::now(),
                window.as_deref(),
            )
            .map_err(Error::InvalidParameterName)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&snapshot).map_err(json_error)?
                );
            } else {
                println!("Saved snapshot {}.", snapshot.id);
            }
        }
        "export" => {
            let id = args.get(3).filter(|value| !value.starts_with("--")).ok_or_else(|| {
                Error::InvalidParameterName("A snapshot identifier is required".into())
            })?;
            let destination = args.get(4).filter(|value| !value.starts_with("--")).ok_or_else(|| {
                Error::InvalidParameterName("A Full Backup destination is required".into())
            })?;
            let db = DbState::open_existing(db_path)?;
            let report = snapshots::export(
                &db,
                &session.app_data,
                id,
                &PathBuf::from(destination),
            )
            .map_err(Error::InvalidParameterName)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report).map_err(json_error)?);
            } else {
                println!("Exported snapshot {id} to {}.", report.path);
            }
        }
        "delete" => {
            let id = args
                .get(3)
                .filter(|id| !id.starts_with("--"))
                .ok_or_else(|| {
                    Error::InvalidParameterName("A snapshot identifier is required".into())
                })?;
            if !args.iter().any(|argument| argument == "--yes") {
                return Err(Error::InvalidParameterName(
                    "Deleting a snapshot is permanent. Repeat with --yes to confirm.".into(),
                ));
            }
            let db = DbState::open_existing(db_path)?;
            snapshots::delete(&db, &session.app_data, id).map_err(Error::InvalidParameterName)?;
            if json {
                println!("{}", serde_json::json!({ "id": id, "deleted": true }));
            } else {
                println!("Deleted snapshot {id}.");
            }
        }
        "restore" => {
            let id = args
                .get(3)
                .filter(|id| !id.starts_with("--"))
                .ok_or_else(|| {
                    Error::InvalidParameterName("A snapshot identifier is required".into())
                })?;
            let (_lease, source) = snapshots::restore_source(&session.app_data, id)
                .map_err(Error::InvalidParameterName)?;
            let mut restore = vec![
                args[0].clone(),
                "backup".into(),
                "restore".into(),
                source.to_string_lossy().into_owned(),
            ];
            restore.extend(args.iter().skip(4).cloned());
            super::portability::run_backup_with_feature(
                &restore,
                db_path,
                conn,
                session,
                pasted_lib::features::Feature::Snapshots,
            )?;
        }
        _ => {
            return Err(Error::InvalidParameterName(
                "Usage: pasted snapshots list|check|create|export|restore|delete [arguments] [--yes] [--json]".into(),
            ))
        }
    }
    Ok(())
}
