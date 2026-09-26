use super::*;
use crate::db::DbState;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("pasted-auto-{}", files::nonce()));
        fs::create_dir_all(root.join("destination")).unwrap();
        Self(fs::canonicalize(root).unwrap())
    }
    fn app(&self) -> PathBuf {
        self.0.join("app")
    }
    fn moved(&self) -> LibraryMoveReport {
        let (session, db) = open_library(&self.app()).unwrap();
        db.save_clip("text", Some("Before move"), None, None, "before", "Fixture")
            .unwrap();
        let report = db
            .move_library(&session, &self.0.join("destination"), false)
            .unwrap();
        db.save_clip("text", Some("After move"), None, None, "after", "Fixture")
            .unwrap();
        report
    }
    fn disconnect(&self) {
        fs::rename(self.0.join("destination"), self.0.join("offline")).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn recovery_opens_history_and_persists_a_note_without_returning_to_a_reconnected_library() {
    let fixture = Fixture::new();
    fixture.moved();
    fixture.disconnect();
    let (session, db) = open_library_automatically(&fixture.app()).unwrap();
    assert_eq!(db.get_clips(None, false).unwrap().len(), 1);
    let notice = recovery_notice(&fixture.app()).unwrap();
    assert_eq!(notice.outcome, "recovered");
    assert!(notice.recovery_created_at.is_some());
    assert_eq!(db.database_path(), fixture.app().join("pasted.db"));
    db.save_clip("text", Some("New capture"), None, None, "new", "Fixture")
        .unwrap();
    drop(db);
    drop(session);
    fs::rename(fixture.0.join("offline"), fixture.0.join("destination")).unwrap();
    let (session, db) = open_library_automatically(&fixture.app()).unwrap();
    assert_eq!(db.database_path(), fixture.app().join("pasted.db"));
    assert_eq!(db.get_clips(None, false).unwrap().len(), 2);
    assert_eq!(
        recovery_notice(&fixture.app()).unwrap().occurred_at,
        notice.occurred_at
    );
    db.move_library(&session, &fixture.0, false).unwrap();
    assert_eq!(
        recovery_notice(&fixture.app()).unwrap().occurred_at,
        notice.occurred_at
    );
}

#[test]
fn failed_snapshot_starts_empty_and_preserves_all_existing_history_files() {
    let fixture = Fixture::new();
    let report = fixture.moved();
    fixture.disconnect();
    fs::write(&report.recovery_path, b"unusable recovery copy").unwrap();
    let (session, db) = open_library_automatically(&fixture.app()).unwrap();
    assert!(db.get_clips(None, false).unwrap().is_empty());
    assert_eq!(
        db.get_setting(crate::external_import::ONBOARDING_SETTING_KEY)
            .unwrap()
            .as_deref(),
        Some("1")
    );
    assert_eq!(
        fs::read(report.recovery_path).unwrap(),
        b"unusable recovery copy"
    );
    let notice = recovery_notice(&fixture.app()).unwrap();
    assert_eq!(notice.outcome, "fresh");
    assert!(notice.recovery_created_at.is_none());
    let previous = DbState::open_existing(fixture.0.join("offline/pasted.db")).unwrap();
    assert_eq!(previous.get_clips(None, false).unwrap().len(), 2);
    drop(previous);
    db.save_setting("after_repair", "preserved").unwrap();
    drop(db);
    drop(session);
    assert_eq!(
        open_library_automatically(&fixture.app())
            .unwrap()
            .1
            .get_setting("after_repair")
            .unwrap()
            .as_deref(),
        Some("preserved")
    );
}

#[test]
fn corrupted_location_record_and_default_are_preserved_before_fresh_start() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.app()).unwrap();
    fs::write(
        fixture.app().join("library-location.json"),
        b"unreadable record",
    )
    .unwrap();
    fs::write(fixture.app().join("pasted.db"), b"unreadable database").unwrap();
    fs::write(fixture.app().join("pasted.db-wal"), b"orphan changes").unwrap();
    let (session, db) = open_library_automatically(&fixture.app()).unwrap();
    assert!(db.get_clips(None, false).unwrap().is_empty());
    let note = recovery_notice(&fixture.app()).unwrap();
    assert_eq!(note.outcome, "fresh");
    assert_eq!(
        fs::read(note.preserved_path.join("library-location.json")).unwrap(),
        b"unreadable record"
    );
    let preserved: Vec<Vec<u8>> = fs::read_dir(&note.preserved_path)
        .unwrap()
        .map(|entry| fs::read(entry.unwrap().path()).unwrap())
        .collect();
    assert!(preserved.contains(&b"unreadable database".to_vec()));
    assert!(preserved.contains(&b"orphan changes".to_vec()));
    drop(db);
    drop(session);
}

#[test]
fn healthy_start_and_busy_session_never_trigger_fresh_fallback() {
    let fixture = Fixture::new();
    let (session, db) = open_library_automatically(&fixture.app()).unwrap();
    assert!(recovery_notice(&fixture.app()).is_none());
    let current = db.database_path();
    persist_location(&fixture.app(), &fixture.0.join("missing/pasted.db")).unwrap();
    assert!(open_library_automatically(&fixture.app()).is_err());
    assert!(current.is_file());
    assert!(!fixture.app().join("library-recovery").exists());
    drop(db);
    drop(session);
}

#[test]
fn corrupt_moved_database_uses_verified_snapshot_and_keeps_the_corrupt_file() {
    let fixture = Fixture::new();
    fixture.moved();
    let damaged = fixture.0.join("destination/pasted.db");
    fs::write(&damaged, b"damaged library").unwrap();
    let (_, db) = open_library_automatically(&fixture.app()).unwrap();
    assert_eq!(db.get_clips(None, false).unwrap().len(), 1);
    assert_eq!(fs::read(damaged).unwrap(), b"damaged library");
    assert_eq!(
        recovery_notice(&fixture.app()).unwrap().outcome,
        "recovered"
    );
}

#[test]
fn unavailable_default_folder_returns_an_error_without_discarding_the_obstruction() {
    let fixture = Fixture::new();
    fs::write(fixture.app(), b"existing file").unwrap();
    assert!(open_library_automatically(&fixture.app()).is_err());
    assert_eq!(fs::read(fixture.app()).unwrap(), b"existing file");
}

#[test]
fn interrupted_activation_resumes_but_committed_activation_never_replays_old_history() {
    for (committed, published) in [(false, false), (false, true), (true, false)] {
        let fixture = Fixture::new();
        fixture.moved();
        fixture.disconnect();
        let (session, db) = open_library_automatically(&fixture.app()).unwrap();
        db.save_setting("later_capture", "yes").unwrap();
        let mut notice = recovery_notice(&fixture.app()).unwrap();
        drop(db);
        drop(session);
        let prepared = notice.preserved_path.join("prepared.db");
        if !committed {
            notice.occurred_at = "2026-09-06T09:00:00Z".into();
        }
        files::write_json(&fixture.app().join("library-startup.json"), &serde_json::json!({
            "prepared": { "path": prepared, "sha256": files::digest(&prepared).unwrap(), "createdAt": notice.occurred_at },
            "recovery": null, "notice": notice, "published": published,
        })).unwrap();
        let (session, db) = open_library_automatically(&fixture.app()).unwrap();
        assert_eq!(
            db.get_setting("later_capture").unwrap().is_some(),
            committed || published
        );
        assert_eq!(db.get_clips(None, false).unwrap().len(), 1);
        assert!(!fixture.app().join("library-startup.json").exists());
        drop(db);
        drop(session);
    }
}
