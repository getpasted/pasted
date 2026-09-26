use super::DbState;
use crate::library_storage::{self, LibraryMoveReport, LibrarySession};
use std::path::Path;

impl DbState {
    pub fn move_library(
        &self,
        session: &LibrarySession,
        directory: &Path,
        return_to_default: bool,
    ) -> Result<LibraryMoveReport, String> {
        crate::features::require(self, crate::features::Feature::LibraryMove)?;
        self.move_library_with_commit_check(session, directory, return_to_default, || Ok(()))
    }

    pub(crate) fn move_library_with_commit_check(
        &self,
        session: &LibrarySession,
        directory: &Path,
        return_to_default: bool,
        before_commit: impl FnMut() -> Result<(), String>,
    ) -> Result<LibraryMoveReport, String> {
        session.exclusive(|| {
            // Hold this across snapshot, location persistence, and activation.
            // Captures waiting on the connection then commit to the new library.
            let mut source = self.conn.lock();
            let current = self.database_path();
            if session.database_path()? != current {
                return Err(
                    "The library location changed in another process; reopen Pasted".into(),
                );
            }
            let target = if return_to_default {
                library_storage::default_database_path(&session.app_data)
            } else {
                library_storage::validate_destination_directory(directory, &current)?
            };
            if target == current {
                return Ok(LibraryMoveReport {
                    location: library_storage::location_info(&session.app_data, &current),
                    recovery_path: current.to_string_lossy().into_owned(),
                });
            }
            if return_to_default {
                library_storage::archive_existing_database(&target)?;
            }
            let (replacement, report) = library_storage::transfer::transfer(
                &source,
                &session.app_data,
                &current,
                &target,
                None,
                before_commit,
            )?;
            *source = replacement;
            *self.path.lock() = target;
            let _ =
                self.log_activity_internal(&source, "library_moved", "Moved the database location");
            Ok(report)
        })
    }
}
