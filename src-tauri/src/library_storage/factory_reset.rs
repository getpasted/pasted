use super::{files, snapshots, LibrarySession};
use crate::db::{DbState, FactoryResetReport};
use std::fs;

pub fn factory_reset_library(
    db: &DbState,
    session: &LibrarySession,
) -> Result<FactoryResetReport, String> {
    session.exclusive(|| {
        let snapshot_count = snapshots::list(&session.app_data)?.len();
        let source = snapshots::directory(&session.app_data);
        let staged = session
            .app_data
            .join(format!(".snapshots-reset-{}", files::nonce()));
        let has_snapshots = source.exists();
        if has_snapshots {
            fs::rename(&source, &staged).map_err(|error| error.to_string())?;
        }

        let mut report = match db.factory_reset() {
            Ok(report) => report,
            Err(error) => {
                if has_snapshots {
                    fs::rename(&staged, &source).map_err(|rollback| {
                        format!("{error}. Automatic snapshots could not be restored: {rollback}")
                    })?;
                }
                return Err(error.to_string());
            }
        };
        report.snapshots_deleted = snapshot_count;
        if has_snapshots {
            // Publication is already committed. The staged directory is no longer
            // discoverable as Snapshots even if the filesystem defers its removal.
            let _ = fs::remove_dir_all(&staged);
        }
        Ok(report)
    })
}
