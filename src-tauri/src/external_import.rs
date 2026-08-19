use chrono::{DateTime, SecondsFormat, Utc};
use plist::Value as PlistValue;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use crate::db::DbState;
use crate::resource_limits::{
    MAX_CLIP_TEXT_BYTES, MAX_EXTERNAL_IMPORT_DATABASE_BYTES, MAX_EXTERNAL_IMPORT_ROWS,
    MAX_EXTERNAL_IMPORT_TEXT_BYTES,
};

pub const ONBOARDING_SETTING_KEY: &str = "onboardingVersion";
pub const ONBOARDING_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExternalImportSource {
    Alfred,
    Pastebot,
    Pasta,
    Paste,
    CopyClip,
    Maccy,
    Flycut,
}

impl ExternalImportSource {
    pub const ALL: [Self; 7] = [
        Self::Alfred,
        Self::Pastebot,
        Self::Pasta,
        Self::Paste,
        Self::CopyClip,
        Self::Maccy,
        Self::Flycut,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Alfred => "alfred",
            Self::Pastebot => "pastebot",
            Self::Pasta => "pasta",
            Self::Paste => "paste",
            Self::CopyClip => "copyclip",
            Self::Maccy => "maccy",
            Self::Flycut => "flycut",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Alfred => "Alfred",
            Self::Pastebot => "Pastebot",
            Self::Pasta => "Pasta",
            Self::Paste => "Paste",
            Self::CopyClip => "CopyClip 2",
            Self::Maccy => "Maccy",
            Self::Flycut => "Flycut",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Alfred => "Clipboard history from Alfred Powerpack",
            Self::Pastebot => "Text history from Pastebot",
            Self::Pasta => "Text history from Pasta",
            Self::Paste => "Text history from Paste",
            Self::CopyClip => "Text history from CopyClip 2",
            Self::Maccy => "Text history from Maccy",
            Self::Flycut => "Text history from Flycut",
        }
    }

    pub fn prefers_folder_selection(self) -> bool {
        matches!(self, Self::Pastebot | Self::Paste)
    }

    fn default_paths(self) -> Vec<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let Some(home) = dirs::home_dir() else {
                return Vec::new();
            };
            match self {
                Self::Alfred => vec![
                    home.join("Library/Application Support/Alfred/Databases/clipboard.alfdb"),
                ],
                Self::Pastebot => {
                    let v3 = home.join("Library/Group Containers/group.9JTH7AWHE6.com.tapbots.Pastebot3Mac");
                    let v2 = home.join("Library/Group Containers/9JTH7AWHE6.com.tapbots.Pastebot2Mac");
                    let legacy = home.join("Library/Group Containers/9272N75U7L.com.tapbots.Pastebot");
                    vec![
                        v3.join("Pastebot.sqlite"),
                        v3.join("Library/Application Support/Pastebot/Pastebot.sqlite"),
                        v2.join("Pastebot.sqlite"),
                        v2.join("Library/Application Support/Pastebot/Pastebot.sqlite"),
                        legacy.join("Pastebot.sqlite"),
                    ]
                }
                Self::Pasta => vec![
                    home.join("Library/Application Support/Pasta/pasta.sqlite"),
                    home.join("Library/Containers/com.pasta.app/Data/Library/Application Support/Pasta/pasta.sqlite"),
                ],
                Self::Paste => vec![
                    home.join("Library/Containers/com.widetape.Paste/Data/Library/Application Support/Paste/Paste.sqlite"),
                    home.join("Library/Containers/com.wiheads.paste/Data/Library/Application Support/Paste/Paste.sqlite"),
                ],
                Self::CopyClip => vec![
                    home.join("Library/Application Support/com.fiplab.copyclip2/Data/com.fiplab.copyclip2.data"),
                    home.join("Library/Containers/com.fiplab.copyclip2/Data/Library/Application Support/com.fiplab.copyclip2/Data/com.fiplab.copyclip2.data"),
                ],
                Self::Maccy => vec![
                    home.join("Library/Containers/org.p0deje.Maccy/Data/Library/Application Support/Maccy/Storage.sqlite"),
                    home.join("Library/Application Support/Maccy/Storage.sqlite"),
                ],
                Self::Flycut => {
                    vec![home.join("Library/Preferences/net.sogao.Flycut.plist")]
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            Vec::new()
        }
    }

    pub fn default_path(self) -> Option<PathBuf> {
        let paths = self.default_paths();
        paths
            .iter()
            .find(|path| path.is_file())
            .cloned()
            .or_else(|| paths.into_iter().next())
    }

    fn detection_paths(self) -> Vec<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let Some(home) = dirs::home_dir() else {
                return Vec::new();
            };
            match self {
                Self::Pastebot => vec![
                    home.join("Library/Group Containers/group.9JTH7AWHE6.com.tapbots.Pastebot3Mac"),
                    home.join("Library/Group Containers/9JTH7AWHE6.com.tapbots.Pastebot2Mac"),
                    home.join("Library/Containers/com.tapbots.Pastebot3Mac"),
                    home.join("Library/Containers/com.tapbots.Pastebot2Mac"),
                ],
                Self::Paste => vec![
                    home.join("Library/Containers/com.widetape.Paste"),
                    home.join("Library/Containers/com.wiheads.paste"),
                    home.join("Library/Group Containers/group.com.widetape.Paste"),
                ],
                _ => self.default_paths(),
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            Vec::new()
        }
    }

    pub fn suggested_directory(self) -> Option<PathBuf> {
        self.detection_paths()
            .into_iter()
            .find(|path| path.is_dir())
            .or_else(|| {
                self.default_path()
                    .and_then(|path| path.parent().map(Path::to_path_buf))
            })
    }
}

