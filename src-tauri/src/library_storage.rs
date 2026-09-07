use serde::{Deserialize, Serialize};

mod automatic_recovery;
mod factory_reset;
mod recovery_activation;
mod recovery_dismissal;
pub use automatic_recovery::{open_library_automatically, recovery_notice, LibraryRecoveryNotice};
pub use factory_reset::factory_reset_library;
pub use recovery_dismissal::{dismiss_recovery_notice, visible_recovery_notice};
mod files;
mod location;
mod session;
pub mod settings_watch;
pub mod snapshots;
mod startup;
pub use startup::{open_library, LibraryStartupStatus};
#[cfg(test)]
mod factory_reset_tests;
#[cfg(test)]
mod tests;
pub(crate) mod transfer;
pub use location::{default_database_path, persist_location, resolve_database_path, RecoveryCopy};
pub use session::LibrarySession;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
pub use transfer::LibraryMoveReport;

const CONFIG_FILE_NAME: &str = "library-location.json";
const DATABASE_FILE_NAME: &str = "pasted.db";
const MAX_PATH_LENGTH: usize = 4096;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryLocationInfo {
    pub path: String,
    pub directory: String,
    pub is_default: bool,
}

pub fn location_info(app_data_dir: &Path, database_path: &Path) -> LibraryLocationInfo {
    let directory = database_path.parent().unwrap_or(app_data_dir);
    LibraryLocationInfo {
        path: database_path.to_string_lossy().into_owned(),
        directory: directory.to_string_lossy().into_owned(),
        is_default: database_path == default_database_path(app_data_dir),
    }
}

pub fn validate_destination_directory(
    directory: &Path,
    current_database_path: &Path,
) -> Result<PathBuf, String> {
    if !directory.is_absolute() {
        return Err("Choose an absolute local folder for the Pasted library.".to_string());
    }
    if directory.as_os_str().len() > MAX_PATH_LENGTH {
        return Err("That library location is too long.".to_string());
    }
    let metadata = fs::symlink_metadata(directory)
        .map_err(|_| "That library folder is not available.".to_string())?;
    if metadata.file_type().is_symlink() {
        return Err("Choose the folder itself rather than a symbolic link.".to_string());
    }
    if !metadata.is_dir() {
        return Err("Choose a folder for the Pasted library.".to_string());
    }
    let canonical = fs::canonicalize(directory)
        .map_err(|_| "That library folder could not be resolved.".to_string())?;
    if canonical.parent().is_none() {
        return Err("The filesystem root cannot be used as the Pasted library.".to_string());
    }

    let target = canonical.join(DATABASE_FILE_NAME);
    if target == current_database_path {
        return Ok(target);
    }
    if target.exists() {
        return Err("That folder already contains a Pasted library.".to_string());
    }

    reject_orphan_sidecars(&target)?;
    let probe = canonical.join(format!(".pasted-write-test-{}", files::nonce()));
    let probe_result = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .and_then(|mut file| file.write_all(b"pasted"));
    let _ = fs::remove_file(&probe);
    probe_result.map_err(|_| "Pasted cannot write to that folder.".to_string())?;
    Ok(target)
}

pub fn archive_existing_database(database_path: &Path) -> Result<Option<PathBuf>, String> {
    if !database_path.exists() {
        reject_orphan_sidecars(database_path)?;
        return Ok(None);
    }
    let parent = database_path
        .parent()
        .ok_or_else(|| "The existing library has no parent folder.".to_string())?;
    let archived = parent.join(format!("pasted-recovery-{}.db", files::nonce()));
    fs::rename(database_path, &archived)
        .map_err(|error| format!("Could not preserve the existing default library: {error}"))?;
    for suffix in ["-wal", "-shm", "-journal"] {
        let sidecar = PathBuf::from(format!("{}{suffix}", database_path.display()));
        if sidecar.exists() {
            let archived_sidecar = PathBuf::from(format!("{}{suffix}", archived.display()));
            if let Err(error) = fs::rename(&sidecar, &archived_sidecar) {
                restore_archived_database(&archived, database_path);
                return Err(format!(
                    "Could not preserve the existing library sidecar: {error}"
                ));
            }
        }
    }
    Ok(Some(archived))
}

pub fn restore_archived_database(archived: &Path, database_path: &Path) {
    let _ = fs::rename(archived, database_path);
    for suffix in ["-wal", "-shm", "-journal"] {
        let archived_sidecar = PathBuf::from(format!("{}{suffix}", archived.display()));
        if archived_sidecar.exists() {
            let sidecar = PathBuf::from(format!("{}{suffix}", database_path.display()));
            let _ = fs::rename(archived_sidecar, sidecar);
        }
    }
}

pub fn remove_database_files(database_path: &Path) {
    let _ = fs::remove_file(database_path);
    for suffix in ["-wal", "-shm", "-journal"] {
        let sidecar = PathBuf::from(format!("{}{suffix}", database_path.display()));
        let _ = fs::remove_file(sidecar);
    }
}

#[cfg(test)]
mod move_tests;

#[cfg(test)]
mod journal_tests;

fn reject_orphan_sidecars(path: &Path) -> Result<(), String> {
    for suffix in ["-wal", "-shm", "-journal"] {
        if fs::symlink_metadata(PathBuf::from(format!("{}{suffix}", path.display()))).is_ok() {
            return Err(
                "The destination contains database recovery files; they have been preserved".into(),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod automatic_recovery_tests;

#[cfg(test)]
mod snapshot_tests;
