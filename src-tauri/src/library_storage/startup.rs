use super::{files, location, LibrarySession, RecoveryCopy};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStartupStatus {
    pub ready: bool,
    pub path: Option<PathBuf>,
    pub recovery_created_at: Option<String>,
}

impl LibraryStartupStatus {
    pub fn ready() -> Self {
        Self {
            ready: true,
            path: None,
            recovery_created_at: None,
        }
    }

    pub fn unavailable(app_data: &Path) -> Self {
        let record = location::read_location(app_data).ok().flatten();
        let path = record
            .as_ref()
            .map(|record| record.directory.join("pasted.db"));
        let recovery_created_at = record
            .filter(|record| !record.directory.join("pasted.db").is_file())
            .and_then(|record| record.recovery)
            .filter(verified_recovery)
            .map(|copy| copy.created_at);
        Self {
            ready: false,
            path,
            recovery_created_at,
        }
    }
}

pub fn verified_recovery(copy: &RecoveryCopy) -> bool {
    let wal = PathBuf::from(format!("{}-wal", copy.path.display()));
    !wal.exists() && files::digest(&copy.path).is_ok_and(|digest| digest == copy.sha256)
}

pub fn open_library(app_data: &Path) -> Result<(LibrarySession, crate::db::DbState), String> {
    let session = LibrarySession::open(app_data)?;
    let db = open_session_database(&session)?;
    Ok((session, db))
}

pub(super) fn open_session_database(
    session: &LibrarySession,
) -> Result<crate::db::DbState, String> {
    let path = session.database_path()?;
    let db = if path.exists() || location::read_location(&session.app_data)?.is_some() {
        crate::db::DbState::open_existing(path)
    } else {
        crate::db::DbState::new(path)
    }
    .map_err(|error| error.to_string())?;
    if location::read_location(&session.app_data)?.is_none() {
        super::persist_location(&session.app_data, &db.database_path())?;
    }
    Ok(db)
}