impl FromStr for ExternalImportSource {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "alfred" => Ok(Self::Alfred),
            "pastebot" => Ok(Self::Pastebot),
            "pasta" => Ok(Self::Pasta),
            "paste" => Ok(Self::Paste),
            "copyclip" | "copyclip2" | "copyclip-2" => Ok(Self::CopyClip),
            "maccy" => Ok(Self::Maccy),
            "flycut" => Ok(Self::Flycut),
            _ => Err(format!("Unsupported import source '{value}'.")),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExternalImportSourceInfo {
    pub id: String,
    pub label: String,
    pub description: String,
    pub available: bool,
    pub detected: bool,
    pub default_path: Option<String>,
    pub supports_custom_file: bool,
    pub selection_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExternalImportReport {
    pub source: String,
    pub scanned_count: usize,
    pub imported_count: usize,
    pub duplicate_count: usize,
    pub skipped_count: usize,
    pub history_capacity_adjusted_to: Option<usize>,
}

#[derive(Debug, Clone)]
pub(crate) struct ExternalTextClip {
    pub text: String,
    pub content_hash: String,
    pub source: String,
    pub created_at: Option<String>,
}

#[derive(Default)]
struct ParsedImport {
    scanned_count: usize,
    skipped_count: usize,
    text_bytes: usize,
    clips: Vec<ExternalTextClip>,
}

impl ParsedImport {
    fn push(
        &mut self,
        text: String,
        source: Option<String>,
        created_at: Option<String>,
    ) -> Result<(), String> {
        self.scanned_count += 1;
        if text.is_empty() || text.len() > MAX_CLIP_TEXT_BYTES {
            self.skipped_count += 1;
            return Ok(());
        }
        if self.text_bytes.saturating_add(text.len()) > MAX_EXTERNAL_IMPORT_TEXT_BYTES {
            return Err("Import content exceeds Pasted's 256 MB safety limit.".to_string());
        }
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        let text_len = text.len();
        self.clips.push(ExternalTextClip {
            text,
            content_hash: format!("{:x}", hasher.finalize()),
            source: source
                .filter(|value| !value.trim().is_empty())
                .map(|value| value.chars().take(256).collect())
                .unwrap_or_else(|| "Imported history".to_string()),
            created_at,
        });
        self.text_bytes += text_len;
        Ok(())
    }

    fn skip(&mut self) {
        self.scanned_count += 1;
        self.skipped_count += 1;
    }

    fn ensure_row_limit(&self) -> Result<(), String> {
        if self.scanned_count >= MAX_EXTERNAL_IMPORT_ROWS {
            return Err(format!(
                "Import exceeds Pasted's {MAX_EXTERNAL_IMPORT_ROWS}-row safety limit."
            ));
        }
        Ok(())
    }
}

pub fn source_infos() -> Vec<ExternalImportSourceInfo> {
    ExternalImportSource::ALL
        .into_iter()
        .map(|source| {
            let default_path = source.default_path();
            let available = default_path.as_ref().is_some_and(|path| path.is_file());
            ExternalImportSourceInfo {
                id: source.id().to_string(),
                label: source.label().to_string(),
                description: source.description().to_string(),
                available,
                detected: available || source.detection_paths().iter().any(|path| path.exists()),
                default_path: default_path.map(|path| path.to_string_lossy().into_owned()),
                supports_custom_file: true,
                selection_kind: if source.prefers_folder_selection() {
                    "folder"
                } else {
                    "file"
                }
                .to_string(),
            }
        })
        .collect()
}

pub fn import_history(
    db: &DbState,
    source: ExternalImportSource,
    requested_path: Option<PathBuf>,
) -> Result<ExternalImportReport, String> {
    let path = requested_path
        .or_else(|| source.default_path())
        .ok_or_else(|| format!("Choose a {} history file to import.", source.label()))?;
    let parsed = if path.is_dir() {
        parse_history_folder(source, &path)?
    } else {
        validate_import_file(&path)?;
        parse_history_file(source, &path)?
    };
    let (imported_count, duplicate_count, history_capacity_adjusted_to) = db
        .merge_external_text_clips(source.label(), &parsed.clips)
        .map_err(|error| format!("Could not merge imported history: {error}"))?;

    Ok(ExternalImportReport {
        source: source.id().to_string(),
        scanned_count: parsed.scanned_count,
        imported_count,
        duplicate_count,
        skipped_count: parsed.skipped_count,
        history_capacity_adjusted_to,
    })
}

fn parse_history_file(source: ExternalImportSource, path: &Path) -> Result<ParsedImport, String> {
    match source {
        ExternalImportSource::Alfred => parse_alfred(path),
        ExternalImportSource::Pastebot => parse_pastebot(path),
        ExternalImportSource::Pasta => parse_pasta(path),
        ExternalImportSource::Paste => parse_paste(path),
        ExternalImportSource::CopyClip => parse_copyclip(path),
        ExternalImportSource::Maccy => parse_maccy(path),
        ExternalImportSource::Flycut => parse_flycut(path),
    }
}

fn parse_history_folder(
    source: ExternalImportSource,
    folder: &Path,
) -> Result<ParsedImport, String> {
    let candidates = discover_history_files(folder)?;
    for candidate in candidates {
        if validate_import_file(&candidate).is_ok() {
            if let Ok(parsed) = parse_history_file(source, &candidate) {
                return Ok(parsed);
            }
        }
    }
    Err(format!(
        "No recognized {} history database was found in the selected folder.",
        source.label()
    ))
}

fn discover_history_files(folder: &Path) -> Result<Vec<PathBuf>, String> {
    const MAX_DISCOVERED_ENTRIES: usize = 4_096;
    const MAX_DEPTH: usize = 6;
    let mut pending = vec![(folder.to_path_buf(), 0usize)];
    let mut files = Vec::new();
    let mut visited = 0usize;
    while let Some((directory, depth)) = pending.pop() {
        let entries = match std::fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) if depth == 0 => {
                return Err(format!(
                    "Could not inspect the selected history folder '{}': {error}",
                    directory.display()
                ));
            }
            Err(_) => continue,
        };
        for entry in entries {
            let entry = entry.map_err(|error| error.to_string())?;
            visited += 1;
            if visited > MAX_DISCOVERED_ENTRIES {
                return Err(
                    "The selected folder contains too many files to inspect safely.".into(),
                );
            }
            let file_type = entry.file_type().map_err(|error| error.to_string())?;
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            if file_type.is_dir() && depth < MAX_DEPTH {
                pending.push((path, depth + 1));
            } else if file_type.is_file() {
                let name = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                let extension = path
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if !name.ends_with("-wal")
                    && !name.ends_with("-shm")
                    && (["sqlite", "sqlite3", "db", "storedata"].contains(&extension.as_str())
                        || name.contains("sqlite"))
                {
                    files.push(path);
                }
            }
        }
    }
    files.sort_by_key(|path| {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        usize::from(!(name.contains("pastebot") || name == "paste.sqlite"))
    });
    Ok(files)
}

