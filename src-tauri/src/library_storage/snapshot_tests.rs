use super::*;
use crate::db::DbState;
use chrono::{Duration, TimeZone, Utc};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("pasted-snapshots-{}", files::nonce())))
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn add(db: &DbState, value: &str) {
    db.save_clip("text", Some(value), None, None, value, "Fixture")
        .unwrap();
}

#[test]
fn interval_changes_apply_immediately_and_only_new_clips_trigger_snapshots() {
    let fixture = Fixture::new();
    let (_session, db) = open_library(&fixture.0).unwrap();
    let now = Utc.with_ymd_and_hms(2026, 9, 6, 0, 0, 0).unwrap();
    assert!(snapshots::check(&db, &fixture.0, now, None)
        .unwrap()
        .is_none());
    add(&db, "first");
    let first = snapshots::check(&db, &fixture.0, now, None)
        .unwrap()
        .unwrap();
    assert_eq!(first.created_at, "2026-09-06T00:00:00Z");
    add(&db, "second");
    assert!(
        snapshots::check(&db, &fixture.0, now + Duration::minutes(30), None)
            .unwrap()
            .is_none()
    );
    db.save_setting(snapshots::INTERVAL_KEY, "30").unwrap();
    assert!(
        snapshots::check(&db, &fixture.0, now + Duration::minutes(30), None)
            .unwrap()
            .is_some()
    );
    assert!(
        snapshots::check(&db, &fixture.0, now + Duration::hours(1), None)
            .unwrap()
            .is_none()
    );
    db.conn
        .lock()
        .execute("DELETE FROM clips WHERE content_hash = 'first'", [])
        .unwrap();
    assert!(
        snapshots::check(&db, &fixture.0, now + Duration::hours(1), None)
            .unwrap()
            .is_none()
    );
    add(&db, "replacement"); // Same count as the previous snapshot, different clip.
    assert!(
        snapshots::check(&db, &fixture.0, now + Duration::hours(1), None)
            .unwrap()
            .is_some()
    );
    db.save_setting(crate::features::Feature::Snapshots.setting_key(), "false")
        .unwrap();
    add(&db, "disabled");
    assert!(
        snapshots::check(&db, &fixture.0, now + Duration::days(1), None)
            .unwrap()
            .is_none()
    );
    assert_eq!(snapshots::list(&fixture.0).unwrap().len(), 3);
    db.save_setting(crate::features::Feature::Snapshots.setting_key(), "true")
        .unwrap();
    assert!(
        snapshots::check(&db, &fixture.0, now + Duration::days(1), None)
            .unwrap()
            .is_some()
    );
}

#[test]
fn retention_and_restore_preserve_complete_state_and_validate_before_replacement() {
    let fixture = Fixture::new();
    let (session, db) = open_library(&fixture.0).unwrap();
    let now = Utc.with_ymd_and_hms(2026, 9, 6, 0, 0, 0).unwrap();
    db.save_setting(snapshots::RETENTION_KEY, "2").unwrap();
    for index in 0..3 {
        add(&db, &format!("clip{index}"));
        snapshots::check(&db, &fixture.0, now + Duration::hours(index), None)
            .unwrap()
            .unwrap();
    }
    let available = snapshots::list(&fixture.0).unwrap();
    assert_eq!(available.len(), 2);
    assert_eq!(available[0].clip_count, 3);
    let (lease, source) = snapshots::restore_source(&fixture.0, &available[1].id).unwrap();
    assert!(snapshots::check(&db, &fixture.0, now + Duration::hours(3), None).is_err());
    let (report, _, _) = session
        .exclusive(|| {
            db.restore_full_backup(&source, None, None)
                .map_err(|error| error.to_string())
        })
        .unwrap();
    assert_eq!(db.get_clips(None, false).unwrap().len(), 2);
    let recovery = DbState::open_existing(PathBuf::from(report.recovery_path)).unwrap();
    assert_eq!(recovery.get_clips(None, false).unwrap().len(), 3);
    drop(recovery);
    drop(lease);
    fs::write(&source, b"damaged").unwrap();
    assert!(snapshots::restore_source(&fixture.0, &available[1].id).is_err());
    assert_eq!(db.get_clips(None, false).unwrap().len(), 2);
    assert!(snapshots::restore_source(&fixture.0, "../pasted").is_err());
}

