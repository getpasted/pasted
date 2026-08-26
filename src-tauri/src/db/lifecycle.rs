use super::*;

fn configure_connection(conn: &Connection) -> Result<()> {
    conn.set_db_config(rusqlite::config::DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    let _ = conn.pragma_update(None, "synchronous", "NORMAL");
    let _ = conn.pragma_update(None, "temp_store", "MEMORY");
    let _ = conn.pragma_update(None, "wal_autocheckpoint", "500");
    Ok(())
}

/// Opens a Pasted-owned SQLite database and applies the shared connection policy.
/// Keep keying or storage-engine setup here so the GUI, CLI, backup, restore, and
/// library relocation paths cannot silently diverge.
pub fn open_pasted_database(path: &Path) -> Result<Connection> {
    let connection = Connection::open(path)?;
    configure_connection(&connection)?;
    Ok(connection)
}

pub(super) fn open_pasted_database_read_only(path: &Path) -> Result<Connection> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    connection.set_db_config(rusqlite::config::DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true)?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    let _ = connection.pragma_update(None, "temp_store", "MEMORY");
    Ok(connection)
}

impl DbState {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let conn = open_pasted_database(&db_path)?;
        let state = DbState {
            conn: Mutex::new(conn),
            path: Mutex::new(db_path),
        };
        state.init_tables()?;
        Ok(state)
    }

    pub fn database_path(&self) -> PathBuf {
        self.path.lock().clone()
    }

    pub fn relocate_database(&self, target_path: PathBuf) -> Result<PathBuf> {
        let previous_path = self.database_path();
        if previous_path == target_path {
            return Ok(previous_path);
        }
        if target_path.exists() {
            return Err(rusqlite::Error::InvalidPath(target_path));
        }
        let parent = target_path
            .parent()
            .ok_or_else(|| rusqlite::Error::InvalidPath(target_path.clone()))?;
        fs::create_dir_all(parent).map_err(|_| rusqlite::Error::InvalidPath(parent.into()))?;
        let temporary = parent.join(format!(".pasted-library-{}.tmp", std::process::id()));
        if temporary.exists() {
            fs::remove_file(&temporary)
                .map_err(|_| rusqlite::Error::InvalidPath(temporary.clone()))?;
        }

        let mut source = self.conn.lock();
        let _ = source.pragma_update(None, "wal_checkpoint", "TRUNCATE");
        let mut destination = open_pasted_database(&temporary)?;
        {
            let backup = rusqlite::backup::Backup::new(&source, &mut destination)?;
            backup.run_to_completion(128, std::time::Duration::from_millis(5), None)?;
        }
        let integrity: String =
            destination.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if integrity != "ok" {
            let _ = fs::remove_file(&temporary);
            return Err(rusqlite::Error::InvalidQuery);
        }
        drop(destination);
        fs::rename(&temporary, &target_path)
            .map_err(|_| rusqlite::Error::InvalidPath(target_path.clone()))?;
        let replacement = open_pasted_database(&target_path)?;
        *source = replacement;
        *self.path.lock() = target_path;
        Ok(previous_path)
    }

    pub fn switch_to_database(&self, database_path: PathBuf) -> Result<()> {
        let replacement = open_pasted_database(&database_path)?;
        let integrity: String =
            replacement.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if integrity != "ok" {
            return Err(rusqlite::Error::InvalidQuery);
        }
        *self.conn.lock() = replacement;
        *self.path.lock() = database_path;
        Ok(())
    }

    /// Removes all user-owned application state while preserving the initialized schema.
    /// The transaction recreates the starter Bins so every caller observes a valid,
    /// first-launch database immediately after it commits.
    pub fn factory_reset(&self) -> Result<FactoryResetReport> {
        let mut conn = self.conn.lock();
        let transaction = conn.transaction()?;
        let report = FactoryResetReport {
            clips_deleted: transaction.query_row("SELECT COUNT(*) FROM clips", [], sqlite_count)?,
            bins_deleted: transaction.query_row("SELECT COUNT(*) FROM bins", [], sqlite_count)?,
            transforms_deleted: transaction.query_row(
                "SELECT (SELECT COUNT(*) FROM saved_transforms) + (SELECT COUNT(*) FROM custom_operations)",
                [], sqlite_count,
            )?,
            connections_deleted: transaction.query_row(
                "SELECT COUNT(*) FROM intelligence_connections", [], sqlite_count,
            )?,
            activity_entries_deleted: transaction.query_row(
                "SELECT COUNT(*) FROM activity_logs", [], sqlite_count,
            )?,
        };
        transaction.execute_batch(
            "DELETE FROM automation_conditions;
             DELETE FROM automations;
             DELETE FROM clip_transformations;
             DELETE FROM transformation_executions;
             DELETE FROM saved_transforms;
             DELETE FROM custom_operations;
             DELETE FROM intelligence_connections;
             DELETE FROM clip_versions;
             DELETE FROM clip_bins;
             DELETE FROM clips;
             DELETE FROM bins;
             DELETE FROM activity_logs;
             DELETE FROM search_history;
             DELETE FROM extractor_authoring_messages;
             DELETE FROM extractor_recipe_revisions;
             DELETE FROM extractor_authoring_sessions;
             DELETE FROM content_extractors;
             DELETE FROM content_classifiers;
             DELETE FROM content_types;
             DELETE FROM content_type_groups;
             DELETE FROM settings;",
        )?;
        transaction.execute(
            "DELETE FROM sqlite_sequence WHERE name IN (
                'clips', 'bins', 'clip_versions', 'activity_logs', 'custom_operations',
                'search_history', 'saved_transforms', 'automations', 'intelligence_connections',
                'extractor_authoring_sessions', 'extractor_authoring_messages',
                'extractor_recipe_revisions'
            )",
            [],
        )?;
        super::factory_reset::restore_factory_defaults(&transaction)?;
        transaction.commit()?;
        let _ = conn.pragma_update(None, "optimize", "");
        Ok(report)
    }
}
