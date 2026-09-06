mod creation;
mod export;

use super::{files, RecoveryCopy};
use crate::db::DbState;
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

pub use creation::{check, create, schedule, SnapshotSchedule};
pub use export::export;

pub const INTERVAL_KEY: &str = "snapshotIntervalMinutes";
pub const RETENTION_KEY: &str = "snapshotKeepCount";
const BASELINE_KEY: &str = "snapshotBaselineMaxClipId";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub id: String,
    pub created_at: String,
    pub clip_count: u64,
    pub size_bytes: u64,
    pub sha256: String,
}

struct SnapshotPublishGuard {
    database: PathBuf,
    metadata: PathBuf,
    armed: bool,
}

impl Drop for SnapshotPublishGuard {
    fn drop(&mut self) {
        if self.armed {
            super::remove_database_files(&self.database);
            let _ = fs::remove_file(&self.metadata);
        }
    }
}

pub(super) fn directory(app_data: &Path) -> PathBuf {
    app_data.join("snapshots")
}

fn path(app_data: &Path, id: &str) -> Result<PathBuf, String> {
    if id.is_empty()
        || id.len() > 100
        || !id.bytes().all(|byte| byte.is_ascii_digit() || byte == b'-')
    {
        return Err("Invalid snapshot identifier".into());
    }
    Ok(directory(app_data).join(format!("{id}.pastedbackup")))
}

fn lease(app_data: &Path) -> Result<File, String> {
    fs::create_dir_all(directory(app_data)).map_err(|error| error.to_string())?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(directory(app_data).join(".lock"))
        .map_err(|error| error.to_string())?;
    file.try_lock()
        .map_err(|_| "Snapshots are busy".to_string())?;
    Ok(file)
}

pub fn list(app_data: &Path) -> Result<Vec<Snapshot>, String> {
    let entries = match fs::read_dir(directory(app_data)) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.to_string()),
    };
    let mut snapshots = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry.path().extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Ok(snapshot) = files::read_json::<Snapshot>(&entry.path()) else {
            continue;
        };
        if entry.path().file_stem().and_then(|name| name.to_str()) != Some(&snapshot.id) {
            continue;
        }
        if DateTime::parse_from_rfc3339(&snapshot.created_at).is_err() {
            continue;
        }
        if path(app_data, &snapshot.id).is_ok_and(|path| path.is_file()) {
            snapshots.push(snapshot);
        }
    }
    snapshots.sort_by(|a, b| {
        DateTime::parse_from_rfc3339(&b.created_at)
            .unwrap()
            .cmp(&DateTime::parse_from_rfc3339(&a.created_at).unwrap())
            .then_with(|| b.id.cmp(&a.id))
    });
    Ok(snapshots)
}

pub fn recovery_candidates(app_data: &Path) -> Vec<RecoveryCopy> {
    list(app_data)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|snapshot| {
            Some(RecoveryCopy {
                path: path(app_data, &snapshot.id).ok()?,
                sha256: snapshot.sha256,
                created_at: snapshot.created_at,
            })
        })
        .collect()
}

pub fn restore_source(app_data: &Path, id: &str) -> Result<(File, PathBuf), String> {
    let lock = lease(app_data)?;
    let snapshot = list(app_data)?
        .into_iter()
        .find(|snapshot| snapshot.id == id)
        .ok_or("Snapshot is no longer available")?;
    let source = path(app_data, id)?;
    if files::digest(&source)? != snapshot.sha256 {
        return Err("Snapshot verification failed".into());
    }
    Ok((lock, source))
}

pub fn delete(db: &DbState, app_data: &Path, id: &str) -> Result<(), String> {
    let _lease = lease(app_data)?;
    crate::features::require(db, crate::features::Feature::Snapshots)?;
    let source = path(app_data, id)?;
    if !list(app_data)?.iter().any(|snapshot| snapshot.id == id) {
        return Err("Snapshot is no longer available".into());
    }
    fs::remove_file(source).map_err(|error| error.to_string())?;
    let _ = fs::remove_file(directory(app_data).join(format!("{id}.json")));
    files::sync_directory(&directory(app_data))
}

fn retention_count(db: &DbState) -> Result<usize, String> {
    Ok(db
        .get_setting(RETENTION_KEY)
        .map_err(|error| error.to_string())?
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value <= 10000)
        .unwrap_or(24))
}

fn prune_locked(app_data: &Path, keep: usize) -> Result<(), String> {
    if keep == 0 {
        return Ok(());
    }
    for old in list(app_data)?.into_iter().skip(keep) {
        if fs::remove_file(path(app_data, &old.id)?).is_ok() {
            let _ = fs::remove_file(directory(app_data).join(format!("{}.json", old.id)));
        }
    }
    files::sync_directory(&directory(app_data))
}

fn remove_unpublished_locked(app_data: &Path) -> Result<(), String> {
    let valid_ids = list(app_data)?
        .into_iter()
        .map(|snapshot| snapshot.id)
        .collect::<HashSet<_>>();
    for entry in fs::read_dir(directory(app_data)).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let entry_path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(".pasted-full-backup-") {
            super::remove_database_files(&entry_path);
            continue;
        }
        let extension = entry_path.extension().and_then(|value| value.to_str());
        let id = entry_path.file_stem().and_then(|value| value.to_str());
        if matches!(extension, Some("json") | Some("pastedbackup"))
            && id.is_none_or(|id| !valid_ids.contains(id))
        {
            if extension == Some("pastedbackup") {
                super::remove_database_files(&entry_path);
            } else {
                let _ = fs::remove_file(entry_path);
            }
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
