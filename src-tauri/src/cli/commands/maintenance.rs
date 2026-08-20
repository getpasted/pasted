use super::super::*;

pub(crate) fn run_clear(args: Vec<String>, db_path: PathBuf, conn: Connection) -> Result<()> {
    if !args.iter().any(|argument| argument == "--yes") {
        eprintln!("Clearing History is permanent. Re-run with --yes to continue.");
        std::process::exit(2);
    }
    drop(conn);
    let db = DbState::new(db_path.clone())?;
    db.purge_unpinned_clips()?;
    if args.iter().any(|argument| argument == "--json") {
        println!("{}", serde_json::json!({ "cleared": true }));
    } else {
        println!("Cleared unpinned, unprotected History clips.");
    }
    Ok(())
}

pub(crate) fn run_reset(args: Vec<String>, db_path: PathBuf, conn: Connection) -> Result<()> {
    if !args.iter().any(|argument| argument == "--yes") {
        eprintln!(
        "Refusing to reset without --yes. Quit Pasted first, and export a backup if you may need this data."
    );
        std::process::exit(2);
    }
    drop(conn);
    let db = DbState::new(db_path.clone())?;
    let report = db.factory_reset()?;
    if let Some(cache_directory) = dirs::cache_dir() {
        let app_cache = cache_directory.join(APP_IDENTIFIER);
        if app_cache.exists() {
            let _ = fs::remove_dir_all(app_cache);
        }
    }
    if args.iter().any(|argument| argument == "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .map_err(|error| { rusqlite::Error::ToSqlConversionFailure(Box::new(error)) })?
        );
    } else {
        println!(
        "Reset Pasted: removed {} clips, {} bins, {} Transforms, {} connections, and {} activity entries.",
        report.clips_deleted,
        report.bins_deleted,
        report.transforms_deleted,
        report.connections_deleted,
        report.activity_entries_deleted
    );
    }
    Ok(())
}
