use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

use crate::db::DbState;
use rusqlite::{params, Connection};

#[derive(Clone, Debug)]
struct StoredFileReferenceHealth {
    index: usize,
    reference_hash: String,
    availability: String,
    checked_at: String,
}

struct FileReferenceHealthUpdate {
    index: usize,
    reference_hash: String,
    availability: FileReferenceAvailability,
    checked_at: String,
}

pub(crate) fn create_file_reference_health_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS clip_file_reference_health (
            clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
            file_index INTEGER NOT NULL,
            reference_hash TEXT NOT NULL,
            availability TEXT NOT NULL CHECK (
                availability IN ('available', 'missing', 'inaccessible', 'unavailable')
            ),
            checked_at TEXT NOT NULL,
            PRIMARY KEY (clip_id, file_index)
        );",
    )
}

impl DbState {
    fn get_file_reference_health(
        &self,
        clip_id: i64,
    ) -> rusqlite::Result<Vec<StoredFileReferenceHealth>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT file_index, reference_hash, availability, checked_at
             FROM clip_file_reference_health WHERE clip_id = ?1",
        )?;
        let health = statement
            .query_map([clip_id], |row| {
                let stored_index: i64 = row.get(0)?;
                Ok(StoredFileReferenceHealth {
                    index: usize::try_from(stored_index).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Integer,
                            Box::new(error),
                        )
                    })?,
                    reference_hash: row.get(1)?,
                    availability: row.get(2)?,
                    checked_at: row.get(3)?,
                })
            })?
            .collect();
        health
    }

    fn record_file_reference_health(
        &self,
        clip_id: i64,
        updates: &[FileReferenceHealthUpdate],
    ) -> rusqlite::Result<()> {
        let mut conn = self.conn.lock();
        let transaction = conn.transaction()?;
        for update in updates {
            let index = i64::try_from(update.index)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            transaction.execute(
                "INSERT INTO clip_file_reference_health
                    (clip_id, file_index, reference_hash, availability, checked_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(clip_id, file_index) DO UPDATE SET
                    reference_hash = excluded.reference_hash,
                    availability = excluded.availability,
                    checked_at = excluded.checked_at",
                params![
                    clip_id,
                    index,
                    update.reference_hash,
                    update.availability.as_str(),
                    update.checked_at
                ],
            )?;
        }
        transaction.commit()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileReferenceAvailability {
    Available,
    Missing,
    Inaccessible,
    Unavailable,
}

impl FileReferenceAvailability {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Missing => "missing",
            Self::Inaccessible => "inaccessible",
            Self::Unavailable => "unavailable",
        }
    }

    pub(crate) fn from_stored(value: &str) -> Option<Self> {
        match value {
            "available" => Some(Self::Available),
            "missing" => Some(Self::Missing),
            "inaccessible" => Some(Self::Inaccessible),
            "unavailable" => Some(Self::Unavailable),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileReferenceHealth {
    pub index: usize,
    pub availability: FileReferenceAvailability,
    pub checked_at: String,
}

fn reference_hash(path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"pasted-file-reference-v1\0");
    hasher.update(path.as_bytes());
    crate::hashing::finalize_sha256_hex(hasher)
}

fn retry_interval(availability: FileReferenceAvailability) -> Option<Duration> {
    match availability {
        FileReferenceAvailability::Available => None,
        FileReferenceAvailability::Missing => Some(Duration::hours(24)),
        FileReferenceAvailability::Inaccessible | FileReferenceAvailability::Unavailable => {
            Some(Duration::minutes(15))
        }
    }
}

fn reusable_health(
    stored: StoredFileReferenceHealth,
    now: DateTime<Utc>,
) -> Option<FileReferenceHealth> {
    let availability = FileReferenceAvailability::from_stored(&stored.availability)?;
    let retry_interval = retry_interval(availability)?;
    let checked_at = DateTime::parse_from_rfc3339(&stored.checked_at)
        .ok()?
        .with_timezone(&Utc);
    let age = now.signed_duration_since(checked_at);
    (age >= Duration::zero() && age < retry_interval).then_some(FileReferenceHealth {
        index: stored.index,
        availability,
        checked_at: stored.checked_at,
    })
}

fn inspect_reference(path: &Path) -> FileReferenceAvailability {
    match std::fs::symlink_metadata(path) {
        Ok(_) => FileReferenceAvailability::Available,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            FileReferenceAvailability::Missing
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            FileReferenceAvailability::Inaccessible
        }
        Err(_) => FileReferenceAvailability::Unavailable,
    }
}

pub fn resolve_file_reference_health(
    db: &DbState,
    clip_id: i64,
    paths: &[String],
    force_recheck: bool,
) -> rusqlite::Result<Vec<FileReferenceHealth>> {
    let now = Utc::now();
    let stored = db
        .get_file_reference_health(clip_id)?
        .into_iter()
        .map(|health| (health.index, health))
        .collect::<std::collections::HashMap<_, _>>();
    let checked_at = now.to_rfc3339_opts(SecondsFormat::Secs, true);
    let mut health = Vec::with_capacity(paths.len());
    let mut updates = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        let path_hash = reference_hash(path);
        if !force_recheck {
            if let Some(reusable) = stored.get(&index).and_then(|stored| {
                (stored.reference_hash == path_hash)
                    .then(|| stored.clone())
                    .and_then(|stored| reusable_health(stored, now))
            }) {
                health.push(reusable);
                continue;
            }
        }
        let availability = inspect_reference(Path::new(path));
        updates.push(FileReferenceHealthUpdate {
            index,
            reference_hash: path_hash,
            availability,
            checked_at: checked_at.clone(),
        });
        health.push(FileReferenceHealth {
            index,
            availability,
            checked_at: checked_at.clone(),
        });
    }
    if !updates.is_empty() {
        db.record_file_reference_health(clip_id, &updates)?;
    }
    Ok(health)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_references_are_persisted_and_explicitly_rechecked() {
        let root = std::env::temp_dir().join(format!(
            "pasted-file-health-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let db = DbState::new(root.join("library.db")).unwrap();
        let path = root.join("temporary.png");
        std::fs::write(&path, b"image").unwrap();
        let paths = vec![path.to_string_lossy().into_owned()];
        let clip = db
            .save_clip(
                "file",
                Some(&serde_json::to_string(&paths).unwrap()),
                None,
                None,
                "file-health-test",
                "Tests",
            )
            .unwrap();

        let available = resolve_file_reference_health(&db, clip.id, &paths, false).unwrap();
        assert_eq!(
            available[0].availability,
            FileReferenceAvailability::Available
        );
        std::fs::remove_file(&path).unwrap();
        let missing = resolve_file_reference_health(&db, clip.id, &paths, false).unwrap();
        assert_eq!(missing[0].availability, FileReferenceAvailability::Missing);

        let backup_path = root.join("file-health.pastedbackup");
        db.create_full_backup(&backup_path, None, None).unwrap();
        std::fs::write(&path, b"image restored").unwrap();
        let available = resolve_file_reference_health(&db, clip.id, &paths, true).unwrap();
        assert_eq!(
            available[0].availability,
            FileReferenceAvailability::Available
        );
        let (restore, _, _) = db.restore_full_backup(&backup_path, None, None).unwrap();
        let restored = resolve_file_reference_health(&db, clip.id, &paths, false).unwrap();
        assert_eq!(restored[0].availability, FileReferenceAvailability::Missing);
        let rechecked = resolve_file_reference_health(&db, clip.id, &paths, true).unwrap();
        assert_eq!(
            rechecked[0].availability,
            FileReferenceAvailability::Available
        );

        drop(db);
        let _ = std::fs::remove_file(restore.recovery_path);
        std::fs::remove_dir_all(root).unwrap();
    }
}
