use super::{env, get_app_data_dir, library_storage, DbState, PathBuf};
use library_storage::{LibrarySession, LibraryStartupStatus};

pub(super) fn open(args: &[String]) -> Result<(LibrarySession, DbState), String> {
    // Explicit database paths are an existing standalone CLI/testing contract.
    // They never rewrite the installed app's saved location.
    if let Some(path) = env::var_os("PASTED_DATABASE_PATH") {
        if matches!(
            args.get(1).map(String::as_str),
            Some("database" | "library")
        ) && matches!(args.get(2).map(String::as_str), Some("move" | "default"))
        {
            return Err(
                "Library moves require the saved application location; unset PASTED_DATABASE_PATH"
                    .into(),
            );
        }
        let path = PathBuf::from(path);
        let session = LibrarySession::open(&path.with_extension("session"))?;
        let db = DbState::new(path).map_err(|error| error.to_string())?;
        return Ok((session, db));
    }
    let app_data = get_app_data_dir();
    let opened = library_storage::open_library_automatically(&app_data);
    if matches!(
        args.get(1).map(String::as_str),
        Some("database" | "library")
    ) && args.get(2).map(String::as_str) == Some("status")
    {
        if opened.is_err() {
            report_status(
                args,
                &app_data,
                LibraryStartupStatus::unavailable(&app_data),
            );
            std::process::exit(1);
        }
    }
    opened
}

pub(super) fn report_status(
    args: &[String],
    app_data: &std::path::Path,
    status: LibraryStartupStatus,
) {
    let notice = library_storage::recovery_notice(app_data);
    if args.iter().any(|arg| arg == "--json") {
        println!(
            "{}",
            serde_json::json!({ "ready": status.ready, "path": status.path, "notice": notice })
        );
    } else {
        println!(
            "Library {}",
            if status.ready {
                "available"
            } else {
                "unavailable"
            }
        );
        if let Some(notice) = notice {
            println!("Automatic library recovery: {}", notice.outcome);
            if let Some(date) = notice.recovery_created_at {
                println!("Recovery snapshot: {date}");
            }
            println!("Preserved files: {}", notice.preserved_path.display());
        }
    }
}
