use std::{fs, path::Path};

use rusqlite::{params, Connection, OptionalExtension, Result};

use super::{
    open_pasted_database, open_pasted_database_read_only, validate_backup_json, DbState,
    FullBackupInspection, FullBackupManifest, FullBackupReport, FullRestoreReport,
    FULL_BACKUP_FORMAT_VERSION, PENDING_CLIENT_STATE_SETTING,
};

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
            fs::remove_file(&temporary)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        }

        let created_at = chrono::Utc::now().to_rfc3339();
        let source = self.conn.lock();
        let _ = source.pragma_update(None, "wal_checkpoint", "PASSIVE");
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
            drop(destination);
            let _ = fs::remove_file(&temporary);
            return Err(rusqlite::Error::InvalidQuery);
        }
        drop(destination);
        drop(source);

        if destination_path.exists() {
            fs::remove_file(destination_path)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        }
        fs::rename(&temporary, destination_path)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(destination_path, fs::Permissions::from_mode(0o600))
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
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

    pub fn restore_full_backup(
        &self,
        backup_path: &Path,
        current_client_state_json: Option<&str>,
        current_window_state_json: Option<&str>,
    ) -> Result<(FullRestoreReport, Option<String>, Option<String>)> {
        let (source, manifest) = self.open_validated_full_backup(backup_path)?;

        let current_path = self.database_path();
        let parent = current_path
            .parent()
            .ok_or_else(|| rusqlite::Error::InvalidPath(current_path.clone()))?;
        let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S-%3f");
        let recovery_path = parent.join(format!("Pasted_Pre_Restore_{stamp}.pastedbackup"));
        self.create_full_backup(
            &recovery_path,
            current_client_state_json,
            current_window_state_json,
        )?;

        let temporary = parent.join(format!(
            ".pasted-full-restore-{}-{}.tmp",
            std::process::id(),
            chrono::Utc::now().timestamp_millis()
        ));
        let mut restored = open_pasted_database(&temporary)?;
        {
            let backup = rusqlite::backup::Backup::new(&source, &mut restored)?;
            backup.run_to_completion(128, std::time::Duration::from_millis(5), None)?;
        }
        drop(restored);
        drop(source);

        // Opening through DbState applies any forward migrations before the live
        // library is replaced. A failed migration leaves the current library intact.
        let migrated = DbState::new(temporary.clone())?;
        if let Some(client_state) = manifest.client_state_json.as_deref() {
            migrated.save_setting(PENDING_CLIENT_STATE_SETTING, client_state)?;
        }
        let migrated_integrity: String =
            migrated
                .conn
                .lock()
                .query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if migrated_integrity != "ok" {
            drop(migrated);
            let _ = fs::remove_file(&temporary);
            return Err(rusqlite::Error::InvalidQuery);
        }
        let _ = migrated
            .conn
            .lock()
            .pragma_update(None, "wal_checkpoint", "TRUNCATE");
        drop(migrated);

        let mut current = self.conn.lock();
        let _ = current.pragma_update(None, "wal_checkpoint", "TRUNCATE");
        let placeholder = Connection::open_in_memory()?;
        let previous = std::mem::replace(&mut *current, placeholder);
        drop(previous);
        crate::library_storage::remove_database_files(&current_path);
        let activate_result = fs::rename(&temporary, &current_path);
        if let Err(error) = activate_result {
            let _ = fs::copy(&recovery_path, &current_path);
            let replacement = open_pasted_database(&current_path)?;
            *current = replacement;
            return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(error)));
        }
        let replacement = match open_pasted_database(&current_path) {
            Ok(connection) => connection,
            Err(error) => {
                let _ = fs::copy(&recovery_path, &current_path);
                let fallback = open_pasted_database(&current_path)?;
                *current = fallback;
                return Err(error);
            }
        };
        *current = replacement;

        Ok((
            FullRestoreReport {
                recovery_path: recovery_path.to_string_lossy().into_owned(),
                backup_created_at: manifest.created_at,
            },
            manifest.client_state_json,
            manifest.window_state_json,
        ))
    }

    pub fn inspect_full_backup(&self, backup_path: &Path) -> Result<FullBackupInspection> {
        let (_source, manifest) = self.open_validated_full_backup(backup_path)?;
        let size_bytes = fs::metadata(backup_path)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?
            .len();
        Ok(FullBackupInspection {
            format_version: manifest.format_version,
            created_at: manifest.created_at,
            size_bytes,
        })
    }

    fn open_validated_full_backup(
        &self,
        backup_path: &Path,
    ) -> Result<(Connection, FullBackupManifest)> {
        if !backup_path.is_file() || backup_path == self.database_path() {
            return Err(rusqlite::Error::InvalidPath(backup_path.to_path_buf()));
        }
        let source = open_pasted_database_read_only(backup_path)?;
        let integrity: String = source.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if integrity != "ok" {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let manifest = source
            .query_row(
                "SELECT format_version, created_at, client_state_json, window_state_json
                 FROM pasted_backup_manifest LIMIT 1",
                [],
                |row| {
                    Ok(FullBackupManifest {
                        format_version: row.get(0)?,
                        created_at: row.get(1)?,
                        client_state_json: row.get(2)?,
                        window_state_json: row.get(3)?,
                    })
                },
            )
            .map_err(|_| {
                rusqlite::Error::InvalidParameterName(
                    "The selected file is not a complete Pasted backup".to_string(),
                )
            })?;
        if manifest.format_version != FULL_BACKUP_FORMAT_VERSION {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "Unsupported full-backup format version {}",
                manifest.format_version
            )));
        }
        validate_backup_json(manifest.client_state_json.as_deref(), "Backup UI state")?;
        validate_backup_json(manifest.window_state_json.as_deref(), "Backup window state")?;
        Ok((source, manifest))
    }

    pub fn consume_pending_full_restore_client_state(&self) -> Result<Option<String>> {
        let conn = self.conn.lock();
        let state = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![PENDING_CLIENT_STATE_SETTING],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if state.is_some() {
            conn.execute(
                "DELETE FROM settings WHERE key = ?1",
                params![PENDING_CLIENT_STATE_SETTING],
            )?;
        }
        Ok(state)
    }
}
