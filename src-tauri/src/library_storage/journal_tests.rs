use super::*;

#[test]
fn startup_settles_each_interrupted_publish_state() {
    for phase in ["staged", "published", "committed", "changed"] {
        let root = std::env::temp_dir().join(format!("pasted-journal-{}", files::nonce()));
        fs::create_dir_all(root.join("destination")).unwrap();
        let root = fs::canonicalize(root).unwrap();
        let (session, db) = open_library(&root).unwrap();
        db.save_setting("authority", "original").unwrap();
        let original = db.database_path();
        drop(db);
        drop(session);
        let staging = root.join("destination/.pasted-library-interrupted.tmp");
        fs::copy(&original, &staging).unwrap();
        let digest = files::digest(&staging).unwrap();
        let target = root.join("destination/pasted.db");
        if phase != "staged" {
            fs::rename(&staging, &target).unwrap();
        }
        let recovery_path = root.join("retained.db");
        fs::copy(&original, &recovery_path).unwrap();
        files::write_json(
            &root.join("library-move.json"),
            &serde_json::json!({
                "source": original, "target": target, "staging": staging,
                "sha256": digest, "recovery_path": recovery_path,
            }),
        )
        .unwrap();
        if phase == "committed" {
            location::write_location(
                &root,
                &target,
                Some(RecoveryCopy {
                    path: recovery_path,
                    sha256: digest,
                    created_at: "2026-09-06T07:00:00Z".into(),
                }),
            )
            .unwrap();
            let db = crate::db::DbState::open_existing(target.clone()).unwrap();
            db.save_setting("authority", "committed").unwrap();
        } else if phase == "changed" {
            fs::write(&target, b"externally changed").unwrap();
        }
        let opened = open_library(&root);
        if phase == "changed" {
            assert!(opened.is_err());
            assert_eq!(fs::read(&target).unwrap(), b"externally changed");
            assert!(root.join("library-move.json").exists());
        } else {
            let (session, db) = opened.unwrap();
            assert!(!root.join("library-move.json").exists());
            assert!(!staging.exists());
            let expected = if phase == "committed" {
                "committed"
            } else {
                "original"
            };
            assert_eq!(
                db.get_setting("authority").unwrap().as_deref(),
                Some(expected)
            );
            assert_eq!(target.exists(), phase == "committed");
            drop(db);
            drop(session);
        }
        fs::remove_dir_all(root).unwrap();
    }
}
