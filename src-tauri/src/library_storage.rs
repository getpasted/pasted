use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LibraryLocationConfig {
    directory: PathBuf,
}

pub fn default_database_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(DATABASE_FILE_NAME)
}

pub fn resolve_database_path(app_data_dir: &Path) -> PathBuf {
    let default = default_database_path(app_data_dir);
    let Ok(bytes) = fs::read(app_data_dir.join(CONFIG_FILE_NAME)) else {
        return default;
    };
    if bytes.len() > 16 * 1024 {
        return default;
    }
    let Ok(config) = serde_json::from_slice::<LibraryLocationConfig>(&bytes) else {
        return default;
    };
    if !config.directory.is_absolute() {
        return default;
    }
    let configured = config.directory.join(DATABASE_FILE_NAME);
    if configured.is_file() {
        configured
    } else {
        default
    }
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

    let probe = canonical.join(format!(".pasted-write-test-{}", std::process::id()));
    let probe_result = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .and_then(|mut file| file.write_all(b"pasted"));
    let _ = fs::remove_file(&probe);
    probe_result.map_err(|_| "Pasted cannot write to that folder.".to_string())?;
    Ok(target)
}

pub fn persist_location(app_data_dir: &Path, database_path: &Path) -> Result<(), String> {
    let directory = database_path
        .parent()
        .ok_or_else(|| "The library location has no parent folder.".to_string())?;
    fs::create_dir_all(app_data_dir)
        .map_err(|error| format!("Could not prepare Pasted’s settings folder: {error}"))?;
    let config_path = app_data_dir.join(CONFIG_FILE_NAME);
    if database_path == default_database_path(app_data_dir) {
        if config_path.exists() {
            fs::remove_file(&config_path).map_err(|error| {
                format!("Could not restore the default library location: {error}")
            })?;
        }
        return Ok(());
    }
    let temporary = app_data_dir.join(format!(".{CONFIG_FILE_NAME}.{}.tmp", std::process::id()));
    let contents = serde_json::to_vec_pretty(&LibraryLocationConfig {
        directory: directory.to_path_buf(),
    })
    .map_err(|error| error.to_string())?;
    fs::write(&temporary, contents)
        .map_err(|error| format!("Could not save the library location: {error}"))?;
    fs::rename(&temporary, &config_path)
        .map_err(|error| format!("Could not activate the library location: {error}"))?;
    Ok(())
}

pub fn archive_existing_database(database_path: &Path) -> Result<Option<PathBuf>, String> {
    if !database_path.exists() {
        return Ok(None);
    }
    let parent = database_path
        .parent()
        .ok_or_else(|| "The existing library has no parent folder.".to_string())?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    let archived = parent.join(format!("pasted-recovery-{stamp}.db"));
    fs::rename(database_path, &archived)
        .map_err(|error| format!("Could not preserve the existing default library: {error}"))?;
    for suffix in ["-wal", "-shm"] {
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
    for suffix in ["-wal", "-shm"] {
        let archived_sidecar = PathBuf::from(format!("{}{suffix}", archived.display()));
        if archived_sidecar.exists() {
            let sidecar = PathBuf::from(format!("{}{suffix}", database_path.display()));
            let _ = fs::rename(archived_sidecar, sidecar);
        }
    }
}

pub fn remove_database_files(database_path: &Path) {
    let _ = fs::remove_file(database_path);
    for suffix in ["-wal", "-shm"] {
        let sidecar = PathBuf::from(format!("{}{suffix}", database_path.display()));
        let _ = fs::remove_file(sidecar);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_directory(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("pasted-{label}-{nonce}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn invalid_or_missing_configuration_falls_back_to_default() {
        let app_data = temp_directory("library-config");
        fs::write(app_data.join(CONFIG_FILE_NAME), b"not json").unwrap();
        assert_eq!(
            resolve_database_path(&app_data),
            default_database_path(&app_data)
        );
        let _ = fs::remove_dir_all(app_data);
    }

    #[test]
    fn configured_library_must_exist_before_it_is_selected() {
        let app_data = temp_directory("library-app-data");
        let custom = temp_directory("library-custom");
        persist_location(&app_data, &custom.join(DATABASE_FILE_NAME)).unwrap();
        assert_eq!(
            resolve_database_path(&app_data),
            default_database_path(&app_data)
        );
        fs::write(custom.join(DATABASE_FILE_NAME), b"database").unwrap();
        assert_eq!(
            resolve_database_path(&app_data),
            custom.join(DATABASE_FILE_NAME)
        );
        let _ = fs::remove_dir_all(app_data);
        let _ = fs::remove_dir_all(custom);
    }

    #[test]
    fn destination_refuses_an_existing_library() {
        let current_dir = temp_directory("library-current");
        let destination = temp_directory("library-destination");
        let current = current_dir.join(DATABASE_FILE_NAME);
        fs::write(destination.join(DATABASE_FILE_NAME), b"do not overwrite").unwrap();
        let error = validate_destination_directory(&destination, &current).unwrap_err();
        assert!(error.contains("already contains"));
        let _ = fs::remove_dir_all(current_dir);
        let _ = fs::remove_dir_all(destination);
    }

    #[test]
    fn persisting_default_location_removes_the_custom_pointer() {
        let app_data = temp_directory("library-default-pointer");
        let custom = temp_directory("library-default-custom");
        persist_location(&app_data, &custom.join(DATABASE_FILE_NAME)).unwrap();
        assert!(app_data.join(CONFIG_FILE_NAME).is_file());

        persist_location(&app_data, &default_database_path(&app_data)).unwrap();

        assert!(!app_data.join(CONFIG_FILE_NAME).exists());
        let _ = fs::remove_dir_all(app_data);
        let _ = fs::remove_dir_all(custom);
    }

    #[test]
    fn archiving_an_existing_default_never_deletes_it() {
        let directory = temp_directory("library-archive");
        let database = directory.join(DATABASE_FILE_NAME);
        fs::write(&database, b"old default").unwrap();

        let archived = archive_existing_database(&database).unwrap().unwrap();

        assert!(!database.exists());
        assert_eq!(fs::read(&archived).unwrap(), b"old default");
        restore_archived_database(&archived, &database);
        assert_eq!(fs::read(&database).unwrap(), b"old default");
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn removing_a_database_also_removes_its_sidecars() {
        let directory = temp_directory("library-remove");
        let database = directory.join(DATABASE_FILE_NAME);
        fs::write(&database, b"database").unwrap();
        fs::write(format!("{}-wal", database.display()), b"wal").unwrap();
        fs::write(format!("{}-shm", database.display()), b"shm").unwrap();

        remove_database_files(&database);

        assert!(!database.exists());
        assert!(!PathBuf::from(format!("{}-wal", database.display())).exists());
        assert!(!PathBuf::from(format!("{}-shm", database.display())).exists());
        let _ = fs::remove_dir_all(directory);
    }

    #[cfg(unix)]
    #[test]
    fn destination_refuses_symbolic_links() {
        use std::os::unix::fs::symlink;
        let current_dir = temp_directory("library-current-link");
        let destination = temp_directory("library-real-destination");
        let link = current_dir.join("linked-destination");
        symlink(&destination, &link).unwrap();
        let error = validate_destination_directory(&link, &current_dir.join(DATABASE_FILE_NAME))
            .unwrap_err();
        assert!(error.contains("symbolic link"));
        let _ = fs::remove_dir_all(current_dir);
        let _ = fs::remove_dir_all(destination);
    }
}