fn validate_import_file(path: &Path) -> Result<(), String> {
    let metadata = path
        .metadata()
        .map_err(|error| format!("Could not read '{}': {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("'{}' is not a history file.", path.display()));
    }
    if metadata.len() > MAX_EXTERNAL_IMPORT_DATABASE_BYTES {
        return Err("History file exceeds Pasted's 2 GB safety limit.".to_string());
    }
    Ok(())
}

fn open_read_only(path: &Path) -> Result<Connection, String> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| format!("Could not open the history database read-only: {error}"))?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|error| error.to_string())?;
    connection
        .pragma_update(None, "query_only", "ON")
        .map_err(|error| error.to_string())?;
    Ok(connection)
}

fn table_names(connection: &Connection) -> Result<HashSet<String>, String> {
    let mut statement = connection
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<HashSet<_>, _>>()
        .map_err(|error| error.to_string())
}

fn column_names(connection: &Connection, table: &str) -> Result<HashSet<String>, String> {
    let mut statement = connection
        .prepare("SELECT name FROM pragma_table_info(?1)")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([table], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<HashSet<_>, _>>()
        .map_err(|error| error.to_string())
}

fn quoted_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn string_value(row: &rusqlite::Row<'_>, index: usize) -> Option<String> {
    let value = row.get_ref(index).ok()?;
    match value {
        rusqlite::types::ValueRef::Text(bytes) => String::from_utf8(bytes.to_vec()).ok(),
        rusqlite::types::ValueRef::Blob(bytes) => String::from_utf8(bytes.to_vec()).ok(),
        _ => None,
    }
}

fn number_value(row: &rusqlite::Row<'_>, index: usize) -> Option<f64> {
    let value = row.get_ref(index).ok()?;
    match value {
        rusqlite::types::ValueRef::Integer(value) => Some(value as f64),
        rusqlite::types::ValueRef::Real(value) => Some(value),
        rusqlite::types::ValueRef::Text(value) => std::str::from_utf8(value).ok()?.parse().ok(),
        _ => None,
    }
}

fn unix_timestamp(seconds: f64) -> Option<String> {
    if !seconds.is_finite() || seconds <= 0.0 {
        return None;
    }
    DateTime::<Utc>::from_timestamp(seconds as i64, 0)
        .map(|date| date.to_rfc3339_opts(SecondsFormat::Secs, true))
}

fn core_data_timestamp(seconds: f64) -> Option<String> {
    unix_timestamp(seconds + 978_307_200.0)
}

fn stored_datetime_value(row: &rusqlite::Row<'_>, index: usize) -> Option<String> {
    if let Some(value) = string_value(row, index) {
        let value = value.trim();
        if !value.is_empty() && value.len() <= 64 {
            return Some(value.to_string());
        }
    }
    number_value(row, index).and_then(unix_timestamp)
}

fn parse_alfred(path: &Path) -> Result<ParsedImport, String> {
    let connection = open_read_only(path)?;
    let tables = table_names(&connection)?;
    if !tables.contains("clipboard") {
        return Err("The selected file is not a recognized Alfred clipboard database.".to_string());
    }
    let columns = column_names(&connection, "clipboard")?;
    if !columns.contains("item") || !columns.contains("ts") {
        return Err("This Alfred clipboard database uses an unsupported schema.".to_string());
    }
    let app_column = if columns.contains("app") {
        "app"
    } else {
        "NULL"
    };
    let data_type_column = if columns.contains("dataType") {
        "dataType"
    } else {
        "0"
    };
    let sql = format!(
        "SELECT item, ts, {app_column}, {data_type_column} FROM clipboard ORDER BY ts ASC LIMIT ?1"
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| format!("Could not read Alfred history: {error}"))?;
    let mut rows = statement
        .query([MAX_EXTERNAL_IMPORT_ROWS as i64 + 1])
        .map_err(|error| error.to_string())?;
    let mut parsed = ParsedImport::default();
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        parsed.ensure_row_limit()?;
        let data_type = number_value(row, 3).unwrap_or(0.0) as i64;
        if data_type == 2 {
            parsed.skip();
            continue;
        }
        let Some(text) = string_value(row, 0) else {
            parsed.skip();
            continue;
        };
        parsed.push(
            text,
            string_value(row, 2),
            number_value(row, 1).and_then(unix_timestamp),
        )?;
    }
    Ok(parsed)
}

