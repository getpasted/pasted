use super::{files, location, startup, LibrarySession, RecoveryCopy};
use crate::db::DbState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecoveryNotice {
    pub outcome: String,
    pub occurred_at: String,
    pub previous_path: Option<PathBuf>,
    pub recovery_created_at: Option<String>,
    pub preserved_path: PathBuf,
}

pub fn recovery_notice(app_data: &Path) -> Option<LibraryRecoveryNotice> {
    location::read_location(app_data)
        .ok()
        .flatten()
        .and_then(|record| record.notice)
}

pub fn open_library_automatically(app_data: &Path) -> Result<(LibrarySession, DbState), String> {
    let session = LibrarySession::open_unsettled(app_data)?;
    let pending = session.app_data.join("library-move.json");
    let activation = session.app_data.join("library-startup.json");
    // Healthy simultaneous app and CLI sessions remain shared. Recovery alone
    // needs exclusivity: another open app is never treated as damaged storage.
    if !pending.exists() && !activation.exists() {
        if let Ok(db) = startup::open_session_database(&session) {
            return Ok((session, db));
        }
    }
    let db = session.exclusive_unsettled(|| {
        if activation.exists() {
            match super::recovery_activation::resume(&session.app_data) {
                Ok(db) => return Ok(db),
                Err(error) if super::recovery_activation::retryable(&session.app_data) => {
                    return Err(error)
                }
                Err(_) => {
                    let preserved = preservation_directory(&session.app_data)?;
                    if activation.exists() {
                        fs::rename(&activation, preserved.join("library-startup.json"))
                            .map_err(|error| error.to_string())?;
                    }
                    return recover_or_create(&session);
                }
            }
        }
        if pending.exists() && super::transfer::reconcile(&session.app_data).is_err() {
            // An unresolvable journal may reference changed files. Preserve it
            // and all referenced files instead of guessing which ones to delete.
            let preserved = preservation_directory(&session.app_data)?;
            fs::rename(&pending, preserved.join("library-move.json"))
                .map_err(|error| error.to_string())?;
            if let Ok(db) = startup::open_session_database(&session) {
                let notice = notice("continued", Some(db.database_path()), None, preserved);
                let record = location::read_location(&session.app_data)?;
                location::write_location_with_notice(
                    &session.app_data,
                    &db.database_path(),
                    record.and_then(|record| record.recovery),
                    Some(notice),
                )?;
                return Ok(db);
            }
        } else if let Ok(db) = startup::open_session_database(&session) {
            return Ok(db);
        }
        recover_or_create(&session)
    })?;
    Ok((session, db))
}

fn preservation_directory(app_data: &Path) -> Result<PathBuf, String> {
    let directory = app_data
        .join("library-recovery")
        .join(format!("startup-{}", files::nonce()));
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    Ok(directory)
}

fn notice(
    outcome: &str,
    previous_path: Option<PathBuf>,
    recovery: Option<&RecoveryCopy>,
    preserved_path: PathBuf,
) -> LibraryRecoveryNotice {
    LibraryRecoveryNotice {
        outcome: outcome.into(),
        occurred_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        previous_path,
        recovery_created_at: recovery.map(|copy| copy.created_at.clone()),
        preserved_path,
    }
}

fn recovered_database(path: &Path, recovery: &RecoveryCopy) -> Result<DbState, String> {
    if !startup::verified_recovery(recovery) {
        return Err("Recovery copy is unavailable".into());
    }
    fs::copy(&recovery.path, path).map_err(|error| error.to_string())?;
    let db = DbState::open_existing(path.into()).map_err(|error| error.to_string())?;
    let integrity: String = db
        .conn
        .lock()
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if integrity != "ok" {
        return Err("Recovery copy failed validation".into());
    }
    Ok(db)
}

fn recover_or_create(session: &LibrarySession) -> Result<DbState, String> {
    let app_data = &session.app_data;
    let record = location::read_location(app_data).ok().flatten();
    let previous_path = record
        .as_ref()
        .map(|record| record.directory.join("pasted.db"));
    let preserved = preservation_directory(app_data)?;
    let saved_record = app_data.join("library-location.json");
    if saved_record.is_file() {
        fs::copy(&saved_record, preserved.join("library-location.json"))
            .map_err(|error| error.to_string())?;
    }
    let staged = preserved.join("prepared.db");
    let mut candidates = super::snapshots::recovery_candidates(app_data);
    if let Some(copy) = record.and_then(|record| record.recovery) {
        candidates.push(copy);
    }
    let mut recovery = None;
    let mut restored = None;
    for copy in candidates {
        match recovered_database(&staged, &copy) {
            Ok(db) => {
                recovery = Some(copy);
                restored = Some(db);
                break;
            }
            Err(_) => super::remove_database_files(&staged),
        }
    }
    let (prepared, outcome, recovery) = match restored {
        Some(db) => (db, "recovered", recovery),
        None => {
            super::remove_database_files(&staged);
            let db = DbState::new(staged.clone()).map_err(|error| error.to_string())?;
            // An automatic repair is not first-run onboarding.
            db.save_setting(
                crate::external_import::ONBOARDING_SETTING_KEY,
                &crate::external_import::ONBOARDING_VERSION.to_string(),
            )
            .map_err(|error| error.to_string())?;
            (db, "fresh", None)
        }
    };
    {
        let conn = prepared.conn.lock();
        let busy: i64 = conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        if busy != 0 {
            return Err("The prepared library is busy".into());
        }
    }
    drop(prepared);
    let notice = notice(outcome, previous_path, recovery.as_ref(), preserved);
    super::recovery_activation::activate(app_data, &staged, recovery, notice)
}
