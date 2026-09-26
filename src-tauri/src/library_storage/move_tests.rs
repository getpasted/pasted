use super::*;
use crate::db::DbState;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("pasted-moves-{}", files::nonce()));
        fs::create_dir_all(root.join("app")).unwrap();
        fs::create_dir_all(root.join("destination")).unwrap();
        Self(fs::canonicalize(root).unwrap())
    }
    fn app(&self) -> PathBuf {
        self.0.join("app")
    }
    fn destination(&self) -> PathBuf {
        self.0.join("destination")
    }
    fn open(&self) -> (LibrarySession, DbState) {
        open_library(&self.app()).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn move_preserves_durable_tables_and_routes_later_writes_to_destination() {
    let fixture = Fixture::new();
    let (session, db) = fixture.open();
    db.save_setting("move_test", "before").unwrap();
    db.conn.lock().execute_batch("CREATE TABLE future_durable(value TEXT); INSERT INTO future_durable VALUES ('retained');").unwrap();
    let original = db.database_path();
    let report = db
        .move_library(&session, &fixture.destination(), false)
        .unwrap();
    assert_eq!(session.database_path().unwrap(), db.database_path());
    assert_eq!(
        db.conn
            .lock()
            .query_row("SELECT value FROM future_durable", [], |row| row
                .get::<_, String>(0))
            .unwrap(),
        "retained"
    );
    db.save_setting("move_test", "after").unwrap();
    assert_eq!(
        DbState::open_existing(original)
            .unwrap()
            .get_setting("move_test")
            .unwrap()
            .as_deref(),
        Some("before")
    );
    assert_eq!(
        DbState::open_existing(PathBuf::from(report.recovery_path))
            .unwrap()
            .get_setting("move_test")
            .unwrap()
            .as_deref(),
        Some("before")
    );
    drop(db);
    drop(session);
    assert_eq!(
        fixture
            .open()
            .1
            .get_setting("move_test")
            .unwrap()
            .as_deref(),
        Some("after")
    );
}

#[test]
fn failed_commit_preserves_original_connection_and_restart_location() {
    let fixture = Fixture::new();
    let (session, db) = fixture.open();
    let original = db.database_path();
    let result = db.move_library_with_commit_check(&session, &fixture.destination(), false, || {
        Err("injected write failure".into())
    });
    assert!(result.is_err());
    assert_eq!(db.database_path(), original);
    assert!(!fixture.destination().join("pasted.db").exists());
    db.save_setting("survived", "yes").unwrap();
    drop(db);
    drop(session);
    assert_eq!(
        fixture.open().1.get_setting("survived").unwrap().as_deref(),
        Some("yes")
    );
}

#[test]
fn another_session_blocks_moves_without_changing_either_library() {
    let fixture = Fixture::new();
    let (session, db) = fixture.open();
    let (other_session, other_db) = fixture.open();
    assert!(db
        .move_library(&session, &fixture.destination(), false)
        .is_err());
    assert!(!fixture.destination().join("pasted.db").exists());
    other_db.save_setting("shared", "yes").unwrap();
    assert_eq!(db.get_setting("shared").unwrap().as_deref(), Some("yes"));
    drop(other_db);
    drop(other_session);
    db.move_library(&session, &fixture.destination(), false)
        .unwrap();
}

#[test]
fn missing_moved_library_recovers_automatically() {
    let fixture = Fixture::new();
    let (session, db) = fixture.open();
    db.save_setting("snapshot", "before").unwrap();
    let report = db
        .move_library(&session, &fixture.destination(), false)
        .unwrap();
    db.save_setting("snapshot", "after").unwrap();
    drop(db);
    drop(session);
    fs::rename(fixture.destination(), fixture.0.join("disconnected")).unwrap();
    assert!(open_library(&fixture.app()).is_err());
    let status = LibraryStartupStatus::unavailable(&fixture.app());
    assert!(!status.ready);
    assert!(status.recovery_created_at.is_some());
    drop(open_library_automatically(&fixture.app()).unwrap());
    let (_, db) = fixture.open();
    assert_eq!(
        db.get_setting("snapshot").unwrap().as_deref(),
        Some("before")
    );
    assert!(fixture.0.join("disconnected/pasted.db").is_file());
    assert!(PathBuf::from(report.recovery_path).is_file());
}

#[test]
fn returning_to_default_retains_previous_default_and_records_missing_default() {
    let fixture = Fixture::new();
    let (session, db) = fixture.open();
    db.move_library(&session, &fixture.destination(), false)
        .unwrap();
    db.save_setting("latest", "yes").unwrap();
    db.move_library(&session, &fixture.app(), true).unwrap();
    assert_eq!(db.get_setting("latest").unwrap().as_deref(), Some("yes"));
    assert!(fs::read_dir(fixture.app()).unwrap().any(|entry| entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with("pasted-recovery-")));
    drop(db);
    drop(session);
    fs::remove_file(fixture.app().join("pasted.db")).unwrap();
    assert!(open_library(&fixture.app()).is_err());
}

#[test]
fn existing_destination_and_invalid_database_remain_untouched() {
    let fixture = Fixture::new();
    let (session, db) = fixture.open();
    let target = fixture.destination().join("pasted.db");
    fs::write(&target, b"unrelated file").unwrap();
    assert!(db
        .move_library(&session, &fixture.destination(), false)
        .is_err());
    assert_eq!(fs::read(&target).unwrap(), b"unrelated file");
    persist_location(&fixture.app(), &target).unwrap();
    drop(db);
    drop(session);
    assert!(open_library(&fixture.app()).is_err());
    assert_eq!(fs::read(target).unwrap(), b"unrelated file");
}

#[test]
fn recovery_at_the_same_default_path_is_not_rolled_back() {
    let fixture = Fixture::new();
    let (session, db) = fixture.open();
    db.move_library(&session, &fixture.destination(), false)
        .unwrap();
    db.save_setting("recovery", "yes").unwrap();
    db.move_library(&session, &fixture.app(), true).unwrap();
    drop(db);
    fs::remove_file(fixture.app().join("pasted.db")).unwrap();
    drop(session);
    drop(open_library_automatically(&fixture.app()).unwrap());
    assert_eq!(
        fixture.open().1.get_setting("recovery").unwrap().as_deref(),
        Some("yes")
    );
}

#[test]
fn orphan_recovery_files_at_destination_are_preserved() {
    let fixture = Fixture::new();
    let (session, db) = fixture.open();
    let orphan = fixture.destination().join("pasted.db-wal");
    fs::write(&orphan, b"uncheckpointed changes").unwrap();
    assert!(db
        .move_library(&session, &fixture.destination(), false)
        .is_err());
    assert_eq!(fs::read(&orphan).unwrap(), b"uncheckpointed changes");
    assert_eq!(db.database_path(), fixture.app().join("pasted.db"));
}

#[test]
fn failed_worker_restores_its_session_lease() {
    let fixture = Fixture::new();
    let (session, db) = fixture.open();
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        session.exclusive::<()>(|| panic!("injected worker failure"))
    }));
    assert!(failure.is_err());
    let other = LibrarySession::open(&fixture.app()).unwrap();
    assert!(other.exclusive(|| Ok(())).is_err());
    drop(db);
    drop(session);
    assert!(other.exclusive(|| Ok(())).is_ok());
}