fn parse_maccy(path: &Path) -> Result<ParsedImport, String> {
    let connection = open_read_only(path)?;
    let tables = table_names(&connection)?;
    let (items, contents) = if tables.contains("ZHISTORYITEM")
        && tables.contains("ZHISTORYITEMCONTENT")
    {
        ("ZHISTORYITEM", "ZHISTORYITEMCONTENT")
    } else if tables.contains("HistoryItem") && tables.contains("HistoryItemContent") {
        ("HistoryItem", "HistoryItemContent")
    } else {
        return Err("The selected file is not a recognized Maccy history database.".to_string());
    };
    let sql = format!(
        "SELECT c.ZVALUE, i.ZLASTCOPIEDAT, i.ZAPPLICATION FROM {} i JOIN {} c ON c.ZITEM = i.Z_PK WHERE c.ZTYPE = 'public.utf8-plain-text' ORDER BY i.ZLASTCOPIEDAT ASC LIMIT ?1",
        quoted_identifier(items),
        quoted_identifier(contents),
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| format!("This Maccy database uses an unsupported schema: {error}"))?;
    let mut rows = statement
        .query([MAX_EXTERNAL_IMPORT_ROWS as i64 + 1])
        .map_err(|error| error.to_string())?;
    let mut parsed = ParsedImport::default();
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        parsed.ensure_row_limit()?;
        let Some(text) = string_value(row, 0) else {
            parsed.skip();
            continue;
        };
        parsed.push(
            text,
            string_value(row, 2),
            number_value(row, 1).and_then(core_data_timestamp),
        )?;
    }
    Ok(parsed)
}

