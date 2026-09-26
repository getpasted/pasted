use super::{files, location, LibraryRecoveryNotice, RecoveryCopy};
use crate::db::DbState;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize)]
struct Activation {
    #[serde(default)]
    published: bool,
    prepared: RecoveryCopy,
    recovery: Option<RecoveryCopy>,
    notice: LibraryRecoveryNotice,
}

pub(super) fn activate(
    app_data: &Path,
    staged: &Path,
    recovery: Option<RecoveryCopy>,
    notice: LibraryRecoveryNotice,
) -> Result<DbState, String> {
    fs::OpenOptions::new()
        .write(true)
        .open(staged)
        .and_then(|file| file.sync_all())
        .map_err(|error| error.to_string())?;
    let plan = Activation {
        published: false,
        prepared: RecoveryCopy {
            path: staged.into(),
            sha256: files::digest(staged)?,
            created_at: notice.occurred_at.clone(),
        },
        recovery,
        notice,
    };
    files::sync_directory(staged.parent().ok_or("Prepared library has no folder")?)?;
    files::write_json(&app_data.join("library-startup.json"), &plan)?;
    resume(app_data)
}

pub(super) fn resume(app_data: &Path) -> Result<DbState, String> {
    let journal = app_data.join("library-startup.json");
    let mut plan: Activation = files::read_json(&journal)?;
    let target = super::default_database_path(app_data);
    if location::read_location(app_data)
        .ok()
        .flatten()
        .is_some_and(|record| {
            record.directory.join("pasted.db") == target
                && record.notice.is_some_and(|notice| {
                    notice.occurred_at == plan.notice.occurred_at
                        && notice.preserved_path == plan.notice.preserved_path
                })
        })
    {
        // Captures may have arrived after activation. Never replay the earlier
        // snapshot over an already committed library.
        let opened = DbState::open_existing(target).map_err(|error| error.to_string());
        // A committed journal has finished its job, even if the library has
        // subsequently become unavailable. Let normal recovery handle that.
        fs::remove_file(journal).map_err(|error| error.to_string())?;
        return opened;
    }
    if !valid_prepared(app_data, &plan) {
        return Err("The prepared recovery library could not be verified".into());
    }
    if !plan.published || !target.exists() {
        preserve_default(&target, &plan.notice.preserved_path)?;
        let mut destination = files::private_file(&target)?;
        std::io::copy(
            &mut File::open(&plan.prepared.path).map_err(|error| error.to_string())?,
            &mut destination,
        )
        .map_err(|error| error.to_string())?;
        destination.sync_all().map_err(|error| error.to_string())?;
        drop(destination);
        files::sync_directory(app_data)?;
        plan.published = true;
        files::write_json(&journal, &plan)?;
    }
    let db = DbState::open_existing(target.clone()).map_err(|error| error.to_string())?;
    let pointer = app_data.join("library-location.json");
    if fs::symlink_metadata(&pointer).is_ok_and(|metadata| !metadata.is_file()) {
        fs::rename(
            &pointer,
            plan.notice
                .preserved_path
                .join(format!("location-record-{}", files::nonce())),
        )
        .map_err(|error| error.to_string())?;
    }
    location::write_location_with_notice(app_data, &target, plan.recovery, Some(plan.notice))?;
    // If removal fails, resume recognizes the committed notice next time.
    let _ = fs::remove_file(journal);
    Ok(db)
}

fn preserve_default(target: &Path, preserved: &Path) -> Result<(), String> {
    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
    let basename = format!("pasted-{}", files::nonce());
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let source = PathBuf::from(format!("{}{suffix}", target.display()));
        match fs::symlink_metadata(&source) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.to_string()),
            Ok(_) => {}
        }
        let destination = preserved.join(format!("{basename}.db{suffix}"));
        if let Err(error) = fs::rename(&source, &destination) {
            for (source, destination) in moved.iter().rev() {
                let _ = fs::rename(destination, source);
            }
            return Err(error.to_string());
        }
        moved.push((source, destination));
    }
    files::sync_directory(preserved)
}

fn valid_prepared(app_data: &Path, plan: &Activation) -> bool {
    let Ok(root) = fs::canonicalize(app_data.join("library-recovery")) else {
        return false;
    };
    let Ok(parent) = fs::canonicalize(&plan.notice.preserved_path) else {
        return false;
    };
    plan.prepared.path.parent() == Some(plan.notice.preserved_path.as_path())
        && parent.starts_with(root)
        && super::startup::verified_recovery(&plan.prepared)
}

pub(super) fn retryable(app_data: &Path) -> bool {
    files::read_json::<Activation>(&app_data.join("library-startup.json"))
        .is_ok_and(|plan| valid_prepared(app_data, &plan))
}
