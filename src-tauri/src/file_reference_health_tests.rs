use super::*;

#[test]
fn missing_references_are_persisted_and_explicitly_rechecked() {
    let root = std::env::temp_dir().join(format!(
        "pasted-file-health-{}-{}",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let db = DbState::new(root.join("library.db")).unwrap();
    let path = root.join("temporary.png");
    std::fs::write(&path, b"image").unwrap();
    let paths = vec![path.to_string_lossy().into_owned()];
    let clip = db
        .save_clip(
            "file",
            Some(&serde_json::to_string(&paths).unwrap()),
            None,
            None,
            "file-health-test",
            "Tests",
        )
        .unwrap();

    let available = resolve_file_reference_health(&db, clip.id, &paths, false, None).unwrap();
    assert_eq!(
        available[0].availability,
        FileReferenceAvailability::Available
    );
    std::fs::remove_file(&path).unwrap();
    let missing = resolve_file_reference_health(&db, clip.id, &paths, false, None).unwrap();
    assert_eq!(missing[0].availability, FileReferenceAvailability::Missing);

    let backup_path = root.join("file-health.pastedbackup");
    db.create_full_backup(&backup_path, None, None).unwrap();
    std::fs::write(&path, b"image restored").unwrap();
    let available = resolve_file_reference_health(&db, clip.id, &paths, true, None).unwrap();
    assert_eq!(
        available[0].availability,
        FileReferenceAvailability::Available
    );
    let (restore, _, _) = db.restore_full_backup(&backup_path, None, None).unwrap();
    let restored = resolve_file_reference_health(&db, clip.id, &paths, false, None).unwrap();
    assert_eq!(restored[0].availability, FileReferenceAvailability::Missing);
    let rechecked = resolve_file_reference_health(&db, clip.id, &paths, true, None).unwrap();
    assert_eq!(
        rechecked[0].availability,
        FileReferenceAvailability::Available
    );

    drop(db);
    let _ = std::fs::remove_file(restore.recovery_path);
    std::fs::remove_dir_all(root).unwrap();
}