#[test]
fn restoring_an_older_snapshot_restores_its_creation_baseline() {
    let fixture = Fixture::new();
    let (session, db) = open_library(&fixture.0).unwrap();
    let now = Utc.with_ymd_and_hms(2026, 9, 6, 0, 0, 0).unwrap();
    add(&db, "first");
    let first = snapshots::check(&db, &fixture.0, now, None)
        .unwrap()
        .unwrap();
    add(&db, "second");
    snapshots::check(&db, &fixture.0, now + Duration::hours(1), None)
        .unwrap()
        .unwrap();

    let (lease, source) = snapshots::restore_source(&fixture.0, &first.id).unwrap();
    session
        .exclusive(|| {
            db.restore_full_backup(&source, None, None)
                .map_err(|error| error.to_string())
        })
        .unwrap();
    drop(lease);
    add(&db, "after restore");

    let created = snapshots::check(&db, &fixture.0, now + Duration::hours(2), None)
        .unwrap()
        .expect("a clip captured after restore should create a new snapshot");
    assert_eq!(created.clip_count, 2);
}

#[test]
fn automatic_recovery_skips_damaged_newest_snapshot_and_preserves_failed_library() {
    let fixture = Fixture::new();
    let (session, db) = open_library(&fixture.0).unwrap();
    let now = Utc.with_ymd_and_hms(2026, 9, 6, 0, 0, 0).unwrap();
    add(&db, "first");
    snapshots::check(&db, &fixture.0, now, None).unwrap();
    add(&db, "second");
    let latest = snapshots::check(&db, &fixture.0, now + Duration::hours(1), None)
        .unwrap()
        .unwrap();
    let (_, source) = snapshots::restore_source(&fixture.0, &latest.id).unwrap();
    fs::write(source, b"bad snapshot").unwrap();
    drop(db);
    drop(session);
    fs::write(default_database_path(&fixture.0), b"damaged library").unwrap();
    let (_session, db) = open_library_automatically(&fixture.0).unwrap();
    assert_eq!(db.get_clips(None, false).unwrap().len(), 1);
    let notice = recovery_notice(&fixture.0).unwrap();
    assert_eq!(notice.outcome, "recovered");
    assert_eq!(
        notice.recovery_created_at.as_deref(),
        Some("2026-09-06T00:00:00Z")
    );
    assert!(fs::read_dir(notice.preserved_path)
        .unwrap()
        .flatten()
        .any(|entry| fs::read(entry.path()).ok().as_deref() == Some(b"damaged library")));
}

#[test]
fn zero_count_pauses_creation_without_removing_snapshots_or_restore_access() {
    let fixture = Fixture::new();
    let (_session, db) = open_library(&fixture.0).unwrap();
    let now = Utc.with_ymd_and_hms(2026, 9, 6, 0, 0, 0).unwrap();
    add(&db, "first");
    db.save_setting(snapshots::RETENTION_KEY, "0").unwrap();
    assert!(snapshots::check(&db, &fixture.0, now, None)
        .unwrap()
        .is_none());
    assert!(!fixture.0.join("snapshots").exists());
    db.save_setting(snapshots::RETENTION_KEY, "1").unwrap();
    let first = snapshots::check(&db, &fixture.0, now, None)
        .unwrap()
        .unwrap();
    db.save_setting(snapshots::RETENTION_KEY, "0").unwrap();
    add(&db, "second");
    assert!(
        snapshots::check(&db, &fixture.0, now + Duration::days(1), None)
            .unwrap()
            .is_none()
    );
    assert_eq!(snapshots::list(&fixture.0).unwrap().len(), 1);
    let (lease, source) = snapshots::restore_source(&fixture.0, &first.id).unwrap();
    assert!(source.is_file());
    drop(lease);
    db.save_setting(snapshots::RETENTION_KEY, "1").unwrap();
    let next = snapshots::check(&db, &fixture.0, now + Duration::days(1), None)
        .unwrap()
        .unwrap();
    assert_eq!(snapshots::list(&fixture.0).unwrap()[0].id, next.id);
    assert!(!source.exists());
}

