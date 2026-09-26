use super::*;
use chrono::Utc;

struct Fixture(std::path::PathBuf);

impl Fixture {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("pasted-reset-{}", files::nonce())))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn factory_reset_deletes_snapshots_and_preserves_manual_backups() {
    let fixture = Fixture::new();
    let (session, db) = open_library(&fixture.0).unwrap();
    db.save_clip("text", Some("reset"), None, None, "reset", "Fixture")
        .unwrap();
    snapshots::check(&db, &fixture.0, Utc::now(), None)
        .unwrap()
        .unwrap();
    let manual = fixture.0.join("Manual.pastedbackup");
    db.create_full_backup(&manual, None, None).unwrap();

    let report = factory_reset_library(&db, &session).unwrap();

    assert_eq!(report.snapshots_deleted, 1);
    assert!(snapshots::list(&fixture.0).unwrap().is_empty());
    assert!(manual.is_file());
    assert!(db.get_clips(None, false).unwrap().is_empty());
}

#[test]
fn failed_factory_reset_restores_staged_snapshots() {
    let fixture = Fixture::new();
    let (session, db) = open_library(&fixture.0).unwrap();
    db.save_clip("text", Some("keep"), None, None, "keep", "Fixture")
        .unwrap();
    let snapshot = snapshots::check(&db, &fixture.0, Utc::now(), None)
        .unwrap()
        .unwrap();
    db.conn
        .lock()
        .execute_batch(
            "CREATE TRIGGER reject_snapshot_reset BEFORE DELETE ON clips
             BEGIN SELECT RAISE(ABORT, 'simulated reset failure'); END;",
        )
        .unwrap();

    assert!(factory_reset_library(&db, &session).is_err());

    assert_eq!(snapshots::list(&fixture.0).unwrap()[0].id, snapshot.id);
    assert_eq!(db.get_clips(None, false).unwrap().len(), 1);
}
