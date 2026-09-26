use crate::db::{DbState, FullBackupInspection, LibraryArchiveInspection};
use std::path::PathBuf;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFileInspection {
    path: String,
    name: String,
    kind: String,
    format: String,
    size_bytes: u64,
    report: Option<serde_json::Value>,
    library: Option<LibraryArchiveInspection>,
    backup: Option<FullBackupInspection>,
}

pub(super) fn inspect_import_file_path(
    path: PathBuf,
    db: &DbState,
) -> Result<ImportFileInspection, String> {
    let metadata = std::fs::metadata(&path)
        .map_err(|error| format!("The selected file is not accessible: {error}"))?;
    if !metadata.is_file() {
        return Err("The selected item is not a file.".to_string());
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Selected file")
        .to_string();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let base = |kind: &str, format: &str| ImportFileInspection {
        path: path.to_string_lossy().into_owned(),
        name: name.clone(),
        kind: kind.to_string(),
        format: format.to_string(),
        size_bytes: metadata.len(),
        report: None,
        library: None,
        backup: None,
    };

    if extension == "pastedbackup" {
        let inspection = db
            .inspect_full_backup(&path)
            .map_err(|error| format!("The backup is not valid: {error}"))?;
        return Ok(ImportFileInspection {
            backup: Some(inspection),
            ..base("backup", "backup")
        });
    }
    if !matches!(extension.as_str(), "json" | "csv") {
        return Err("Choose a JSON, CSV, or Pasted Backup file.".to_string());
    }
    let contents = std::fs::read_to_string(&path)
        .map_err(|error| format!("The selected file could not be read: {error}"))?;
    if extension == "csv" {
        let header = contents.lines().next().unwrap_or_default();
        if header.starts_with("timestamp,observed_timestamp,event_name,") {
            let report = db
                .inspect_activity_csv(&contents)
                .map_err(|error| format!("The Activity CSV is not valid: {error}"))?;
            return Ok(ImportFileInspection {
                report: Some(serde_json::to_value(report).map_err(|error| error.to_string())?),
                ..base("activity", "csv")
            });
        }
        if header.starts_with("id,content_type,source,") {
            let report = db
                .inspect_clips_csv(&contents)
                .map_err(|error| format!("The Clips CSV is not valid: {error}"))?;
            return Ok(ImportFileInspection {
                report: Some(serde_json::to_value(report).map_err(|error| error.to_string())?),
                ..base("clips", "csv")
            });
        }
        return Err("The CSV does not match a supported Clips or Activity export.".to_string());
    }

    let parsed: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|error| format!("The selected file is not valid JSON: {error}"))?;
    if parsed.is_array() {
        let report = db
            .inspect_clips_json(&contents)
            .map_err(|error| format!("The Clips JSON is not valid: {error}"))?;
        return Ok(ImportFileInspection {
            report: Some(serde_json::to_value(report).map_err(|error| error.to_string())?),
            ..base("clips", "json")
        });
    }
    let object = parsed
        .as_object()
        .ok_or_else(|| "The JSON does not match a supported export.".to_string())?;
    if object
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .is_some()
        && object
            .get("schemaVersion")
            .and_then(serde_json::Value::as_u64)
            .is_some()
    {
        let report = db
            .inspect_activity_json(&contents)
            .map_err(|error| format!("The Activity JSON is not valid: {error}"))?;
        return Ok(ImportFileInspection {
            report: Some(serde_json::to_value(report).map_err(|error| error.to_string())?),
            ..base("activity", "json")
        });
    }
    if object
        .get("clips")
        .and_then(serde_json::Value::as_array)
        .is_some()
        && object
            .get("bins")
            .and_then(serde_json::Value::as_array)
            .is_some()
        && object
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .is_some()
    {
        let inspection = DbState::inspect_library_archive_json(&contents)
            .map_err(|error| format!("The History and Organization JSON is not valid: {error}"))?;
        return Ok(ImportFileInspection {
            library: Some(inspection),
            ..base("organization", "json")
        });
    }
    Err("The JSON does not match a supported export.".to_string())
}