#[test]
fn unchanged_checks_use_the_persisted_clip_baseline_and_retention_applies_immediately() {
    let fixture = Fixture::new();
    let (_session, db) = open_library(&fixture.0).unwrap();
    let now = Utc.with_ymd_and_hms(2026, 9, 6, 0, 0, 0).unwrap();
    db.save_setting(snapshots::RETENTION_KEY, "3").unwrap();
    add(&db, "first");
    let first = snapshots::check(&db, &fixture.0, now, None)
        .unwrap()
        .unwrap();
    let (_, first_path) = snapshots::restore_source(&fixture.0, &first.id).unwrap();
    fs::write(&first_path, b"damaged after publication").unwrap();

    // An unchanged library does not need to hash or open the large snapshot.
    assert!(
        snapshots::check(&db, &fixture.0, now + Duration::days(1), None)
            .unwrap()
            .is_none()
    );
    add(&db, "second");
    assert!(
        snapshots::check(&db, &fixture.0, now + Duration::days(1), None)
            .unwrap()
            .is_some()
    );

    add(&db, "third");
    snapshots::check(&db, &fixture.0, now + Duration::days(2), None)
        .unwrap()
        .unwrap();
    assert_eq!(snapshots::list(&fixture.0).unwrap().len(), 3);
    db.save_setting(snapshots::RETENTION_KEY, "1").unwrap();
    snapshots::enforce_retention(&db, &fixture.0, 1).unwrap();
    assert_eq!(snapshots::list(&fixture.0).unwrap().len(), 1);
}

#[test]
fn deletion_is_gated_locked_and_limited_to_the_selected_indexed_snapshot() {
    let fixture = Fixture::new();
    let (_session, db) = open_library(&fixture.0).unwrap();
    add(&db, "preserved");
    let snapshot = snapshots::check(&db, &fixture.0, Utc::now(), None)
        .unwrap()
        .unwrap();
    let manual = fixture.0.join("manual.pastedbackup");
    db.create_full_backup(&manual, None, None).unwrap();
    let (lease, source) = snapshots::restore_source(&fixture.0, &snapshot.id).unwrap();
    assert!(snapshots::delete(&db, &fixture.0, &snapshot.id).is_err());
    drop(lease);
    db.save_setting("enableSnapshots", "false").unwrap();
    assert!(snapshots::delete(&db, &fixture.0, &snapshot.id).is_err());
    db.save_setting("enableSnapshots", "true").unwrap();
    for invalid in ["../manual", "", "100-absent", "123-456"] {
        assert!(snapshots::delete(&db, &fixture.0, invalid).is_err());
    }
    assert!(source.is_file());
    // Damaged snapshots remain deletable; removal does not require a usable backup.
    fs::write(&source, b"damaged snapshot").unwrap();
    let sidecar = PathBuf::from(format!("{}-wal", source.display()));
    fs::write(&sidecar, b"stale sidecar").unwrap();
    db.save_setting(snapshots::RETENTION_KEY, "0").unwrap();
    snapshots::delete(&db, &fixture.0, &snapshot.id).unwrap();
    assert!(!source.exists());
    assert!(!sidecar.exists());
    assert!(!fixture
        .0
        .join("snapshots")
        .join(format!("{}.json", snapshot.id))
        .exists());
    assert!(snapshots::list(&fixture.0).unwrap().is_empty());
    assert!(snapshots::delete(&db, &fixture.0, &snapshot.id).is_err());
    assert!(manual.is_file());
    assert_eq!(db.get_clips(None, false).unwrap().len(), 1);
}

