use super::*;

fn unique_test_directory(label: &str) -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("pasted-{label}-{}-{nonce}", std::process::id()))
}

#[test]
fn native_lock_transition_uses_the_shared_app_lock_state() {
    let root = unique_test_directory("hotkey-app-lock");
    std::fs::create_dir_all(&root).unwrap();
    let db = crate::db::DbState::new(root.join("pasted.db")).unwrap();
    crate::app_lock::configure(&db, "test passphrase").unwrap();
    let state = crate::app_lock::AppLockState::from_db(&db);
    state.unlock();

    let status = lock_app_state(&db, &state).unwrap();
    assert!(status.enabled);
    assert!(status.locked);
    assert!(state.is_locked());

    state.unlock();
    db.save_setting("enableAppLock", "false").unwrap();
    assert_eq!(
        lock_app_state(&db, &state).unwrap_err(),
        "App Lock is disabled in Settings → Functionality"
    );
    assert!(!state.is_locked());

    drop(db);
    std::fs::remove_dir_all(root).unwrap();
}
