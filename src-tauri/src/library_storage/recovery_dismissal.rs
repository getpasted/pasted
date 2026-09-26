use super::{files, recovery_notice, LibraryRecoveryNotice};
use std::path::{Path, PathBuf};

// Presentation state is separate from the recovery record: dismissing a note
// must never change the active location or the files needed for recovery.
pub fn visible_recovery_notice(app_data: &Path) -> Option<LibraryRecoveryNotice> {
    let notice = recovery_notice(app_data)?;
    let dismissed: Option<(String, PathBuf)> =
        files::read_json(&app_data.join("library-recovery-dismissed.json")).ok();
    if dismissed
        .as_ref()
        .is_some_and(|(date, path)| date == &notice.occurred_at && path == &notice.preserved_path)
    {
        None
    } else {
        Some(notice)
    }
}

pub fn dismiss_recovery_notice(
    app_data: &Path,
    occurred_at: &str,
    preserved_path: &Path,
) -> Result<(), String> {
    let Some(notice) = recovery_notice(app_data) else {
        return Ok(());
    };
    // A stale window may dismiss its own note, but never a later recovery.
    if notice.occurred_at != occurred_at || notice.preserved_path != preserved_path {
        return Ok(());
    }
    files::write_json(
        &app_data.join("library-recovery-dismissed.json"),
        &(notice.occurred_at, notice.preserved_path),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library_storage::{default_database_path, location};

    #[test]
    fn dismissal_persists_without_hiding_future_recoveries_or_changing_location() {
        let root = std::env::temp_dir().join(format!("pasted-dismiss-{}", files::nonce()));
        std::fs::create_dir_all(&root).unwrap();
        let mut notice = LibraryRecoveryNotice {
            outcome: "fresh".into(),
            occurred_at: "2026-09-06T12:00:00Z".into(),
            previous_path: None,
            recovery_created_at: None,
            preserved_path: root.join("first"),
        };
        let target = default_database_path(&root);
        location::write_location_with_notice(&root, &target, None, Some(notice.clone())).unwrap();
        let pointer = std::fs::read(root.join("library-location.json")).unwrap();
        dismiss_recovery_notice(&root, &notice.occurred_at, &notice.preserved_path).unwrap();
        assert!(visible_recovery_notice(&root).is_none());
        assert!(recovery_notice(&root).is_some());
        assert_eq!(
            std::fs::read(root.join("library-location.json")).unwrap(),
            pointer
        );
        let previous = notice.clone();
        notice.preserved_path = root.join("second");
        location::write_location_with_notice(&root, &target, None, Some(notice)).unwrap();
        dismiss_recovery_notice(&root, &previous.occurred_at, &previous.preserved_path).unwrap();
        assert!(visible_recovery_notice(&root).is_some());
        std::fs::remove_dir_all(root).unwrap();
    }
}
