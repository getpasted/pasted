use pasted_lib::db::DbState;
use pasted_lib::library_storage::{snapshots, LibrarySession};
use rusqlite::Result;

fn enforce(db: &DbState, session: &LibrarySession, keep: usize) -> Result<()> {
    if !pasted_lib::features::is_enabled(db, pasted_lib::features::Feature::Snapshots) {
        return Ok(());
    }
    session
        .stable(|| snapshots::enforce_retention(db, &session.app_data, keep))
        .map_err(rusqlite::Error::InvalidParameterName)
}

pub(super) fn after_set(
    db: &DbState,
    session: &LibrarySession,
    key: &str,
    value: &str,
) -> Result<()> {
    if key == snapshots::RETENTION_KEY {
        enforce(db, session, value.parse::<usize>().unwrap_or(24))?;
    }
    Ok(())
}

pub(super) fn after_reset(
    db: &DbState,
    session: &LibrarySession,
    dry_run: bool,
    changes: &[pasted_lib::settings_service::SettingChange],
) -> Result<()> {
    if dry_run
        || !changes
            .iter()
            .any(|change| change.key == snapshots::RETENTION_KEY)
    {
        return Ok(());
    }
    let keep = db
        .get_setting(snapshots::RETENTION_KEY)?
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(24);
    enforce(db, session, keep)
}