fn parse_pasta(path: &Path) -> Result<ParsedImport, String> {
    let connection = open_read_only(path)?;
    let tables = table_names(&connection)?;
    if !tables.contains("clipboard_entries") {
        return Err("The selected file is not a recognized Pasta history database.".into());
    }
    let columns = column_names(&connection, "clipboard_entries")?;
    for required in ["content", "contentType", "timestamp"] {
        if !columns.contains(required) {
            return Err("This Pasta database uses an unsupported schema.".into());
        }
    }
    let source_column = if columns.contains("sourceApp") {
        quoted_identifier("sourceApp")
    } else {
        "NULL".to_string()
    };
    let sql = format!(
        "SELECT {}, {}, {}, {} FROM {} WHERE {} NOT IN ('image', 'screenshot') ORDER BY {} ASC LIMIT ?1",
        quoted_identifier("content"),
        quoted_identifier("timestamp"),
        source_column,
        quoted_identifier("contentType"),
        quoted_identifier("clipboard_entries"),
        quoted_identifier("contentType"),
        quoted_identifier("timestamp"),
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| format!("Could not read Pasta history: {error}"))?;
    let mut rows = statement
        .query([MAX_EXTERNAL_IMPORT_ROWS as i64 + 1])
        .map_err(|error| error.to_string())?;
    let mut parsed = ParsedImport::default();
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        parsed.ensure_row_limit()?;
        let Some(text) = string_value(row, 0) else {
            parsed.skip();
            continue;
        };
        parsed.push(text, string_value(row, 2), stored_datetime_value(row, 1))?;
    }
    Ok(parsed)
}

fn parse_paste(path: &Path) -> Result<ParsedImport, String> {
    let connection = open_read_only(path)?;
    let tables = table_names(&connection)?;
    let items_table = tables
        .iter()
        .filter(|table| {
            let name = table.to_ascii_uppercase();
            name.contains("ITEM") && !name.contains("CONTENT")
        })
        .find(|table| {
            column_names(&connection, table).is_ok_and(|columns| columns.contains("Z_PK"))
        });
    let content_table = tables.iter().find(|table| {
        table.to_ascii_uppercase().contains("CONTENT")
            && column_names(&connection, table).is_ok_and(|columns| {
                columns.contains("ZITEM")
                    && ["ZPLAINTEXT", "ZTEXT", "ZCONTENT"]
                        .iter()
                        .any(|column| columns.contains(*column))
            })
    });

    if let (Some(items_table), Some(content_table)) = (items_table, content_table) {
        let item_columns = column_names(&connection, items_table)?;
        let content_columns = column_names(&connection, content_table)?;
        let content = ["ZPLAINTEXT", "ZTEXT", "ZCONTENT"]
            .into_iter()
            .find(|column| content_columns.contains(*column))
            .expect("validated Paste content column");
        let timestamp = ["ZCREATETIME", "ZTIMESTAMP", "ZDATE"]
            .into_iter()
            .find(|column| item_columns.contains(*column));
        let source = ["ZAPPNAME", "ZSOURCEAPP", "ZSOURCE"]
            .into_iter()
            .find(|column| item_columns.contains(*column));
        let timestamp_sql = timestamp
            .map(|column| format!("i.{}", quoted_identifier(column)))
            .unwrap_or_else(|| "NULL".to_string());
        let source_sql = source
            .map(|column| format!("i.{}", quoted_identifier(column)))
            .unwrap_or_else(|| "NULL".to_string());
        let order_sql = timestamp
            .map(|column| format!("i.{}", quoted_identifier(column)))
            .unwrap_or_else(|| "i.rowid".to_string());
        let sql = format!(
            "SELECT c.{}, {}, {} FROM {} i JOIN {} c ON c.{} = i.{} ORDER BY {} ASC LIMIT ?1",
            quoted_identifier(content),
            timestamp_sql,
            source_sql,
            quoted_identifier(items_table),
            quoted_identifier(content_table),
            quoted_identifier("ZITEM"),
            quoted_identifier("Z_PK"),
            order_sql,
        );
        return parse_core_data_rows(&connection, &sql, "Paste");
    }

    parse_core_data_text_table(&connection, "Paste", &tables, &["ITEM", "PASTE"])
}

