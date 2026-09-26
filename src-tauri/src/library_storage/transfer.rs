use super::{files, location, LibraryLocationInfo, RecoveryCopy};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryMoveReport {
    pub location: LibraryLocationInfo,
    pub recovery_path: String,
}

#[derive(Serialize, Deserialize)]
struct PendingMove {
    source: PathBuf,
    target: PathBuf,
    staging: PathBuf,
    sha256: String,
    recovery_path: PathBuf,
}

fn remove_owned(path: &Path, digest: &str) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    if files::digest(path)? != digest {
        return Err("An interrupted move contains changed files; they have been preserved".into());
    }
    super::remove_database_files(path);
    if path.exists() {
        return Err("The interrupted move could not be settled".into());
    }
    Ok(())
}

pub(super) fn reconcile(app_data: &Path) -> Result<(), String> {
    let journal_path = app_data.join("library-move.json");
    let pending: PendingMove = files::read_json(&journal_path)?;
    if !pending.source.is_absolute()
        || !pending.target.is_absolute()
        || pending.source.file_name() != Some(std::ffi::OsStr::new("pasted.db"))
        || pending.target.file_name() != Some(std::ffi::OsStr::new("pasted.db"))
        || pending.staging.parent() != pending.target.parent()
        || !pending
            .staging
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(".pasted-library-"))
        || pending.sha256.len() != 64
    {
        return Err("The interrupted move record is invalid".into());
    }
    let current = location::recorded_path(app_data)?;
    let committed = location::read_location(app_data)?
        .and_then(|record| record.recovery)
        .is_some_and(|copy| copy.path == pending.recovery_path);
    if committed && current == pending.target {
        // The pointer and recovery reference together identify this commit.
    } else if !committed && current == pending.source {
        remove_owned(&pending.target, &pending.sha256)?;
    } else {
        return Err("The saved library location does not match the interrupted move".into());
    }
    // Once committed, target contents may include new captures. Never roll that
    // destination back or compare its contents with the earlier snapshot.
    remove_owned(&pending.staging, &pending.sha256)?;
    fs::remove_file(journal_path).map_err(|error| error.to_string())?;
    files::sync_directory(app_data)
}

pub(crate) fn transfer(
    source: &Connection,
    app_data: &Path,
    source_path: &Path,
    target: &Path,
    recovery_date: Option<&str>,
    mut before_commit: impl FnMut() -> Result<(), String>,
) -> Result<(Connection, LibraryMoveReport), String> {
    let parent = target
        .parent()
        .ok_or("The destination has no parent folder")?;
    let nonce = files::nonce();
    let staging = parent.join(format!(".pasted-library-{nonce}.tmp"));
    drop(files::private_file(&staging)?);
    let result = (|| {
        let mut destination =
            crate::db::open_pasted_database(&staging).map_err(|error| error.to_string())?;
        {
            let backup = rusqlite::backup::Backup::new(source, &mut destination)
                .map_err(|error| error.to_string())?;
            backup
                .run_to_completion(128, std::time::Duration::from_millis(5), None)
                .map_err(|error| error.to_string())?;
        }
        let integrity: String = destination
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        if integrity != "ok" {
            return Err("The moved library failed its integrity check".into());
        }
        destination
            .close()
            .map_err(|(_, error)| error.to_string())?;
        fs::OpenOptions::new()
            .write(true)
            .open(&staging)
            .and_then(|file| file.sync_all())
            .map_err(|error| error.to_string())?;
        let digest = files::digest(&staging)?;
        let recovery_directory = app_data.join("library-recovery");
        fs::create_dir_all(&recovery_directory).map_err(|error| error.to_string())?;
        let recovery_path = recovery_directory.join(format!("move-{nonce}.db"));
        let mut recovery_file = files::private_file(&recovery_path)?;
        std::io::copy(
            &mut File::open(&staging).map_err(|error| error.to_string())?,
            &mut recovery_file,
        )
        .map_err(|error| error.to_string())?;
        recovery_file
            .sync_all()
            .map_err(|error| error.to_string())?;
        files::sync_directory(&recovery_directory)?;
        let recovery = RecoveryCopy {
            path: recovery_path.clone(),
            sha256: digest.clone(),
            created_at: recovery_date.map(str::to_owned).unwrap_or_else(|| {
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
            }),
        };
        let pending = PendingMove {
            source: source_path.into(),
            target: target.into(),
            staging: staging.clone(),
            sha256: digest,
            recovery_path: recovery_path.clone(),
        };
        files::write_json(&app_data.join("library-move.json"), &pending)?;
        super::reject_orphan_sidecars(target)?;
        // Publish without replacing any file that appeared after validation.
        fs::hard_link(&staging, target).map_err(|error| error.to_string())?;
        // Remove the staging link before SQLite opens target: WAL must never have
        // two names for the same database inode.
        fs::remove_file(&staging).map_err(|error| error.to_string())?;
        files::sync_directory(parent)?;
        let replacement =
            crate::db::open_existing_database(target).map_err(|error| error.to_string())?;
        before_commit()?;
        if let Err(error) = location::write_location(app_data, target, Some(recovery)) {
            if !location::read_location(app_data)
                .ok()
                .flatten()
                .and_then(|record| record.recovery)
                .is_some_and(|copy| copy.path == recovery_path)
            {
                return Err(error);
            }
            // A post-rename sync failure cannot make the old path authoritative.
            // Reconcile according to the committed pointer below.
            eprintln!("Library location was committed but final synchronization failed: {error}");
        }
        Ok((
            replacement,
            LibraryMoveReport {
                location: super::location_info(app_data, target),
                recovery_path: recovery_path.to_string_lossy().into_owned(),
            },
        ))
    })();
    if app_data.join("library-move.json").exists() {
        // Failure leaves the original pointer and connection authoritative.
        // If cleanup cannot finish, startup replays the same bounded journal.
        let _ = reconcile(app_data);
    } else if result.is_err() {
        super::remove_database_files(&staging);
    }
    result
}
