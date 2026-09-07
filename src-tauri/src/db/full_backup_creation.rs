use std::{
    fs,
    path::{Path, PathBuf},
};

use rusqlite::{params, OptionalExtension, Result};

use super::{
    open_pasted_database, open_pasted_database_read_only, validate_backup_json, DbState,
    FullBackupReport, FULL_BACKUP_FORMAT_VERSION,
};

struct TemporaryBackupGuard {
    path: PathBuf,
    armed: bool,
}

impl Drop for TemporaryBackupGuard {
    fn drop(&mut self) {
        if self.armed {
            crate::library_storage::remove_database_files(&self.path);
        }
    }
}

impl DbState {
    pub fn create_full_backup(
        &self,
        destination_path: &Path,
        client_state_json: Option<&str>,
        window_state_json: Option<&str>,
    ) -> Result<FullBackupReport> {
        if destination_path == self.database_path() {
            return Err(rusqlite::Error::InvalidPath(destination_path.to_path_buf()));
        }
        validate_backup_json(client_state_json, "Backup UI state")?;
        validate_backup_json(window_state_json, "Backup window state")?;
        let parent = destination_path
            .parent()
            .ok_or_else(|| rusqlite::Error::InvalidPath(destination_path.to_path_buf()))?;
        fs::create_dir_all(parent)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let temporary = parent.join(format!(
            ".pasted-full-backup-{}-{}.tmp",
            std::process::id(),
            chrono::Utc::now().timestamp_millis()
        ));
        if temporary.exists() {
            crate::library_storage::remove_database_files(&temporary);
        }
        let mut temporary_guard = TemporaryBackupGuard {
            path: temporary.clone(),
            armed: true,
        };

        let created_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        // SQLite's online backup API provides a consistent snapshot while writes
        // continue. Keep the application's shared connection free during the copy.
        let _ = self
            .conn
            .lock()
            .pragma_update(None, "wal_checkpoint", "PASSIVE");
        let source = open_pasted_database_read_only(&self.database_path())?;
        let mut destination = open_pasted_database(&temporary)?;
        {
            let backup = rusqlite::backup::Backup::new(&source, &mut destination)?;
            backup.run_to_completion(128, std::time::Duration::from_millis(5), None)?;
        }
        let effective_client_state = client_state_json.map(str::to_owned).or_else(|| {
            destination
                .query_row(
                    "SELECT value FROM settings WHERE key = 'backedUpClientState'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .ok()
                .flatten()
        });
        destination.execute_batch(
            "DROP TABLE IF EXISTS pasted_backup_manifest;
             CREATE TABLE pasted_backup_manifest (
                format_version INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                app_version TEXT NOT NULL,
                platform TEXT NOT NULL,
                client_state_json TEXT,
                window_state_json TEXT,
                external_state_notice TEXT NOT NULL
             );",
        )?;
        destination.execute(
            "INSERT INTO pasted_backup_manifest
                (format_version, created_at, app_version, platform, client_state_json,
                 window_state_json, external_state_notice)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                FULL_BACKUP_FORMAT_VERSION,
                created_at,
                env!("CARGO_PKG_VERSION"),
                std::env::consts::OS,
                effective_client_state,
                window_state_json,
                "Copied file clips contain paths to original files rather than copies of those files. Paths are preserved. API keys and passwords remain in their credential stores."
            ],
        )?;
        let _ = destination.pragma_update(None, "wal_checkpoint", "TRUNCATE");
        let integrity: String =
            destination.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if integrity != "ok" {
            return Err(rusqlite::Error::InvalidQuery);
        }
        drop(destination);
        if destination_path.exists() {
            fs::remove_file(destination_path)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        }
        fs::rename(&temporary, destination_path)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        temporary_guard.armed = false;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(destination_path, fs::Permissions::from_mode(0o600));
        }
        let size_bytes = fs::metadata(destination_path)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?
            .len();
        Ok(FullBackupReport {
            path: destination_path.to_string_lossy().into_owned(),
            created_at,
            size_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_publication_removes_temporary_database_files() {
        let root = std::env::temp_dir().join(format!(
            "pasted-backup-cleanup-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&root).unwrap();
        let db = DbState::new(root.join("pasted.db")).unwrap();
        let destination = root.join("occupied.pastedbackup");
        fs::create_dir(&destination).unwrap();

        assert!(db.create_full_backup(&destination, None, None).is_err());
        assert!(!fs::read_dir(&root).unwrap().flatten().any(|entry| entry
            .file_name()
            .to_string_lossy()
            .starts_with(".pasted-full-backup-")));
        drop(db);
        fs::remove_dir_all(root).unwrap();
    }
}
