use super::super::*;

pub(crate) fn migrate_app_exclusion_hotkey_setting(conn: &Connection) -> Result<()> {
    if !table_exists(conn, "settings")? {
        return Ok(());
    }
    let stored: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'blacklistApps'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let Some(stored) = stored else {
        return Ok(());
    };
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&stored) else {
        return Ok(());
    };
    let Some(entries) = value.as_array_mut() else {
        return Ok(());
    };
    let mut changed = false;
    for entry in entries {
        let Some(rule) = entry.as_object_mut() else {
            continue;
        };
        let legacy = rule.remove("ignoreShortcuts");
        if let Some(legacy) = legacy {
            rule.entry("ignoreHotkeys").or_insert(legacy);
            changed = true;
        }
    }
    if changed {
        let serialized = serde_json::to_string(&value)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        conn.execute(
            "UPDATE settings SET value = ?1 WHERE key = 'blacklistApps'",
            params![serialized],
        )?;
    }
    Ok(())
}