#[test]
fn retention_removes_only_unpublished_snapshot_artifacts() {
    let fixture = Fixture::new();
    let (_session, db) = open_library(&fixture.0).unwrap();
    add(&db, "preserved");
    let published = snapshots::check(&db, &fixture.0, Utc::now(), None)
        .unwrap()
        .unwrap();
    let directory = fixture.0.join("snapshots");
    let published_path = directory.join(format!("{}.pastedbackup", published.id));
    let published_sidecar = PathBuf::from(format!("{}-shm", published_path.display()));
    fs::write(&published_sidecar, b"stale sidecar").unwrap();
    add(&db, "newer");
    snapshots::create(&db, &fixture.0, Utc::now() + Duration::seconds(1), None).unwrap();
    fs::write(directory.join("orphan.pastedbackup"), b"partial").unwrap();
    fs::write(directory.join("orphan.json"), b"invalid").unwrap();
    fs::write(
        directory.join(".pasted-full-backup-interrupted.tmp"),
        b"partial",
    )
    .unwrap();

    snapshots::enforce_retention(&db, &fixture.0, 1).unwrap();

    assert_eq!(snapshots::list(&fixture.0).unwrap().len(), 1);
    assert!(!published_path.exists());
    assert!(!published_sidecar.exists());
    assert!(!directory.join("orphan.pastedbackup").exists());
    assert!(!directory.join("orphan.json").exists());
    assert!(!directory
        .join(".pasted-full-backup-interrupted.tmp")
        .exists());
}

#[test]
fn schedule_tracks_new_clips_and_manual_creation_works_while_automatic_is_paused() {
    let fixture = Fixture::new();
    let (_session, db) = open_library(&fixture.0).unwrap();
    let now = Utc.with_ymd_and_hms(2026, 9, 6, 12, 0, 0).unwrap();
    let empty = snapshots::schedule(&db, &fixture.0, now).unwrap();
    assert!(empty.waiting_for_new_clips);
    add(&db, "first");
    let due = snapshots::schedule(&db, &fixture.0, now).unwrap();
    assert_eq!(
        due.next_automatic_snapshot_at.as_deref(),
        Some("2026-09-06T12:00:00Z")
    );
    snapshots::create(&db, &fixture.0, now, Some("{\"view\":\"history\"}")).unwrap();
    assert!(
        snapshots::schedule(&db, &fixture.0, now)
            .unwrap()
            .waiting_for_new_clips
    );
    add(&db, "second");
    let next = snapshots::schedule(&db, &fixture.0, now).unwrap();
    assert_eq!(
        next.next_automatic_snapshot_at.as_deref(),
        Some("2026-09-06T13:00:00Z")
    );
    db.save_setting(snapshots::RETENTION_KEY, "0").unwrap();
    let manual = snapshots::create(&db, &fixture.0, now + Duration::minutes(1), None).unwrap();
    assert_eq!(manual.clip_count, 2);
    assert_eq!(snapshots::list(&fixture.0).unwrap().len(), 2);
    let paused = snapshots::schedule(&db, &fixture.0, now).unwrap();
    assert!(paused.next_automatic_snapshot_at.is_none());
    assert!(!paused.waiting_for_new_clips);
}

#[test]
fn exporting_a_snapshot_creates_a_verified_full_backup_without_changing_the_snapshot() {
    let fixture = Fixture::new();
    let (_session, db) = open_library(&fixture.0).unwrap();
    add(&db, "preserved");
    let snapshot = snapshots::create(&db, &fixture.0, Utc::now(), None).unwrap();
    let destination = fixture.0.join("Exported.pastedbackup");
    let report = snapshots::export(&db, &fixture.0, &snapshot.id, &destination).unwrap();
    assert_eq!(report.path, destination.to_string_lossy());
    assert_eq!(files::digest(&destination).unwrap(), snapshot.sha256);
    assert_eq!(snapshots::list(&fixture.0).unwrap().len(), 1);
    let inspection = db.inspect_full_backup(&destination).unwrap();
    assert_eq!(inspection.created_at, report.created_at);
}
