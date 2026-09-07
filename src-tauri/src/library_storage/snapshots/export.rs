use super::{files, list, restore_source};
use crate::db::DbState;
use std::fs::{self, File};
use std::path::Path;

pub fn export(
    db: &DbState,
    app_data: &Path,
    id: &str,
    destination: &Path,
) -> Result<crate::db::FullBackupReport, String> {
    crate::features::require(db, crate::features::Feature::Snapshots)?;
    crate::features::require(db, crate::features::Feature::Backups)?;
    let (_lease, source) = restore_source(app_data, id)?;
    let inspection = db
        .inspect_full_backup(&source)
        .map_err(|error| error.to_string())?;
    let snapshot = list(app_data)?
        .into_iter()
        .find(|snapshot| snapshot.id == id)
        .ok_or("Snapshot is no longer available")?;
    let parent = destination
        .parent()
        .ok_or("The selected backup location has no parent folder")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = parent.join(format!(".pasted-snapshot-export-{}.tmp", files::nonce()));
    let result: Result<crate::db::FullBackupReport, String> = (|| {
        fs::copy(&source, &temporary).map_err(|error| error.to_string())?;
        File::open(&temporary)
            .and_then(|file| file.sync_all())
            .map_err(|error| error.to_string())?;
        if files::digest(&temporary)? != snapshot.sha256 {
            return Err("Exported snapshot verification failed".into());
        }
        if destination.exists() {
            fs::remove_file(destination).map_err(|error| error.to_string())?;
        }
        fs::rename(&temporary, destination).map_err(|error| error.to_string())?;
        files::sync_directory(parent)?;
        Ok(crate::db::FullBackupReport {
            path: destination.to_string_lossy().into_owned(),
            created_at: inspection.created_at,
            size_bytes: snapshot.size_bytes,
        })
    })();
    let _ = fs::remove_file(temporary);
    let report = result?;
    let _ = db.log_activity(
        "backup_created",
        "Exported a Snapshot as a complete recovery backup",
    );
    Ok(report)
}
