use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection, OptionalExtension, Result};

use super::{ClipItem, DbState};

pub const MAX_CLIP_NAME_BYTES: usize = 512;
pub const MAX_CLIP_NAME_CHARS: usize = 120;

pub const fn clip_name_input_limit() -> usize {
    MAX_CLIP_NAME_BYTES
}

fn invalid_name(message: &str) -> rusqlite::Error {
    rusqlite::Error::InvalidParameterName(message.to_string())
}

pub(super) fn normalize_clip_name(name: Option<&str>) -> Result<Option<String>> {
    let trimmed = name.map(str::trim).filter(|value| !value.is_empty());
    let Some(value) = trimmed else {
        return Ok(None);
    };
    if value.len() > MAX_CLIP_NAME_BYTES || value.chars().count() > MAX_CLIP_NAME_CHARS {
        return Err(invalid_name("Clip name exceeds its safety limit"));
    }
    if value.chars().any(char::is_control) {
        return Err(invalid_name("Clip name must be a single line of text"));
    }
    Ok(Some(value.to_string()))
}

pub(super) fn append_clip_names(conn: &Connection, clips: &mut [ClipItem]) -> Result<()> {
    if clips.is_empty() {
        return Ok(());
    }
    let requested_ids = clips.iter().map(|clip| clip.id).collect::<HashSet<_>>();
    let ids_json = serde_json::to_string(&requested_ids.iter().copied().collect::<Vec<_>>())
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    let mut statement = conn.prepare(
        "SELECT id, name FROM clips
         WHERE id IN (SELECT CAST(value AS INTEGER) FROM json_each(?1))",
    )?;
    let mut names = statement
        .query_map([ids_json], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))
        })?
        .collect::<Result<HashMap<_, _>>>()?;
    for clip in clips {
        clip.name = names.remove(&clip.id).flatten();
    }
    Ok(())
}

impl DbState {
    pub fn update_clip_name(&self, clip_id: i64, name: Option<&str>) -> Result<ClipItem> {
        let name = normalize_clip_name(name)?;
        {
            let conn = self.conn.lock();
            let trashed = conn
                .query_row(
                    "SELECT COALESCE(is_trashed, 0) FROM clips WHERE id = ?1",
                    [clip_id],
                    |row| row.get::<_, bool>(0),
                )
                .optional()?;
            match trashed {
                None => return Err(rusqlite::Error::QueryReturnedNoRows),
                Some(true) => return Err(invalid_name("Cannot name a trashed clip")),
                Some(false) => {}
            }
            conn.execute(
                "UPDATE clips SET name = ?1
                 WHERE id = ?2 AND COALESCE(is_trashed, 0) = 0",
                params![name, clip_id],
            )?;
            let description = if name.is_some() {
                format!("Named clip #{clip_id}")
            } else {
                format!("Cleared the name from clip #{clip_id}")
            };
            let _ = self.log_activity_internal(&conn, "clip_name_updated", &description);
        }
        self.get_clip_by_id(clip_id)
    }
}
