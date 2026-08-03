use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use parking_lot::Mutex;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClipItem {
    pub id: i64,
    pub content_type: String, // "text", "image", "color", "link", "code"
    pub text_content: Option<String>,
    pub html_content: Option<String>,
    pub image_base64: Option<String>,
    pub image_path: Option<String>,
    pub content_hash: String,
    pub source_app: String,
    pub is_pinned: bool,
    pub is_protected: bool,
    pub pin_order: i32,
    pub board_id: Option<i64>,
    pub board_ids: Option<Vec<i64>>,
    pub note: Option<String>,
    pub is_trashed: bool,
    pub trashed_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActivityLog {
    pub id: i64,
    pub event_type: String,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Board {
    pub id: i64,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub smart_rule: Option<String>, // JSON string for auto-smart rules
    pub board_type: String, // "category" or "tag"
    pub shortcut: Option<String>,
    pub clip_count: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClipVersion {
    pub id: i64,
    pub clip_id: i64,
    pub text_content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStat {
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeStat {
    pub content_type: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStat {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSummary {
    pub total_clips: i64,
    pub total_chars: i64,
    pub kb_saved: f64,
    pub top_apps: Vec<AppStat>,
    pub content_types: Vec<TypeStat>,
    pub daily_activity: Vec<DailyStat>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupPayload {
    pub version: u32,
    pub timestamp: String,
    pub clips: Vec<ClipItem>,
    pub boards: Vec<Board>,
    pub filters: Vec<FilterRule>,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FilterRule {
    pub id: i64,
    pub name: String,
    pub filter_type: String,
    pub config: Option<String>,
    pub shortcut: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Operation {
    pub id: i64,
    pub name: String,
    pub op_type: String,
    pub config: Option<String>,
    pub category: String,
    pub created_at: String,
}

pub struct DbState {
    pub conn: Mutex<Connection>,
}

impl DbState {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let conn = Connection::open(db_path)?;
        let state = DbState {
            conn: Mutex::new(conn),
        };
        state.init_tables()?;
        Ok(state)
    }

    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock();

        // High-performance SQLite configuration
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        let _ = conn.pragma_update(None, "synchronous", "NORMAL");
        let _ = conn.pragma_update(None, "temp_store", "MEMORY");
        let _ = conn.pragma_update(None, "wal_autocheckpoint", "500");
        let _ = conn.pragma_update(None, "auto_vacuum", "INCREMENTAL");
        let _ = conn.pragma_update(None, "optimize", "");

        conn.execute(
            "CREATE TABLE IF NOT EXISTS clips (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content_type TEXT NOT NULL,
                text_content TEXT,
                html_content TEXT,
                image_base64 TEXT,
                content_hash TEXT UNIQUE NOT NULL,
                source_app TEXT DEFAULT 'Unknown',
                is_pinned INTEGER DEFAULT 0,
                board_id INTEGER,
                note TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // High-speed composite indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_pinned_created ON clips (is_pinned, created_at DESC)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_board_created ON clips (board_id, created_at DESC)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_hash ON clips (content_hash)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS boards (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                icon TEXT DEFAULT 'Folder',
                color TEXT DEFAULT '#3b82f6',
                smart_rule TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // Migrations if existing tables don't have new columns
        let _ = conn.execute("ALTER TABLE clips ADD COLUMN note TEXT", []);
        let _ = conn.execute("ALTER TABLE clips ADD COLUMN is_trashed INTEGER DEFAULT 0", []);
        let _ = conn.execute("ALTER TABLE clips ADD COLUMN trashed_at DATETIME", []);
        let _ = conn.execute("ALTER TABLE clips ADD COLUMN is_protected INTEGER DEFAULT 0", []);
        let _ = conn.execute("ALTER TABLE clips ADD COLUMN image_path TEXT", []);
        let _ = conn.execute("ALTER TABLE clips ADD COLUMN pin_order INTEGER DEFAULT 0", []);
        let _ = conn.execute("ALTER TABLE boards ADD COLUMN smart_rule TEXT", []);
        let _ = conn.execute("ALTER TABLE boards ADD COLUMN board_type TEXT DEFAULT 'category'", []);
        let _ = conn.execute("ALTER TABLE boards ADD COLUMN shortcut TEXT", []);
        let _ = conn.execute("ALTER TABLE filters ADD COLUMN shortcut TEXT", []);

        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS clip_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                text_content TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clip_versions_clip_id ON clip_versions(clip_id, created_at DESC)",
            [],
        );

        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_trashed ON clips (is_trashed, created_at DESC)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_protected ON clips (is_protected, created_at DESC)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_active_timeline ON clips (is_trashed, is_pinned DESC, created_at DESC)",
            [],
        );

        // FTS5 Full-Text Search Virtual Table Setup
        let fts_res = conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS clips_fts USING fts5(
                text_content,
                note,
                source_app,
                content='clips',
                content_rowid='id'
            )",
            [],
        );

        if fts_res.is_ok() {
            let _ = conn.execute(
                "CREATE TRIGGER IF NOT EXISTS clips_ai AFTER INSERT ON clips BEGIN
                    INSERT INTO clips_fts(rowid, text_content, note, source_app)
                    VALUES (new.id, new.text_content, new.note, new.source_app);
                END;",
                [],
            );
            let _ = conn.execute(
                "CREATE TRIGGER IF NOT EXISTS clips_ad AFTER DELETE ON clips BEGIN
                    INSERT INTO clips_fts(clips_fts, rowid, text_content, note, source_app)
                    VALUES ('delete', old.id, old.text_content, old.note, old.source_app);
                END;",
                [],
            );
            let _ = conn.execute(
                "CREATE TRIGGER IF NOT EXISTS clips_au AFTER UPDATE ON clips BEGIN
                    INSERT INTO clips_fts(clips_fts, rowid, text_content, note, source_app)
                    VALUES ('delete', old.id, old.text_content, old.note, old.source_app);
                    INSERT INTO clips_fts(rowid, text_content, note, source_app)
                    VALUES (new.id, new.text_content, new.note, new.source_app);
                END;",
                [],
            );

            let _ = conn.execute(
                "INSERT INTO clips_fts(rowid, text_content, note, source_app)
                 SELECT id, text_content, note, source_app FROM clips
                 WHERE id NOT IN (SELECT rowid FROM clips_fts)",
                [],
            );
        }

        conn.execute(
            "CREATE TABLE IF NOT EXISTS clip_boards (
                clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                board_id INTEGER NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
                PRIMARY KEY (clip_id, board_id)
            )",
            [],
        )?;
        let _ = conn.execute("CREATE INDEX IF NOT EXISTS idx_clip_boards_board_id ON clip_boards (board_id)", []);
        let _ = conn.execute("CREATE INDEX IF NOT EXISTS idx_clip_boards_clip_id ON clip_boards (clip_id)", []);

        let _ = conn.execute(
            "INSERT OR IGNORE INTO clip_boards (clip_id, board_id)
             SELECT id, board_id FROM clips WHERE board_id IS NOT NULL",
            [],
        );

        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS activity_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                description TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_logs_created ON activity_logs (created_at DESC)",
            [],
        );

        conn.execute(
            "CREATE TABLE IF NOT EXISTS filters (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                filter_type TEXT NOT NULL,
                config TEXT,
                shortcut TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS operations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                op_type TEXT NOT NULL,
                config TEXT,
                category TEXT DEFAULT 'Custom',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        // Insert default boards if empty
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM boards", [], |r| r.get(0)).unwrap_or(0);
        if count == 0 {
            conn.execute(
                "INSERT INTO boards (name, icon, color, smart_rule) VALUES ('Code Snippets', 'Code', '#10b981', '{\"type\":\"content_type\",\"value\":\"code\"}')",
                [],
            )?;
            conn.execute(
                "INSERT INTO boards (name, icon, color, smart_rule) VALUES ('Links & Web', 'Link', '#3b82f6', '{\"type\":\"content_type\",\"value\":\"link\"}')",
                [],
            )?;
            conn.execute(
                "INSERT INTO boards (name, icon, color, smart_rule) VALUES ('Colors & Swatches', 'Palette', '#f59e0b', '{\"type\":\"content_type\",\"value\":\"color\"}')",
                [],
            )?;
        }

        // Seed all 35 default filters as pipeline rules backed by operations
        let default_filters = [
            ("Clean URL Tracking", "clean_url_tracking"),
            ("Plain Text Only", "strip_html"),
            ("UPPERCASE", "uppercase"),
            ("lowercase", "lowercase"),
            ("Title Case", "titlecase"),
            ("Sentence case", "sentence_case"),
            ("camelCase", "camelcase"),
            ("snake_case", "snakecase"),
            ("kebab-case", "kebabcase"),
            ("CONSTANT_CASE", "constant_case"),
            ("aLtErNaTiNg cAsE", "alternating_case"),
            ("Smart Punctuation", "smart_punctuation"),
            ("Straighten Punctuation", "straighten_punctuation"),
            ("Strip Markdown", "strip_markdown"),
            ("Emoji Remover", "strip_emojis"),
            ("Convert Smileys to Emoji", "smileys_to_emoji"),
            ("Extract URLs", "extract_urls"),
            ("Extract Emails", "extract_emails"),
            ("Extract Phone Numbers", "extract_phones"),
            ("Extract IP Addresses", "extract_ips"),
            ("Sort Lines (A-Z)", "sort_lines_asc"),
            ("Sort Lines (By Length)", "sort_by_length"),
            ("Deduplicate Lines", "dedupe_lines"),
            ("Number Lines", "number_lines"),
            ("Quote Text", "quote_text"),
            ("Trim Spaces", "trim"),
            ("Strip Newlines", "strip_newlines"),
            ("Format JSON", "json_format"),
            ("Minify JSON", "json_minify"),
            ("HTML Entity Encode", "html_encode"),
            ("HTML Entity Decode", "html_decode"),
            ("Hex Encode", "hex_encode"),
            ("Hex Decode", "hex_decode"),
            ("URL Encode", "url_encode"),
            ("URL Decode", "url_decode"),
        ];

        for (name, ftype) in &default_filters {
            let exists: i64 = conn
                .query_row("SELECT COUNT(*) FROM filters WHERE name = ?1", params![name], |r| r.get(0))
                .unwrap_or(0);
            if exists == 0 {
                let pipeline_config = format!(r#"[={{"filter_type":"{}"}}=]"#, ftype).replace("=", "");
                let _ = conn.execute(
                    "INSERT INTO filters (name, filter_type, config) VALUES (?1, 'pipeline', ?2)",
                    params![name, pipeline_config],
                );
            }
        }

        // Migrate any legacy pre-existing non-pipeline filters to pipeline filters backed by operations
        if let Ok(mut stmt) = conn.prepare("SELECT id, filter_type, config FROM filters WHERE filter_type != 'pipeline'") {
            let legacy_filters: Vec<(i64, String, Option<String>)> = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .ok()
                .map(|iter| iter.filter_map(Result::ok).collect())
                .unwrap_or_default();

            for (id, ftype, cfg) in legacy_filters {
                let step_obj = if let Some(ref c) = cfg {
                    serde_json::json!({ "filter_type": ftype, "config": c })
                } else {
                    serde_json::json!({ "filter_type": ftype })
                };
                let pipeline_json = serde_json::to_string(&vec![step_obj]).unwrap_or_default();
                let _ = conn.execute(
                    "UPDATE filters SET filter_type = 'pipeline', config = ?1 WHERE id = ?2",
                    params![pipeline_json, id],
                );
            }
        }

        // Seed all built-in operations into operations table with actual Regex patterns & Shell Pipe commands
        let default_ops = [
            ("Clean URL Tracking", "regex", Some(r#"{"pattern":"([?&])(utm_[^&=]+|fbclid|gclid|msclkid|ref|source)=[^&]*&?","replacement":"$1"}"#), "Cleaners & Sanitizers"),
            ("Plain Text / Strip HTML", "strip_html", None, "Cleaners & Sanitizers"),
            ("Strip Markdown Formatting", "regex", Some(r#"{"pattern":"(\\*{1,2}|_{1,2}|`|#+\\s*|\\[([^\\]]+)\\]\\([^)]+\\))","replacement":"$2"}"#), "Cleaners & Sanitizers"),
            ("Emoji Remover", "regex", Some(r#"{"pattern":"[\\x{1F600}-\\x{1F64F}\\x{1F300}-\\x{1F5FF}\\x{1F680}-\\x{1F6FF}]","replacement":""}"#), "Cleaners & Sanitizers"),
            ("Convert Text Smileys to Emoji", "smileys_to_emoji", None, "Cleaners & Sanitizers"),
            ("Trim Whitespace", "trim", None, "Cleaners & Sanitizers"),
            ("Strip Newlines", "strip_newlines", None, "Cleaners & Sanitizers"),
            ("Smart Punctuation (“ ” — …)", "smart_punctuation", None, "Smart Formatting"),
            ("Straighten Punctuation", "straighten_punctuation", None, "Smart Formatting"),
            ("UPPERCASE", "uppercase", None, "Case Transformations"),
            ("lowercase", "lowercase", None, "Case Transformations"),
            ("Title Case", "titlecase", None, "Case Transformations"),
            ("Sentence case", "sentence_case", None, "Case Transformations"),
            ("camelCase", "camelcase", None, "Case Transformations"),
            ("snake_case", "snakecase", None, "Case Transformations"),
            ("kebab-case", "kebabcase", None, "Case Transformations"),
            ("CONSTANT_CASE", "constant_case", None, "Case Transformations"),
            ("aLtErNaTiNg cAsE", "alternating_case", None, "Case Transformations"),
            ("Extract URLs", "regex", Some(r#"{"pattern":"https?://[^\\s\\)]+","replacement":""}"#), "Data Extraction"),
            ("Extract Emails", "regex", Some(r#"{"pattern":"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}","replacement":""}"#), "Data Extraction"),
            ("Extract Phone Numbers", "regex", Some(r#"{"pattern":"\\b(?:\\+?\\d{1,3}[-.\\s]?)?\\(?\\d{3}\\)?[-.\\s]?\\d{3}[-.\\s]?\\d{4}\\b","replacement":""}"#), "Data Extraction"),
            ("Extract IP Addresses", "regex", Some(r#"{"pattern":"\\b(?:\\d{1,3}\\.){3}\\d{1,3}\\b","replacement":""}"#), "Data Extraction"),
            ("Extract Numbers", "regex", Some(r#"{"pattern":"\\b\\d+(?:\\.\\d+)?\\b","replacement":""}"#), "Data Extraction"),
            ("Sort Lines (A-Z)", "sort_lines_asc", None, "Line Operations"),
            ("Sort Lines (Z-A)", "sort_lines_desc", None, "Line Operations"),
            ("Sort Lines (By Length)", "sort_by_length", None, "Line Operations"),
            ("Deduplicate Lines", "dedupe_lines", None, "Line Operations"),
            ("Reverse Lines", "reverse_lines", None, "Line Operations"),
            ("Strip Empty Lines", "strip_empty_lines", None, "Line Operations"),
            ("Number Lines (1. 2. 3.)", "number_lines", None, "Line Operations"),
            ("Quote Text (> )", "quote_text", None, "Line Operations"),
            ("Wrap in HTML Code Tag", "wrap_tags", Some("code"), "Structure & Formatting"),
            ("Format JSON", "json_format", None, "Structure & Formatting"),
            ("Minify JSON", "json_minify", None, "Structure & Formatting"),
            ("HTML Entity Encode", "html_encode", None, "Encodings & Decodings"),
            ("HTML Entity Decode", "html_decode", None, "Encodings & Decodings"),
            ("Hex Encode", "hex_encode", None, "Encodings & Decodings"),
            ("Hex Decode", "hex_decode", None, "Encodings & Decodings"),
            ("URL Encode", "url_encode", None, "Encodings & Decodings"),
            ("URL Decode", "url_decode", None, "Encodings & Decodings"),
            ("Shell Script (TR Uppercase Pipe)", "shell_script", Some("tr \"a-z\" \"A-Z\""), "Advanced & Shell Scripts"),
            ("External Script / OCR Pipe Engine", "shell_script", Some("tesseract stdin stdout 2>/dev/null || cat"), "Advanced & Shell Scripts"),
        ];

        for (name, optype, cfg, category) in &default_ops {
            let exists: i64 = conn
                .query_row("SELECT COUNT(*) FROM operations WHERE op_type = ?1 AND name = ?2", params![optype, name], |r| r.get(0))
                .unwrap_or(0);
            if exists == 0 {
                let _ = conn.execute(
                    "INSERT INTO operations (name, op_type, config, category) VALUES (?1, ?2, ?3, ?4)",
                    params![name, optype, cfg, category],
                );
            } else {
                // Always sync and organize category for seeded operations
                let _ = conn.execute(
                    "UPDATE operations SET category = ?1 WHERE op_type = ?2 AND name = ?3",
                    params![category, optype, name],
                );
            }
        }

        // Fill any remaining unassigned operation categories with 'Custom Operations'
        let _ = conn.execute(
            "UPDATE operations SET category = 'Custom Operations' WHERE category IS NULL OR category = '' OR category = 'Custom'",
            [],
        );

        Ok(())
    }

    pub fn get_total_clip_count(&self) -> Result<i64> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT COUNT(*) FROM clips WHERE is_trashed IS NULL OR is_trashed = 0",
            [],
            |r| r.get(0),
        )
    }

    pub fn save_clip(
        &self,
        content_type: &str,
        text_content: Option<&str>,
        html_content: Option<&str>,
        image_base64: Option<&str>,
        content_hash: &str,
        source_app: &str,
    ) -> Result<ClipItem> {
        let conn = self.conn.lock();

        let existing: Result<i64> = conn.query_row(
            "SELECT id FROM clips WHERE content_hash = ?1",
            params![content_hash],
            |r| r.get(0),
        );

        if let Ok(id) = existing {
            conn.execute(
                "UPDATE clips SET created_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), is_trashed = 0, trashed_at = NULL WHERE id = ?1",
                params![id],
            )?;
            return self.get_clip_by_id_internal(&conn, id);
        }

        conn.execute(
            "INSERT INTO clips (content_type, text_content, html_content, image_base64, content_hash, source_app, created_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
            params![content_type, text_content, html_content, image_base64, content_hash, source_app],
        )?;

        let id = conn.last_insert_rowid();
        let _ = self.enforce_history_limit_internal(&conn);
        let _ = self.enforce_trash_limit_internal(&conn);
        self.get_clip_by_id_internal(&conn, id)
    }

    pub fn enforce_history_limit_internal(&self, conn: &Connection) -> Result<()> {
        let keep_count: i64 = conn
            .query_row("SELECT value FROM settings WHERE key = 'keepClipCount'", [], |r| r.get(0))
            .ok()
            .and_then(|v: String| v.parse().ok())
            .unwrap_or(900);

        self.enforce_history_limit_with_count_internal(conn, keep_count)
    }

    fn enforce_history_limit_with_count_internal(
        &self,
        conn: &Connection,
        keep_count: i64,
    ) -> Result<()> {
        let keep_count = keep_count.max(0);

        let enable_trash: String = conn
            .query_row("SELECT value FROM settings WHERE key = 'enableTrash'", [], |r| r.get(0))
            .unwrap_or_else(|_| "true".to_string());

        let active_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM clips
             WHERE is_pinned = 0
               AND (is_protected IS NULL OR is_protected = 0)
               AND (is_trashed IS NULL OR is_trashed = 0)",
            [],
            |r| r.get(0),
        ).unwrap_or(0);

        if active_count > keep_count {
            let excess = active_count - keep_count;
            let mut stmt = conn.prepare(
                "SELECT id FROM clips
                 WHERE is_pinned = 0
                   AND (is_protected IS NULL OR is_protected = 0)
                   AND (is_trashed IS NULL OR is_trashed = 0)
                 ORDER BY created_at ASC, id ASC LIMIT ?1"
            )?;
            let ids: Vec<i64> = stmt
                .query_map(params![excess], |r| r.get(0))?
                .filter_map(|r| r.ok())
                .collect();

            for id in ids {
                if enable_trash == "true" {
                    let _ = conn.execute(
                        "UPDATE clips SET is_trashed = 1, trashed_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?1",
                        params![id],
                    );
                    let _ = self.log_activity_internal(
                        conn,
                        "clip_auto_trashed",
                        &format!("Auto-trashed clip #{} (history retention limit exceeded)", id),
                    );
                } else {
                    let _ = conn.execute("DELETE FROM clips WHERE id = ?1", params![id]);
                    let _ = self.log_activity_internal(
                        conn,
                        "clip_deleted",
                        &format!("Auto-purged clip #{} (history retention limit exceeded)", id),
                    );
                }
            }
        }
        Ok(())
    }

    pub fn enforce_trash_limit_internal(&self, conn: &Connection) -> Result<()> {
        let capacity: i64 = conn
            .query_row("SELECT value FROM settings WHERE key = 'trashCapacityCount'", [], |r| r.get(0))
            .ok()
            .and_then(|v: String| v.parse().ok())
            .unwrap_or(500);

        let _ = conn.execute(
            "DELETE FROM clips
             WHERE is_trashed = 1
               AND (is_protected IS NULL OR is_protected = 0)
               AND id NOT IN (
                   SELECT id FROM clips
                   WHERE is_trashed = 1 AND (is_protected IS NULL OR is_protected = 0)
                   ORDER BY trashed_at DESC, id DESC LIMIT ?1
               )",
            params![capacity],
        );
        Ok(())
    }

    pub fn get_clip_image(&self, id: i64) -> Result<Option<String>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT image_base64 FROM clips WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
    }

    fn get_clip_by_id_internal(&self, conn: &Connection, id: i64) -> Result<ClipItem> {
        conn.query_row(
            "SELECT id, content_type, text_content, html_content, image_base64, image_path, content_hash, source_app, is_pinned, is_protected, COALESCE(pin_order, 0), board_id, note, is_trashed, trashed_at, created_at 
             FROM clips WHERE id = ?1",
            params![id],
            |row| {
                let bid: Option<i64> = row.get(11)?;
                Ok(ClipItem {
                    id: row.get(0)?,
                    content_type: row.get(1)?,
                    text_content: row.get(2)?,
                    html_content: row.get(3)?,
                    image_base64: row.get(4)?,
                    image_path: row.get(5)?,
                    content_hash: row.get(6)?,
                    source_app: row.get(7)?,
                    is_pinned: row.get::<_, i32>(8)? != 0,
                    is_protected: row.get::<_, i32>(9)? != 0,
                    pin_order: row.get(10)?,
                    board_id: bid,
                    board_ids: bid.map(|b| vec![b]),
                    note: row.get(12)?,
                    is_trashed: row.get::<_, i32>(13)? != 0,
                    trashed_at: row.get(14)?,
                    created_at: row.get(15)?,
                })
            },
        )
    }

    pub fn get_clips(&self, search_query: Option<&str>, board_id: Option<i64>, only_pinned: bool) -> Result<Vec<ClipItem>> {
        let conn = self.conn.lock();

        // Check if target board has smart_rule
        let mut smart_rule_str: Option<String> = None;
        if let Some(bid) = board_id {
            let res: Result<Option<String>> = conn.query_row(
                "SELECT smart_rule FROM boards WHERE id = ?1",
                params![bid],
                |r| r.get(0),
            );
            if let Ok(sr) = res {
                smart_rule_str = sr;
            }
        }

        let mut sql = String::from(
            "SELECT id, content_type, text_content, NULL as html_content, image_base64, image_path, content_hash, source_app, is_pinned, is_protected, COALESCE(pin_order, 0), board_id, note, is_trashed, trashed_at, created_at,
             (SELECT GROUP_CONCAT(board_id) FROM clip_boards WHERE clip_id = clips.id) as board_ids_str
             FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0)"
        );

        let mut query_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if only_pinned {
            sql.push_str(" AND is_pinned = 1");
        }

        if let Some(ref sr_json) = smart_rule_str {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(sr_json) {
                let match_mode = parsed["match"].as_str().unwrap_or("any");
                let is_and = match_mode == "all";
                let join_op = if is_and { " AND " } else { " OR " };

                let mut cond_sqls: Vec<String> = Vec::new();

                if let Some(conds) = parsed["conditions"].as_array() {
                    for cond in conds {
                        let c_type = cond["type"].as_str().unwrap_or("");
                        let c_val = cond["value"].as_str().unwrap_or("");
                        if !c_val.trim().is_empty() {
                            if c_type == "content_type" {
                                cond_sqls.push("content_type = ?".to_string());
                                query_params.push(Box::new(c_val.to_string()));
                            } else if c_type == "source_app" {
                                cond_sqls.push("source_app LIKE ?".to_string());
                                query_params.push(Box::new(format!("%{}%", c_val)));
                            } else if c_type == "contains" {
                                cond_sqls.push("text_content LIKE ?".to_string());
                                query_params.push(Box::new(format!("%{}%", c_val)));
                            }
                        }
                    }
                } else {
                    let rule_type = parsed["type"].as_str().unwrap_or("");
                    let rule_val = parsed["value"].as_str().unwrap_or("");
                    if !rule_val.trim().is_empty() {
                        if rule_type == "content_type" {
                            cond_sqls.push("content_type = ?".to_string());
                            query_params.push(Box::new(rule_val.to_string()));
                        } else if rule_type == "source_app" {
                            cond_sqls.push("source_app LIKE ?".to_string());
                            query_params.push(Box::new(format!("%{}%", rule_val)));
                        } else if rule_type == "contains" {
                            cond_sqls.push("text_content LIKE ?".to_string());
                            query_params.push(Box::new(format!("%{}%", rule_val)));
                        }
                    }
                }

                if !cond_sqls.is_empty() {
                    let combined = cond_sqls.join(join_op);
                    if let Some(bid) = board_id {
                        sql.push_str(&format!(" AND (({}) OR board_id = ? OR id IN (SELECT clip_id FROM clip_boards WHERE board_id = ?))", combined));
                        query_params.push(Box::new(bid));
                        query_params.push(Box::new(bid));
                    } else {
                        sql.push_str(&format!(" AND ({})", combined));
                    }
                } else if let Some(bid) = board_id {
                    sql.push_str(" AND (board_id = ? OR id IN (SELECT clip_id FROM clip_boards WHERE board_id = ?))");
                    query_params.push(Box::new(bid));
                    query_params.push(Box::new(bid));
                }
            } else if let Some(bid) = board_id {
                sql.push_str(" AND (board_id = ? OR id IN (SELECT clip_id FROM clip_boards WHERE board_id = ?))");
                query_params.push(Box::new(bid));
                query_params.push(Box::new(bid));
            }
        } else if let Some(bid) = board_id {
            sql.push_str(" AND (board_id = ? OR id IN (SELECT clip_id FROM clip_boards WHERE board_id = ?))");
            query_params.push(Box::new(bid));
            query_params.push(Box::new(bid));
        }

        if let Some(q) = search_query {
            let cleaned = q.trim();
            if !cleaned.is_empty() {
                let fts_query = cleaned.replace('"', "\"\"").replace('*', "");
                if !fts_query.trim().is_empty() {
                    sql.push_str(" AND (id IN (SELECT rowid FROM clips_fts WHERE clips_fts MATCH ?) OR content_type LIKE ?)");
                    query_params.push(Box::new(format!("\"{}\"*", fts_query)));
                    query_params.push(Box::new(format!("%{}%", cleaned)));
                } else {
                    sql.push_str(" AND (text_content LIKE ? OR source_app LIKE ? OR content_type LIKE ? OR note LIKE ?)");
                    let pattern = format!("%{}%", cleaned);
                    query_params.push(Box::new(pattern.clone()));
                    query_params.push(Box::new(pattern.clone()));
                    query_params.push(Box::new(pattern.clone()));
                    query_params.push(Box::new(pattern));
                }
            }
        }

        sql.push_str(" ORDER BY is_pinned DESC, pin_order ASC, created_at DESC");

        let param_refs: Vec<&dyn rusqlite::ToSql> = query_params.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let clip_iter = stmt.query_map(param_refs.as_slice(), |row| {
            let primary_bid: Option<i64> = row.get(11)?;
            let board_ids_str: Option<String> = row.get(16)?;
            let mut b_ids = Vec::new();
            if let Some(b) = primary_bid {
                b_ids.push(b);
            }
            if let Some(ref s) = board_ids_str {
                for part in s.split(',') {
                    if let Ok(parsed_id) = part.parse::<i64>() {
                        if !b_ids.contains(&parsed_id) {
                            b_ids.push(parsed_id);
                        }
                    }
                }
            }

            Ok(ClipItem {
                id: row.get(0)?,
                content_type: row.get(1)?,
                text_content: row.get(2)?,
                html_content: row.get(3)?,
                image_base64: row.get(4)?,
                image_path: row.get(5)?,
                content_hash: row.get(6)?,
                source_app: row.get(7)?,
                is_pinned: row.get::<_, i32>(8)? != 0,
                is_protected: row.get::<_, i32>(9)? != 0,
                pin_order: row.get(10)?,
                board_id: primary_bid,
                board_ids: Some(b_ids),
                note: row.get(12)?,
                is_trashed: row.get::<_, i32>(13)? != 0,
                trashed_at: row.get(14)?,
                created_at: row.get(15)?,
            })
        })?;

        let mut clips = Vec::new();
        for clip in clip_iter {
            clips.push(clip?);
        }
        Ok(clips)
    }

    pub fn get_trashed_clips(&self) -> Result<Vec<ClipItem>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare_cached(
            "SELECT id, content_type, text_content, NULL as html_content, NULL as image_base64, image_path, content_hash, source_app, is_pinned, is_protected, COALESCE(pin_order, 0), board_id, note, is_trashed, trashed_at, created_at 
             FROM clips WHERE is_trashed = 1 ORDER BY trashed_at DESC"
        )?;
        let clip_iter = stmt.query_map([], |row| {
            let bid: Option<i64> = row.get(11)?;
            Ok(ClipItem {
                id: row.get(0)?,
                content_type: row.get(1)?,
                text_content: row.get(2)?,
                html_content: row.get(3)?,
                image_base64: row.get(4)?,
                image_path: row.get(5)?,
                content_hash: row.get(6)?,
                source_app: row.get(7)?,
                is_pinned: row.get::<_, i32>(8)? != 0,
                is_protected: row.get::<_, i32>(9)? != 0,
                pin_order: row.get(10)?,
                board_id: bid,
                board_ids: bid.map(|b| vec![b]),
                note: row.get(12)?,
                is_trashed: row.get::<_, i32>(13)? != 0,
                trashed_at: row.get(14)?,
                created_at: row.get(15)?,
            })
        })?;
        let mut clips = Vec::new();
        for clip in clip_iter {
            clips.push(clip?);
        }
        Ok(clips)
    }

    pub fn get_protected_clips(&self) -> Result<Vec<ClipItem>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare_cached(
            "SELECT id, content_type, text_content, NULL as html_content, NULL as image_base64, image_path, content_hash, source_app, is_pinned, is_protected, COALESCE(pin_order, 0), board_id, note, is_trashed, trashed_at, created_at 
             FROM clips WHERE is_protected = 1 AND (is_trashed IS NULL OR is_trashed = 0) ORDER BY created_at DESC"
        )?;
        let clip_iter = stmt.query_map([], |row| {
            let bid: Option<i64> = row.get(11)?;
            Ok(ClipItem {
                id: row.get(0)?,
                content_type: row.get(1)?,
                text_content: row.get(2)?,
                html_content: row.get(3)?,
                image_base64: row.get(4)?,
                image_path: row.get(5)?,
                content_hash: row.get(6)?,
                source_app: row.get(7)?,
                is_pinned: row.get::<_, i32>(8)? != 0,
                is_protected: row.get::<_, i32>(9)? != 0,
                pin_order: row.get(10)?,
                board_id: bid,
                board_ids: bid.map(|b| vec![b]),
                note: row.get(12)?,
                is_trashed: row.get::<_, i32>(13)? != 0,
                trashed_at: row.get(14)?,
                created_at: row.get(15)?,
            })
        })?;
        let mut clips = Vec::new();
        for clip in clip_iter {
            clips.push(clip?);
        }
        Ok(clips)
    }

    pub fn update_clip_note(&self, clip_id: i64, note: Option<&str>) -> Result<()> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare_cached("UPDATE clips SET note = ?1 WHERE id = ?2")?;
        stmt.execute(params![note, clip_id])?;
        let _ = self.log_activity_internal(&conn, "note_updated", &format!("Updated note for clip #{}", clip_id));
        Ok(())
    }

    pub fn update_clip_text(&self, clip_id: i64, text: &str) -> Result<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let previous_text: Option<String> = tx.query_row(
            "SELECT text_content FROM clips WHERE id = ?1",
            params![clip_id],
            |row| row.get(0),
        )?;

        if previous_text.as_deref() == Some(text) {
            return tx.commit();
        }

        if let Some(previous_text) = previous_text {
            tx.execute(
                "INSERT INTO clip_versions (clip_id, text_content) VALUES (?1, ?2)",
                params![clip_id, previous_text],
            )?;
            tx.execute(
                "DELETE FROM clip_versions
                 WHERE clip_id = ?1
                   AND id NOT IN (
                       SELECT id FROM clip_versions
                       WHERE clip_id = ?1
                       ORDER BY id DESC
                       LIMIT 50
                   )",
                params![clip_id],
            )?;
        }
        tx.execute(
            "UPDATE clips SET text_content = ?1 WHERE id = ?2",
            params![text, clip_id],
        )?;
        tx.commit()
    }

    pub fn delete_clip(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        let is_protected: i32 = conn.query_row("SELECT is_protected FROM clips WHERE id = ?1", params![id], |r| r.get(0)).unwrap_or(0);
        if is_protected != 0 {
            return Ok(());
        }
        let mut stmt = conn.prepare_cached(
            "UPDATE clips SET is_trashed = 1, trashed_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?1 AND (is_protected IS NULL OR is_protected = 0)"
        )?;
        stmt.execute(params![id])?;
        let _ = self.log_activity_internal(&conn, "clip_trashed", &format!("Moved clip #{} to Trash", id));
        let _ = self.enforce_trash_limit_internal(&conn);
        Ok(())
    }

    pub fn restore_clip(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare_cached(
            "UPDATE clips SET is_trashed = 0, trashed_at = NULL WHERE id = ?1"
        )?;
        stmt.execute(params![id])?;
        let _ = self.log_activity_internal(&conn, "clip_restored", &format!("Restored clip #{} from Trash", id));
        Ok(())
    }

    pub fn purge_clip_permanently(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        let is_protected: i32 = conn.query_row("SELECT is_protected FROM clips WHERE id = ?1", params![id], |r| r.get(0)).unwrap_or(0);
        if is_protected != 0 {
            return Ok(());
        }
        let mut stmt = conn.prepare_cached("DELETE FROM clips WHERE id = ?1 AND (is_protected IS NULL OR is_protected = 0)")?;
        stmt.execute(params![id])?;
        let _ = self.log_activity_internal(&conn, "clip_deleted", &format!("Permanently deleted clip #{}", id));
        Ok(())
    }

    pub fn empty_trash(&self) -> Result<()> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM clips WHERE is_trashed = 1 AND (is_protected IS NULL OR is_protected = 0)",
            [],
            |r| r.get(0),
        ).unwrap_or(0);
        let mut stmt = conn.prepare_cached(
            "DELETE FROM clips WHERE is_trashed = 1 AND (is_protected IS NULL OR is_protected = 0)"
        )?;
        stmt.execute([])?;
        let _ = self.log_activity_internal(&conn, "trash_emptied", &format!("Emptied Trash (permanently deleted {} items)", count));
        Ok(())
    }

    pub fn log_activity(&self, event_type: &str, description: &str) -> Result<()> {
        let conn = self.conn.lock();
        self.log_activity_internal(&conn, event_type, description)
    }

    fn log_activity_internal(&self, conn: &Connection, event_type: &str, description: &str) -> Result<()> {
        let is_enabled: String = conn
            .query_row("SELECT value FROM settings WHERE key = 'enableActivityLog'", [], |r| r.get(0))
            .unwrap_or_else(|_| "true".to_string());
        if is_enabled == "false" {
            return Ok(());
        }

        let capacity: i64 = conn
            .query_row("SELECT value FROM settings WHERE key = 'activityLogCapacity'", [], |r| r.get(0))
            .ok()
            .and_then(|v: String| v.parse().ok())
            .unwrap_or(1000);

        let mut stmt = conn.prepare_cached(
            "INSERT INTO activity_logs (event_type, description, created_at) VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))"
        )?;
        stmt.execute(params![event_type, description])?;

        let mut purge_stmt = conn.prepare_cached(
            "DELETE FROM activity_logs WHERE id NOT IN (SELECT id FROM activity_logs ORDER BY created_at DESC, id DESC LIMIT ?1)"
        )?;
        let _ = purge_stmt.execute(params![capacity]);
        Ok(())
    }

    pub fn get_activity_logs(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<ActivityLog>> {
        let conn = self.conn.lock();
        let lim = limit.unwrap_or(100);
        let off = offset.unwrap_or(0);
        let mut stmt = conn.prepare_cached("SELECT id, event_type, description, created_at FROM activity_logs ORDER BY created_at DESC, id DESC LIMIT ?1 OFFSET ?2")?;
        let log_iter = stmt.query_map(params![lim, off], |row| {
            Ok(ActivityLog {
                id: row.get(0)?,
                event_type: row.get(1)?,
                description: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        let mut logs = Vec::new();
        for log in log_iter {
            logs.push(log?);
        }
        Ok(logs)
    }

    pub fn clear_activity_logs(&self) -> Result<()> {
        let conn = self.conn.lock();
        let _ = conn.execute("DELETE FROM activity_logs", [])?;
        Ok(())
    }

    pub fn backfill_analytics(&self) -> Result<usize> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT id, content_type, text_content, source_app FROM clips")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;

        let mut updated_count = 0;
        for row in rows {
            let (id, c_type, text_opt, source_opt) = row?;
            let source = source_opt.unwrap_or_default();
            if source.is_empty() || source == "System Clipboard" || source == "Unknown" {
                let text = text_opt.unwrap_or_default();
                let inferred_app = if c_type == "code"
                    || text.contains("function ")
                    || text.contains("const ")
                    || text.contains("let ")
                    || text.contains("import ")
                    || text.contains("pub fn ")
                    || text.contains("class ")
                {
                    "VS Code"
                } else if c_type == "link" || text.starts_with("http://") || text.starts_with("https://") {
                    "Browser"
                } else if c_type == "color" || text.starts_with('#') {
                    "Color Picker"
                } else if c_type == "image" {
                    "Screenshot"
                } else {
                    "macOS System"
                };

                conn.execute(
                    "UPDATE clips SET source_app = ?1 WHERE id = ?2",
                    params![inferred_app, id],
                )?;
                updated_count += 1;
            }
        }
        Ok(updated_count)
    }
    pub fn batch_pin_clips(&self, ids: Vec<i64>, pin_state: bool) -> Result<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let val = if pin_state { 1 } else { 0 };
        for id in ids {
            tx.execute("UPDATE clips SET is_pinned = ?1 WHERE id = ?2", params![val, id])?;
        }
        tx.commit()
    }

    pub fn batch_trash_clips(&self, ids: Vec<i64>) -> Result<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        for id in ids {
            tx.execute(
                "UPDATE clips SET is_trashed = 1, trashed_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?1 AND (is_protected IS NULL OR is_protected = 0)",
                params![id],
            )?;
        }
        self.enforce_trash_limit_internal(&tx)?;
        tx.commit()
    }

    pub fn batch_assign_board_clips(&self, ids: Vec<i64>, board_id: Option<i64>) -> Result<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        for clip_id in ids {
            tx.execute(
                "DELETE FROM clip_boards
                 WHERE clip_id = ?1
                   AND board_id IN (
                       SELECT id FROM boards WHERE COALESCE(board_type, 'category') != 'tag'
                   )",
                params![clip_id],
            )?;
            if let Some(bid) = board_id {
                tx.execute(
                    "INSERT OR REPLACE INTO clip_boards (clip_id, board_id) VALUES (?1, ?2)",
                    params![clip_id, bid],
                )?;
                tx.execute(
                    "UPDATE clips SET board_id = ?1 WHERE id = ?2",
                    params![bid, clip_id],
                )?;
            } else {
                tx.execute("UPDATE clips SET board_id = NULL WHERE id = ?1", params![clip_id])?;
            }
        }
        tx.commit()
    }

    pub fn get_analytics_summary(&self) -> Result<AnalyticsSummary> {
        self.backfill_analytics()?;
        let conn = self.conn.lock();

        let (total_clips, total_chars): (i64, i64) = conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(LENGTH(text_content)), 0) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0)",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap_or((0, 0));

        let kb_saved = ((total_chars as f64 * 1.2) / 1024.0 * 10.0).round() / 10.0;

        let mut app_stmt = conn.prepare(
            "SELECT source_app, COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) GROUP BY source_app ORDER BY COUNT(*) DESC LIMIT 8"
        )?;
        let top_apps = app_stmt.query_map([], |r| {
            Ok(AppStat {
                name: r.get(0)?,
                count: r.get(1)?,
            })
        })?.filter_map(|r| r.ok()).collect();

        let mut type_stmt = conn.prepare(
            "SELECT content_type, COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) GROUP BY content_type"
        )?;
        let content_types = type_stmt.query_map([], |r| {
            Ok(TypeStat {
                content_type: r.get(0)?,
                count: r.get(1)?,
            })
        })?.filter_map(|r| r.ok()).collect();

        let mut daily_stmt = conn.prepare(
            "SELECT strftime('%Y-%m-%d', created_at) as day, COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) GROUP BY day ORDER BY day DESC LIMIT 14"
        )?;
        let daily_activity = daily_stmt.query_map([], |r| {
            Ok(DailyStat {
                date: r.get(0)?,
                count: r.get(1)?,
            })
        })?.filter_map(|r| r.ok()).collect();

        Ok(AnalyticsSummary {
            total_clips,
            total_chars,
            kb_saved,
            top_apps,
            content_types,
            daily_activity,
        })
    }

    pub fn trash_unpinned_clips(&self) -> Result<()> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM clips WHERE is_pinned = 0 AND (is_protected IS NULL OR is_protected = 0) AND (is_trashed IS NULL OR is_trashed = 0)", [], |r| r.get(0)).unwrap_or(0);
        conn.execute(
            "UPDATE clips SET is_trashed = 1, trashed_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE is_pinned = 0 AND (is_protected IS NULL OR is_protected = 0) AND (is_trashed IS NULL OR is_trashed = 0)",
            [],
        )?;
        let _ = self.log_activity_internal(&conn, "clips_trashed_all", &format!("Moved all unpinned & unprotected clips to Trash ({} items)", count));
        let _ = self.enforce_trash_limit_internal(&conn);
        Ok(())
    }

    pub fn purge_unpinned_clips(&self) -> Result<()> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM clips WHERE is_pinned = 0 AND (is_protected IS NULL OR is_protected = 0)", [], |r| r.get(0)).unwrap_or(0);
        conn.execute("DELETE FROM clips WHERE is_pinned = 0 AND (is_protected IS NULL OR is_protected = 0)", [])?;
        let _ = self.log_activity_internal(&conn, "clips_purged_all", &format!("Permanently deleted all unpinned & unprotected clips ({} items)", count));
        Ok(())
    }

    pub fn clear_all_clips(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM clips WHERE (is_protected IS NULL OR is_protected = 0)", [])?;
        Ok(())
    }

    pub fn toggle_protected(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock();
        let current_protected: i32 = conn.query_row(
            "SELECT is_protected FROM clips WHERE id = ?1",
            params![id],
            |r| r.get(0),
        ).unwrap_or(0);
        let new_protected = if current_protected == 0 { 1 } else { 0 };
        conn.execute(
            "UPDATE clips SET is_protected = ?1 WHERE id = ?2",
            params![new_protected, id],
        )?;
        let action_str = if new_protected == 1 { "Protected" } else { "Unprotected" };
        let _ = self.log_activity_internal(&conn, "clip_protected_toggled", &format!("{} clip #{}", action_str, id));
        Ok(new_protected == 1)
    }

    pub fn toggle_pin(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock();
        let current_pinned: i32 = conn.query_row(
            "SELECT is_pinned FROM clips WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )?;
        let new_pinned = if current_pinned == 0 { 1 } else { 0 };
        conn.execute(
            "UPDATE clips SET is_pinned = ?1 WHERE id = ?2",
            params![new_pinned, id],
        )?;
        Ok(new_pinned == 1)
    }

    pub fn assign_to_board(&self, clip_id: i64, board_id: Option<i64>) -> Result<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM clip_boards
             WHERE clip_id = ?1
               AND board_id IN (
                   SELECT id FROM boards WHERE COALESCE(board_type, 'category') != 'tag'
               )",
            params![clip_id],
        )?;
        if let Some(bid) = board_id {
            tx.execute(
                "INSERT OR REPLACE INTO clip_boards (clip_id, board_id) VALUES (?1, ?2)",
                params![clip_id, bid],
            )?;
            tx.execute(
                "UPDATE clips SET board_id = ?1 WHERE id = ?2",
                params![bid, clip_id],
            )?;
        } else {
            tx.execute("UPDATE clips SET board_id = NULL WHERE id = ?1", params![clip_id])?;
        }
        tx.commit()
    }

    pub fn add_clip_to_board(&self, clip_id: i64, board_id: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO clip_boards (clip_id, board_id) VALUES (?1, ?2)",
            params![clip_id, board_id],
        )?;
        conn.execute(
            "UPDATE clips SET board_id = ?1 WHERE id = ?2",
            params![board_id, clip_id],
        )?;
        Ok(())
    }

    pub fn remove_clip_from_board(&self, clip_id: i64, board_id: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM clip_boards WHERE clip_id = ?1 AND board_id = ?2",
            params![clip_id, board_id],
        )?;
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    pub fn get_boards(&self) -> Result<Vec<Board>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT id, name, icon, color, smart_rule, COALESCE(board_type, 'category'), shortcut, created_at FROM boards ORDER BY id ASC")?;
        let board_rows: Vec<(i64, String, String, String, Option<String>, String, Option<String>, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut boards = Vec::new();
        for (id, name, icon, color, smart_rule, board_type, shortcut, created_at) in board_rows {
            let count: i64 = if let Some(ref sr_json) = smart_rule {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(sr_json) {
                    let match_mode = parsed["match"].as_str().unwrap_or("any");
                    let is_and = match_mode == "all";
                    let join_op = if is_and { " AND " } else { " OR " };

                    let mut cond_sqls: Vec<String> = Vec::new();
                    let mut query_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

                    if let Some(conds) = parsed["conditions"].as_array() {
                        for cond in conds {
                            let c_type = cond["type"].as_str().unwrap_or("");
                            let c_val = cond["value"].as_str().unwrap_or("");
                            if !c_val.trim().is_empty() {
                                if c_type == "content_type" {
                                    cond_sqls.push("content_type = ?".to_string());
                                    query_params.push(Box::new(c_val.to_string()));
                                } else if c_type == "source_app" {
                                    cond_sqls.push("source_app LIKE ?".to_string());
                                    query_params.push(Box::new(format!("%{}%", c_val)));
                                } else if c_type == "contains" {
                                    cond_sqls.push("text_content LIKE ?".to_string());
                                    query_params.push(Box::new(format!("%{}%", c_val)));
                                }
                            }
                        }
                    } else {
                        let rule_type = parsed["type"].as_str().unwrap_or("");
                        let rule_val = parsed["value"].as_str().unwrap_or("");
                        if !rule_val.trim().is_empty() {
                            if rule_type == "content_type" {
                                cond_sqls.push("content_type = ?".to_string());
                                query_params.push(Box::new(rule_val.to_string()));
                            } else if rule_type == "source_app" {
                                cond_sqls.push("source_app LIKE ?".to_string());
                                query_params.push(Box::new(format!("%{}%", rule_val)));
                            } else if rule_type == "contains" {
                                cond_sqls.push("text_content LIKE ?".to_string());
                                query_params.push(Box::new(format!("%{}%", rule_val)));
                            }
                        }
                    }

                    if !cond_sqls.is_empty() {
                        let combined = cond_sqls.join(join_op);
                        let sql = format!("SELECT COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) AND (({}) OR board_id = ? OR id IN (SELECT clip_id FROM clip_boards WHERE board_id = ?))", combined);
                        query_params.push(Box::new(id));
                        query_params.push(Box::new(id));
                        let param_refs: Vec<&dyn rusqlite::ToSql> = query_params.iter().map(|p| p.as_ref()).collect();
                        conn.query_row(&sql, param_refs.as_slice(), |r| r.get(0)).unwrap_or(0)
                    } else {
                        conn.query_row("SELECT COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) AND (board_id = ?1 OR id IN (SELECT clip_id FROM clip_boards WHERE board_id = ?1))", params![id], |r| r.get(0)).unwrap_or(0)
                    }
                } else {
                    conn.query_row("SELECT COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) AND (board_id = ?1 OR id IN (SELECT clip_id FROM clip_boards WHERE board_id = ?1))", params![id], |r| r.get(0)).unwrap_or(0)
                }
            } else {
                conn.query_row("SELECT COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) AND (board_id = ?1 OR id IN (SELECT clip_id FROM clip_boards WHERE board_id = ?1))", params![id], |r| r.get(0)).unwrap_or(0)
            };

            boards.push(Board {
                id,
                name,
                icon,
                color,
                smart_rule,
                board_type,
                shortcut,
                clip_count: Some(count),
                created_at,
            });
        }
        Ok(boards)
    }

    pub fn update_board_shortcut(&self, id: i64, shortcut: Option<&str>) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("UPDATE boards SET shortcut = ?1 WHERE id = ?2", params![shortcut, id])?;
        Ok(())
    }

    pub fn create_board_with_type(&self, name: &str, icon: &str, color: &str, smart_rule: Option<&str>, board_type: &str) -> Result<Board> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO boards (name, icon, color, smart_rule, board_type) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![name, icon, color, smart_rule, board_type],
        )?;
        let id = conn.last_insert_rowid();
        conn.query_row(
            "SELECT id, name, icon, color, smart_rule, COALESCE(board_type, 'category'), shortcut, created_at FROM boards WHERE id = ?1",
            params![id],
            |row| {
                Ok(Board {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    icon: row.get(2)?,
                    color: row.get(3)?,
                    smart_rule: row.get(4)?,
                    board_type: row.get(5)?,
                    shortcut: row.get(6)?,
                    clip_count: Some(0),
                    created_at: row.get(7)?,
                })
            },
        )
    }

    pub fn create_board(&self, name: &str, icon: &str, color: &str, smart_rule: Option<&str>) -> Result<Board> {
        self.create_board_with_type(name, icon, color, smart_rule, "category")
    }

    pub fn reorder_pinned_clips(&self, ids: Vec<i64>) -> Result<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        for (idx, id) in ids.iter().enumerate() {
            tx.execute(
                "UPDATE clips SET pin_order = ?1 WHERE id = ?2",
                params![idx as i32, id],
            )?;
        }
        tx.commit()
    }

    pub fn get_clip_versions(&self, clip_id: i64) -> Result<Vec<ClipVersion>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, clip_id, text_content, created_at FROM clip_versions WHERE clip_id = ?1 ORDER BY created_at DESC, id DESC",
        )?;
        let rows = stmt.query_map(params![clip_id], |row| {
            Ok(ClipVersion {
                id: row.get(0)?,
                clip_id: row.get(1)?,
                text_content: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn update_board(&self, id: i64, name: &str, icon: &str, color: &str, smart_rule: Option<&str>) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE boards SET name = ?1, icon = ?2, color = ?3, smart_rule = ?4 WHERE id = ?5",
            params![name, icon, color, smart_rule, id],
        )?;
        Ok(())
    }

    pub fn delete_board(&self, id: i64) -> Result<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM clip_boards WHERE board_id = ?1", params![id])?;
        tx.execute("UPDATE clips SET board_id = NULL WHERE board_id = ?1", params![id])?;
        tx.execute("DELETE FROM boards WHERE id = ?1", params![id])?;
        tx.commit()
    }

    pub fn clear_history(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM clips WHERE is_pinned = 0 AND (is_protected IS NULL OR is_protected = 0)",
            [],
        )?;
        Ok(())
    }

    pub fn export_backup_json(&self) -> Result<String> {
        let clips = self.get_all_clips_for_backup()?;
        let boards = self.get_boards()?;
        let filters = self.get_filters()?;
        let operations = self.get_operations()?;

        let payload = BackupPayload {
            version: 2,
            timestamp: chrono::Utc::now().to_rfc3339(),
            clips,
            boards,
            filters,
            operations,
        };

        serde_json::to_string_pretty(&payload).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
    }

    pub fn import_backup_json(&self, json_str: &str) -> Result<usize> {
        let payload: BackupPayload = serde_json::from_str(json_str)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let mut board_id_map = std::collections::HashMap::new();

        for board in payload.boards {
            let existing_id = tx.query_row(
                "SELECT id FROM boards WHERE name = ?1 AND COALESCE(board_type, 'category') = ?2 LIMIT 1",
                params![board.name, board.board_type],
                |row| row.get::<_, i64>(0),
            ).ok();
            let new_id = if let Some(id) = existing_id {
                tx.execute(
                    "UPDATE boards SET icon = ?1, color = ?2, smart_rule = ?3, shortcut = ?4 WHERE id = ?5",
                    params![board.icon, board.color, board.smart_rule, board.shortcut, id],
                )?;
                id
            } else {
                tx.execute(
                    "INSERT INTO boards (name, icon, color, smart_rule, board_type, shortcut, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![board.name, board.icon, board.color, board.smart_rule, board.board_type, board.shortcut, board.created_at],
                )?;
                tx.last_insert_rowid()
            };
            board_id_map.insert(board.id, new_id);
        }

        for filter in payload.filters {
            let existing_id = tx.query_row(
                "SELECT id FROM filters WHERE name = ?1 AND filter_type = ?2 LIMIT 1",
                params![filter.name, filter.filter_type],
                |row| row.get::<_, i64>(0),
            ).ok();
            if let Some(id) = existing_id {
                tx.execute(
                    "UPDATE filters SET config = ?1, shortcut = ?2 WHERE id = ?3",
                    params![filter.config, filter.shortcut, id],
                )?;
            } else {
                tx.execute(
                    "INSERT INTO filters (name, filter_type, config, shortcut, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![filter.name, filter.filter_type, filter.config, filter.shortcut, filter.created_at],
                )?;
            }
        }

        for operation in payload.operations {
            let existing_id = tx.query_row(
                "SELECT id FROM operations WHERE name = ?1 AND op_type = ?2 LIMIT 1",
                params![operation.name, operation.op_type],
                |row| row.get::<_, i64>(0),
            ).ok();
            if let Some(id) = existing_id {
                tx.execute(
                    "UPDATE operations SET config = ?1, category = ?2 WHERE id = ?3",
                    params![operation.config, operation.category, id],
                )?;
            } else {
                tx.execute(
                    "INSERT INTO operations (name, op_type, config, category, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![operation.name, operation.op_type, operation.config, operation.category, operation.created_at],
                )?;
            }
        }

        let mut imported = 0;
        for clip in payload.clips {
            let mapped_primary_board = clip.board_id.and_then(|id| board_id_map.get(&id).copied());
            tx.execute(
                "INSERT INTO clips (
                    content_type, text_content, html_content, image_base64, image_path, content_hash,
                    source_app, is_pinned, is_protected, pin_order, board_id, note,
                    is_trashed, trashed_at, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                 ON CONFLICT(content_hash) DO UPDATE SET
                    content_type = excluded.content_type,
                    text_content = excluded.text_content,
                    html_content = excluded.html_content,
                    image_base64 = excluded.image_base64,
                    image_path = excluded.image_path,
                    source_app = excluded.source_app,
                    is_pinned = excluded.is_pinned,
                    is_protected = excluded.is_protected,
                    pin_order = excluded.pin_order,
                    board_id = excluded.board_id,
                    note = excluded.note,
                    is_trashed = excluded.is_trashed,
                    trashed_at = excluded.trashed_at,
                    created_at = excluded.created_at",
                params![
                    clip.content_type, clip.text_content, clip.html_content, clip.image_base64,
                    clip.image_path, clip.content_hash, clip.source_app, clip.is_pinned,
                    clip.is_protected, clip.pin_order, mapped_primary_board, clip.note,
                    clip.is_trashed, clip.trashed_at, clip.created_at,
                ],
            )?;
            let new_clip_id = tx.query_row(
                "SELECT id FROM clips WHERE content_hash = ?1",
                params![clip.content_hash],
                |row| row.get::<_, i64>(0),
            )?;
            tx.execute("DELETE FROM clip_boards WHERE clip_id = ?1", params![new_clip_id])?;
            for old_board_id in clip.board_ids.unwrap_or_default() {
                if let Some(new_board_id) = board_id_map.get(&old_board_id) {
                    tx.execute(
                        "INSERT OR IGNORE INTO clip_boards (clip_id, board_id) VALUES (?1, ?2)",
                        params![new_clip_id, new_board_id],
                    )?;
                }
            }
            if let Some(new_board_id) = mapped_primary_board {
                tx.execute(
                    "INSERT OR IGNORE INTO clip_boards (clip_id, board_id) VALUES (?1, ?2)",
                    params![new_clip_id, new_board_id],
                )?;
            }
            imported += 1;
        }

        tx.commit()?;
        Ok(imported)
    }

    fn get_all_clips_for_backup(&self) -> Result<Vec<ClipItem>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, content_type, text_content, html_content, image_base64, image_path,
                    content_hash, source_app, is_pinned, is_protected, COALESCE(pin_order, 0),
                    board_id, note, COALESCE(is_trashed, 0), trashed_at, created_at,
                    (SELECT GROUP_CONCAT(board_id) FROM clip_boards WHERE clip_id = clips.id)
             FROM clips ORDER BY created_at DESC, id DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            let primary_board_id: Option<i64> = row.get(11)?;
            let board_ids_csv: Option<String> = row.get(16)?;
            let mut board_ids = primary_board_id.into_iter().collect::<Vec<_>>();
            for value in board_ids_csv.unwrap_or_default().split(',') {
                if let Ok(id) = value.parse::<i64>() {
                    if !board_ids.contains(&id) {
                        board_ids.push(id);
                    }
                }
            }
            Ok(ClipItem {
                id: row.get(0)?,
                content_type: row.get(1)?,
                text_content: row.get(2)?,
                html_content: row.get(3)?,
                image_base64: row.get(4)?,
                image_path: row.get(5)?,
                content_hash: row.get(6)?,
                source_app: row.get(7)?,
                is_pinned: row.get::<_, i32>(8)? != 0,
                is_protected: row.get::<_, i32>(9)? != 0,
                pin_order: row.get(10)?,
                board_id: primary_board_id,
                board_ids: Some(board_ids),
                note: row.get(12)?,
                is_trashed: row.get::<_, i32>(13)? != 0,
                trashed_at: row.get(14)?,
                created_at: row.get(15)?,
            })
        })?;
        rows.collect()
    }

    pub fn set_vault_passcode(&self, passcode: &str) -> Result<()> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(passcode.as_bytes());
        let hash_hex = format!("{:x}", hasher.finalize());
        self.save_setting("vaultPasscodeHash", &hash_hex)
    }

    pub fn verify_vault_passcode(&self, passcode: &str) -> Result<bool> {
        use sha2::{Sha256, Digest};
        let stored = self.get_setting("vaultPasscodeHash")?;
        if let Some(stored_hash) = stored {
            if stored_hash.trim().is_empty() {
                return Ok(true);
            }
            let mut hasher = Sha256::new();
            hasher.update(passcode.as_bytes());
            let input_hash = format!("{:x}", hasher.finalize());
            Ok(stored_hash == input_hash)
        } else {
            Ok(true)
        }
    }

    pub fn get_filters(&self) -> Result<Vec<FilterRule>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT id, name, filter_type, config, shortcut, created_at FROM filters ORDER BY id ASC")?;
        let filter_iter = stmt.query_map([], |row| {
            Ok(FilterRule {
                id: row.get(0)?,
                name: row.get(1)?,
                filter_type: row.get(2)?,
                config: row.get(3)?,
                shortcut: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        let mut filters = Vec::new();
        for f in filter_iter {
            filters.push(f?);
        }
        Ok(filters)
    }

    pub fn create_filter(&self, name: &str, filter_type: &str, config: Option<&str>, shortcut: Option<&str>) -> Result<FilterRule> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO filters (name, filter_type, config, shortcut) VALUES (?1, ?2, ?3, ?4)",
            params![name, filter_type, config, shortcut],
        )?;
        let id = conn.last_insert_rowid();
        conn.query_row(
            "SELECT id, name, filter_type, config, shortcut, created_at FROM filters WHERE id = ?1",
            params![id],
            |row| {
                Ok(FilterRule {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    filter_type: row.get(2)?,
                    config: row.get(3)?,
                    shortcut: row.get(4)?,
                    created_at: row.get(5)?,
                })
            },
        )
    }

    pub fn update_filter_shortcut(&self, id: i64, shortcut: Option<&str>) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("UPDATE filters SET shortcut = ?1 WHERE id = ?2", params![shortcut, id])?;
        Ok(())
    }

    pub fn delete_filter(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM filters WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_operations(&self) -> Result<Vec<Operation>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT id, name, op_type, config, category, created_at FROM operations ORDER BY id ASC")?;
        let op_iter = stmt.query_map([], |row| {
            Ok(Operation {
                id: row.get(0)?,
                name: row.get(1)?,
                op_type: row.get(2)?,
                config: row.get(3)?,
                category: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        let mut operations = Vec::new();
        for o in op_iter {
            operations.push(o?);
        }
        Ok(operations)
    }

    pub fn create_operation(&self, name: &str, op_type: &str, config: Option<&str>, category: Option<&str>) -> Result<Operation> {
        let conn = self.conn.lock();
        let cat = category.unwrap_or("Custom");
        conn.execute(
            "INSERT INTO operations (name, op_type, config, category) VALUES (?1, ?2, ?3, ?4)",
            params![name, op_type, config, cat],
        )?;
        let id = conn.last_insert_rowid();
        conn.query_row(
            "SELECT id, name, op_type, config, category, created_at FROM operations WHERE id = ?1",
            params![id],
            |row| {
                Ok(Operation {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    op_type: row.get(2)?,
                    config: row.get(3)?,
                    category: row.get(4)?,
                    created_at: row.get(5)?,
                })
            },
        )
    }

    pub fn update_operation(&self, id: i64, name: &str, op_type: &str, config: Option<&str>, category: Option<&str>) -> Result<()> {
        let conn = self.conn.lock();
        let cat = category.unwrap_or("Custom Operations");
        conn.execute(
            "UPDATE operations SET name = ?1, op_type = ?2, config = ?3, category = ?4 WHERE id = ?5",
            params![name, op_type, config, cat, id],
        )?;
        Ok(())
    }

    pub fn delete_operation(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM operations WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn purge_old_clips(&self, keep_count: i64) -> Result<()> {
        let conn = self.conn.lock();
        self.enforce_history_limit_with_count_internal(&conn, keep_count)
    }

    pub fn save_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            let val: String = row.get(0)?;
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }

    pub fn get_all_settings(&self) -> Result<std::collections::HashMap<String, String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut map = std::collections::HashMap::new();
        for r in rows {
            let (k, v) = r?;
            map.insert(k, v);
        }
        Ok(map)
    }

    pub fn get_distinct_source_apps(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT source_app FROM clips WHERE source_app IS NOT NULL AND source_app != '' ORDER BY source_app ASC"
        )?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut apps = Vec::new();
        for r in rows {
            apps.push(r?);
        }
        Ok(apps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn setup_test_db() -> DbState {
        let temp_dir = std::env::temp_dir();
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let db_file = temp_dir.join(format!("pasted_test_{}_{:?}.db", nanos, std::thread::current().id()));
        DbState::new(db_file).expect("Failed to create test DB")
    }

    #[test]
    fn test_clip_saving_and_retrieval() {
        let db = setup_test_db();
        let clip = db
            .save_clip("text", Some("Hello Rust"), None, None, "hash1", "Safari")
            .unwrap();
        assert!(clip.id > 0);

        let clips = db.get_clips(None, None, false).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].text_content.as_deref(), Some("Hello Rust"));
        assert_eq!(clips[0].source_app, "Safari");
        assert!(!clips[0].is_pinned);
    }

    #[test]
    fn test_protected_clips_immunity() {
        let db = setup_test_db();
        let clip = db
            .save_clip("text", Some("Protected Secret"), None, None, "prot_hash", "Keeper")
            .unwrap();

        // Toggle protected
        let is_prot = db.toggle_protected(clip.id).unwrap();
        assert!(is_prot);

        // Attempt delete_clip (should be blocked)
        db.delete_clip(clip.id).unwrap();
        let active = db.get_clips(None, None, false).unwrap();
        assert_eq!(active.len(), 1);
        assert!(active[0].is_protected);

        // Attempt trash_unpinned_clips (should be blocked)
        db.trash_unpinned_clips().unwrap();
        let active_after_trash = db.get_clips(None, None, false).unwrap();
        assert_eq!(active_after_trash.len(), 1);

        // Attempt purge_unpinned_clips (should be blocked)
        db.purge_unpinned_clips().unwrap();
        let active_after_purge = db.get_clips(None, None, false).unwrap();
        assert_eq!(active_after_purge.len(), 1);

        // Every bulk and retention path must preserve protected clips.
        db.clear_history().unwrap();
        db.purge_old_clips(0).unwrap();
        let active_after_clear = db.get_clips(None, None, false).unwrap();
        assert_eq!(active_after_clear.len(), 1);
        assert!(active_after_clear[0].is_protected);

        // Unprotect and verify delete works
        db.toggle_protected(clip.id).unwrap();
        db.delete_clip(clip.id).unwrap();
        assert_eq!(db.get_clips(None, None, false).unwrap().len(), 0);
    }

    #[test]
    fn test_retention_uses_trash_and_excludes_pinned_and_protected_clips() {
        let db = setup_test_db();
        let pinned = db.save_clip("text", Some("Pinned"), None, None, "ret-pin", "App").unwrap();
        let protected = db.save_clip("text", Some("Protected"), None, None, "ret-prot", "App").unwrap();
        db.toggle_pin(pinned.id).unwrap();
        db.toggle_protected(protected.id).unwrap();

        for index in 0..3 {
            db.save_clip(
                "text",
                Some(&format!("Regular {index}")),
                None,
                None,
                &format!("ret-{index}"),
                "App",
            ).unwrap();
        }

        db.purge_old_clips(1).unwrap();

        let active = db.get_clips(None, None, false).unwrap();
        assert_eq!(active.iter().filter(|clip| !clip.is_pinned && !clip.is_protected).count(), 1);
        assert!(active.iter().any(|clip| clip.id == pinned.id));
        assert!(active.iter().any(|clip| clip.id == protected.id));
        assert_eq!(db.get_trashed_clips().unwrap().len(), 2);
    }

    #[test]
    fn test_retention_without_trash_keeps_requested_unpinned_capacity() {
        let db = setup_test_db();
        db.save_setting("enableTrash", "false").unwrap();
        let pinned = db.save_clip("text", Some("Pinned"), None, None, "purge-pin", "App").unwrap();
        db.toggle_pin(pinned.id).unwrap();
        for index in 0..4 {
            db.save_clip(
                "text",
                Some(&format!("Regular {index}")),
                None,
                None,
                &format!("purge-{index}"),
                "App",
            ).unwrap();
        }

        db.purge_old_clips(2).unwrap();

        let active = db.get_clips(None, None, false).unwrap();
        assert_eq!(active.iter().filter(|clip| !clip.is_pinned).count(), 2);
        assert!(active.iter().any(|clip| clip.id == pinned.id));
        assert!(db.get_trashed_clips().unwrap().is_empty());
    }

    #[test]
    fn test_clip_pinning_and_notes() {
        let db = setup_test_db();
        let clip = db
            .save_clip("text", Some("Pasted Pin Test"), None, None, "hash2", "VSCode")
            .unwrap();

        // Pin clip
        let is_pinned = db.toggle_pin(clip.id).unwrap();
        assert!(is_pinned);

        // Add note
        db.update_clip_note(clip.id, Some("Important note")).unwrap();

        let clips = db.get_clips(None, None, false).unwrap();
        assert!(clips[0].is_pinned);
        assert_eq!(clips[0].note.as_deref(), Some("Important note"));
    }

    #[test]
    fn test_boards_crud() {
        let db = setup_test_db();
        let initial_count = db.get_boards().unwrap().len();

        let board = db.create_board("Work", "💼", "#3b82f6", None).unwrap();
        assert!(board.id > 0);

        let boards = db.get_boards().unwrap();
        assert_eq!(boards.len(), initial_count + 1);

        db.delete_board(board.id).unwrap();
        let boards_after = db.get_boards().unwrap();
        assert_eq!(boards_after.len(), initial_count);
    }

    #[test]
    fn test_settings_storage() {
        let db = setup_test_db();
        db.save_setting("hudHotkey", "CmdOrCtrl+Shift+V").unwrap();
        let val = db.get_setting("hudHotkey").unwrap();
        assert_eq!(val.as_deref(), Some("CmdOrCtrl+Shift+V"));
    }

    #[test]
    fn test_clip_search_and_deletion() {
        let db = setup_test_db();
        let clip1 = db
            .save_clip("text", Some("Unique Search Secret"), None, None, "h1", "Terminal")
            .unwrap();
        let _clip2 = db
            .save_clip("text", Some("Unrelated text"), None, None, "h2", "Finder")
            .unwrap();

        // Search by query
        let search_results = db.get_clips(Some("Secret"), None, false).unwrap();
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0].text_content.as_deref(), Some("Unique Search Secret"));

        // Test distinct apps
        let apps = db.get_distinct_source_apps().unwrap();
        assert!(apps.contains(&"Terminal".to_string()));
        assert!(apps.contains(&"Finder".to_string()));

        // Delete single clip (moves to trash)
        db.delete_clip(clip1.id).unwrap();
        let after_delete = db.get_clips(None, None, false).unwrap();
        assert_eq!(after_delete.len(), 1);

        // Verify clip is in Trash
        let trashed = db.get_trashed_clips().unwrap();
        assert_eq!(trashed.len(), 1);
        assert_eq!(trashed[0].id, clip1.id);
        assert_eq!(db.get_total_clip_count().unwrap(), 1);

        // Restore clip
        db.restore_clip(clip1.id).unwrap();
        let after_restore = db.get_clips(None, None, false).unwrap();
        assert_eq!(after_restore.len(), 2);
    }

    #[test]
    fn test_trash_and_activity_logging() {
        let db = setup_test_db();
        let clip = db.save_clip("text", Some("Trash Me"), None, None, "thash1", "Notes").unwrap();

        // Trash clip
        db.delete_clip(clip.id).unwrap();
        let trashed = db.get_trashed_clips().unwrap();
        assert_eq!(trashed.len(), 1);

        // Empty trash
        db.empty_trash().unwrap();
        assert_eq!(db.get_trashed_clips().unwrap().len(), 0);

        // Check activity logs
        let logs = db.get_activity_logs(None, None).unwrap();
        assert!(logs.len() >= 2); // clip_trashed, trash_emptied
        assert_eq!(logs[0].event_type, "trash_emptied");

        // Clear logs
        db.clear_activity_logs().unwrap();
        assert_eq!(db.get_activity_logs(None, None).unwrap().len(), 0);
    }

    #[test]
    fn test_filters_and_operations_crud() {
        let db = setup_test_db();

        // Filter Pipeline CRUD
        let filter = db
            .create_filter("Trim & Uppercase", "trim", None, Some("Alt+T"))
            .unwrap();
        assert!(filter.id > 0);

        let filters = db.get_filters().unwrap();
        assert!(filters.iter().any(|f| f.name == "Trim & Uppercase"));

        db.delete_filter(filter.id).unwrap();
        let after_delete = db.get_filters().unwrap();
        assert!(!after_delete.iter().any(|f| f.id == filter.id));

        // Operation CRUD
        let op = db
            .create_operation("JSON Prettify", "json_format", None, Some("Format"))
            .unwrap();
        assert!(op.id > 0);

        let ops = db.get_operations().unwrap();
        assert!(ops.iter().any(|o| o.name == "JSON Prettify"));

        db.delete_operation(op.id).unwrap();
        let ops_after = db.get_operations().unwrap();
        assert!(!ops_after.iter().any(|o| o.id == op.id));
    }

    #[test]
    fn test_wal_mode_and_indexing() {
        let db = setup_test_db();
        let conn = db.conn.lock();

        // Verify WAL mode is configured
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert!(
            mode.to_lowercase() == "wal" || mode.to_lowercase() == "memory",
            "journal_mode should be wal or memory (test db), got: {}",
            mode
        );

        // Verify indexes exist
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='index'")
            .unwrap();
        let index_names: Vec<String> = stmt
            .query_map([], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(index_names.contains(&"idx_clips_pinned_created".to_string()));
        assert!(index_names.contains(&"idx_clips_board_created".to_string()));
        assert!(index_names.contains(&"idx_clips_hash".to_string()));
        assert!(index_names.contains(&"idx_clips_active_timeline".to_string()));
    }

    #[test]
    fn test_fts5_search_indexing() {
        let db = setup_test_db();

        let clip1 = db
            .save_clip("text", Some("Supercalifragilisticexpialidocious secret token"), None, None, "HashFTS1", "IntelliJ")
            .unwrap();
        let _clip2 = db
            .save_clip("text", Some("Unrelated standard content text"), None, None, "HashFTS2", "Safari")
            .unwrap();

        let search_res = db.get_clips(Some("Supercalifragilisticexpialidocious"), None, false).unwrap();
        assert_eq!(search_res.len(), 1);
        assert_eq!(search_res[0].id, clip1.id);

        db.delete_clip(clip1.id).unwrap();
        let search_after_delete = db.get_clips(Some("Supercalifragilisticexpialidocious"), None, false).unwrap();
        assert_eq!(search_after_delete.len(), 0);
    }

    #[test]
    fn test_unified_taxonomy_and_tags() {
        let db = setup_test_db();
        let tag = db.create_board_with_type("CodeSnippet", "Tag", "#06b6d4", None, "tag").unwrap();
        assert_eq!(tag.board_type, "tag");

        let boards = db.get_boards().unwrap();
        assert!(boards.iter().any(|b| b.id == tag.id && b.board_type == "tag"));
    }

    #[test]
    fn test_pin_reordering() {
        let db = setup_test_db();
        let clip1 = db.save_clip("text", Some("First Pinned"), None, None, "HashP1", "App").unwrap();
        let clip2 = db.save_clip("text", Some("Second Pinned"), None, None, "HashP2", "App").unwrap();
        db.toggle_pin(clip1.id).unwrap();
        db.toggle_pin(clip2.id).unwrap();

        db.reorder_pinned_clips(vec![clip2.id, clip1.id]).unwrap();
        let clips = db.get_clips(None, None, true).unwrap();
        assert_eq!(clips[0].id, clip2.id);
        assert_eq!(clips[1].id, clip1.id);
    }

    #[test]
    fn test_clip_version_history() {
        let db = setup_test_db();
        let clip = db.save_clip("text", Some("Original Content"), None, None, "HashV1", "App").unwrap();

        db.update_clip_text(clip.id, "Transformed Uppercase Content").unwrap();
        db.update_clip_text(clip.id, "Transformed Uppercase Content").unwrap();
        db.update_clip_text(clip.id, "Final Content").unwrap();

        let versions = db.get_clip_versions(clip.id).unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].text_content, "Transformed Uppercase Content");
        assert_eq!(versions[1].text_content, "Original Content");

        let updated = db.get_clips(None, None, false).unwrap();
        assert_eq!(updated[0].text_content.as_deref(), Some("Final Content"));

        for index in 0..55 {
            db.update_clip_text(clip.id, &format!("Revision {index}")).unwrap();
        }
        assert_eq!(db.get_clip_versions(clip.id).unwrap().len(), 50);

        db.purge_clip_permanently(clip.id).unwrap();
        assert!(db.get_clip_versions(clip.id).unwrap().is_empty());
    }

    #[test]
    fn test_batch_operations() {
        let db = setup_test_db();
        let clip1 = db.save_clip("text", Some("Batch 1"), None, None, "HashB1", "App").unwrap();
        let clip2 = db.save_clip("text", Some("Batch 2"), None, None, "HashB2", "App").unwrap();

        db.batch_pin_clips(vec![clip1.id, clip2.id], true).unwrap();
        let pinned = db.get_clips(None, None, true).unwrap();
        assert_eq!(pinned.len(), 2);

        db.batch_trash_clips(vec![clip1.id]).unwrap();
        let trashed = db.get_trashed_clips().unwrap();
        assert_eq!(trashed.len(), 1);
        assert_eq!(trashed[0].id, clip1.id);
    }

    #[test]
    fn test_bin_assignment_is_exclusive_and_preserves_tags() {
        let db = setup_test_db();
        let clip1 = db.save_clip("text", Some("Exclusive 1"), None, None, "HashE1", "App").unwrap();
        let clip2 = db.save_clip("text", Some("Exclusive 2"), None, None, "HashE2", "App").unwrap();
        let first_bin = db.create_board("First Bin", "Folder", "#3b82f6", None).unwrap();
        let second_bin = db.create_board("Second Bin", "Folder", "#10b981", None).unwrap();
        let tag = db.create_board_with_type("Important", "Tag", "#f59e0b", None, "tag").unwrap();

        db.assign_to_board(clip1.id, Some(first_bin.id)).unwrap();
        db.add_clip_to_board(clip1.id, tag.id).unwrap();
        db.assign_to_board(clip1.id, Some(second_bin.id)).unwrap();

        assert!(db.get_clips(None, Some(first_bin.id), false).unwrap().is_empty());
        let second_bin_clips = db.get_clips(None, Some(second_bin.id), false).unwrap();
        assert_eq!(second_bin_clips.len(), 1);
        assert_eq!(second_bin_clips[0].id, clip1.id);
        assert!(second_bin_clips[0].board_ids.as_ref().unwrap().contains(&tag.id));

        db.assign_to_board(clip1.id, None).unwrap();
        let unassigned = db.get_clips(None, None, false).unwrap();
        let clip1_after_unassign = unassigned.iter().find(|clip| clip.id == clip1.id).unwrap();
        assert_eq!(clip1_after_unassign.board_id, None);
        assert_eq!(clip1_after_unassign.board_ids.as_ref().unwrap(), &vec![tag.id]);

        db.batch_assign_board_clips(vec![clip1.id, clip2.id], Some(first_bin.id)).unwrap();
        db.batch_assign_board_clips(vec![clip1.id, clip2.id], Some(second_bin.id)).unwrap();
        assert!(db.get_clips(None, Some(first_bin.id), false).unwrap().is_empty());
        assert_eq!(db.get_clips(None, Some(second_bin.id), false).unwrap().len(), 2);
    }

    #[test]
    fn test_backup_export_import() {
        let db = setup_test_db();
        let clip = db.save_clip(
            "text",
            Some("Backup Test Item"),
            Some("<strong>Backup Test Item</strong>"),
            None,
            "HashBK1",
            "VSCode",
        ).unwrap();
        let trashed = db.save_clip("text", Some("In Trash"), None, None, "HashBK2", "Notes").unwrap();
        let board = db.create_board("DevBin", "Code", "#3b82f6", None).unwrap();
        let tag = db.create_board_with_type("BackupTag", "Tag", "#f59e0b", None, "tag").unwrap();
        db.assign_to_board(clip.id, Some(board.id)).unwrap();
        db.add_clip_to_board(clip.id, tag.id).unwrap();
        db.update_clip_note(clip.id, Some("Restore this note")).unwrap();
        db.toggle_pin(clip.id).unwrap();
        db.toggle_protected(clip.id).unwrap();
        db.delete_clip(trashed.id).unwrap();
        db.create_filter("Backup Filter", "trim", Some("{}"), Some("Alt+B")).unwrap();
        db.create_operation("Backup Operation", "uppercase", Some("{}"), Some("Backup Tools")).unwrap();

        let json = db.export_backup_json().unwrap();
        assert!(json.contains("Backup Test Item"));
        assert!(json.contains("DevBin"));

        let db2 = setup_test_db();
        let imported_count = db2.import_backup_json(&json).unwrap();
        assert_eq!(imported_count, 2);

        let restored = db2.get_all_clips_for_backup().unwrap();
        let restored_clip = restored.iter().find(|item| item.content_hash == "HashBK1").unwrap();
        assert_eq!(restored_clip.text_content.as_deref(), Some("Backup Test Item"));
        assert_eq!(restored_clip.html_content.as_deref(), Some("<strong>Backup Test Item</strong>"));
        assert_eq!(restored_clip.note.as_deref(), Some("Restore this note"));
        assert!(restored_clip.is_pinned);
        assert!(restored_clip.is_protected);
        assert!(!restored_clip.is_trashed);

        let restored_trashed = restored.iter().find(|item| item.content_hash == "HashBK2").unwrap();
        assert!(restored_trashed.is_trashed);
        assert!(restored_trashed.trashed_at.is_some());

        let restored_boards = db2.get_boards().unwrap();
        let restored_bin = restored_boards.iter().find(|item| item.name == "DevBin").unwrap();
        let restored_tag = restored_boards.iter().find(|item| item.name == "BackupTag").unwrap();
        let restored_board_ids = restored_clip.board_ids.as_ref().unwrap();
        assert!(restored_board_ids.contains(&restored_bin.id));
        assert!(restored_board_ids.contains(&restored_tag.id));
        assert!(db2.get_filters().unwrap().iter().any(|item| item.name == "Backup Filter" && item.shortcut.as_deref() == Some("Alt+B")));
        assert!(db2.get_operations().unwrap().iter().any(|item| item.name == "Backup Operation" && item.category == "Backup Tools"));
    }

    #[test]
    fn test_backup_export_is_not_limited_to_visible_history() {
        let db = setup_test_db();
        for index in 0..501 {
            db.save_clip(
                "text",
                Some(&format!("Backup item {index}")),
                None,
                None,
                &format!("backup-limit-{index}"),
                "App",
            ).unwrap();
        }

        let json = db.export_backup_json().unwrap();
        let payload: BackupPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(payload.version, 2);
        assert_eq!(payload.clips.len(), 501);
        assert_eq!(db.get_clips(None, None, false).unwrap().len(), 501);
    }

    #[test]
    fn test_vault_passcode() {
        let db = setup_test_db();
        assert!(db.verify_vault_passcode("secret123").unwrap()); // Default empty pass

        db.set_vault_passcode("secret123").unwrap();
        assert!(db.verify_vault_passcode("secret123").unwrap());
        assert!(!db.verify_vault_passcode("wrongpass").unwrap());
    }
}
