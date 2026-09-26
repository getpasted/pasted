use super::retention::{prune_locked, retention_count};
use super::{
    files, lease, list, path, remove_unpublished_locked, Snapshot, SnapshotPublishGuard,
    BASELINE_KEY, INTERVAL_KEY,
};
use crate::db::DbState;
use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSchedule {
    pub next_automatic_snapshot_at: Option<String>,
    pub waiting_for_new_clips: bool,
}

fn clip_hashes(connection: &rusqlite::Connection) -> Result<HashSet<String>, String> {
    let mut statement = connection
        .prepare("SELECT content_hash FROM clips")
        .map_err(|error| error.to_string())?;
    let values = statement
        .query_map([], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    values
        .collect::<rusqlite::Result<HashSet<String>>>()
        .map_err(|error| error.to_string())
}

fn max_clip_id(db: &DbState) -> Result<i64, String> {
    db.conn
        .lock()
        .query_row("SELECT COALESCE(MAX(id), 0) FROM clips", [], |row| {
            row.get(0)
        })
        .map_err(|error| error.to_string())
}

pub fn schedule(
    db: &DbState,
    app_data: &Path,
    now: DateTime<Utc>,
) -> Result<SnapshotSchedule, String> {
    let keep = retention_count(db)?;
    if keep == 0 || !crate::features::is_enabled(db, crate::features::Feature::Snapshots) {
        return Ok(SnapshotSchedule {
            next_automatic_snapshot_at: None,
            waiting_for_new_clips: false,
        });
    }
    let current_max_clip_id = max_clip_id(db)?;
    let baseline = db
        .get_setting(BASELINE_KEY)
        .map_err(|error| error.to_string())?
        .and_then(|value| value.parse::<i64>().ok());
    if current_max_clip_id == 0 || baseline.is_some_and(|value| current_max_clip_id <= value) {
        return Ok(SnapshotSchedule {
            next_automatic_snapshot_at: None,
            waiting_for_new_clips: true,
        });
    }
    let interval = db
        .get_setting(INTERVAL_KEY)
        .map_err(|error| error.to_string())?
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| (1..=10080).contains(value))
        .unwrap_or(1440);
    let due = list(app_data)?
        .first()
        .map(|latest| DateTime::parse_from_rfc3339(&latest.created_at))
        .transpose()
        .map_err(|error| error.to_string())?
        .map(|created| created.with_timezone(&Utc) + chrono::Duration::minutes(interval))
        .unwrap_or(now);
    Ok(SnapshotSchedule {
        next_automatic_snapshot_at: Some(due.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
        waiting_for_new_clips: false,
    })
}

fn create_locked(
    db: &DbState,
    app_data: &Path,
    now: DateTime<Utc>,
    window_state: Option<&str>,
    keep: usize,
) -> Result<Snapshot, String> {
    let current_max_clip_id = max_clip_id(db)?;
    let id = files::nonce();
    let target = path(app_data, &id)?;
    db.create_full_backup(&target, None, window_state)
        .map_err(|error| error.to_string())?;
    let metadata_path = super::directory(app_data).join(format!("{id}.json"));
    let mut publication = SnapshotPublishGuard {
        database: target.clone(),
        metadata: metadata_path.clone(),
        armed: true,
    };
    let sealed = rusqlite::Connection::open(&target).map_err(|error| error.to_string())?;
    sealed
        .execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![BASELINE_KEY, current_max_clip_id.to_string()],
        )
        .map_err(|error| error.to_string())?;
    sealed
        .pragma_update(None, "journal_mode", "DELETE")
        .map_err(|error| error.to_string())?;
    drop(sealed);
    fs::OpenOptions::new()
        .write(true)
        .open(&target)
        .and_then(|file| file.sync_all())
        .map_err(|error| error.to_string())?;
    let source =
        rusqlite::Connection::open_with_flags(&target, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|error| error.to_string())?;
    let clip_count: i64 = source
        .query_row("SELECT COUNT(*) FROM clips", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    drop(source);
    let snapshot = Snapshot {
        id,
        created_at: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        clip_count: clip_count as u64,
        size_bytes: fs::metadata(&target)
            .map_err(|error| error.to_string())?
            .len(),
        sha256: files::digest(&target)?,
    };
    files::write_json(&metadata_path, &snapshot)?;
    db.save_setting(BASELINE_KEY, &current_max_clip_id.to_string())
        .map_err(|error| error.to_string())?;
    publication.armed = false;
    prune_locked(app_data, keep)?;
    Ok(snapshot)
}

pub fn create(
    db: &DbState,
    app_data: &Path,
    now: DateTime<Utc>,
    window_state: Option<&str>,
) -> Result<Snapshot, String> {
    crate::features::require(db, crate::features::Feature::Snapshots)?;
    let keep = retention_count(db)?;
    let _lease = lease(app_data)?;
    remove_unpublished_locked(app_data)?;
    create_locked(db, app_data, now, window_state, keep)
}

/// Called by the live app on a short polling cadence; elapsed time and new-clip
/// checks are shared with the CLI and deterministic tests.
pub fn check(
    db: &DbState,
    app_data: &Path,
    now: DateTime<Utc>,
    window_state: Option<&str>,
) -> Result<Option<Snapshot>, String> {
    if !crate::features::is_enabled(db, crate::features::Feature::Snapshots) {
        return Ok(None);
    }
    let keep = retention_count(db)?;
    if keep == 0 {
        return Ok(None);
    }
    let interval = db
        .get_setting(INTERVAL_KEY)
        .map_err(|error| error.to_string())?
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| (1..=10080).contains(value))
        .unwrap_or(1440);
    let _lease = lease(app_data)?;
    remove_unpublished_locked(app_data)?;
    let snapshots = list(app_data)?;
    prune_locked(app_data, keep)?;
    if let Some(latest) = snapshots.first() {
        let previous =
            DateTime::parse_from_rfc3339(&latest.created_at).map_err(|error| error.to_string())?;
        if now.signed_duration_since(previous).num_seconds() < interval * 60 {
            return Ok(None);
        }
    }
    let current_max_clip_id = max_clip_id(db)?;
    if current_max_clip_id == 0 {
        return Ok(None);
    }
    let baseline = db
        .get_setting(BASELINE_KEY)
        .map_err(|error| error.to_string())?
        .and_then(|value| value.parse::<i64>().ok());
    if baseline.is_some_and(|value| current_max_clip_id <= value) {
        return Ok(None);
    }
    if baseline.is_none() {
        let current = clip_hashes(&db.conn.lock())?;
        for snapshot in &snapshots {
            let source = path(app_data, &snapshot.id)?;
            if files::digest(&source).ok().as_deref() != Some(snapshot.sha256.as_str()) {
                continue;
            }
            let previous = rusqlite::Connection::open_with_flags(
                source,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .map_err(|error| error.to_string())?;
            if current.is_subset(&clip_hashes(&previous)?) {
                db.save_setting(BASELINE_KEY, &current_max_clip_id.to_string())
                    .map_err(|error| error.to_string())?;
                return Ok(None);
            }
            break;
        }
        if snapshots.is_empty() && current.is_empty() {
            return Ok(None);
        }
    }
    create_locked(db, app_data, now, window_state, keep).map(Some)
}
