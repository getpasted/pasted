use crate::db::DbState;
use std::collections::HashMap;

const KEYS: [&str; 6] = [
    "enableLibraryMove",
    "enableBackups",
    "enableSnapshots",
    "enableFactoryReset",
    "snapshotIntervalMinutes",
    "snapshotKeepCount",
];

/// Observe persisted changes from other processes without overwriting optimistic
/// GUI edits with repeated reads of unchanged database values.
#[derive(Default)]
pub struct StorageSettingsWatcher(HashMap<&'static str, String>);

impl StorageSettingsWatcher {
    pub fn poll(&mut self, db: &DbState) -> Vec<(&'static str, String)> {
        let mut changes = Vec::new();
        for key in KEYS {
            let Ok(value) = db.get_setting(key) else {
                continue;
            };
            let Some(value) = value.or_else(|| crate::settings_contract::default_value(key)) else {
                continue;
            };
            if self
                .0
                .insert(key, value.clone())
                .is_some_and(|old| old != value)
            {
                changes.push((key, value));
            }
        }
        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_storage_changes_emit_once_and_preserve_unrelated_preferences() {
        let root = std::env::temp_dir().join(format!(
            "pasted-settings-watch-{}",
            super::super::files::nonce()
        ));
        let (session, db) = super::super::open_library(&root).unwrap();
        let external = DbState::open_existing(db.database_path()).unwrap();
        let mut watcher = StorageSettingsWatcher::default();
        assert!(watcher.poll(&db).is_empty());
        external.save_setting("enableBackups", "false").unwrap();
        external
            .save_setting("snapshotIntervalMinutes", "15")
            .unwrap();
        external.save_setting("enableNotes", "false").unwrap();
        assert_eq!(
            watcher.poll(&db),
            vec![
                ("enableBackups", "false".into()),
                ("snapshotIntervalMinutes", "15".into())
            ]
        );
        assert!(watcher.poll(&db).is_empty());
        external
            .conn
            .lock()
            .execute("DELETE FROM settings WHERE key = 'enableBackups'", [])
            .unwrap();
        assert_eq!(watcher.poll(&db), vec![("enableBackups", "true".into())]);
        drop(external);
        drop(db);
        drop(session);
        std::fs::remove_dir_all(root).unwrap();
    }
}
