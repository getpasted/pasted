use super::{files, CONFIG_FILE_NAME, DATABASE_FILE_NAME};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryCopy {
    pub path: PathBuf,
    pub sha256: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct LocationRecord {
    pub directory: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery: Option<RecoveryCopy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notice: Option<super::LibraryRecoveryNotice>,
}

pub fn default_database_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(DATABASE_FILE_NAME)
}

pub(super) fn read_location(app_data: &Path) -> Result<Option<LocationRecord>, String> {
    let path = app_data.join(CONFIG_FILE_NAME);
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
        Ok(metadata) if !metadata.is_file() || metadata.file_type().is_symlink() => {
            return Err("The saved library location is not a regular file".into())
        }
        Ok(_) => {}
    }
    let record: LocationRecord = files::read_json(&path)?;
    if !record.directory.is_absolute() {
        return Err("The saved library location is not absolute".into());
    }
    Ok(Some(record))
}

pub(super) fn recorded_path(app_data: &Path) -> Result<PathBuf, String> {
    Ok(read_location(app_data)?
        .map(|record| record.directory.join(DATABASE_FILE_NAME))
        .unwrap_or_else(|| default_database_path(app_data)))
}

pub fn resolve_database_path(app_data: &Path) -> Result<PathBuf, String> {
    let record = read_location(app_data)?;
    let path = record
        .as_ref()
        .map(|record| record.directory.join(DATABASE_FILE_NAME))
        .unwrap_or_else(|| default_database_path(app_data));
    // Only a first launch may create a database. A recorded move must never
    // silently create a replacement or select an older default copy.
    if record.is_some() && !path.is_file() {
        return Err("The library is unavailable at its saved location".into());
    }
    Ok(path)
}

pub fn persist_location(app_data: &Path, database_path: &Path) -> Result<(), String> {
    write_location(app_data, database_path, None)
}

pub(super) fn write_location(
    app_data: &Path,
    database_path: &Path,
    recovery: Option<RecoveryCopy>,
) -> Result<(), String> {
    let notice = read_location(app_data)
        .ok()
        .flatten()
        .and_then(|record| record.notice);
    write_location_with_notice(app_data, database_path, recovery, notice)
}

pub(super) fn write_location_with_notice(
    app_data: &Path,
    database_path: &Path,
    recovery: Option<RecoveryCopy>,
    notice: Option<super::LibraryRecoveryNotice>,
) -> Result<(), String> {
    let directory = database_path
        .parent()
        .ok_or("The library location has no parent folder")?;
    files::write_json(
        &app_data.join(CONFIG_FILE_NAME),
        &LocationRecord {
            directory: directory.into(),
            recovery,
            notice,
        },
    )
}
