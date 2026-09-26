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
fn invalid_configuration_never_falls_back_to_default() {
    let app_data = temp_directory("library-config");
    fs::write(app_data.join(CONFIG_FILE_NAME), b"not json").unwrap();
    assert!(resolve_database_path(&app_data).is_err());
    let _ = fs::remove_dir_all(app_data);
}

#[test]
fn configured_library_must_exist_before_it_is_selected() {
    let app_data = temp_directory("library-app-data");
    let custom = temp_directory("library-custom");
    persist_location(&app_data, &custom.join(DATABASE_FILE_NAME)).unwrap();
    assert!(resolve_database_path(&app_data).is_err());
    fs::write(custom.join(DATABASE_FILE_NAME), b"database").unwrap();
    assert_eq!(
        resolve_database_path(&app_data).unwrap(),
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
fn persisting_default_location_records_it_explicitly() {
    let app_data = temp_directory("library-default-pointer");
    let custom = temp_directory("library-default-custom");
    persist_location(&app_data, &custom.join(DATABASE_FILE_NAME)).unwrap();
    assert!(app_data.join(CONFIG_FILE_NAME).is_file());

    persist_location(&app_data, &default_database_path(&app_data)).unwrap();

    assert!(app_data.join(CONFIG_FILE_NAME).is_file());
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
    let error =
        validate_destination_directory(&link, &current_dir.join(DATABASE_FILE_NAME)).unwrap_err();
    assert!(error.contains("symbolic link"));
    let _ = fs::remove_dir_all(current_dir);
    let _ = fs::remove_dir_all(destination);
}
