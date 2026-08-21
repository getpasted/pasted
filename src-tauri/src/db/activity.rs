use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

use super::{canonical_utc_timestamp, ensure_resource_size, DbState};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActivityLog {
    pub id: i64,
    pub event_type: String,
    pub description: String,
    pub created_at: String,
    pub observed_at: String,
    pub severity_text: String,
    pub category: String,
    pub outcome: String,
    pub attributes: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActivityArchiveEntry {
    pub timestamp: String,
    pub observed_timestamp: String,
    pub event_name: String,
    pub severity_text: String,
    pub body: String,
    pub attributes: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityArchive {
    pub schema_version: u32,
    pub exported_at: String,
    pub resource: serde_json::Map<String, serde_json::Value>,
    pub entries: Vec<ActivityArchiveEntry>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActivityImportReport {
    pub scanned_count: usize,
    pub imported_count: usize,
    pub duplicate_count: usize,
    pub retained_count: usize,
}

fn activity_classification(event_name: &str) -> (&'static str, &'static str, &'static str) {
    let severity = if event_name.contains("failed") || event_name.contains("error") {
        "error"
    } else if event_name.contains("ignored")
        || event_name.contains("skipped")
        || event_name.contains("cancelled")
        || event_name.contains("auto_paused")
    {
        "warn"
    } else {
        "info"
    };
    let category = if event_name.starts_with("clip_")
        || event_name.starts_with("clips_")
        || event_name.starts_with("trash_")
        || event_name.starts_with("note_")
    {
        "clip"
    } else if event_name.starts_with("recording_") || event_name.starts_with("clipboard_") {
        "capture"
    } else if event_name.starts_with("bin_")
        || event_name.starts_with("type_")
        || event_name.starts_with("classifier_")
        || event_name.starts_with("content_")
    {
        "organization"
    } else if event_name.starts_with("transform")
        || event_name.starts_with("operation_")
        || event_name.starts_with("intelligence_")
    {
        "transformation"
    } else if event_name.starts_with("setting_") || event_name == "settings_changed" {
        "settings"
    } else if event_name.starts_with("queue_") || event_name.starts_with("hud_") {
        "workflow"
    } else if event_name.starts_with("app_")
        || event_name.starts_with("library_")
        || event_name.starts_with("backup_")
        || event_name.starts_with("data_export_")
        || event_name.starts_with("external_")
    {
        "system"
    } else {
        "general"
    };
    let outcome = if event_name.contains("failed") || event_name.contains("error") {
        "failure"
    } else if event_name.contains("succeeded") || event_name.ends_with("_completed") {
        "success"
    } else {
        "unknown"
    };
    (severity, category, outcome)
}

fn canonical_activity_timestamp(value: &str) -> Result<String> {
    canonical_utc_timestamp(value, "Activity")
}

impl DbState {
    pub fn log_activity(&self, event_type: &str, description: &str) -> Result<()> {
        let conn = self.conn.lock();
        self.log_activity_internal(&conn, event_type, description)
    }

    pub(super) fn log_activity_with_attributes(
        &self,
        event_type: &str,
        description: &str,
        attributes: &serde_json::Value,
    ) -> Result<()> {
        let attributes_json = serde_json::to_string(attributes)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        if !attributes.is_object() || attributes_json.len() > 4 * 1024 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Activity attributes must be a bounded JSON object".into(),
            ));
        }
        let conn = self.conn.lock();
        self.log_activity_internal_with_attributes(&conn, event_type, description, &attributes_json)
    }

    pub(super) fn log_activity_internal(
        &self,
        conn: &Connection,
        event_type: &str,
        description: &str,
    ) -> Result<()> {
        self.log_activity_internal_with_attributes(conn, event_type, description, "{}")
    }

    fn log_activity_internal_with_attributes(
        &self,
        conn: &Connection,
        event_type: &str,
        description: &str,
        attributes_json: &str,
    ) -> Result<()> {
        let is_enabled: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'enableActivityLog'",
                [],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "true".to_string());
        if is_enabled == "false" {
            return Ok(());
        }

        let keep_count: i64 = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'activityLogCapacity'",
                [],
                |r| r.get(0),
            )
            .ok()
            .and_then(|v: String| v.parse().ok())
            .unwrap_or(1000);
        let keep_age_days: i64 = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'activityLogAgeDays'",
                [],
                |r| r.get(0),
            )
            .ok()
            .and_then(|v: String| v.parse().ok())
            .unwrap_or(0);

        let (severity, category, outcome) = activity_classification(event_type);
        let mut stmt = conn.prepare_cached(
            "INSERT INTO activity_logs (
                event_type, description, created_at, observed_at,
                severity_text, category, outcome, attributes_json
             ) VALUES (
                ?1, ?2,
                strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                ?3, ?4, ?5, ?6
             )",
        )?;
        stmt.execute(params![
            event_type,
            description,
            severity,
            category,
            outcome,
            attributes_json
        ])?;

        self.enforce_activity_retention_internal(conn, keep_count, keep_age_days)
    }

    pub(super) fn enforce_activity_retention_internal(
        &self,
        conn: &Connection,
        keep_count: i64,
        keep_age_days: i64,
    ) -> Result<()> {
        let keep_count = keep_count.max(0);
        let keep_age_days = keep_age_days.max(0);

        if keep_age_days > 0 {
            let age_modifier = format!("-{keep_age_days} days");
            conn.execute(
                "DELETE FROM activity_logs WHERE datetime(created_at) < datetime('now', ?1)",
                [age_modifier],
            )?;
        }

        if keep_count > 0 {
            let mut purge_stmt = conn.prepare_cached(
                "DELETE FROM activity_logs WHERE id NOT IN (SELECT id FROM activity_logs ORDER BY created_at DESC, id DESC LIMIT ?1)"
            )?;
            purge_stmt.execute(params![keep_count])?;
        }
        Ok(())
    }

    pub fn get_activity_logs(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<ActivityLog>> {
        self.get_activity_logs_filtered(limit, offset, None, None, None)
    }

    pub fn get_activity_logs_filtered(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
        category: Option<&str>,
        severity: Option<&str>,
        event_name: Option<&str>,
    ) -> Result<Vec<ActivityLog>> {
        let conn = self.conn.lock();
        let lim = limit.unwrap_or(100).clamp(1, 100_000);
        let off = offset.unwrap_or(0).max(0);
        let mut stmt = conn.prepare_cached(
            "SELECT id, event_type, description, created_at,
                    COALESCE(observed_at, created_at), severity_text, category, outcome, attributes_json
             FROM activity_logs
             WHERE (?1 IS NULL OR category = ?1)
               AND (?2 IS NULL OR severity_text = ?2)
               AND (?3 IS NULL OR event_type = ?3)
             ORDER BY created_at DESC, id DESC LIMIT ?4 OFFSET ?5"
        )?;
        let log_iter =
            stmt.query_map(params![category, severity, event_name, lim, off], |row| {
                let attributes_json: String = row.get(8)?;
                Ok(ActivityLog {
                    id: row.get(0)?,
                    event_type: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    observed_at: row.get(4)?,
                    severity_text: row.get(5)?,
                    category: row.get(6)?,
                    outcome: row.get(7)?,
                    attributes: serde_json::from_str(&attributes_json)
                        .unwrap_or_else(|_| serde_json::json!({})),
                })
            })?;
        let mut logs = Vec::new();
        for log in log_iter {
            logs.push(log?);
        }
        Ok(logs)
    }

    pub fn export_activity_json(&self) -> Result<String> {
        let entries = self
            .get_activity_logs(Some(i64::MAX), Some(0))?
            .into_iter()
            .map(Self::activity_archive_entry)
            .collect::<Result<Vec<_>>>()?;
        let mut resource = serde_json::Map::new();
        resource.insert("service.name".to_string(), serde_json::json!("Pasted"));
        resource.insert(
            "service.version".to_string(),
            serde_json::json!(env!("CARGO_PKG_VERSION")),
        );
        resource.insert(
            "telemetry.schema".to_string(),
            serde_json::json!("pasted.activity.v1"),
        );
        let archive = ActivityArchive {
            schema_version: 1,
            exported_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            resource,
            entries,
        };
        serde_json::to_string_pretty(&archive)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
    }

    pub fn export_activity_csv(&self) -> Result<String> {
        fn cell(value: &str) -> String {
            let escaped = value.replace('"', "\"\"");
            let neutralized = if matches!(
                value.chars().next(),
                Some('=' | '+' | '-' | '@' | '\t' | '\r')
            ) {
                format!("'{escaped}")
            } else {
                escaped
            };
            format!("\"{neutralized}\"")
        }

        let entries = self
            .get_activity_logs(Some(i64::MAX), Some(0))?
            .into_iter()
            .map(Self::activity_archive_entry)
            .collect::<Result<Vec<_>>>()?;
        let mut csv = String::from(
            "timestamp,observed_timestamp,event_name,severity_text,body,category,outcome,attributes_json\n",
        );
        for entry in entries {
            let category = entry
                .attributes
                .get("pasted.category")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("general");
            let outcome = entry
                .attributes
                .get("pasted.outcome")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown");
            let attributes_json = serde_json::Value::Object(entry.attributes.clone()).to_string();
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                cell(&entry.timestamp),
                cell(&entry.observed_timestamp),
                cell(&entry.event_name),
                cell(&entry.severity_text),
                cell(&entry.body),
                cell(category),
                cell(outcome),
                cell(&attributes_json),
            ));
        }
        Ok(csv)
    }

    fn activity_archive_entry(log: ActivityLog) -> Result<ActivityArchiveEntry> {
        let mut attributes = log.attributes.as_object().cloned().unwrap_or_default();
        attributes.insert(
            "pasted.category".to_string(),
            serde_json::json!(log.category),
        );
        attributes.insert("pasted.outcome".to_string(), serde_json::json!(log.outcome));
        attributes.insert("event.sequence".to_string(), serde_json::json!(log.id));
        Ok(ActivityArchiveEntry {
            timestamp: canonical_activity_timestamp(&log.created_at)?,
            observed_timestamp: canonical_activity_timestamp(&log.observed_at)?,
            event_name: log.event_type,
            severity_text: log.severity_text,
            body: log.description,
            attributes,
        })
    }

    pub fn import_activity_json(&self, json: &str) -> Result<ActivityImportReport> {
        let entries = Self::parse_activity_json_import(json)?;
        self.apply_activity_entries(entries, true)
    }

    pub fn inspect_activity_json(&self, json: &str) -> Result<ActivityImportReport> {
        let entries = Self::parse_activity_json_import(json)?;
        self.apply_activity_entries(entries, false)
    }

    fn parse_activity_json_import(json: &str) -> Result<Vec<ActivityArchiveEntry>> {
        use crate::resource_limits::{MAX_ACTIVITY_IMPORT_BYTES, MAX_ACTIVITY_IMPORT_ROWS};

        ensure_resource_size(json, MAX_ACTIVITY_IMPORT_BYTES, "Activity import")?;
        let archive: ActivityArchive = serde_json::from_str(json).map_err(|error| {
            rusqlite::Error::InvalidParameterName(format!("invalid Activity JSON: {error}"))
        })?;
        if archive.schema_version != 1 {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "unsupported Activity JSON schema version {} (supported: 1)",
                archive.schema_version
            )));
        }
        if archive.entries.len() > MAX_ACTIVITY_IMPORT_ROWS {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "Activity import contains more than {MAX_ACTIVITY_IMPORT_ROWS} entries"
            )));
        }

        Ok(archive.entries)
    }

    pub fn import_activity_csv(&self, csv: &str) -> Result<ActivityImportReport> {
        let entries = Self::parse_activity_csv_import(csv)?;
        self.apply_activity_entries(entries, true)
    }

    pub fn inspect_activity_csv(&self, csv: &str) -> Result<ActivityImportReport> {
        let entries = Self::parse_activity_csv_import(csv)?;
        self.apply_activity_entries(entries, false)
    }

    fn parse_activity_csv_import(csv: &str) -> Result<Vec<ActivityArchiveEntry>> {
        use crate::resource_limits::{MAX_ACTIVITY_IMPORT_BYTES, MAX_ACTIVITY_IMPORT_ROWS};

        ensure_resource_size(csv, MAX_ACTIVITY_IMPORT_BYTES, "Activity CSV import")?;
        let records = Self::parse_csv(csv)?;
        if records.len().saturating_sub(1) > MAX_ACTIVITY_IMPORT_ROWS {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "Activity import contains more than {MAX_ACTIVITY_IMPORT_ROWS} entries"
            )));
        }
        let expected = [
            "timestamp",
            "observed_timestamp",
            "event_name",
            "severity_text",
            "body",
            "category",
            "outcome",
            "attributes_json",
        ];
        if records.first().map(|header| {
            header
                .iter()
                .map(String::as_str)
                .eq(expected.iter().copied())
        }) != Some(true)
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "Activity CSV header does not match the supported export format".to_string(),
            ));
        }

        let mut entries = Vec::with_capacity(records.len().saturating_sub(1));
        for (index, row) in records.into_iter().skip(1).enumerate() {
            if row.len() != expected.len() {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "Activity CSV row {} has {} columns; expected {}",
                    index + 2,
                    row.len(),
                    expected.len()
                )));
            }
            let attributes_value: serde_json::Value =
                serde_json::from_str(&row[7]).map_err(|_| {
                    rusqlite::Error::InvalidParameterName(format!(
                        "Activity CSV row {} has invalid attributes JSON",
                        index + 2
                    ))
                })?;
            let mut attributes = attributes_value.as_object().cloned().ok_or_else(|| {
                rusqlite::Error::InvalidParameterName(format!(
                    "Activity CSV row {} attributes must be a JSON object",
                    index + 2
                ))
            })?;
            attributes.insert(
                "pasted.category".to_string(),
                serde_json::Value::String(row[5].clone()),
            );
            attributes.insert(
                "pasted.outcome".to_string(),
                serde_json::Value::String(row[6].clone()),
            );
            entries.push(ActivityArchiveEntry {
                timestamp: row[0].clone(),
                observed_timestamp: row[1].clone(),
                event_name: row[2].clone(),
                severity_text: row[3].clone(),
                body: row[4].clone(),
                attributes,
            });
        }

        Ok(entries)
    }

    fn apply_activity_entries(
        &self,
        entries: Vec<ActivityArchiveEntry>,
        commit: bool,
    ) -> Result<ActivityImportReport> {
        use crate::resource_limits::{
            MAX_ACTIVITY_ATTRIBUTES_BYTES, MAX_ACTIVITY_DESCRIPTION_BYTES,
            MAX_ACTIVITY_EVENT_TYPE_BYTES,
        };

        let scanned_count = entries.len();
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let mut imported_count = 0usize;
        let mut duplicate_count = 0usize;
        {
            let mut duplicate = tx.prepare_cached(
                "SELECT EXISTS(SELECT 1 FROM activity_logs WHERE event_type = ?1 AND description = ?2 AND created_at = ?3)",
            )?;
            let mut insert = tx.prepare_cached(
                "INSERT INTO activity_logs (
                    event_type, description, created_at, observed_at,
                    severity_text, category, outcome, attributes_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )?;
            for entry in entries {
                let event_type = entry.event_name.trim();
                let description = entry.body.trim();
                if event_type.is_empty()
                    || event_type.len() > MAX_ACTIVITY_EVENT_TYPE_BYTES
                    || !event_type.chars().all(|character| {
                        character.is_ascii_alphanumeric()
                            || matches!(character, '_' | '-' | '.' | ':')
                    })
                {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "Activity import contains an invalid event type".to_string(),
                    ));
                }
                if description.is_empty() || description.len() > MAX_ACTIVITY_DESCRIPTION_BYTES {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "Activity import contains an invalid description".to_string(),
                    ));
                }
                let created_at = chrono::DateTime::parse_from_rfc3339(&entry.timestamp)
                    .map_err(|_| {
                        rusqlite::Error::InvalidParameterName(
                            "Activity import contains an invalid timestamp".to_string(),
                        )
                    })?
                    .with_timezone(&chrono::Utc)
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                let observed_at = chrono::DateTime::parse_from_rfc3339(&entry.observed_timestamp)
                    .map_err(|_| {
                        rusqlite::Error::InvalidParameterName(
                            "Activity import contains an invalid observed timestamp".to_string(),
                        )
                    })?
                    .with_timezone(&chrono::Utc)
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                let severity = entry.severity_text.to_ascii_lowercase();
                if !matches!(severity.as_str(), "info" | "warn" | "error") {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "Activity import contains an unsupported severity".to_string(),
                    ));
                }
                let category = entry
                    .attributes
                    .get("pasted.category")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("general");
                if category.is_empty()
                    || category.len() > 64
                    || !category.chars().all(|character| {
                        character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
                    })
                {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "Activity import contains an invalid category".to_string(),
                    ));
                }
                let outcome = entry
                    .attributes
                    .get("pasted.outcome")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown");
                if !matches!(outcome, "success" | "failure" | "unknown") {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "Activity import contains an unsupported outcome".to_string(),
                    ));
                }
                let attributes_json = serde_json::to_string(&entry.attributes)
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
                if attributes_json.len() > MAX_ACTIVITY_ATTRIBUTES_BYTES {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "Activity import contains oversized attributes".to_string(),
                    ));
                }
                let exists: bool = duplicate
                    .query_row(params![event_type, description, created_at], |row| {
                        row.get(0)
                    })?;
                if exists {
                    duplicate_count += 1;
                    continue;
                }
                insert.execute(params![
                    event_type,
                    description,
                    created_at,
                    observed_at,
                    severity,
                    category,
                    outcome,
                    attributes_json,
                ])?;
                imported_count += 1;
            }
        }

        let keep_count = tx
            .query_row(
                "SELECT value FROM settings WHERE key = 'activityLogCapacity'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(1000);
        let keep_age_days = tx
            .query_row(
                "SELECT value FROM settings WHERE key = 'activityLogAgeDays'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        self.enforce_activity_retention_internal(&tx, keep_count, keep_age_days)?;
        let retained_count = tx.query_row("SELECT COUNT(*) FROM activity_logs", [], |row| {
            row.get::<_, i64>(0)
        })? as usize;
        if commit {
            tx.commit()?;
        } else {
            tx.rollback()?;
        }

        Ok(ActivityImportReport {
            scanned_count,
            imported_count,
            duplicate_count,
            retained_count,
        })
    }

    pub fn clear_activity_logs(&self) -> Result<()> {
        let conn = self.conn.lock();
        let _ = conn.execute("DELETE FROM activity_logs", [])?;
        Ok(())
    }
}
