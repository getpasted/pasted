use super::{directory, files, lease, list, path, remove_unpublished_locked, RETENTION_KEY};
use crate::db::DbState;
use std::fs;
use std::path::Path;

pub fn delete(db: &DbState, app_data: &Path, id: &str) -> Result<(), String> {
    let _lease = lease(app_data)?;
    crate::features::require(db, crate::features::Feature::Snapshots)?;
    let source = path(app_data, id)?;
    if !list(app_data)?.iter().any(|snapshot| snapshot.id == id) {
        return Err("Snapshot is no longer available".into());
    }
    fs::remove_file(&source).map_err(|error| error.to_string())?;
    super::super::remove_database_files(&source);
    let _ = fs::remove_file(directory(app_data).join(format!("{id}.json")));
    files::sync_directory(&directory(app_data))
}

pub(super) fn retention_count(db: &DbState) -> Result<usize, String> {
    Ok(db
        .get_setting(RETENTION_KEY)
        .map_err(|error| error.to_string())?
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value <= 10000)
        .unwrap_or(24))
}

pub(super) fn prune_locked(app_data: &Path, keep: usize) -> Result<(), String> {
    if keep == 0 {
        return Ok(());
    }
    for old in list(app_data)?.into_iter().skip(keep) {
        let source = path(app_data, &old.id)?;
        if fs::remove_file(&source).is_ok() {
            super::super::remove_database_files(&source);
            let _ = fs::remove_file(directory(app_data).join(format!("{}.json", old.id)));
        }
    }
    files::sync_directory(&directory(app_data))
}

pub fn enforce_retention(db: &DbState, app_data: &Path, keep: usize) -> Result<(), String> {
    crate::features::require(db, crate::features::Feature::Snapshots)?;
    if keep > 10000 {
        return Err("Snapshot retention cannot exceed 10000".into());
    }
    let _lease = lease(app_data)?;
    remove_unpublished_locked(app_data)?;
    prune_locked(app_data, keep)
}