fn parse_copyclip(path: &Path) -> Result<ParsedImport, String> {
    let value = PlistValue::from_file(path)
        .map_err(|error| format!("Could not read CopyClip 2 history: {error}"))?;
    let mut parsed = ParsedImport::default();
    collect_copyclip_text(&value, &mut parsed)?;
    if parsed.scanned_count == 0 {
        return Err("The selected file is not a recognized CopyClip 2 history file.".into());
    }
    Ok(parsed)
}

fn collect_copyclip_text(value: &PlistValue, parsed: &mut ParsedImport) -> Result<(), String> {
    match value {
        PlistValue::Array(values) => {
            for value in values {
                collect_copyclip_text(value, parsed)?;
            }
        }
        PlistValue::Dictionary(values) => {
            for (key, value) in values {
                let is_content = ["content", "string", "text"]
                    .iter()
                    .any(|candidate| key.eq_ignore_ascii_case(candidate));
                if is_content {
                    if let Some(text) = value.as_string() {
                        parsed.ensure_row_limit()?;
                        parsed.push(text.to_string(), Some("CopyClip 2".into()), None)?;
                    } else {
                        parsed.skip();
                    }
                } else {
                    collect_copyclip_text(value, parsed)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn parse_pastebot(path: &Path) -> Result<ParsedImport, String> {
    let connection = open_read_only(path)?;
    let tables = table_names(&connection)?;
    parse_core_data_text_table(&connection, "Pastebot", &tables, &["CLIPPING", "PASTE"])
}

fn parse_core_data_text_table(
    connection: &Connection,
    app_label: &str,
    tables: &HashSet<String>,
    preferred_names: &[&str],
) -> Result<ParsedImport, String> {
    let mut candidates = tables
        .iter()
        .cloned()
        .filter_map(|table| {
            let columns = column_names(connection, &table).ok()?;
            let content = ["ZPLAINTEXT", "ZTEXT", "ZCONTENT", "ZTITLE"]
                .into_iter()
                .find(|column| columns.contains(*column))?;
            let timestamp = ["ZCREATETIME", "ZTIMESTAMP", "ZDATE", "ZCREATED"]
                .into_iter()
                .find(|column| columns.contains(*column));
            let source = ["ZAPPNAME", "ZSOURCEAPP", "ZSOURCE"]
                .into_iter()
                .find(|column| columns.contains(*column));
            let upper_table = table.to_ascii_uppercase();
            let name_score = preferred_names
                .iter()
                .enumerate()
                .map(|(index, name)| usize::from(upper_table.contains(name)) * (10 - index))
                .sum::<usize>();
            Some((name_score, table, content, timestamp, source))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    let Some((_, table, content, timestamp, source)) = candidates.into_iter().next() else {
        return Err(format!(
            "The selected file is not a recognized {app_label} history database."
        ));
    };
    let timestamp_sql = timestamp
        .map(quoted_identifier)
        .unwrap_or_else(|| "NULL".to_string());
    let source_sql = source
        .map(quoted_identifier)
        .unwrap_or_else(|| "NULL".to_string());
    let order_sql = timestamp
        .map(quoted_identifier)
        .unwrap_or_else(|| "rowid".to_string());
    let sql = format!(
        "SELECT {}, {}, {} FROM {} ORDER BY {} ASC LIMIT ?1",
        quoted_identifier(content),
        timestamp_sql,
        source_sql,
        quoted_identifier(&table),
        order_sql,
    );
    parse_core_data_rows(connection, &sql, app_label)
}

fn parse_core_data_rows(
    connection: &Connection,
    sql: &str,
    app_label: &str,
) -> Result<ParsedImport, String> {
    let mut statement = connection
        .prepare(sql)
        .map_err(|error| format!("Could not read {app_label} history: {error}"))?;
    let mut rows = statement
        .query([MAX_EXTERNAL_IMPORT_ROWS as i64 + 1])
        .map_err(|error| error.to_string())?;
    let mut parsed = ParsedImport::default();
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        parsed.ensure_row_limit()?;
        let Some(text) = string_value(row, 0) else {
            parsed.skip();
            continue;
        };
        parsed.push(
            text,
            string_value(row, 2),
            number_value(row, 1).and_then(core_data_timestamp),
        )?;
    }
    Ok(parsed)
}

fn plist_number(value: &PlistValue) -> Option<f64> {
    value
        .as_real()
        .or_else(|| value.as_signed_integer().map(|value| value as f64))
}

fn parse_flycut(path: &Path) -> Result<ParsedImport, String> {
    let plist = PlistValue::from_file(path)
        .map_err(|error| format!("Could not read Flycut history: {error}"))?;
    let root = plist
        .as_dictionary()
        .ok_or_else(|| "The selected file is not a recognized Flycut history file.".to_string())?;
    let store = root
        .get("store")
        .and_then(PlistValue::as_dictionary)
        .ok_or_else(|| "This Flycut history uses an unsupported schema.".to_string())?;
    let items = store
        .get("jcList")
        .and_then(PlistValue::as_array)
        .ok_or_else(|| "This Flycut history uses an unsupported schema.".to_string())?;
    if items.len() > MAX_EXTERNAL_IMPORT_ROWS {
        return Err(format!(
            "Import exceeds Pasted's {MAX_EXTERNAL_IMPORT_ROWS}-row safety limit."
        ));
    }
    let mut parsed = ParsedImport::default();
    for item in items {
        let Some(item) = item.as_dictionary() else {
            parsed.skip();
            continue;
        };
        let Some(text) = item.get("Contents").and_then(PlistValue::as_string) else {
            parsed.skip();
            continue;
        };
        let source = item
            .get("AppLocalizedName")
            .and_then(PlistValue::as_string)
            .map(str::to_string);
        let created_at = item
            .get("Timestamp")
            .and_then(plist_number)
            .and_then(unix_timestamp);
        parsed.push(text.to_string(), source, created_at)?;
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("pasted-{name}-{nonce}"))
    }

    #[test]
    fn rejects_unknown_sources() {
        assert!(ExternalImportSource::from_str("unknown").is_err());
        assert_eq!(
            ExternalImportSource::from_str("PASTEBOT").unwrap(),
            ExternalImportSource::Pastebot
        );
        assert_eq!(ExternalImportSource::ALL[1], ExternalImportSource::Pastebot);
        assert_eq!(
            ExternalImportSource::from_str("copyclip2").unwrap(),
            ExternalImportSource::CopyClip
        );
    }

    #[test]
    fn parses_maccy_text_without_mutating_source() {
        let path = temp_path("maccy.sqlite");
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch(
            "CREATE TABLE ZHISTORYITEM (Z_PK INTEGER PRIMARY KEY, ZLASTCOPIEDAT REAL, ZAPPLICATION TEXT);
             CREATE TABLE ZHISTORYITEMCONTENT (ZITEM INTEGER, ZTYPE TEXT, ZVALUE BLOB);
             INSERT INTO ZHISTORYITEM VALUES (1, 100, 'Test App');
             INSERT INTO ZHISTORYITEMCONTENT VALUES (1, 'public.utf8-plain-text', X'68656C6C6F');",
        ).unwrap();
        drop(connection);
        let parsed = parse_maccy(&path).unwrap();
        assert_eq!(parsed.clips.len(), 1);
        assert_eq!(parsed.clips[0].text, "hello");
        assert_eq!(parsed.clips[0].source, "Test App");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn maccy_import_merges_atomically_and_reports_duplicates() {
        let source_path = temp_path("maccy-merge.sqlite");
        let source = Connection::open(&source_path).unwrap();
        source
            .execute_batch(
                "CREATE TABLE ZHISTORYITEM (Z_PK INTEGER PRIMARY KEY, ZLASTCOPIEDAT REAL, ZAPPLICATION TEXT);
                 CREATE TABLE ZHISTORYITEMCONTENT (ZITEM INTEGER, ZTYPE TEXT, ZVALUE BLOB);
                 INSERT INTO ZHISTORYITEM VALUES (1, 100, 'Test App');
                 INSERT INTO ZHISTORYITEMCONTENT VALUES (1, 'public.utf8-plain-text', X'68656C6C6F');",
            )
            .unwrap();
        drop(source);

        let destination_path = temp_path("destination.sqlite");
        let destination = DbState::new(destination_path.clone()).unwrap();
        let first = import_history(
            &destination,
            ExternalImportSource::Maccy,
            Some(source_path.clone()),
        )
        .unwrap();
        let second = import_history(
            &destination,
            ExternalImportSource::Maccy,
            Some(source_path.clone()),
        )
        .unwrap();

        assert_eq!(first.imported_count, 1);
        assert_eq!(first.duplicate_count, 0);
        assert_eq!(second.imported_count, 0);
        assert_eq!(second.duplicate_count, 1);
        assert_eq!(destination.get_clips(None, false).unwrap().len(), 1);

        drop(destination);
        let _ = fs::remove_file(source_path);
        let _ = fs::remove_file(destination_path);
    }

    #[test]
    fn parses_flycut_property_list() {
        let path = temp_path("flycut.plist");
        let mut item = plist::Dictionary::new();
        item.insert(
            "Contents".to_string(),
            PlistValue::String("from flycut".to_string()),
        );
        item.insert(
            "AppLocalizedName".to_string(),
            PlistValue::String("Terminal".to_string()),
        );
        item.insert("Timestamp".to_string(), PlistValue::Real(1_700_000_000.0));
        let mut store = plist::Dictionary::new();
        store.insert(
            "jcList".to_string(),
            PlistValue::Array(vec![PlistValue::Dictionary(item)]),
        );
        let mut root = plist::Dictionary::new();
        root.insert("store".to_string(), PlistValue::Dictionary(store));
        PlistValue::Dictionary(root).to_file_xml(&path).unwrap();

        let parsed = parse_flycut(&path).unwrap();
        assert_eq!(parsed.clips.len(), 1);
        assert_eq!(parsed.clips[0].text, "from flycut");
        assert_eq!(parsed.clips[0].source, "Terminal");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn parses_copyclip_property_list_content_fields() {
        let path = temp_path("copyclip.data");
        let mut clip = plist::Dictionary::new();
        clip.insert(
            "content".to_string(),
            PlistValue::String("from copyclip".to_string()),
        );
        let mut root = plist::Dictionary::new();
        root.insert(
            "history".to_string(),
            PlistValue::Array(vec![PlistValue::Dictionary(clip)]),
        );
        PlistValue::Dictionary(root).to_file_binary(&path).unwrap();

        let parsed = parse_copyclip(&path).unwrap();
        assert_eq!(parsed.clips.len(), 1);
        assert_eq!(parsed.clips[0].text, "from copyclip");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn parses_pasta_text_entries_and_skips_images() {
        let path = temp_path("pasta.sqlite");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE clipboard_entries (
                    id TEXT PRIMARY KEY,
                    content TEXT NOT NULL,
                    contentType TEXT NOT NULL,
                    timestamp DATETIME NOT NULL,
                    sourceApp TEXT
                 );
                 INSERT INTO clipboard_entries VALUES ('1', 'from pasta', 'text', '2026-08-12 10:00:00', 'Editor');
                 INSERT INTO clipboard_entries VALUES ('2', 'image label', 'image', '2026-08-12 10:01:00', 'Editor');",
            )
            .unwrap();
        drop(connection);

        let parsed = parse_pasta(&path).unwrap();
        assert_eq!(parsed.clips.len(), 1);
        assert_eq!(parsed.clips[0].text, "from pasta");
        assert_eq!(parsed.clips[0].source, "Editor");
        assert_eq!(
            parsed.clips[0].created_at.as_deref(),
            Some("2026-08-12 10:00:00")
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn parses_paste_joined_core_data_tables() {
        let path = temp_path("paste.sqlite");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE ZITEM (Z_PK INTEGER PRIMARY KEY, ZCREATETIME REAL, ZAPPNAME TEXT);
                 CREATE TABLE ZITEMCONTENT (ZITEM INTEGER, ZPLAINTEXT TEXT);
                 INSERT INTO ZITEM VALUES (1, 100, 'Browser');
                 INSERT INTO ZITEMCONTENT VALUES (1, 'from paste');",
            )
            .unwrap();
        drop(connection);

        let parsed = parse_paste(&path).unwrap();
        assert_eq!(parsed.clips.len(), 1);
        assert_eq!(parsed.clips[0].text, "from paste");
        assert_eq!(parsed.clips[0].source, "Browser");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn discovers_pastebot_database_inside_selected_folder() {
        let folder = temp_path("pastebot-folder");
        let nested = folder.join("Library/Application Support/Pastebot");
        fs::create_dir_all(&nested).unwrap();
        let path = nested.join("History.sqlite");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute(
                "CREATE TABLE ZCLIPPING (ZPLAINTEXT TEXT, ZDATE REAL, ZAPPNAME TEXT)",
                [],
            )
            .unwrap();
        connection
            .execute("INSERT INTO ZCLIPPING VALUES ('clip', 10, 'Editor')", [])
            .unwrap();
        drop(connection);

        let parsed = parse_history_folder(ExternalImportSource::Pastebot, &folder).unwrap();
        assert_eq!(parsed.clips[0].text, "clip");
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn pastebot_schema_detection_requires_a_known_content_column() {
        let path = temp_path("pastebot.sqlite");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute(
                "CREATE TABLE ZCLIPPING (ZPLAINTEXT TEXT, ZDATE REAL, ZAPPNAME TEXT)",
                [],
            )
            .unwrap();
        connection
            .execute("INSERT INTO ZCLIPPING VALUES ('clip', 10, 'Editor')", [])
            .unwrap();
        drop(connection);
        let parsed = parse_pastebot(&path).unwrap();
        assert_eq!(parsed.clips[0].text, "clip");
        fs::remove_file(path).unwrap();
    }
}
