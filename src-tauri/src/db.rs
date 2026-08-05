use parking_lot::Mutex;
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const BACKUP_SCHEMA_VERSION: u32 = 5;

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
    pub is_transformed: bool,
    pub pin_order: i32,
    pub bin_id: Option<i64>,
    pub bin_ids: Option<Vec<i64>>,
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
pub struct Bin {
    pub id: i64,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub smart_rule: Option<String>, // JSON string for auto-smart rules
    pub bin_type: String,           // "category" or "tag"
    pub shortcut: Option<String>,
    pub clip_count: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClipVersion {
    pub id: i64,
    pub clip_id: i64,
    pub text_content: String,
    pub action_kind: Option<String>,
    pub action_label: Option<String>,
    pub restores_organization: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ClipRevisionContext {
    schema_version: i64,
    action_kind: String,
    action_label: String,
    organization: Option<ClipRevisionOrganization>,
    #[serde(default)]
    current_transformation_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ClipRevisionOrganization {
    category_bin_id: Option<i64>,
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
    pub bins: Vec<Bin>,
    pub pipelines: Vec<Pipeline>,
    pub operations: Vec<Operation>,
    #[serde(default)]
    pub saved_transforms: Vec<SavedTransform>,
    #[serde(default)]
    pub bin_transforms: Vec<BinTransformBinding>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BinTransformBinding {
    pub bin_id: i64,
    pub transform_ref: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Pipeline {
    pub id: i64,
    pub stable_ref: String,
    pub name: String,
    pub shortcut: Option<String>,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
    pub steps: Vec<PipelineStep>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SavedTransform {
    pub id: i64,
    pub stable_ref: String,
    pub name: String,
    pub plan: crate::transformation_intent::TransformationPlan,
    pub connection_id: Option<String>,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClipTransformationProvenance {
    pub transform_ref: String,
    pub transform_name: String,
    pub transform_revision: i64,
    pub connection_id: Option<String>,
    pub duration_ms: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransformationExecution {
    pub id: String,
    pub target_kind: String,
    pub target_ref: String,
    pub target_revision: Option<i64>,
    pub source_clip_id: Option<i64>,
    pub trigger_kind: String,
    pub destination_kind: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub status: String,
    pub error_summary: Option<String>,
}

pub struct TransformationExecutionStart<'a> {
    pub target_kind: &'a str,
    pub target_ref: &'a str,
    pub target_revision: Option<i64>,
    pub source_clip_id: Option<i64>,
    pub trigger_kind: &'a str,
    pub destination_kind: &'a str,
    pub input_hash: &'a str,
}

pub struct TransformClipApplication<'a> {
    pub clip_id: i64,
    pub transform_ref: &'a str,
    pub expected_input: &'a str,
    pub output: &'a str,
    pub connection_id: Option<&'a str>,
    pub duration_ms: i64,
    pub bin_move: Option<(Option<i64>, i64)>,
}

pub struct IntelligenceConnectionUpdate<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub provider_kind: &'a str,
    pub endpoint: Option<&'a str>,
    pub model: Option<&'a str>,
    pub credential_ref: Option<&'a str>,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStepInput {
    pub operation_ref: String,
    pub config_json: Option<String>,
    #[serde(default = "default_pipeline_failure_policy")]
    pub failure_policy: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStep {
    pub position: i64,
    pub operation_ref: String,
    pub config_json: Option<String>,
    pub failure_policy: String,
}

fn default_pipeline_failure_policy() -> String {
    "stop".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Operation {
    pub id: i64,
    #[serde(default)]
    pub stable_id: String,
    pub name: String,
    pub op_type: String,
    pub config: Option<String>,
    pub category: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedCustomOperation {
    pub executor_kind: String,
    pub config_json: String,
    pub enabled: bool,
    pub trusted: bool,
}

#[derive(Debug, Clone)]
pub struct ResolvedPipelineStep {
    pub position: i64,
    pub operation_ref: String,
    pub config_json: Option<String>,
    pub failure_policy: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedPipeline {
    pub revision: i64,
    pub steps: Vec<ResolvedPipelineStep>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IntelligenceConnection {
    pub id: String,
    pub name: String,
    pub provider_kind: String,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub credential_ref: Option<String>,
    pub enabled: bool,
    pub priority: i64,
    pub created_at: String,
    pub updated_at: String,
}

pub struct DbState {
    pub conn: Mutex<Connection>,
}

fn table_exists(conn: &Connection, name: &str) -> Result<bool> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        params![name],
        |row| row.get(0),
    )
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let names = statement.query_map([], |row| row.get::<_, String>(1))?;
    for name in names {
        if name? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn migrate_legacy_container_schema(conn: &Connection) -> Result<()> {
    // Pre-release databases used "board" for the same concept now consistently named "bin".
    if table_exists(conn, "boards")? && !table_exists(conn, "bins")? {
        conn.execute("ALTER TABLE boards RENAME TO bins", [])?;
    }
    if table_exists(conn, "clips")?
        && column_exists(conn, "clips", "board_id")?
        && !column_exists(conn, "clips", "bin_id")?
    {
        conn.execute("ALTER TABLE clips RENAME COLUMN board_id TO bin_id", [])?;
    }
    if table_exists(conn, "bins")?
        && column_exists(conn, "bins", "board_type")?
        && !column_exists(conn, "bins", "bin_type")?
    {
        conn.execute("ALTER TABLE bins RENAME COLUMN board_type TO bin_type", [])?;
    }
    if table_exists(conn, "clip_boards")? && !table_exists(conn, "clip_bins")? {
        conn.execute("ALTER TABLE clip_boards RENAME TO clip_bins", [])?;
    }
    if table_exists(conn, "clip_bins")?
        && column_exists(conn, "clip_bins", "board_id")?
        && !column_exists(conn, "clip_bins", "bin_id")?
    {
        conn.execute("ALTER TABLE clip_bins RENAME COLUMN board_id TO bin_id", [])?;
    }

    // SQLite cannot rename indexes; replace any pre-release names after their tables move.
    conn.execute("DROP INDEX IF EXISTS idx_clips_board_created", [])?;
    conn.execute("DROP INDEX IF EXISTS idx_clip_boards_board_id", [])?;
    conn.execute("DROP INDEX IF EXISTS idx_clip_boards_clip_id", [])?;
    Ok(())
}

impl DbState {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let conn = Connection::open(db_path)?;
        conn.set_db_config(rusqlite::config::DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true)?;
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

        migrate_legacy_container_schema(&conn)?;

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
                bin_id INTEGER,
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
            "CREATE INDEX IF NOT EXISTS idx_clips_bin_created ON clips (bin_id, created_at DESC)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clips_hash ON clips (content_hash)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS bins (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                icon TEXT DEFAULT 'Folder',
                color TEXT DEFAULT 'default',
                smart_rule TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // Migrations if existing tables don't have new columns
        let _ = conn.execute("ALTER TABLE clips ADD COLUMN note TEXT", []);
        let _ = conn.execute(
            "ALTER TABLE clips ADD COLUMN is_trashed INTEGER DEFAULT 0",
            [],
        );
        let _ = conn.execute("ALTER TABLE clips ADD COLUMN trashed_at DATETIME", []);
        let _ = conn.execute(
            "ALTER TABLE clips ADD COLUMN is_protected INTEGER DEFAULT 0",
            [],
        );
        let _ = conn.execute("ALTER TABLE clips ADD COLUMN image_path TEXT", []);
        let _ = conn.execute(
            "ALTER TABLE clips ADD COLUMN pin_order INTEGER DEFAULT 0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE clips ADD COLUMN current_transformation_id TEXT",
            [],
        );
        let _ = conn.execute("ALTER TABLE bins ADD COLUMN smart_rule TEXT", []);
        let _ = conn.execute(
            "ALTER TABLE bins ADD COLUMN bin_type TEXT DEFAULT 'category'",
            [],
        );
        let _ = conn.execute("ALTER TABLE bins ADD COLUMN shortcut TEXT", []);

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
        let _ = conn.execute("ALTER TABLE clip_versions ADD COLUMN context_json TEXT", []);

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

            // FTS5 is a derived cache. Rebuild it at startup so an interrupted write or an
            // older trigger implementation cannot leave clip updates failing with a
            // misleading "database disk image is malformed" error.
            let _ = conn.execute("INSERT INTO clips_fts(clips_fts) VALUES('rebuild')", []);
        }

        conn.execute(
            "CREATE TABLE IF NOT EXISTS clip_bins (
                clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                bin_id INTEGER NOT NULL REFERENCES bins(id) ON DELETE CASCADE,
                PRIMARY KEY (clip_id, bin_id)
            )",
            [],
        )?;
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clip_bins_bin_id ON clip_bins (bin_id)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clip_bins_clip_id ON clip_bins (clip_id)",
            [],
        );

        let _ = conn.execute(
            "INSERT OR IGNORE INTO clip_bins (clip_id, bin_id)
             SELECT id, bin_id FROM clips WHERE bin_id IS NOT NULL",
            [],
        );

        // Trash is deliberately outside the organizational hierarchy. Clean up
        // legacy rows so restored clips never silently reappear in an old Bin.
        let _ = conn.execute(
            "DELETE FROM clip_bins
             WHERE clip_id IN (SELECT id FROM clips WHERE is_trashed = 1)
               AND bin_id IN (
                   SELECT id FROM bins WHERE COALESCE(bin_type, 'category') != 'tag'
               )",
            [],
        );
        let _ = conn.execute("UPDATE clips SET bin_id = NULL WHERE is_trashed = 1", []);

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

        self.init_transformation_tables(&conn)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        // Insert default bins if empty
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM bins", [], |r| r.get(0))
            .unwrap_or(0);
        if count == 0 {
            conn.execute(
                "INSERT INTO bins (name, icon, color, smart_rule) VALUES ('Code Snippets', 'Code', '#10b981', '{\"type\":\"content_type\",\"value\":\"code\"}')",
                [],
            )?;
            conn.execute(
                "INSERT INTO bins (name, icon, color, smart_rule) VALUES ('Links & Web', 'Link', '#3b82f6', '{\"type\":\"content_type\",\"value\":\"link\"}')",
                [],
            )?;
            conn.execute(
                "INSERT INTO bins (name, icon, color, smart_rule) VALUES ('Colors & Swatches', 'Palette', '#f59e0b', '{\"type\":\"content_type\",\"value\":\"color\"}')",
                [],
            )?;
        }

        Ok(())
    }

    fn init_transformation_tables(&self, conn: &Connection) -> Result<()> {
        // The app has not shipped, so keep the domain and storage vocabulary
        // aligned. These renames preserve development data without carrying a
        // second set of compatibility APIs through the codebase.
        let has_legacy_transforms = table_exists(conn, "transformation_recipes")?;
        let has_saved_transforms = table_exists(conn, "saved_transforms")?;
        if has_legacy_transforms && !has_saved_transforms {
            conn.execute(
                "ALTER TABLE transformation_recipes RENAME TO saved_transforms",
                [],
            )?;
        } else if has_legacy_transforms && has_saved_transforms {
            // A hot-reloaded frontend can call the new API before the Rust
            // process restarts, leaving both pre-release tables behind. Merge
            // them instead of treating the new-but-empty table as authoritative.
            conn.execute(
                "INSERT OR IGNORE INTO saved_transforms
                    (row_id, id, name, plan_json, connection_id, revision, created_at, updated_at)
                 SELECT row_id, id, name, plan_json, connection_id, revision, created_at, updated_at
                 FROM transformation_recipes",
                [],
            )?;
        }
        if column_exists(conn, "clip_transformations", "recipe_id")?
            && !column_exists(conn, "clip_transformations", "transform_id")?
        {
            conn.execute(
                "ALTER TABLE clip_transformations RENAME COLUMN recipe_id TO transform_id",
                [],
            )?;
            conn.execute(
                "ALTER TABLE clip_transformations RENAME COLUMN recipe_name TO transform_name",
                [],
            )?;
            conn.execute(
                "ALTER TABLE clip_transformations RENAME COLUMN recipe_revision TO transform_revision",
                [],
            )?;
        }
        let has_legacy_bin_transform = column_exists(conn, "bins", "default_recipe_id")?;
        let has_current_bin_transform = column_exists(conn, "bins", "default_transform_id")?;
        if has_legacy_bin_transform && !has_current_bin_transform {
            conn.execute(
                "ALTER TABLE bins RENAME COLUMN default_recipe_id TO default_transform_id",
                [],
            )?;
        } else if has_legacy_bin_transform && has_current_bin_transform {
            conn.execute(
                "UPDATE bins SET default_transform_id = default_recipe_id
                 WHERE default_transform_id IS NULL AND default_recipe_id IS NOT NULL",
                [],
            )?;
            conn.execute("ALTER TABLE bins DROP COLUMN default_recipe_id", [])?;
        }

        if has_legacy_transforms && has_saved_transforms {
            let provenance_sql: String = conn.query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'clip_transformations'",
                [],
                |row| row.get(0),
            )?;
            if provenance_sql.contains("transformation_recipes") {
                conn.execute_batch(
                    "CREATE TABLE clip_transformations_migrated (
                        id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                        clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                        transform_id TEXT REFERENCES saved_transforms(id) ON DELETE SET NULL,
                        transform_name TEXT NOT NULL,
                        transform_revision INTEGER NOT NULL,
                        connection_id TEXT REFERENCES intelligence_connections(id) ON DELETE SET NULL,
                        duration_ms INTEGER NOT NULL DEFAULT 0 CHECK (duration_ms >= 0),
                        created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
                    );
                    INSERT INTO clip_transformations_migrated
                        (id, clip_id, transform_id, transform_name, transform_revision,
                         connection_id, duration_ms, created_at)
                    SELECT id, clip_id, transform_id, transform_name, transform_revision,
                           connection_id, duration_ms, created_at
                    FROM clip_transformations;
                    DROP TABLE clip_transformations;
                    ALTER TABLE clip_transformations_migrated RENAME TO clip_transformations;",
                )?;
            }
            conn.execute("DROP TABLE transformation_recipes", [])?;
        }

        let execution_ledger_exists = table_exists(conn, "transformation_executions")?;
        let legacy_execution_has_destination = execution_ledger_exists
            && column_exists(conn, "transformation_executions", "destination_kind")?;
        let legacy_execution_has_completed = execution_ledger_exists
            && column_exists(conn, "transformation_executions", "completed_at")?;
        let rebuild_execution_ledger = if execution_ledger_exists {
            let table_sql: String = conn.query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'transformation_executions'",
                [],
                |row| row.get(0),
            )?;
            !table_sql.contains("'transform'")
                || !table_sql.contains("'queued'")
                || !table_sql.contains("'cancelled'")
        } else {
            false
        };
        if rebuild_execution_ledger {
            conn.execute(
                "ALTER TABLE transformation_executions RENAME TO transformation_executions_legacy",
                [],
            )?;
        }

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS custom_operations (
                row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                id TEXT NOT NULL UNIQUE DEFAULT (lower(hex(randomblob(16)))),
                name TEXT NOT NULL,
                executor_kind TEXT NOT NULL CHECK (
                    executor_kind IN ('builtin', 'regex', 'cli', 'shell', 'http', 'ai')
                ),
                config_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(config_json)),
                category TEXT NOT NULL DEFAULT 'Custom Operations',
                enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
                trusted INTEGER NOT NULL DEFAULT 0 CHECK (trusted IN (0, 1)),
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS pipelines (
                row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                id TEXT NOT NULL UNIQUE DEFAULT (lower(hex(randomblob(16)))),
                name TEXT NOT NULL,
                shortcut TEXT,
                revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS pipeline_steps (
                id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                pipeline_id TEXT NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
                position INTEGER NOT NULL CHECK (position >= 0),
                operation_ref TEXT NOT NULL CHECK (
                    operation_ref GLOB 'builtin:*' OR operation_ref GLOB 'custom:*'
                ),
                config_json TEXT CHECK (config_json IS NULL OR json_valid(config_json)),
                failure_policy TEXT NOT NULL DEFAULT 'stop' CHECK (failure_policy IN ('stop', 'skip')),
                UNIQUE (pipeline_id, position)
            );
            CREATE INDEX IF NOT EXISTS idx_pipeline_steps_operation_ref
                ON pipeline_steps(operation_ref);

            CREATE TABLE IF NOT EXISTS saved_transforms (
                row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                id TEXT NOT NULL UNIQUE DEFAULT (lower(hex(randomblob(16)))),
                name TEXT NOT NULL,
                plan_json TEXT NOT NULL CHECK (json_valid(plan_json)),
                connection_id TEXT REFERENCES intelligence_connections(id) ON DELETE SET NULL,
                revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS clip_transformations (
                id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                transform_id TEXT REFERENCES saved_transforms(id) ON DELETE SET NULL,
                transform_name TEXT NOT NULL,
                transform_revision INTEGER NOT NULL,
                connection_id TEXT REFERENCES intelligence_connections(id) ON DELETE SET NULL,
                duration_ms INTEGER NOT NULL DEFAULT 0 CHECK (duration_ms >= 0),
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_clip_transformations_clip
                ON clip_transformations(clip_id, created_at DESC);

            CREATE TABLE IF NOT EXISTS automations (
                row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                id TEXT NOT NULL UNIQUE DEFAULT (lower(hex(randomblob(16)))),
                name TEXT NOT NULL,
                trigger_kind TEXT NOT NULL CHECK (trigger_kind IN ('capture', 'copy', 'paste')),
                pipeline_id TEXT NOT NULL REFERENCES pipelines(id) ON DELETE RESTRICT,
                enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
                trusted INTEGER NOT NULL DEFAULT 0 CHECK (trusted IN (0, 1)),
                priority INTEGER NOT NULL DEFAULT 0,
                action_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(action_json)),
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS automation_conditions (
                id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                automation_id TEXT NOT NULL REFERENCES automations(id) ON DELETE CASCADE,
                position INTEGER NOT NULL CHECK (position >= 0),
                condition_kind TEXT NOT NULL,
                config_json TEXT NOT NULL CHECK (json_valid(config_json)),
                UNIQUE (automation_id, position)
            );

            CREATE TABLE IF NOT EXISTS transformation_executions (
                id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                target_kind TEXT NOT NULL CHECK (target_kind IN ('operation', 'pipeline', 'transform')),
                target_ref TEXT NOT NULL,
                target_revision INTEGER,
                source_clip_id INTEGER REFERENCES clips(id) ON DELETE SET NULL,
                trigger_kind TEXT NOT NULL CHECK (
                    trigger_kind IN ('manual', 'shortcut', 'bin', 'automation', 'cli')
                ),
                destination_kind TEXT NOT NULL DEFAULT 'preview' CHECK (
                    destination_kind IN ('preview', 'replace', 'copy', 'paste', 'route')
                ),
                started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                completed_at DATETIME,
                duration_ms INTEGER CHECK (duration_ms IS NULL OR duration_ms >= 0),
                status TEXT NOT NULL DEFAULT 'queued' CHECK (
                    status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')
                ),
                error_summary TEXT,
                input_hash TEXT NOT NULL,
                output_hash TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_transformation_executions_started
                ON transformation_executions(started_at DESC);
            CREATE INDEX IF NOT EXISTS idx_transformation_executions_target
                ON transformation_executions(target_kind, target_ref, started_at DESC);

            CREATE TABLE IF NOT EXISTS intelligence_connections (
                row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                id TEXT NOT NULL UNIQUE DEFAULT (lower(hex(randomblob(16)))),
                name TEXT NOT NULL,
                provider_kind TEXT NOT NULL CHECK (
                    provider_kind IN ('openai_compatible', 'anthropic', 'gemini', 'ollama', 'lm_studio', 'cli')
                ),
                endpoint TEXT,
                model TEXT,
                credential_ref TEXT,
                enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
                priority INTEGER NOT NULL DEFAULT 0 CHECK (priority >= 0),
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_intelligence_connections_enabled
                ON intelligence_connections(enabled, provider_kind);

            CREATE TRIGGER IF NOT EXISTS custom_operation_delete_guard
            BEFORE DELETE ON custom_operations
            WHEN EXISTS (
                SELECT 1 FROM pipeline_steps
                WHERE operation_ref = 'custom:' || OLD.id
            )
            BEGIN
                SELECT RAISE(ABORT, 'operation is used by a pipeline');
            END;",
        )?;

        if rebuild_execution_ledger {
            let destination_expression = if legacy_execution_has_destination {
                "destination_kind"
            } else {
                "'preview'"
            };
            let completed_expression = if legacy_execution_has_completed {
                "completed_at"
            } else {
                "CASE WHEN status = 'running' THEN NULL ELSE started_at END"
            };
            conn.execute(
                &format!(
                    "INSERT INTO transformation_executions
                    (id, target_kind, target_ref, target_revision, source_clip_id,
                     trigger_kind, destination_kind, started_at, completed_at,
                     duration_ms, status, error_summary, input_hash, output_hash)
                 SELECT id, target_kind, target_ref, target_revision, source_clip_id,
                        trigger_kind, {destination_expression}, started_at,
                        {completed_expression},
                        duration_ms, status, error_summary, input_hash, output_hash
                 FROM transformation_executions_legacy"
                ),
                [],
            )?;
            conn.execute("DROP TABLE transformation_executions_legacy", [])?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_transformation_executions_started
                 ON transformation_executions(started_at DESC)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_transformation_executions_target
                 ON transformation_executions(target_kind, target_ref, started_at DESC)",
                [],
            )?;
        }

        if !column_exists(conn, "bins", "default_pipeline_id")? {
            conn.execute("ALTER TABLE bins ADD COLUMN default_pipeline_id TEXT", [])?;
        }
        if !column_exists(conn, "bins", "default_transform_id")? {
            conn.execute("ALTER TABLE bins ADD COLUMN default_transform_id TEXT", [])?;
        }
        if !column_exists(conn, "intelligence_connections", "priority")? {
            conn.execute(
                "ALTER TABLE intelligence_connections ADD COLUMN priority INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }
        if !column_exists(conn, "transformation_executions", "destination_kind")? {
            conn.execute(
                "ALTER TABLE transformation_executions ADD COLUMN destination_kind TEXT NOT NULL DEFAULT 'preview'",
                [],
            )?;
        }
        if !column_exists(conn, "transformation_executions", "completed_at")? {
            conn.execute(
                "ALTER TABLE transformation_executions ADD COLUMN completed_at DATETIME",
                [],
            )?;
        }
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                key TEXT PRIMARY KEY,
                applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        let transform_terms_migrated: i64 = conn.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE key = 'transformTerminologyV1'",
            [],
            |row| row.get(0),
        )?;
        if transform_terms_migrated == 0 {
            conn.execute(
                "UPDATE activity_logs
                 SET event_type = replace(event_type, 'recipe_', 'transform_'),
                     description = replace(replace(description, 'Recipes', 'Transforms'), 'Recipe', 'Transform')
                 WHERE event_type LIKE '%recipe%' OR description LIKE '%Recipe%'",
                [],
            )?;
            conn.execute(
                "INSERT INTO schema_migrations (key) VALUES ('transformTerminologyV1')",
                [],
            )?;
        }
        let provenance_backfilled: i64 = conn.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE key = 'currentTransformationBackfillV1'",
            [],
            |row| row.get(0),
        )?;
        if provenance_backfilled == 0 {
            conn.execute(
                "UPDATE clips SET current_transformation_id = (
                    SELECT id FROM clip_transformations
                    WHERE clip_id = clips.id
                    ORDER BY created_at DESC, rowid DESC LIMIT 1
                 )
                 WHERE current_transformation_id IS NULL
                   AND EXISTS (SELECT 1 FROM clip_transformations WHERE clip_id = clips.id)",
                [],
            )?;
            conn.execute(
                "INSERT INTO schema_migrations (key) VALUES ('currentTransformationBackfillV1')",
                [],
            )?;
        }

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
            .query_row(
                "SELECT value FROM settings WHERE key = 'keepClipCount'",
                [],
                |r| r.get(0),
            )
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
            .query_row(
                "SELECT value FROM settings WHERE key = 'enableTrash'",
                [],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "true".to_string());

        let active_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM clips
             WHERE is_pinned = 0
               AND (is_protected IS NULL OR is_protected = 0)
               AND (is_trashed IS NULL OR is_trashed = 0)",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        if active_count > keep_count {
            let excess = active_count - keep_count;
            let mut stmt = conn.prepare(
                "SELECT id FROM clips
                 WHERE is_pinned = 0
                   AND (is_protected IS NULL OR is_protected = 0)
                   AND (is_trashed IS NULL OR is_trashed = 0)
                 ORDER BY created_at ASC, id ASC LIMIT ?1",
            )?;
            let ids: Vec<i64> = stmt
                .query_map(params![excess], |r| r.get(0))?
                .filter_map(|r| r.ok())
                .collect();

            for id in ids {
                if enable_trash == "true" {
                    let changed = conn.execute(
                        "UPDATE clips SET is_trashed = 1, trashed_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?1",
                        params![id],
                    ).unwrap_or(0);
                    if changed > 0 {
                        let _ = self.clear_category_bin_assignments_internal(conn, id);
                    }
                    let _ = self.log_activity_internal(
                        conn,
                        "clip_auto_trashed",
                        &format!(
                            "Auto-trashed clip #{} (history retention limit exceeded)",
                            id
                        ),
                    );
                } else {
                    let _ = conn.execute("DELETE FROM clips WHERE id = ?1", params![id]);
                    let _ = self.log_activity_internal(
                        conn,
                        "clip_deleted",
                        &format!(
                            "Auto-purged clip #{} (history retention limit exceeded)",
                            id
                        ),
                    );
                }
            }
        }
        Ok(())
    }

    pub fn enforce_trash_limit_internal(&self, conn: &Connection) -> Result<()> {
        let capacity: i64 = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'trashCapacityCount'",
                [],
                |r| r.get(0),
            )
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

    pub fn get_active_clip_text(&self, id: i64) -> Result<Option<String>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT text_content FROM clips
             WHERE id = ?1 AND (is_trashed IS NULL OR is_trashed = 0)",
            params![id],
            |row| row.get(0),
        )
    }

    pub fn get_clip_by_id(&self, id: i64) -> Result<ClipItem> {
        let conn = self.conn.lock();
        self.get_clip_by_id_internal(&conn, id)
    }

    fn get_clip_by_id_internal(&self, conn: &Connection, id: i64) -> Result<ClipItem> {
        conn.query_row(
            "SELECT id, content_type, text_content, html_content, image_base64, image_path, content_hash, source_app, is_pinned, is_protected, COALESCE(pin_order, 0), bin_id, note, is_trashed, trashed_at, created_at,
                    (SELECT GROUP_CONCAT(bin_id) FROM clip_bins WHERE clip_id = clips.id),
                    current_transformation_id IS NOT NULL
             FROM clips WHERE id = ?1",
            params![id],
            |row| {
                let bid: Option<i64> = row.get(11)?;
                let bin_ids_str: Option<String> = row.get(16)?;
                let mut bin_ids = bid.into_iter().collect::<Vec<_>>();
                if let Some(value) = bin_ids_str {
                    for value in value.split(',').filter_map(|part| part.parse::<i64>().ok()) {
                        if !bin_ids.contains(&value) {
                            bin_ids.push(value);
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
                    is_transformed: row.get::<_, i32>(17)? != 0,
                    pin_order: row.get(10)?,
                    bin_id: bid,
                    bin_ids: Some(bin_ids),
                    note: row.get(12)?,
                    is_trashed: row.get::<_, i32>(13)? != 0,
                    trashed_at: row.get(14)?,
                    created_at: row.get(15)?,
                })
            },
        )
    }

    pub fn get_clips(
        &self,
        search_query: Option<&str>,
        bin_id: Option<i64>,
        only_pinned: bool,
    ) -> Result<Vec<ClipItem>> {
        let conn = self.conn.lock();

        // Check if target bin has smart_rule
        let mut smart_rule_str: Option<String> = None;
        if let Some(bid) = bin_id {
            let res: Result<Option<String>> = conn.query_row(
                "SELECT smart_rule FROM bins WHERE id = ?1",
                params![bid],
                |r| r.get(0),
            );
            if let Ok(sr) = res {
                smart_rule_str = sr;
            }
        }

        let mut sql = String::from(
            "SELECT id, content_type, text_content, NULL as html_content, NULL as image_base64, image_path, content_hash, source_app, is_pinned, is_protected, COALESCE(pin_order, 0), bin_id, note, is_trashed, trashed_at, created_at,
             (SELECT GROUP_CONCAT(bin_id) FROM clip_bins WHERE clip_id = clips.id) as bin_ids_str,
             current_transformation_id IS NOT NULL
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
                    if let Some(bid) = bin_id {
                        sql.push_str(&format!(" AND (({}) OR bin_id = ? OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?))", combined));
                        query_params.push(Box::new(bid));
                        query_params.push(Box::new(bid));
                    } else {
                        sql.push_str(&format!(" AND ({})", combined));
                    }
                } else if let Some(bid) = bin_id {
                    sql.push_str(" AND (bin_id = ? OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?))");
                    query_params.push(Box::new(bid));
                    query_params.push(Box::new(bid));
                }
            } else if let Some(bid) = bin_id {
                sql.push_str(
                    " AND (bin_id = ? OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?))",
                );
                query_params.push(Box::new(bid));
                query_params.push(Box::new(bid));
            }
        } else if let Some(bid) = bin_id {
            sql.push_str(
                " AND (bin_id = ? OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?))",
            );
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

        let param_refs: Vec<&dyn rusqlite::ToSql> =
            query_params.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let clip_iter = stmt.query_map(param_refs.as_slice(), |row| {
            let primary_bid: Option<i64> = row.get(11)?;
            let bin_ids_str: Option<String> = row.get(16)?;
            let mut b_ids = Vec::new();
            if let Some(b) = primary_bid {
                b_ids.push(b);
            }
            if let Some(ref s) = bin_ids_str {
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
                is_transformed: row.get::<_, i32>(17)? != 0,
                pin_order: row.get(10)?,
                bin_id: primary_bid,
                bin_ids: Some(b_ids),
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
            "SELECT id, content_type, text_content, NULL as html_content, NULL as image_base64, image_path, content_hash, source_app, is_pinned, is_protected, COALESCE(pin_order, 0), bin_id, note, is_trashed, trashed_at, created_at,
                    current_transformation_id IS NOT NULL
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
                is_transformed: row.get::<_, i32>(16)? != 0,
                pin_order: row.get(10)?,
                bin_id: bid,
                bin_ids: bid.map(|b| vec![b]),
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
            "SELECT id, content_type, text_content, NULL as html_content, NULL as image_base64, image_path, content_hash, source_app, is_pinned, is_protected, COALESCE(pin_order, 0), bin_id, note, is_trashed, trashed_at, created_at,
                    current_transformation_id IS NOT NULL
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
                is_transformed: row.get::<_, i32>(16)? != 0,
                pin_order: row.get(10)?,
                bin_id: bid,
                bin_ids: bid.map(|b| vec![b]),
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
        let mut stmt = conn.prepare_cached(
            "UPDATE clips SET note = ?1
             WHERE id = ?2 AND (is_trashed IS NULL OR is_trashed = 0)",
        )?;
        let changed = stmt.execute(params![note, clip_id])?;
        if changed > 0 {
            let _ = self.log_activity_internal(
                &conn,
                "note_updated",
                &format!("Updated note for clip #{}", clip_id),
            );
        }
        Ok(())
    }

    fn revision_history_limit_internal(conn: &Connection) -> i64 {
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'revisionHistoryLimit'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(50)
        .max(0)
    }

    fn prune_clip_versions_internal(conn: &Connection, clip_id: i64) -> Result<()> {
        let limit = Self::revision_history_limit_internal(conn);
        if limit == 0 {
            return Ok(());
        }
        conn.execute(
            "DELETE FROM clip_versions
             WHERE clip_id = ?1 AND id NOT IN (
                SELECT id FROM clip_versions
                WHERE clip_id = ?1 ORDER BY id DESC LIMIT ?2
             )",
            params![clip_id, limit],
        )?;
        Ok(())
    }

    pub fn update_clip_text(&self, clip_id: i64, text: &str) -> Result<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let (previous_text, is_trashed, current_transformation_id): (
            Option<String>,
            i32,
            Option<String>,
        ) = tx.query_row(
            "SELECT text_content, COALESCE(is_trashed, 0), current_transformation_id FROM clips WHERE id = ?1",
            params![clip_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;

        if is_trashed != 0 {
            return tx.commit();
        }

        if previous_text.as_deref() == Some(text) {
            return tx.commit();
        }

        if let Some(previous_text) = previous_text {
            let context_json = serde_json::to_string(&ClipRevisionContext {
                schema_version: 1,
                action_kind: "edit".to_string(),
                action_label: "Edited clip content".to_string(),
                organization: None,
                current_transformation_id,
            })
            .map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))?;
            tx.execute(
                "INSERT INTO clip_versions (clip_id, text_content, context_json) VALUES (?1, ?2, ?3)",
                params![clip_id, previous_text, context_json],
            )?;
            Self::prune_clip_versions_internal(&tx, clip_id)?;
        }
        tx.execute(
            "UPDATE clips SET text_content = ?1, current_transformation_id = NULL WHERE id = ?2",
            params![text, clip_id],
        )?;
        tx.commit()
    }

    fn clear_category_bin_assignments_internal(
        &self,
        conn: &Connection,
        clip_id: i64,
    ) -> Result<()> {
        conn.execute(
            "DELETE FROM clip_bins
             WHERE clip_id = ?1
               AND bin_id IN (
                   SELECT id FROM bins WHERE COALESCE(bin_type, 'category') != 'tag'
               )",
            params![clip_id],
        )?;
        conn.execute(
            "UPDATE clips SET bin_id = NULL WHERE id = ?1",
            params![clip_id],
        )?;
        Ok(())
    }

    pub fn delete_clip(&self, id: i64) -> Result<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let changed = tx.execute(
            "UPDATE clips SET is_trashed = 1, trashed_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
             WHERE id = ?1
               AND (is_protected IS NULL OR is_protected = 0)
               AND (is_trashed IS NULL OR is_trashed = 0)",
            params![id],
        )?;
        if changed > 0 {
            self.clear_category_bin_assignments_internal(&tx, id)?;
        }
        tx.commit()?;
        if changed > 0 {
            let _ = self.log_activity_internal(
                &conn,
                "clip_trashed",
                &format!("Moved clip #{} to Trash", id),
            );
            let _ = self.enforce_trash_limit_internal(&conn);
        }
        Ok(())
    }

    pub fn restore_clip(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare_cached("UPDATE clips SET is_trashed = 0, trashed_at = NULL WHERE id = ?1")?;
        stmt.execute(params![id])?;
        let _ = self.log_activity_internal(
            &conn,
            "clip_restored",
            &format!("Restored clip #{} from Trash", id),
        );
        Ok(())
    }

    pub fn purge_clip_permanently(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        let is_protected: i32 = conn
            .query_row(
                "SELECT is_protected FROM clips WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if is_protected != 0 {
            return Ok(());
        }
        let mut stmt = conn.prepare_cached(
            "DELETE FROM clips WHERE id = ?1 AND (is_protected IS NULL OR is_protected = 0)",
        )?;
        stmt.execute(params![id])?;
        let _ = self.log_activity_internal(
            &conn,
            "clip_deleted",
            &format!("Permanently deleted clip #{}", id),
        );
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
            "DELETE FROM clips WHERE is_trashed = 1 AND (is_protected IS NULL OR is_protected = 0)",
        )?;
        stmt.execute([])?;
        let _ = self.log_activity_internal(
            &conn,
            "trash_emptied",
            &format!("Emptied Trash (permanently deleted {} items)", count),
        );
        Ok(())
    }

    pub fn log_activity(&self, event_type: &str, description: &str) -> Result<()> {
        let conn = self.conn.lock();
        self.log_activity_internal(&conn, event_type, description)
    }

    fn log_activity_internal(
        &self,
        conn: &Connection,
        event_type: &str,
        description: &str,
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

        let capacity: i64 = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'activityLogCapacity'",
                [],
                |r| r.get(0),
            )
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

    pub fn get_activity_logs(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<ActivityLog>> {
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
        let mut stmt =
            conn.prepare("SELECT id, content_type, text_content, source_app FROM clips")?;
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
                } else if c_type == "link"
                    || text.starts_with("http://")
                    || text.starts_with("https://")
                {
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
        if pin_state {
            tx.execute(
                "UPDATE clips SET pin_order = COALESCE(pin_order, 0) + ?1 WHERE is_pinned = 1",
                params![ids.len() as i32],
            )?;
        }
        for (index, id) in ids.into_iter().enumerate() {
            tx.execute(
                "UPDATE clips SET is_pinned = ?1, pin_order = ?2 WHERE id = ?3",
                params![
                    if pin_state { 1 } else { 0 },
                    if pin_state { index as i32 } else { 0 },
                    id
                ],
            )?;
        }
        tx.commit()
    }

    pub fn batch_trash_clips(&self, ids: Vec<i64>) -> Result<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        for id in ids {
            let changed = tx.execute(
                "UPDATE clips SET is_trashed = 1, trashed_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 WHERE id = ?1
                   AND (is_protected IS NULL OR is_protected = 0)
                   AND (is_trashed IS NULL OR is_trashed = 0)",
                params![id],
            )?;
            if changed > 0 {
                self.clear_category_bin_assignments_internal(&tx, id)?;
            }
        }
        self.enforce_trash_limit_internal(&tx)?;
        tx.commit()
    }

    pub fn batch_assign_bin_clips(&self, ids: Vec<i64>, bin_id: Option<i64>) -> Result<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        for clip_id in ids {
            let is_active = tx
                .query_row(
                    "SELECT CASE WHEN is_trashed IS NULL OR is_trashed = 0 THEN 1 ELSE 0 END
                 FROM clips WHERE id = ?1",
                    params![clip_id],
                    |row| row.get::<_, i32>(0),
                )
                .unwrap_or(0)
                != 0;
            if !is_active {
                continue;
            }
            tx.execute(
                "DELETE FROM clip_bins
                 WHERE clip_id = ?1
                   AND bin_id IN (
                       SELECT id FROM bins WHERE COALESCE(bin_type, 'category') != 'tag'
                   )",
                params![clip_id],
            )?;
            if let Some(bid) = bin_id {
                tx.execute(
                    "INSERT OR REPLACE INTO clip_bins (clip_id, bin_id) VALUES (?1, ?2)",
                    params![clip_id, bid],
                )?;
                tx.execute(
                    "UPDATE clips SET bin_id = ?1 WHERE id = ?2",
                    params![bid, clip_id],
                )?;
            } else {
                tx.execute(
                    "UPDATE clips SET bin_id = NULL WHERE id = ?1",
                    params![clip_id],
                )?;
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
        let top_apps = app_stmt
            .query_map([], |r| {
                Ok(AppStat {
                    name: r.get(0)?,
                    count: r.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        let mut type_stmt = conn.prepare(
            "SELECT content_type, COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) GROUP BY content_type"
        )?;
        let content_types = type_stmt
            .query_map([], |r| {
                Ok(TypeStat {
                    content_type: r.get(0)?,
                    count: r.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        let mut daily_stmt = conn.prepare(
            "SELECT strftime('%Y-%m-%d', created_at) as day, COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) GROUP BY day ORDER BY day DESC LIMIT 14"
        )?;
        let daily_activity = daily_stmt
            .query_map([], |r| {
                Ok(DailyStat {
                    date: r.get(0)?,
                    count: r.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

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
        conn.execute(
            "DELETE FROM clip_bins
             WHERE clip_id IN (SELECT id FROM clips WHERE is_trashed = 1)
               AND bin_id IN (
                   SELECT id FROM bins WHERE COALESCE(bin_type, 'category') != 'tag'
               )",
            [],
        )?;
        conn.execute("UPDATE clips SET bin_id = NULL WHERE is_trashed = 1", [])?;
        let _ = self.log_activity_internal(
            &conn,
            "clips_trashed_all",
            &format!(
                "Moved all unpinned & unprotected clips to Trash ({} items)",
                count
            ),
        );
        let _ = self.enforce_trash_limit_internal(&conn);
        Ok(())
    }

    pub fn purge_unpinned_clips(&self) -> Result<()> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM clips WHERE is_pinned = 0 AND (is_protected IS NULL OR is_protected = 0)", [], |r| r.get(0)).unwrap_or(0);
        conn.execute(
            "DELETE FROM clips WHERE is_pinned = 0 AND (is_protected IS NULL OR is_protected = 0)",
            [],
        )?;
        let _ = self.log_activity_internal(
            &conn,
            "clips_purged_all",
            &format!(
                "Permanently deleted all unpinned & unprotected clips ({} items)",
                count
            ),
        );
        Ok(())
    }

    pub fn clear_all_clips(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM clips WHERE (is_protected IS NULL OR is_protected = 0)",
            [],
        )?;
        Ok(())
    }

    pub fn toggle_protected(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock();
        let current_protected: i32 = conn
            .query_row(
                "SELECT is_protected FROM clips WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let new_protected = if current_protected == 0 { 1 } else { 0 };
        conn.execute(
            "UPDATE clips SET is_protected = ?1 WHERE id = ?2",
            params![new_protected, id],
        )?;
        let action_str = if new_protected == 1 {
            "Protected"
        } else {
            "Unprotected"
        };
        let _ = self.log_activity_internal(
            &conn,
            "clip_protected_toggled",
            &format!("{} clip #{}", action_str, id),
        );
        Ok(new_protected == 1)
    }

    pub fn toggle_pin(&self, id: i64) -> Result<bool> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let current_pinned: i32 = tx.query_row(
            "SELECT is_pinned FROM clips WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )?;
        let new_pinned = if current_pinned == 0 { 1 } else { 0 };
        if new_pinned == 1 {
            tx.execute(
                "UPDATE clips SET pin_order = COALESCE(pin_order, 0) + 1 WHERE is_pinned = 1",
                [],
            )?;
        }
        tx.execute(
            "UPDATE clips SET is_pinned = ?1, pin_order = 0 WHERE id = ?2",
            params![new_pinned, id],
        )?;
        tx.commit()?;
        Ok(new_pinned == 1)
    }

    pub fn assign_to_bin(&self, clip_id: i64, bin_id: Option<i64>) -> Result<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let is_active = tx
            .query_row(
                "SELECT CASE WHEN is_trashed IS NULL OR is_trashed = 0 THEN 1 ELSE 0 END
             FROM clips WHERE id = ?1",
                params![clip_id],
                |row| row.get::<_, i32>(0),
            )
            .unwrap_or(0)
            != 0;
        if !is_active {
            return tx.commit();
        }
        tx.execute(
            "DELETE FROM clip_bins
             WHERE clip_id = ?1
               AND bin_id IN (
                   SELECT id FROM bins WHERE COALESCE(bin_type, 'category') != 'tag'
               )",
            params![clip_id],
        )?;
        if let Some(bid) = bin_id {
            tx.execute(
                "INSERT OR REPLACE INTO clip_bins (clip_id, bin_id) VALUES (?1, ?2)",
                params![clip_id, bid],
            )?;
            tx.execute(
                "UPDATE clips SET bin_id = ?1 WHERE id = ?2",
                params![bid, clip_id],
            )?;
        } else {
            tx.execute(
                "UPDATE clips SET bin_id = NULL WHERE id = ?1",
                params![clip_id],
            )?;
        }
        tx.commit()
    }

    pub fn add_clip_to_bin(&self, clip_id: i64, bin_id: i64) -> Result<()> {
        let conn = self.conn.lock();
        let is_active = conn
            .query_row(
                "SELECT CASE WHEN is_trashed IS NULL OR is_trashed = 0 THEN 1 ELSE 0 END
             FROM clips WHERE id = ?1",
                params![clip_id],
                |row| row.get::<_, i32>(0),
            )
            .unwrap_or(0)
            != 0;
        if !is_active {
            return Ok(());
        }
        conn.execute(
            "INSERT OR REPLACE INTO clip_bins (clip_id, bin_id) VALUES (?1, ?2)",
            params![clip_id, bin_id],
        )?;
        conn.execute(
            "UPDATE clips SET bin_id = ?1 WHERE id = ?2",
            params![bin_id, clip_id],
        )?;
        Ok(())
    }

    pub fn remove_clip_from_bin(&self, clip_id: i64, bin_id: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM clip_bins WHERE clip_id = ?1 AND bin_id = ?2",
            params![clip_id, bin_id],
        )?;
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    pub fn get_bins(&self) -> Result<Vec<Bin>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT id, name, icon, color, smart_rule, COALESCE(bin_type, 'category'), shortcut, created_at FROM bins ORDER BY id ASC")?;
        let bin_rows: Vec<(
            i64,
            String,
            String,
            String,
            Option<String>,
            String,
            Option<String>,
            String,
        )> = stmt
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

        let mut bins = Vec::new();
        for (id, name, icon, color, smart_rule, bin_type, shortcut, created_at) in bin_rows {
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
                        let sql = format!("SELECT COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) AND (({}) OR bin_id = ? OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?))", combined);
                        query_params.push(Box::new(id));
                        query_params.push(Box::new(id));
                        let param_refs: Vec<&dyn rusqlite::ToSql> =
                            query_params.iter().map(|p| p.as_ref()).collect();
                        conn.query_row(&sql, param_refs.as_slice(), |r| r.get(0))
                            .unwrap_or(0)
                    } else {
                        conn.query_row("SELECT COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) AND (bin_id = ?1 OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?1))", params![id], |r| r.get(0)).unwrap_or(0)
                    }
                } else {
                    conn.query_row("SELECT COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) AND (bin_id = ?1 OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?1))", params![id], |r| r.get(0)).unwrap_or(0)
                }
            } else {
                conn.query_row("SELECT COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) AND (bin_id = ?1 OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?1))", params![id], |r| r.get(0)).unwrap_or(0)
            };

            bins.push(Bin {
                id,
                name,
                icon,
                color,
                smart_rule,
                bin_type,
                shortcut,
                clip_count: Some(count),
                created_at,
            });
        }
        Ok(bins)
    }

    pub fn update_bin_shortcut(&self, id: i64, shortcut: Option<&str>) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE bins SET shortcut = ?1 WHERE id = ?2",
            params![shortcut, id],
        )?;
        Ok(())
    }

    pub fn get_bin_transform_ref(&self, bin_id: i64) -> Result<Option<String>> {
        let conn = self.conn.lock();
        let transform_id: Option<String> = conn.query_row(
            "SELECT default_transform_id FROM bins WHERE id = ?1",
            params![bin_id],
            |row| row.get(0),
        )?;
        Ok(transform_id.map(|id| format!("transform:{id}")))
    }

    pub fn set_bin_transform_ref(&self, bin_id: i64, transform_ref: Option<&str>) -> Result<()> {
        let transform_id =
            transform_ref.map(|value| value.strip_prefix("transform:").unwrap_or(value));
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE bins SET default_transform_id = ?1 WHERE id = ?2",
            params![transform_id, bin_id],
        )?;
        Ok(())
    }

    pub fn matching_smart_bin_transforms(
        &self,
        content_type: &str,
        text: &str,
        source_app: &str,
    ) -> Result<Vec<(i64, String)>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, smart_rule, default_transform_id FROM bins
             WHERE smart_rule IS NOT NULL AND default_transform_id IS NOT NULL ORDER BY id ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        let mut matches = Vec::new();
        for row in rows {
            let (bin_id, rule_json, transform_id) = row?;
            let Ok(rule) = serde_json::from_str::<serde_json::Value>(&rule_json) else {
                continue;
            };
            let condition_matches = |kind: &str, value: &str| match kind {
                "content_type" => content_type.eq_ignore_ascii_case(value),
                "source_app" => source_app.to_lowercase().contains(&value.to_lowercase()),
                "contains" => text.to_lowercase().contains(&value.to_lowercase()),
                _ => false,
            };
            let matched = if let Some(conditions) = rule["conditions"].as_array() {
                let values = conditions
                    .iter()
                    .map(|condition| {
                        condition_matches(
                            condition["type"].as_str().unwrap_or(""),
                            condition["value"].as_str().unwrap_or(""),
                        )
                    })
                    .collect::<Vec<_>>();
                if rule["match"].as_str() == Some("all") {
                    values.iter().all(|v| *v)
                } else {
                    values.iter().any(|v| *v)
                }
            } else {
                condition_matches(
                    rule["type"].as_str().unwrap_or(""),
                    rule["value"].as_str().unwrap_or(""),
                )
            };
            if matched {
                matches.push((bin_id, format!("transform:{transform_id}")));
            }
        }
        Ok(matches)
    }

    pub fn create_bin_with_type(
        &self,
        name: &str,
        icon: &str,
        color: &str,
        smart_rule: Option<&str>,
        bin_type: &str,
    ) -> Result<Bin> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO bins (name, icon, color, smart_rule, bin_type) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![name, icon, color, smart_rule, bin_type],
        )?;
        let id = conn.last_insert_rowid();
        conn.query_row(
            "SELECT id, name, icon, color, smart_rule, COALESCE(bin_type, 'category'), shortcut, created_at FROM bins WHERE id = ?1",
            params![id],
            |row| {
                Ok(Bin {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    icon: row.get(2)?,
                    color: row.get(3)?,
                    smart_rule: row.get(4)?,
                    bin_type: row.get(5)?,
                    shortcut: row.get(6)?,
                    clip_count: Some(0),
                    created_at: row.get(7)?,
                })
            },
        )
    }

    pub fn create_bin(
        &self,
        name: &str,
        icon: &str,
        color: &str,
        smart_rule: Option<&str>,
    ) -> Result<Bin> {
        self.create_bin_with_type(name, icon, color, smart_rule, "category")
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

    #[cfg(test)]
    pub fn get_clip_versions(&self, clip_id: i64) -> Result<Vec<ClipVersion>> {
        self.get_clip_versions_page(clip_id, -1, 0)
    }

    pub fn get_clip_versions_page(
        &self,
        clip_id: i64,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ClipVersion>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, clip_id, text_content, context_json, created_at
             FROM clip_versions WHERE clip_id = ?1
             ORDER BY created_at DESC, id DESC LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(params![clip_id, limit, offset.max(0)], |row| {
            let context_json: Option<String> = row.get(3)?;
            let context = context_json
                .as_deref()
                .and_then(|value| serde_json::from_str::<ClipRevisionContext>(value).ok());
            Ok(ClipVersion {
                id: row.get(0)?,
                clip_id: row.get(1)?,
                text_content: row.get(2)?,
                action_kind: context.as_ref().map(|value| value.action_kind.clone()),
                action_label: context.as_ref().map(|value| value.action_label.clone()),
                restores_organization: context
                    .as_ref()
                    .and_then(|value| value.organization.as_ref())
                    .is_some(),
                created_at: row.get(4)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_clip_version_count(&self, clip_id: i64) -> Result<i64> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT COUNT(*) FROM clip_versions WHERE clip_id = ?1",
            params![clip_id],
            |row| row.get(0),
        )
    }

    pub fn restore_clip_version(&self, clip_id: i64, version_id: i64) -> Result<ClipItem> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let (target_text, context_json): (String, Option<String>) = tx.query_row(
            "SELECT text_content, context_json FROM clip_versions WHERE id = ?1 AND clip_id = ?2",
            params![version_id, clip_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let target_context = context_json
            .as_deref()
            .and_then(|value| serde_json::from_str::<ClipRevisionContext>(value).ok());
        let (current_text, current_bin_id, is_trashed, current_transformation_id): (
            Option<String>,
            Option<i64>,
            i32,
            Option<String>,
        ) = tx
            .query_row(
                "SELECT text_content, bin_id, COALESCE(is_trashed, 0), current_transformation_id FROM clips WHERE id = ?1",
                params![clip_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;
        if is_trashed != 0 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Restore this clip from Trash before restoring a revision".to_string(),
            ));
        }

        let target_bin_id = target_context
            .as_ref()
            .and_then(|context| context.organization.as_ref())
            .map(|organization| organization.category_bin_id);
        let organization_changes = target_bin_id
            .map(|target| target != current_bin_id)
            .unwrap_or(false);
        let target_transformation_id = target_context
            .as_ref()
            .and_then(|context| context.current_transformation_id.clone());
        if current_text.as_deref() == Some(target_text.as_str())
            && !organization_changes
            && current_transformation_id == target_transformation_id
        {
            tx.commit()?;
            return self.get_clip_by_id_internal(&conn, clip_id);
        }

        if let Some(current_text) = current_text {
            let inverse_context = target_bin_id.map(|_| ClipRevisionContext {
                schema_version: 1,
                action_kind: "restore".to_string(),
                action_label: "Before restoring an earlier revision".to_string(),
                organization: Some(ClipRevisionOrganization {
                    category_bin_id: current_bin_id,
                }),
                current_transformation_id: current_transformation_id.clone(),
            });
            let inverse_context = inverse_context.or_else(|| {
                Some(ClipRevisionContext {
                    schema_version: 1,
                    action_kind: "restore".to_string(),
                    action_label: "Before restoring an earlier revision".to_string(),
                    organization: None,
                    current_transformation_id: current_transformation_id.clone(),
                })
            });
            let inverse_json = inverse_context
                .map(|context| serde_json::to_string(&context))
                .transpose()
                .map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))?;
            tx.execute(
                "INSERT INTO clip_versions (clip_id, text_content, context_json) VALUES (?1, ?2, ?3)",
                params![clip_id, current_text, inverse_json],
            )?;
        }

        tx.execute(
            "UPDATE clips SET text_content = ?1, current_transformation_id = ?2 WHERE id = ?3",
            params![target_text, target_transformation_id, clip_id],
        )?;
        if let Some(target_bin_id) = target_bin_id {
            tx.execute(
                "DELETE FROM clip_bins
                 WHERE clip_id = ?1 AND bin_id IN (
                    SELECT id FROM bins WHERE COALESCE(bin_type, 'category') != 'tag'
                 )",
                params![clip_id],
            )?;
            let restored_bin_id = if let Some(bin_id) = target_bin_id {
                let changed = tx.execute(
                    "INSERT OR REPLACE INTO clip_bins (clip_id, bin_id)
                     SELECT ?1, id FROM bins
                     WHERE id = ?2 AND COALESCE(bin_type, 'category') != 'tag'",
                    params![clip_id, bin_id],
                )?;
                (changed > 0).then_some(bin_id)
            } else {
                None
            };
            tx.execute(
                "UPDATE clips SET bin_id = ?1 WHERE id = ?2",
                params![restored_bin_id, clip_id],
            )?;
        }
        Self::prune_clip_versions_internal(&tx, clip_id)?;
        tx.commit()?;
        let _ = self.log_activity_internal(
            &conn,
            "clip_revision_restored",
            &format!("Restored revision #{version_id} for clip #{clip_id}"),
        );
        self.get_clip_by_id_internal(&conn, clip_id)
    }

    pub fn update_bin(
        &self,
        id: i64,
        name: &str,
        icon: &str,
        color: &str,
        smart_rule: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE bins SET name = ?1, icon = ?2, color = ?3, smart_rule = ?4 WHERE id = ?5",
            params![name, icon, color, smart_rule, id],
        )?;
        Ok(())
    }

    pub fn delete_bin(
        &self,
        id: i64,
        disposition: &str,
        destination_bin_id: Option<i64>,
    ) -> Result<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;

        let bin_name: String =
            tx.query_row("SELECT name FROM bins WHERE id = ?1", params![id], |row| {
                row.get(0)
            })?;
        let clip_ids = {
            let mut stmt = tx.prepare(
                "SELECT id FROM clips
                 WHERE (is_trashed IS NULL OR is_trashed = 0)
                   AND (bin_id = ?1 OR id IN (SELECT clip_id FROM clip_bins WHERE bin_id = ?1))",
            )?;
            let ids = stmt
                .query_map(params![id], |row| row.get::<_, i64>(0))?
                .collect::<Result<Vec<_>>>()?;
            ids
        };

        match disposition {
            "keep" => {
                for clip_id in &clip_ids {
                    tx.execute(
                        "UPDATE clips SET bin_id = NULL WHERE id = ?1 AND bin_id = ?2",
                        params![clip_id, id],
                    )?;
                }
            }
            "trash" => {
                for clip_id in &clip_ids {
                    let changed = tx.execute(
                        "UPDATE clips
                         SET is_trashed = 1,
                             trashed_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                         WHERE id = ?1 AND (is_protected IS NULL OR is_protected = 0)",
                        params![clip_id],
                    )?;
                    if changed > 0 {
                        self.clear_category_bin_assignments_internal(&tx, *clip_id)?;
                    } else {
                        tx.execute(
                            "UPDATE clips SET bin_id = NULL WHERE id = ?1 AND bin_id = ?2",
                            params![clip_id, id],
                        )?;
                    }
                }
                self.enforce_trash_limit_internal(&tx)?;
            }
            "move" => {
                let destination_id = destination_bin_id.ok_or_else(|| {
                    rusqlite::Error::InvalidParameterName(
                        "A destination Bin is required when moving clips".to_string(),
                    )
                })?;
                if destination_id == id {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "The destination Bin must be different from the deleted Bin".to_string(),
                    ));
                }
                let destination_exists = tx.query_row(
                    "SELECT EXISTS(
                         SELECT 1 FROM bins
                         WHERE id = ?1
                           AND (smart_rule IS NULL OR TRIM(smart_rule) = '')
                           AND COALESCE(bin_type, 'category') != 'tag'
                     )",
                    params![destination_id],
                    |row| row.get::<_, bool>(0),
                )?;
                if !destination_exists {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "The destination must be another manual Bin".to_string(),
                    ));
                }
                for clip_id in &clip_ids {
                    self.clear_category_bin_assignments_internal(&tx, *clip_id)?;
                    tx.execute(
                        "INSERT OR REPLACE INTO clip_bins (clip_id, bin_id) VALUES (?1, ?2)",
                        params![clip_id, destination_id],
                    )?;
                    tx.execute(
                        "UPDATE clips SET bin_id = ?1 WHERE id = ?2",
                        params![destination_id, clip_id],
                    )?;
                }
            }
            _ => {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Unknown Bin deletion outcome".to_string(),
                ));
            }
        }

        tx.execute("DELETE FROM clip_bins WHERE bin_id = ?1", params![id])?;
        tx.execute("DELETE FROM bins WHERE id = ?1", params![id])?;
        let outcome = match disposition {
            "trash" => "moved its clips to Trash",
            "move" => "moved its clips to another Bin",
            _ => "kept its clips in No Bin",
        };
        self.log_activity_internal(
            &tx,
            "bin_deleted",
            &format!(
                "Deleted Bin \"{}\" and {} ({} clips)",
                bin_name,
                outcome,
                clip_ids.len()
            ),
        )?;
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
        let bins = self.get_bins()?;
        let pipelines = self.get_pipelines()?;
        let operations = self.get_operations()?;
        let saved_transforms = self.get_saved_transforms()?;
        let bin_transforms = bins
            .iter()
            .filter_map(|bin| {
                self.get_bin_transform_ref(bin.id)
                    .ok()
                    .flatten()
                    .map(|transform_ref| BinTransformBinding {
                        bin_id: bin.id,
                        transform_ref,
                    })
            })
            .collect();

        let payload = BackupPayload {
            version: BACKUP_SCHEMA_VERSION,
            timestamp: chrono::Utc::now().to_rfc3339(),
            clips,
            bins,
            pipelines,
            operations,
            saved_transforms,
            bin_transforms,
        };

        serde_json::to_string_pretty(&payload)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
    }

    pub fn import_backup_json(&self, json_str: &str) -> Result<usize> {
        let payload: BackupPayload = serde_json::from_str(json_str)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        if !(1..=BACKUP_SCHEMA_VERSION).contains(&payload.version) {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "unsupported backup schema version {} (supported: 1-{BACKUP_SCHEMA_VERSION})",
                payload.version
            )));
        }
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let mut bin_id_map = std::collections::HashMap::new();

        for bin in payload.bins {
            let existing_id = tx.query_row(
                "SELECT id FROM bins WHERE name = ?1 AND COALESCE(bin_type, 'category') = ?2 LIMIT 1",
                params![bin.name, bin.bin_type],
                |row| row.get::<_, i64>(0),
            ).ok();
            let new_id = if let Some(id) = existing_id {
                tx.execute(
                    "UPDATE bins SET icon = ?1, color = ?2, smart_rule = ?3, shortcut = ?4 WHERE id = ?5",
                    params![bin.icon, bin.color, bin.smart_rule, bin.shortcut, id],
                )?;
                id
            } else {
                tx.execute(
                    "INSERT INTO bins (name, icon, color, smart_rule, bin_type, shortcut, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![bin.name, bin.icon, bin.color, bin.smart_rule, bin.bin_type, bin.shortcut, bin.created_at],
                )?;
                tx.last_insert_rowid()
            };
            bin_id_map.insert(bin.id, new_id);
        }

        for operation in payload.operations {
            // Registry built-ins are definitions, not persisted records.
            if operation.id < 0 {
                continue;
            }
            let operation_id = operation.stable_id.strip_prefix("custom:").ok_or_else(|| {
                rusqlite::Error::InvalidParameterName(
                    "custom operation backup is missing a stable reference".to_string(),
                )
            })?;
            let (executor_kind, config_json) =
                Self::operation_storage_fields(&operation.op_type, operation.config.as_deref());
            tx.execute(
                "INSERT INTO custom_operations
                    (id, name, executor_kind, config_json, category, trusted, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    executor_kind = excluded.executor_kind,
                    config_json = excluded.config_json,
                    category = excluded.category,
                    trusted = 0,
                    updated_at = CURRENT_TIMESTAMP",
                params![
                    operation_id,
                    operation.name,
                    executor_kind,
                    config_json,
                    operation.category,
                    operation.created_at
                ],
            )?;
        }

        for pipeline in payload.pipelines {
            let pipeline_id = pipeline
                .stable_ref
                .strip_prefix("pipeline:")
                .ok_or_else(|| {
                    rusqlite::Error::InvalidParameterName(
                        "pipeline backup is missing a stable reference".to_string(),
                    )
                })?;
            let steps = pipeline
                .steps
                .iter()
                .map(|step| PipelineStepInput {
                    operation_ref: step.operation_ref.clone(),
                    config_json: step.config_json.clone(),
                    failure_policy: step.failure_policy.clone(),
                })
                .collect::<Vec<_>>();
            Self::validate_pipeline_steps(&tx, &steps)?;
            tx.execute(
                "INSERT INTO pipelines
                    (id, name, shortcut, revision, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    shortcut = excluded.shortcut,
                    revision = excluded.revision,
                    updated_at = excluded.updated_at",
                params![
                    pipeline_id,
                    pipeline.name,
                    pipeline.shortcut,
                    pipeline.revision,
                    pipeline.created_at,
                    pipeline.updated_at
                ],
            )?;
            tx.execute(
                "DELETE FROM pipeline_steps WHERE pipeline_id = ?1",
                params![pipeline_id],
            )?;
            Self::insert_pipeline_steps(&tx, pipeline_id, &steps)?;
        }

        for transform in payload.saved_transforms {
            let transform_id =
                transform
                    .stable_ref
                    .strip_prefix("transform:")
                    .ok_or_else(|| {
                        rusqlite::Error::InvalidParameterName(
                            "saved Transform backup is missing a stable reference".to_string(),
                        )
                    })?;
            transform
                .plan
                .validate()
                .map_err(rusqlite::Error::InvalidParameterName)?;
            let plan_json = serde_json::to_string(&transform.plan)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            tx.execute(
                "INSERT INTO saved_transforms
                    (id, name, plan_json, connection_id, revision, created_at, updated_at)
                 VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    plan_json = excluded.plan_json,
                    connection_id = NULL,
                    revision = excluded.revision,
                    updated_at = excluded.updated_at",
                params![
                    transform_id,
                    transform.name,
                    plan_json,
                    transform.revision,
                    transform.created_at,
                    transform.updated_at
                ],
            )?;
        }

        for binding in payload.bin_transforms {
            let Some(mapped_bin_id) = bin_id_map.get(&binding.bin_id) else {
                continue;
            };
            let transform_id = binding
                .transform_ref
                .strip_prefix("transform:")
                .unwrap_or(&binding.transform_ref);
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM saved_transforms WHERE id = ?1)",
                params![transform_id],
                |row| row.get(0),
            )?;
            if exists {
                tx.execute(
                    "UPDATE bins SET default_transform_id = ?1 WHERE id = ?2",
                    params![transform_id, mapped_bin_id],
                )?;
            }
        }

        let mut imported = 0;
        for clip in payload.clips {
            let mapped_primary_bin = clip.bin_id.and_then(|id| bin_id_map.get(&id).copied());
            tx.execute(
                "INSERT INTO clips (
                    content_type, text_content, html_content, image_base64, image_path, content_hash,
                    source_app, is_pinned, is_protected, pin_order, bin_id, note,
                    is_trashed, trashed_at, created_at
                 ) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                 ON CONFLICT(content_hash) DO UPDATE SET
                    content_type = excluded.content_type,
                    text_content = excluded.text_content,
                    html_content = excluded.html_content,
                    image_base64 = excluded.image_base64,
                    source_app = excluded.source_app,
                    is_pinned = excluded.is_pinned,
                    is_protected = excluded.is_protected,
                    pin_order = excluded.pin_order,
                    bin_id = excluded.bin_id,
                    note = excluded.note,
                    is_trashed = excluded.is_trashed,
                    trashed_at = excluded.trashed_at,
                    created_at = excluded.created_at",
                params![
                    clip.content_type, clip.text_content, clip.html_content, clip.image_base64,
                    clip.content_hash, clip.source_app, clip.is_pinned, clip.is_protected,
                    clip.pin_order, mapped_primary_bin, clip.note, clip.is_trashed,
                    clip.trashed_at, clip.created_at,
                ],
            )?;
            let new_clip_id = tx.query_row(
                "SELECT id FROM clips WHERE content_hash = ?1",
                params![clip.content_hash],
                |row| row.get::<_, i64>(0),
            )?;
            tx.execute(
                "DELETE FROM clip_bins WHERE clip_id = ?1",
                params![new_clip_id],
            )?;
            for old_bin_id in clip.bin_ids.unwrap_or_default() {
                if let Some(new_bin_id) = bin_id_map.get(&old_bin_id) {
                    tx.execute(
                        "INSERT OR IGNORE INTO clip_bins (clip_id, bin_id) VALUES (?1, ?2)",
                        params![new_clip_id, new_bin_id],
                    )?;
                }
            }
            if let Some(new_bin_id) = mapped_primary_bin {
                tx.execute(
                    "INSERT OR IGNORE INTO clip_bins (clip_id, bin_id) VALUES (?1, ?2)",
                    params![new_clip_id, new_bin_id],
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
                    bin_id, note, COALESCE(is_trashed, 0), trashed_at, created_at,
                    (SELECT GROUP_CONCAT(bin_id) FROM clip_bins WHERE clip_id = clips.id),
                    current_transformation_id IS NOT NULL
             FROM clips ORDER BY created_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let primary_bin_id: Option<i64> = row.get(11)?;
            let bin_ids_csv: Option<String> = row.get(16)?;
            let mut bin_ids = primary_bin_id.into_iter().collect::<Vec<_>>();
            for value in bin_ids_csv.unwrap_or_default().split(',') {
                if let Ok(id) = value.parse::<i64>() {
                    if !bin_ids.contains(&id) {
                        bin_ids.push(id);
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
                is_transformed: row.get::<_, i32>(17)? != 0,
                pin_order: row.get(10)?,
                bin_id: primary_bin_id,
                bin_ids: Some(bin_ids),
                note: row.get(12)?,
                is_trashed: row.get::<_, i32>(13)? != 0,
                trashed_at: row.get(14)?,
                created_at: row.get(15)?,
            })
        })?;
        rows.collect()
    }

    fn normalize_json_config(config: Option<&str>) -> String {
        match config {
            Some(value) if serde_json::from_str::<serde_json::Value>(value).is_ok() => {
                value.to_string()
            }
            Some(value) => serde_json::Value::String(value.to_string()).to_string(),
            None => "{}".to_string(),
        }
    }

    fn canonical_executor_kind(operation_type: &str) -> &str {
        match operation_type {
            "shell_script" => "shell",
            "regex" | "cli" | "shell" | "http" | "ai" => operation_type,
            _ => "cli",
        }
    }

    fn operation_storage_fields(op_type: &str, config: Option<&str>) -> (String, String) {
        if crate::operation_registry::is_builtin_operation(op_type) {
            (
                "builtin".to_string(),
                serde_json::json!({
                    "key": op_type,
                    "legacy_config": config.map(|value| Self::normalize_json_config(Some(value))),
                })
                .to_string(),
            )
        } else {
            (
                Self::canonical_executor_kind(op_type).to_string(),
                Self::normalize_json_config(config),
            )
        }
    }

    fn legacy_operation_fields(executor_kind: &str, config_json: &str) -> (String, Option<String>) {
        if executor_kind == "builtin" {
            let value = serde_json::from_str::<serde_json::Value>(config_json).unwrap_or_default();
            let operation_type = value["key"].as_str().unwrap_or("unknown").to_string();
            let config = value.get("legacy_config").and_then(|config| {
                if config.is_null() {
                    None
                } else if let Some(text) = config.as_str() {
                    Some(text.to_string())
                } else {
                    Some(config.to_string())
                }
            });
            (operation_type, config)
        } else {
            let operation_type = if executor_kind == "shell" {
                "shell_script"
            } else {
                executor_kind
            };
            let value = serde_json::from_str::<serde_json::Value>(config_json).ok();
            let config = value.map(|value| {
                value
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| value.to_string())
            });
            (operation_type.to_string(), config)
        }
    }

    pub fn resolve_custom_operation(
        &self,
        operation_ref: &str,
    ) -> Result<Option<ResolvedCustomOperation>> {
        let Some(operation_id) = operation_ref.strip_prefix("custom:") else {
            return Ok(None);
        };
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT executor_kind, config_json, enabled, trusted
             FROM custom_operations WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![operation_id])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        Ok(Some(ResolvedCustomOperation {
            executor_kind: row.get(0)?,
            config_json: row.get(1)?,
            enabled: row.get::<_, i64>(2)? != 0,
            trusted: row.get::<_, i64>(3)? != 0,
        }))
    }

    pub fn resolve_pipeline(&self, pipeline_ref: &str) -> Result<Option<ResolvedPipeline>> {
        let pipeline_id = pipeline_ref
            .strip_prefix("pipeline:")
            .unwrap_or(pipeline_ref);
        let conn = self.conn.lock();
        let revision = match conn.query_row(
            "SELECT revision FROM pipelines WHERE id = ?1",
            params![pipeline_id],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(revision) => revision,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(error) => return Err(error),
        };
        let mut stmt = conn.prepare(
            "SELECT position, operation_ref, config_json, failure_policy
             FROM pipeline_steps WHERE pipeline_id = ?1 ORDER BY position ASC",
        )?;
        let steps = stmt
            .query_map(params![pipeline_id], |row| {
                Ok(ResolvedPipelineStep {
                    position: row.get(0)?,
                    operation_ref: row.get(1)?,
                    config_json: row.get(2)?,
                    failure_policy: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        Ok(Some(ResolvedPipeline { revision, steps }))
    }

    pub fn begin_transformation_execution(
        &self,
        request: TransformationExecutionStart<'_>,
    ) -> Result<String> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO transformation_executions
                (target_kind, target_ref, target_revision, source_clip_id,
                 trigger_kind, destination_kind, input_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                request.target_kind,
                request.target_ref,
                request.target_revision,
                request.source_clip_id,
                request.trigger_kind,
                request.destination_kind,
                request.input_hash
            ],
        )?;
        conn.query_row(
            "SELECT id FROM transformation_executions WHERE rowid = last_insert_rowid()",
            [],
            |row| row.get(0),
        )
    }

    pub fn finish_transformation_execution(
        &self,
        execution_id: &str,
        duration_ms: i64,
        output_hash: Option<&str>,
        error_summary: Option<&str>,
    ) -> Result<()> {
        let status = if error_summary.is_some() {
            "failed"
        } else {
            "succeeded"
        };
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE transformation_executions
             SET duration_ms = ?1, status = ?2, output_hash = ?3, error_summary = ?4,
                 completed_at = CURRENT_TIMESTAMP
             WHERE id = ?5",
            params![
                duration_ms,
                status,
                output_hash,
                error_summary,
                execution_id
            ],
        )?;
        Ok(())
    }

    pub fn cancel_transformation_execution(
        &self,
        execution_id: &str,
        duration_ms: i64,
    ) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE transformation_executions
             SET duration_ms = ?1, status = 'cancelled', output_hash = NULL,
                 error_summary = NULL, completed_at = CURRENT_TIMESTAMP
             WHERE id = ?2",
            params![duration_ms, execution_id],
        )?;
        Ok(())
    }

    pub fn start_transformation_execution(&self, execution_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE transformation_executions SET status = 'running'
             WHERE id = ?1 AND status = 'queued'",
            params![execution_id],
        )?;
        Ok(())
    }

    pub fn get_clip_transformation_executions(
        &self,
        clip_id: i64,
    ) -> Result<Vec<TransformationExecution>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, target_kind, target_ref, target_revision, source_clip_id,
                    trigger_kind, destination_kind, started_at, completed_at,
                    duration_ms, status, error_summary
             FROM transformation_executions
             WHERE source_clip_id = ?1
             ORDER BY started_at DESC, rowid DESC
             LIMIT 25",
        )?;
        let rows = statement.query_map(params![clip_id], |row| {
            Ok(TransformationExecution {
                id: row.get(0)?,
                target_kind: row.get(1)?,
                target_ref: row.get(2)?,
                target_revision: row.get(3)?,
                source_clip_id: row.get(4)?,
                trigger_kind: row.get(5)?,
                destination_kind: row.get(6)?,
                started_at: row.get(7)?,
                completed_at: row.get(8)?,
                duration_ms: row.get(9)?,
                status: row.get(10)?,
                error_summary: row.get(11)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_pipelines(&self) -> Result<Vec<Pipeline>> {
        let conn = self.conn.lock();
        let refs = {
            let mut statement = conn.prepare("SELECT id FROM pipelines ORDER BY row_id ASC")?;
            let refs = statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>>>()?;
            refs
        };
        refs.into_iter()
            .map(|stable_id| Self::pipeline_by_id(&conn, &stable_id))
            .collect()
    }

    fn saved_transform_by_id(conn: &Connection, transform_id: &str) -> Result<SavedTransform> {
        conn.query_row(
            "SELECT row_id, id, name, plan_json, connection_id, revision, created_at, updated_at
             FROM saved_transforms WHERE id = ?1",
            params![transform_id],
            |row| {
                let stable_id: String = row.get(1)?;
                let plan_json: String = row.get(3)?;
                let plan = serde_json::from_str(&plan_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
                Ok(SavedTransform {
                    id: row.get(0)?,
                    stable_ref: format!("transform:{stable_id}"),
                    name: row.get(2)?,
                    plan,
                    connection_id: row.get(4)?,
                    revision: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            },
        )
    }

    pub fn get_saved_transforms(&self) -> Result<Vec<SavedTransform>> {
        let conn = self.conn.lock();
        let ids = {
            let mut statement = conn
                .prepare("SELECT id FROM saved_transforms ORDER BY updated_at DESC, row_id DESC")?;
            let ids = statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>>>()?;
            ids
        };
        ids.into_iter()
            .map(|id| Self::saved_transform_by_id(&conn, &id))
            .collect()
    }

    pub fn resolve_saved_transform(&self, transform_ref: &str) -> Result<Option<SavedTransform>> {
        let transform_id = transform_ref
            .strip_prefix("transform:")
            .unwrap_or(transform_ref);
        let conn = self.conn.lock();
        match Self::saved_transform_by_id(&conn, transform_id) {
            Ok(transform) => Ok(Some(transform)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn create_saved_transform(
        &self,
        name: &str,
        plan: &crate::transformation_intent::TransformationPlan,
        connection_id: Option<&str>,
    ) -> Result<SavedTransform> {
        plan.validate()
            .map_err(rusqlite::Error::InvalidParameterName)?;
        let plan_json = serde_json::to_string(plan).map_err(|error| {
            rusqlite::Error::InvalidParameterName(format!("invalid Transform: {error}"))
        })?;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO saved_transforms (name, plan_json, connection_id)
             VALUES (?1, ?2, ?3)",
            params![name.trim(), plan_json, connection_id],
        )?;
        let row_id = conn.last_insert_rowid();
        let stable_id: String = conn.query_row(
            "SELECT id FROM saved_transforms WHERE row_id = ?1",
            params![row_id],
            |row| row.get(0),
        )?;
        Self::saved_transform_by_id(&conn, &stable_id)
    }

    pub fn update_saved_transform(
        &self,
        transform_ref: &str,
        name: &str,
        plan: &crate::transformation_intent::TransformationPlan,
        connection_id: Option<&str>,
    ) -> Result<SavedTransform> {
        plan.validate()
            .map_err(rusqlite::Error::InvalidParameterName)?;
        let plan_json = serde_json::to_string(plan).map_err(|error| {
            rusqlite::Error::InvalidParameterName(format!("invalid Transform: {error}"))
        })?;
        let transform_id = transform_ref
            .strip_prefix("transform:")
            .unwrap_or(transform_ref);
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE saved_transforms
             SET name = ?1,
                 plan_json = ?2,
                 connection_id = ?3,
                 revision = revision + 1,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?4",
            params![name.trim(), plan_json, connection_id, transform_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Self::saved_transform_by_id(&conn, transform_id)
    }

    pub fn delete_saved_transform(&self, transform_ref: &str) -> Result<()> {
        let transform_id = transform_ref
            .strip_prefix("transform:")
            .unwrap_or(transform_ref);
        let conn = self.conn.lock();
        let changed = conn.execute(
            "DELETE FROM saved_transforms WHERE id = ?1",
            params![transform_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    pub fn apply_transform_output_to_clip(
        &self,
        request: TransformClipApplication<'_>,
    ) -> Result<ClipTransformationProvenance> {
        let TransformClipApplication {
            clip_id,
            transform_ref,
            expected_input,
            output,
            connection_id,
            duration_ms,
            bin_move,
        } = request;
        let transform_id = transform_ref
            .strip_prefix("transform:")
            .unwrap_or(transform_ref);
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let (transform_name, transform_revision): (String, i64) = tx.query_row(
            "SELECT name, revision FROM saved_transforms WHERE id = ?1",
            params![transform_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let (current_text, is_trashed, current_transformation_id): (
            Option<String>,
            i32,
            Option<String>,
        ) = tx.query_row(
            "SELECT text_content, COALESCE(is_trashed, 0), current_transformation_id FROM clips WHERE id = ?1",
            params![clip_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        if is_trashed != 0 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Restore this clip before transforming it".to_string(),
            ));
        }
        if current_text.as_deref() != Some(expected_input) {
            return Err(rusqlite::Error::InvalidParameterName(
                "The clip changed after this preview was generated; preview it again".to_string(),
            ));
        }
        if expected_input == output {
            return Err(rusqlite::Error::InvalidParameterName(
                "The Transform did not change the clip".to_string(),
            ));
        }
        let (action_label, organization) =
            if let Some((previous_bin_id, destination_bin_id)) = bin_move {
                let destination_name = tx
                    .query_row(
                        "SELECT name FROM bins WHERE id = ?1",
                        params![destination_bin_id],
                        |row| row.get::<_, String>(0),
                    )
                    .unwrap_or_else(|_| format!("Bin #{destination_bin_id}"));
                (
                    format!("Moved to {destination_name} · Applied {transform_name}"),
                    Some(ClipRevisionOrganization {
                        category_bin_id: previous_bin_id,
                    }),
                )
            } else {
                (format!("Applied {transform_name}"), None)
            };
        let context_json = serde_json::to_string(&ClipRevisionContext {
            schema_version: 1,
            action_kind: if organization.is_some() {
                "transform_bin_drop".to_string()
            } else {
                "transform".to_string()
            },
            action_label,
            organization,
            current_transformation_id,
        })
        .map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))?;
        tx.execute(
            "INSERT INTO clip_versions (clip_id, text_content, context_json) VALUES (?1, ?2, ?3)",
            params![clip_id, expected_input, context_json],
        )?;
        Self::prune_clip_versions_internal(&tx, clip_id)?;
        let transformation_id: String =
            tx.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))?;
        tx.execute(
            "INSERT INTO clip_transformations
                (id, clip_id, transform_id, transform_name, transform_revision, connection_id, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                transformation_id,
                clip_id,
                transform_id,
                transform_name,
                transform_revision,
                connection_id,
                duration_ms.max(0)
            ],
        )?;
        tx.execute(
            "UPDATE clips SET text_content = ?1, current_transformation_id = ?2 WHERE id = ?3",
            params![output, transformation_id, clip_id],
        )?;
        let created_at: String = tx.query_row(
            "SELECT created_at FROM clip_transformations WHERE rowid = last_insert_rowid()",
            [],
            |row| row.get(0),
        )?;
        tx.commit()?;
        Ok(ClipTransformationProvenance {
            transform_ref: format!("transform:{transform_id}"),
            transform_name,
            transform_revision,
            connection_id: connection_id.map(str::to_string),
            duration_ms: duration_ms.max(0),
            created_at,
        })
    }

    pub fn get_clip_transformation_provenance(
        &self,
        clip_id: i64,
    ) -> Result<Option<ClipTransformationProvenance>> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT transformation.transform_id, transformation.transform_name,
                    transformation.transform_revision, transformation.connection_id,
                    transformation.duration_ms, transformation.created_at
             FROM clips
             JOIN clip_transformations transformation
               ON transformation.id = clips.current_transformation_id
             WHERE clips.id = ?1",
            params![clip_id],
            |row| {
                let transform_id: Option<String> = row.get(0)?;
                Ok(ClipTransformationProvenance {
                    transform_ref: transform_id
                        .map(|id| format!("transform:{id}"))
                        .unwrap_or_else(|| "transform:deleted".to_string()),
                    transform_name: row.get(1)?,
                    transform_revision: row.get(2)?,
                    connection_id: row.get(3)?,
                    duration_ms: row.get(4)?,
                    created_at: row.get(5)?,
                })
            },
        );
        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn pipeline_steps(conn: &Connection, pipeline_id: &str) -> Result<Vec<PipelineStep>> {
        let mut statement = conn.prepare(
            "SELECT position, operation_ref, config_json, failure_policy
             FROM pipeline_steps WHERE pipeline_id = ?1 ORDER BY position ASC",
        )?;
        let steps = statement
            .query_map(params![pipeline_id], |row| {
                Ok(PipelineStep {
                    position: row.get(0)?,
                    operation_ref: row.get(1)?,
                    config_json: row.get(2)?,
                    failure_policy: row.get(3)?,
                })
            })?
            .collect();
        steps
    }

    fn pipeline_by_id(conn: &Connection, pipeline_id: &str) -> Result<Pipeline> {
        let mut pipeline = conn.query_row(
            "SELECT row_id, id, name, shortcut, revision, created_at, updated_at
             FROM pipelines WHERE id = ?1",
            params![pipeline_id],
            |row| {
                let stable_id = row.get::<_, String>(1)?;
                Ok(Pipeline {
                    id: row.get(0)?,
                    stable_ref: format!("pipeline:{stable_id}"),
                    name: row.get(2)?,
                    shortcut: row.get(3)?,
                    revision: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    steps: Vec::new(),
                })
            },
        )?;
        pipeline.steps = Self::pipeline_steps(conn, pipeline_id)?;
        Ok(pipeline)
    }

    fn validate_pipeline_steps(conn: &Connection, steps: &[PipelineStepInput]) -> Result<()> {
        if steps.is_empty() {
            return Err(rusqlite::Error::InvalidParameterName(
                "pipeline requires at least one operation".to_string(),
            ));
        }
        for step in steps {
            if !matches!(step.failure_policy.as_str(), "stop" | "skip") {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "invalid failure policy: {}",
                    step.failure_policy
                )));
            }
            if let Some(config) = &step.config_json {
                serde_json::from_str::<serde_json::Value>(config).map_err(|error| {
                    rusqlite::Error::InvalidParameterName(format!(
                        "invalid step config JSON: {error}"
                    ))
                })?;
            }
            if let Some(key) = step.operation_ref.strip_prefix("builtin:") {
                if !crate::operation_registry::is_builtin_operation(key) {
                    return Err(rusqlite::Error::InvalidParameterName(format!(
                        "unknown operation reference: {}",
                        step.operation_ref
                    )));
                }
            } else if let Some(custom_id) = step.operation_ref.strip_prefix("custom:") {
                let exists: bool = conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM custom_operations WHERE id = ?1)",
                    params![custom_id],
                    |row| row.get(0),
                )?;
                if !exists {
                    return Err(rusqlite::Error::InvalidParameterName(format!(
                        "unknown operation reference: {}",
                        step.operation_ref
                    )));
                }
            } else {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "invalid operation reference: {}",
                    step.operation_ref
                )));
            }
        }
        Ok(())
    }

    fn insert_pipeline_steps(
        conn: &Connection,
        pipeline_id: &str,
        steps: &[PipelineStepInput],
    ) -> Result<()> {
        for (position, step) in steps.iter().enumerate() {
            conn.execute(
                "INSERT INTO pipeline_steps
                    (pipeline_id, position, operation_ref, config_json, failure_policy)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    pipeline_id,
                    position as i64,
                    step.operation_ref,
                    step.config_json,
                    step.failure_policy
                ],
            )?;
        }
        Ok(())
    }

    pub fn create_pipeline(
        &self,
        name: &str,
        steps: &[PipelineStepInput],
        shortcut: Option<&str>,
    ) -> Result<Pipeline> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        Self::validate_pipeline_steps(&tx, steps)?;
        tx.execute(
            "INSERT INTO pipelines (name, shortcut) VALUES (?1, ?2)",
            params![name, shortcut],
        )?;
        let row_id = tx.last_insert_rowid();
        let (stable_id, created_at, updated_at): (String, String, String) = tx.query_row(
            "SELECT id, created_at, updated_at FROM pipelines WHERE row_id = ?1",
            params![row_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        Self::insert_pipeline_steps(&tx, &stable_id, steps)?;
        let pipeline = Self::pipeline_by_id(&tx, &stable_id)?;
        tx.commit()?;
        debug_assert_eq!(pipeline.id, row_id);
        debug_assert_eq!(pipeline.created_at, created_at);
        debug_assert_eq!(pipeline.updated_at, updated_at);
        Ok(pipeline)
    }

    pub fn update_pipeline(
        &self,
        pipeline_ref: &str,
        name: &str,
        steps: &[PipelineStepInput],
        shortcut: Option<&str>,
    ) -> Result<Pipeline> {
        let pipeline_id = pipeline_ref
            .strip_prefix("pipeline:")
            .unwrap_or(pipeline_ref);
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        Self::validate_pipeline_steps(&tx, steps)?;
        let changed = tx.execute(
            "UPDATE pipelines
             SET name = ?1, shortcut = ?2, revision = revision + 1,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?3",
            params![name, shortcut, pipeline_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        tx.execute(
            "DELETE FROM pipeline_steps WHERE pipeline_id = ?1",
            params![pipeline_id],
        )?;
        Self::insert_pipeline_steps(&tx, pipeline_id, steps)?;
        let pipeline = Self::pipeline_by_id(&tx, pipeline_id)?;
        tx.commit()?;
        Ok(pipeline)
    }

    pub fn update_pipeline_shortcut(
        &self,
        pipeline_ref: &str,
        shortcut: Option<&str>,
    ) -> Result<()> {
        let pipeline_id = pipeline_ref
            .strip_prefix("pipeline:")
            .unwrap_or(pipeline_ref);
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE pipelines
             SET shortcut = ?1, revision = revision + 1, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?2",
            params![shortcut, pipeline_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    pub fn delete_pipeline(&self, pipeline_ref: &str) -> Result<()> {
        let pipeline_id = pipeline_ref
            .strip_prefix("pipeline:")
            .unwrap_or(pipeline_ref);
        let conn = self.conn.lock();
        let changed = conn.execute("DELETE FROM pipelines WHERE id = ?1", params![pipeline_id])?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    pub fn get_intelligence_connections(&self) -> Result<Vec<IntelligenceConnection>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, name, provider_kind, endpoint, model, credential_ref,
                    enabled, priority, created_at, updated_at
             FROM intelligence_connections
             ORDER BY priority ASC, row_id ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(IntelligenceConnection {
                id: row.get(0)?,
                name: row.get(1)?,
                provider_kind: row.get(2)?,
                endpoint: row.get(3)?,
                model: row.get(4)?,
                credential_ref: row.get(5)?,
                enabled: row.get::<_, i64>(6)? != 0,
                priority: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;
        rows.collect()
    }

    pub fn create_intelligence_connection(
        &self,
        name: &str,
        provider_kind: &str,
        endpoint: Option<&str>,
        model: Option<&str>,
        credential_ref: Option<&str>,
    ) -> Result<IntelligenceConnection> {
        let conn = self.conn.lock();
        let priority: i64 = conn.query_row(
            "SELECT COALESCE(MAX(priority), -1) + 1 FROM intelligence_connections",
            [],
            |row| row.get(0),
        )?;
        conn.execute(
            "INSERT INTO intelligence_connections
                (name, provider_kind, endpoint, model, credential_ref, priority)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                name.trim(),
                provider_kind,
                endpoint,
                model,
                credential_ref,
                priority
            ],
        )?;
        let row_id = conn.last_insert_rowid();
        conn.query_row(
            "SELECT id, name, provider_kind, endpoint, model, credential_ref,
                    enabled, priority, created_at, updated_at
             FROM intelligence_connections WHERE row_id = ?1",
            params![row_id],
            |row| {
                Ok(IntelligenceConnection {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    provider_kind: row.get(2)?,
                    endpoint: row.get(3)?,
                    model: row.get(4)?,
                    credential_ref: row.get(5)?,
                    enabled: row.get::<_, i64>(6)? != 0,
                    priority: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )
    }

    pub fn ensure_intelligence_connection_candidate(
        &self,
        name: &str,
        provider_kind: &str,
        endpoint: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock();
        let exists = conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM intelligence_connections
                WHERE provider_kind = ?1
                  AND COALESCE(endpoint, '') = COALESCE(?2, '')
            )",
            params![provider_kind, endpoint],
            |row| row.get::<_, bool>(0),
        )?;
        if exists {
            return Ok(());
        }
        let priority: i64 = conn.query_row(
            "SELECT COALESCE(MAX(priority), -1) + 1 FROM intelligence_connections",
            [],
            |row| row.get(0),
        )?;
        conn.execute(
            "INSERT INTO intelligence_connections
                (name, provider_kind, endpoint, enabled, priority)
             VALUES (?1, ?2, ?3, 0, ?4)",
            params![name.trim(), provider_kind, endpoint, priority],
        )?;
        Ok(())
    }

    pub fn update_intelligence_connection(
        &self,
        request: IntelligenceConnectionUpdate<'_>,
    ) -> Result<()> {
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE intelligence_connections
             SET name = ?1, provider_kind = ?2, endpoint = ?3, model = ?4,
                 credential_ref = ?5, enabled = ?6, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?7",
            params![
                request.name.trim(),
                request.provider_kind,
                request.endpoint,
                request.model,
                request.credential_ref,
                request.enabled as i64,
                request.id
            ],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    pub fn delete_intelligence_connection(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock();
        let changed = conn.execute(
            "DELETE FROM intelligence_connections WHERE id = ?1",
            params![id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    pub fn reorder_intelligence_connections(&self, ids: &[String]) -> Result<()> {
        let mut conn = self.conn.lock();
        let transaction = conn.transaction()?;
        for (priority, id) in ids.iter().enumerate() {
            let changed = transaction.execute(
                "UPDATE intelligence_connections SET priority = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![priority as i64, id],
            )?;
            if changed == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
        }
        transaction.commit()
    }

    pub fn get_operations(&self) -> Result<Vec<Operation>> {
        let conn = self.conn.lock();
        let mut operations = crate::operation_registry::BUILTIN_OPERATIONS
            .iter()
            .enumerate()
            .map(|(index, definition)| Operation {
                id: -((index as i64) + 1),
                stable_id: format!("builtin:{}", definition.key),
                name: definition.name.to_string(),
                op_type: definition.key.to_string(),
                config: None,
                category: definition.category_label.to_string(),
                created_at: String::new(),
            })
            .collect::<Vec<_>>();
        let mut stmt = conn.prepare(
            "SELECT row_id, id, name, executor_kind, config_json, category, created_at
             FROM custom_operations ORDER BY row_id ASC",
        )?;
        let op_iter = stmt.query_map([], |row| {
            let operation_id = row.get::<_, String>(1)?;
            let executor_kind = row.get::<_, String>(3)?;
            let config_json = row.get::<_, String>(4)?;
            let (op_type, config) = Self::legacy_operation_fields(&executor_kind, &config_json);
            Ok(Operation {
                id: row.get(0)?,
                stable_id: format!("custom:{operation_id}"),
                name: row.get(2)?,
                op_type,
                config,
                category: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        for o in op_iter {
            operations.push(o?);
        }
        Ok(operations)
    }

    pub fn create_operation(
        &self,
        name: &str,
        op_type: &str,
        config: Option<&str>,
        category: Option<&str>,
    ) -> Result<Operation> {
        let conn = self.conn.lock();
        let cat = category.unwrap_or("Custom Operations");
        let (executor_kind, config_json) = Self::operation_storage_fields(op_type, config);
        conn.execute(
            "INSERT INTO custom_operations
                (name, executor_kind, config_json, category, trusted)
             VALUES (?1, ?2, ?3, ?4, 1)",
            params![name, executor_kind, config_json, cat],
        )?;
        let id = conn.last_insert_rowid();
        let stable_id: String = conn.query_row(
            "SELECT id FROM custom_operations WHERE row_id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        Ok(Operation {
            id,
            stable_id: format!("custom:{stable_id}"),
            name: name.to_string(),
            op_type: op_type.to_string(),
            config: config.map(str::to_string),
            category: cat.to_string(),
            created_at: conn.query_row(
                "SELECT created_at FROM custom_operations WHERE row_id = ?1",
                params![id],
                |row| row.get(0),
            )?,
        })
    }

    pub fn update_operation(
        &self,
        id: i64,
        name: &str,
        op_type: &str,
        config: Option<&str>,
        category: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock();
        let cat = category.unwrap_or("Custom Operations");
        let (executor_kind, config_json) = Self::operation_storage_fields(op_type, config);
        conn.execute(
            "UPDATE custom_operations
             SET name = ?1, executor_kind = ?2, config_json = ?3, category = ?4,
                 updated_at = CURRENT_TIMESTAMP
             WHERE row_id = ?5",
            params![name, executor_kind, config_json, cat, id],
        )?;
        Ok(())
    }

    pub fn delete_operation(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM custom_operations WHERE row_id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn purge_old_clips(&self, keep_count: i64) -> Result<()> {
        let conn = self.conn.lock();
        self.enforce_history_limit_with_count_internal(&conn, keep_count)
    }

    pub fn enforce_revision_retention(&self, keep_count: i64) -> Result<()> {
        let keep_count = keep_count.max(0);
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES ('revisionHistoryLimit', ?1)
             ON CONFLICT(key) DO UPDATE SET value = ?1",
            params![keep_count.to_string()],
        )?;
        if keep_count > 0 {
            tx.execute(
                "DELETE FROM clip_versions WHERE id IN (
                    SELECT id FROM (
                        SELECT id,
                               ROW_NUMBER() OVER (PARTITION BY clip_id ORDER BY id DESC) AS revision_rank
                        FROM clip_versions
                    ) WHERE revision_rank > ?1
                 )",
                params![keep_count],
            )?;
        }
        tx.commit()
    }

    pub fn save_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn delete_setting(&self, key: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
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
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_file = temp_dir.join(format!(
            "pasted_test_{}_{:?}.db",
            nanos,
            std::thread::current().id()
        ));
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
    fn clip_lists_defer_image_payloads_to_the_image_endpoint() {
        let db = setup_test_db();
        let image_payload = "data:image/png;base64,cGFzdGVk";
        let clip = db
            .save_clip(
                "image",
                None,
                None,
                Some(image_payload),
                "image_hash",
                "Screenshot",
            )
            .unwrap();

        let clips = db.get_clips(None, None, false).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].id, clip.id);
        assert!(clips[0].image_base64.is_none());
        assert_eq!(
            db.get_clip_image(clip.id).unwrap().as_deref(),
            Some(image_payload)
        );
    }

    #[test]
    fn test_protected_clips_immunity() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "text",
                Some("Protected Secret"),
                None,
                None,
                "prot_hash",
                "Keeper",
            )
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
        let pinned = db
            .save_clip("text", Some("Pinned"), None, None, "ret-pin", "App")
            .unwrap();
        let protected = db
            .save_clip("text", Some("Protected"), None, None, "ret-prot", "App")
            .unwrap();
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
            )
            .unwrap();
        }

        db.purge_old_clips(1).unwrap();

        let active = db.get_clips(None, None, false).unwrap();
        assert_eq!(
            active
                .iter()
                .filter(|clip| !clip.is_pinned && !clip.is_protected)
                .count(),
            1
        );
        assert!(active.iter().any(|clip| clip.id == pinned.id));
        assert!(active.iter().any(|clip| clip.id == protected.id));
        assert_eq!(db.get_trashed_clips().unwrap().len(), 2);
    }

    #[test]
    fn test_retention_without_trash_keeps_requested_unpinned_capacity() {
        let db = setup_test_db();
        db.save_setting("enableTrash", "false").unwrap();
        let pinned = db
            .save_clip("text", Some("Pinned"), None, None, "purge-pin", "App")
            .unwrap();
        db.toggle_pin(pinned.id).unwrap();
        for index in 0..4 {
            db.save_clip(
                "text",
                Some(&format!("Regular {index}")),
                None,
                None,
                &format!("purge-{index}"),
                "App",
            )
            .unwrap();
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
            .save_clip(
                "text",
                Some("Pasted Pin Test"),
                None,
                None,
                "hash2",
                "VSCode",
            )
            .unwrap();

        // Pin clip
        let is_pinned = db.toggle_pin(clip.id).unwrap();
        assert!(is_pinned);

        // Add note
        db.update_clip_note(clip.id, Some("Important note"))
            .unwrap();

        let clips = db.get_clips(None, None, false).unwrap();
        assert!(clips[0].is_pinned);
        assert_eq!(clips[0].note.as_deref(), Some("Important note"));
    }

    #[test]
    fn test_bins_crud() {
        let db = setup_test_db();
        let initial_count = db.get_bins().unwrap().len();

        let bin = db.create_bin("Work", "💼", "#3b82f6", None).unwrap();
        assert!(bin.id > 0);

        let bins = db.get_bins().unwrap();
        assert_eq!(bins.len(), initial_count + 1);

        db.delete_bin(bin.id, "keep", None).unwrap();
        let bins_after = db.get_bins().unwrap();
        assert_eq!(bins_after.len(), initial_count);
    }

    #[test]
    fn deleting_a_bin_can_keep_move_or_trash_its_clips() {
        let db = setup_test_db();

        let keep_bin = db.create_bin("Keep", "📁", "default", None).unwrap();
        let kept = db
            .save_clip("text", Some("kept"), None, None, "keep_hash", "App")
            .unwrap();
        db.assign_to_bin(kept.id, Some(keep_bin.id)).unwrap();
        db.delete_bin(keep_bin.id, "keep", None).unwrap();
        assert_eq!(db.get_clip_by_id(kept.id).unwrap().bin_id, None);

        let source_bin = db.create_bin("Source", "📁", "default", None).unwrap();
        let destination_bin = db.create_bin("Destination", "📁", "default", None).unwrap();
        let moved = db
            .save_clip("text", Some("moved"), None, None, "move_hash", "App")
            .unwrap();
        db.assign_to_bin(moved.id, Some(source_bin.id)).unwrap();
        db.delete_bin(source_bin.id, "move", Some(destination_bin.id))
            .unwrap();
        assert_eq!(
            db.get_clip_by_id(moved.id).unwrap().bin_id,
            Some(destination_bin.id)
        );

        let trash_bin = db.create_bin("Trash", "📁", "default", None).unwrap();
        let trashed = db
            .save_clip("text", Some("trashed"), None, None, "trash_hash", "App")
            .unwrap();
        let protected = db
            .save_clip(
                "text",
                Some("protected"),
                None,
                None,
                "protected_hash",
                "App",
            )
            .unwrap();
        db.assign_to_bin(trashed.id, Some(trash_bin.id)).unwrap();
        db.assign_to_bin(protected.id, Some(trash_bin.id)).unwrap();
        db.toggle_protected(protected.id).unwrap();
        db.delete_bin(trash_bin.id, "trash", None).unwrap();

        assert!(db
            .get_trashed_clips()
            .unwrap()
            .iter()
            .any(|clip| clip.id == trashed.id));
        let protected_after = db.get_clip_by_id(protected.id).unwrap();
        assert!(protected_after.is_protected);
        assert!(!protected_after.is_trashed);
        assert_eq!(protected_after.bin_id, None);
    }

    #[test]
    fn deleting_a_bin_rejects_invalid_move_destinations_atomically() {
        let db = setup_test_db();
        let source_bin = db.create_bin("Source", "📁", "default", None).unwrap();
        let clip = db
            .save_clip("text", Some("clip"), None, None, "clip_hash", "App")
            .unwrap();
        db.assign_to_bin(clip.id, Some(source_bin.id)).unwrap();

        assert!(db.delete_bin(source_bin.id, "move", None).is_err());
        assert!(db
            .get_bins()
            .unwrap()
            .iter()
            .any(|bin| bin.id == source_bin.id));
        assert_eq!(
            db.get_clip_by_id(clip.id).unwrap().bin_id,
            Some(source_bin.id)
        );
    }

    #[test]
    fn test_legacy_container_schema_migrates_to_bins() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_path = std::env::temp_dir().join(format!("pasted_legacy_schema_{nanos}.db"));
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE clips (
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
                );
                CREATE TABLE boards (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    icon TEXT DEFAULT 'Folder',
                    color TEXT DEFAULT '#3b82f6',
                    smart_rule TEXT,
                    board_type TEXT DEFAULT 'category',
                    shortcut TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE clip_boards (
                    clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                    board_id INTEGER NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
                    PRIMARY KEY (clip_id, board_id)
                );
                INSERT INTO boards (id, name) VALUES (41, 'Migrated Bin');
                INSERT INTO clips (
                    id, content_type, text_content, content_hash, source_app, board_id
                ) VALUES (73, 'text', 'Legacy assignment', 'legacy-hash', 'Test', 41);
                INSERT INTO clip_boards (clip_id, board_id) VALUES (73, 41);",
            )
            .unwrap();
        }

        let db = DbState::new(db_path).unwrap();
        let bins = db.get_bins().unwrap();
        let clips = db.get_clips(None, None, false).unwrap();
        assert_eq!(bins.len(), 1);
        assert_eq!(bins[0].name, "Migrated Bin");
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].bin_id, Some(41));
        assert_eq!(clips[0].bin_ids.as_deref(), Some(&[41][..]));

        let conn = db.conn.lock();
        assert!(!table_exists(&conn, "boards").unwrap());
        assert!(!table_exists(&conn, "clip_boards").unwrap());
        assert!(column_exists(&conn, "clips", "bin_id").unwrap());
        assert!(column_exists(&conn, "bins", "bin_type").unwrap());
        assert!(column_exists(&conn, "clip_bins", "bin_id").unwrap());
    }

    #[test]
    fn partial_pre_release_transform_migration_merges_saved_data() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_path = std::env::temp_dir().join(format!("pasted_transform_terms_{nanos}.db"));
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(
                r#"CREATE TABLE bins (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    icon TEXT DEFAULT 'Folder',
                    color TEXT DEFAULT '#3b82f6',
                    smart_rule TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    default_recipe_id TEXT,
                    default_transform_id TEXT
                );
                CREATE TABLE transformation_recipes (
                    row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    id TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    plan_json TEXT NOT NULL,
                    connection_id TEXT,
                    revision INTEGER NOT NULL DEFAULT 1,
                    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE intelligence_connections (
                    row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    id TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    provider_kind TEXT NOT NULL,
                    endpoint TEXT,
                    model TEXT,
                    credential_ref TEXT,
                    enabled INTEGER NOT NULL DEFAULT 1,
                    priority INTEGER NOT NULL DEFAULT 0,
                    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE clip_transformations (
                    id TEXT PRIMARY KEY,
                    clip_id INTEGER NOT NULL,
                    transform_id TEXT REFERENCES transformation_recipes(id) ON DELETE SET NULL,
                    transform_name TEXT NOT NULL,
                    transform_revision INTEGER NOT NULL,
                    connection_id TEXT,
                    duration_ms INTEGER NOT NULL DEFAULT 0,
                    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE saved_transforms (
                    row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    id TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    plan_json TEXT NOT NULL,
                    connection_id TEXT,
                    revision INTEGER NOT NULL DEFAULT 1,
                    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE transformation_executions (
                    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                    target_kind TEXT NOT NULL CHECK (target_kind IN ('operation', 'pipeline')),
                    target_ref TEXT NOT NULL,
                    target_revision INTEGER,
                    source_clip_id INTEGER,
                    trigger_kind TEXT NOT NULL,
                    started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    duration_ms INTEGER,
                    status TEXT NOT NULL DEFAULT 'running',
                    error_summary TEXT,
                    input_hash TEXT NOT NULL,
                    output_hash TEXT
                );
                INSERT INTO transformation_recipes (id, name, plan_json)
                VALUES ('legacy-transform', 'Legacy Markdown',
                    '{"schema_version":1,"intent":"Markdown","summary":"Markdown","planning_mode":"pinned","steps":[]}');
                INSERT INTO bins (name, default_recipe_id)
                VALUES ('Legacy Bin', 'legacy-transform');"#,
            )
            .unwrap();
        }

        let db = DbState::new(db_path).unwrap();
        let transforms = db.get_saved_transforms().unwrap();
        assert_eq!(transforms.len(), 1);
        assert_eq!(transforms[0].stable_ref, "transform:legacy-transform");
        let legacy_bin_id = db
            .get_bins()
            .unwrap()
            .into_iter()
            .find(|bin| bin.name == "Legacy Bin")
            .unwrap()
            .id;
        assert_eq!(
            db.get_bin_transform_ref(legacy_bin_id).unwrap().as_deref(),
            Some("transform:legacy-transform")
        );

        let execution_id = db
            .begin_transformation_execution(TransformationExecutionStart {
                target_kind: "transform",
                target_ref: "transform:legacy-transform",
                target_revision: Some(1),
                source_clip_id: None,
                trigger_kind: "manual",
                destination_kind: "preview",
                input_hash: "input-hash",
            })
            .unwrap();
        db.finish_transformation_execution(&execution_id, 4, Some("output-hash"), None)
            .unwrap();
        let conn = db.conn.lock();
        assert!(!table_exists(&conn, "transformation_recipes").unwrap());
        assert!(column_exists(&conn, "bins", "default_transform_id").unwrap());
        assert!(!column_exists(&conn, "bins", "default_recipe_id").unwrap());
        assert!(column_exists(&conn, "clip_transformations", "transform_id").unwrap());
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
            .save_clip(
                "text",
                Some("Unique Search Secret"),
                None,
                None,
                "h1",
                "Terminal",
            )
            .unwrap();
        let _clip2 = db
            .save_clip("text", Some("Unrelated text"), None, None, "h2", "Finder")
            .unwrap();

        // Search by query
        let search_results = db.get_clips(Some("Secret"), None, false).unwrap();
        assert_eq!(search_results.len(), 1);
        assert_eq!(
            search_results[0].text_content.as_deref(),
            Some("Unique Search Secret")
        );

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
    fn untrusted_clip_and_metadata_text_cannot_become_sql() {
        let db = setup_test_db();
        let hostile = "'); DROP TABLE clips; DELETE FROM bins; -- \" * OR 1=1";
        let hostile_transform = "AI output: '; UPDATE clips SET is_protected = 0; --";
        let hostile_rule = serde_json::json!({
            "type": "contains",
            "value": hostile,
        })
        .to_string();

        let clip = db
            .save_clip("text", Some(hostile), None, None, "hostile-hash", hostile)
            .unwrap();
        db.update_clip_text(clip.id, hostile_transform).unwrap();
        db.update_clip_note(clip.id, Some(hostile)).unwrap();
        let bin = db
            .create_bin(hostile, hostile, hostile, Some(&hostile_rule))
            .unwrap();

        // Search input is also untrusted. It may use FTS syntax internally, but it must
        // remain a bound value and must never alter the surrounding SQL statement.
        let _ = db.get_clips(Some(hostile), None, false).unwrap();

        let conn = db.conn.lock();
        let clip_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM clips", [], |row| row.get(0))
            .unwrap();
        let stored: (String, String, String) = conn
            .query_row(
                "SELECT text_content, source_app, note FROM clips WHERE id = ?1",
                params![clip.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        let stored_bin_name: String = conn
            .query_row(
                "SELECT name FROM bins WHERE id = ?1",
                params![bin.id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(clip_count, 1);
        assert_eq!(
            stored,
            (hostile_transform.into(), hostile.into(), hostile.into())
        );
        assert_eq!(stored_bin_name, hostile);
    }

    #[test]
    fn test_trash_and_activity_logging() {
        let db = setup_test_db();
        let clip = db
            .save_clip("text", Some("Trash Me"), None, None, "thash1", "Notes")
            .unwrap();

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
    fn test_trashed_clips_are_read_only_and_leave_category_bins() {
        let db = setup_test_db();
        let category = db
            .create_bin("Projects", "Folder", "#3b82f6", None)
            .unwrap();
        let tag = db
            .create_bin_with_type("Keep", "Tag", "#f59e0b", None, "tag")
            .unwrap();
        let clip = db
            .save_clip(
                "text",
                Some("Original searchable text"),
                None,
                None,
                "trash-policy-hash",
                "Tests",
            )
            .unwrap();

        db.update_clip_note(clip.id, Some("Original searchable note"))
            .unwrap();
        db.assign_to_bin(clip.id, Some(category.id)).unwrap();
        db.add_clip_to_bin(clip.id, tag.id).unwrap();
        db.delete_clip(clip.id).unwrap();

        let trashed = db.get_trashed_clips().unwrap();
        assert_eq!(trashed.len(), 1);
        assert_eq!(trashed[0].bin_id, None);
        assert_eq!(trashed[0].note.as_deref(), Some("Original searchable note"));
        let category_after_trash = db
            .get_bins()
            .unwrap()
            .into_iter()
            .find(|bin| bin.id == category.id)
            .unwrap();
        assert_eq!(category_after_trash.clip_count, Some(0));
        {
            let conn = db.conn.lock();
            let category_links: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM clip_bins WHERE clip_id = ?1 AND bin_id = ?2",
                    params![clip.id, category.id],
                    |row| row.get(0),
                )
                .unwrap();
            let tag_links: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM clip_bins WHERE clip_id = ?1 AND bin_id = ?2",
                    params![clip.id, tag.id],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(category_links, 0);
            assert_eq!(tag_links, 1);
        }

        db.assign_to_bin(clip.id, Some(category.id)).unwrap();
        db.update_clip_note(clip.id, Some("Should be ignored"))
            .unwrap();
        db.update_clip_text(clip.id, "Should also be ignored")
            .unwrap();
        let unchanged = db.get_trashed_clips().unwrap();
        assert_eq!(unchanged[0].bin_id, None);
        assert_eq!(
            unchanged[0].note.as_deref(),
            Some("Original searchable note")
        );
        assert_eq!(
            unchanged[0].text_content.as_deref(),
            Some("Original searchable text")
        );

        db.restore_clip(clip.id).unwrap();
        let restored = db.get_clips(None, None, false).unwrap();
        assert_eq!(restored[0].bin_id, None);
        assert!(restored[0].bin_ids.as_ref().unwrap().contains(&tag.id));
        db.assign_to_bin(clip.id, Some(category.id)).unwrap();
        db.update_clip_note(clip.id, Some("Editable after restore"))
            .unwrap();
        let edited = db.get_clips(None, Some(category.id), false).unwrap();
        assert_eq!(edited[0].note.as_deref(), Some("Editable after restore"));
    }

    #[test]
    fn test_pipelines_and_operations_crud() {
        let db = setup_test_db();

        // Built-ins are registry-owned and the old seeded snapshot tables are gone.
        assert!(db.get_pipelines().unwrap().is_empty());
        {
            let conn = db.conn.lock();
            assert!(!table_exists(&conn, "operations").unwrap());
            assert!(table_exists(&conn, "custom_operations").unwrap());
            assert!(table_exists(&conn, "pipelines").unwrap());
            assert!(table_exists(&conn, "pipeline_steps").unwrap());
            let persisted_builtins: i64 = conn
                .query_row("SELECT COUNT(*) FROM custom_operations", [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(persisted_builtins, 0);
        }

        // Pipeline CRUD
        let pipeline = db
            .create_pipeline(
                "Trim",
                &[PipelineStepInput {
                    operation_ref: "builtin:trim".to_string(),
                    config_json: None,
                    failure_policy: "stop".to_string(),
                }],
                Some("Alt+T"),
            )
            .unwrap();
        assert!(pipeline.id > 0);

        let pipelines = db.get_pipelines().unwrap();
        assert_eq!(pipelines[0].name, "Trim");
        assert_eq!(pipelines[0].steps[0].operation_ref, "builtin:trim");

        db.delete_pipeline(&pipeline.stable_ref).unwrap();
        assert!(db.get_pipelines().unwrap().is_empty());

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
    fn intelligence_connections_store_references_but_not_credentials() {
        let db = setup_test_db();
        let connection = db
            .create_intelligence_connection(
                "Local Ollama",
                "ollama",
                Some("http://127.0.0.1:11434"),
                Some("qwen3"),
                None,
            )
            .unwrap();
        assert_eq!(connection.provider_kind, "ollama");
        assert_eq!(connection.credential_ref, None);

        db.update_intelligence_connection(IntelligenceConnectionUpdate {
            id: &connection.id,
            name: "Local Planner",
            provider_kind: "openai_compatible",
            endpoint: Some("http://127.0.0.1:1234/v1"),
            model: Some("local-model"),
            credential_ref: Some("env:PASTED_AI_API_KEY"),
            enabled: false,
        })
        .unwrap();
        let connections = db.get_intelligence_connections().unwrap();
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].name, "Local Planner");
        assert!(!connections[0].enabled);
        assert_eq!(
            connections[0].credential_ref.as_deref(),
            Some("env:PASTED_AI_API_KEY")
        );

        let fallback = db
            .create_intelligence_connection(
                "Fallback Ollama",
                "ollama",
                Some("http://127.0.0.1:11434"),
                None,
                None,
            )
            .unwrap();
        db.reorder_intelligence_connections(&[fallback.id.clone(), connection.id.clone()])
            .unwrap();
        let reordered = db.get_intelligence_connections().unwrap();
        assert_eq!(reordered[0].id, fallback.id);
        assert_eq!(reordered[0].priority, 0);
        assert_eq!(reordered[1].id, connection.id);
        assert_eq!(reordered[1].priority, 1);

        db.delete_intelligence_connection(&connection.id).unwrap();
        db.delete_intelligence_connection(&fallback.id).unwrap();
        assert!(db.get_intelligence_connections().unwrap().is_empty());
    }

    #[test]
    fn detected_intelligence_candidates_are_disabled_and_idempotent() {
        let db = setup_test_db();
        db.ensure_intelligence_connection_candidate(
            "Codex CLI",
            "cli",
            Some("/usr/local/bin/codex"),
        )
        .unwrap();
        db.ensure_intelligence_connection_candidate(
            "Codex CLI",
            "cli",
            Some("/usr/local/bin/codex"),
        )
        .unwrap();

        let connections = db.get_intelligence_connections().unwrap();
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].name, "Codex CLI");
        assert!(!connections[0].enabled);
        assert_eq!(connections[0].priority, 0);
    }

    #[test]
    fn test_pipeline_roundtrip_update_and_validation_rollback() {
        let db = setup_test_db();
        let created = db
            .create_pipeline(
                "Normalize",
                &[
                    PipelineStepInput {
                        operation_ref: "builtin:trim".to_string(),
                        config_json: None,
                        failure_policy: "stop".to_string(),
                    },
                    PipelineStepInput {
                        operation_ref: "builtin:wrap_tags".to_string(),
                        config_json: Some(r#""strong""#.to_string()),
                        failure_policy: "stop".to_string(),
                    },
                ],
                Some("Alt+N"),
            )
            .unwrap();
        assert_eq!(created.revision, 1);
        assert_eq!(created.steps.len(), 2);
        assert_eq!(created.steps[0].position, 0);
        assert_eq!(created.steps[0].operation_ref, "builtin:trim");
        assert_eq!(created.steps[1].position, 1);
        assert_eq!(created.steps[1].config_json.as_deref(), Some(r#""strong""#));

        let updated = db
            .update_pipeline(
                &created.stable_ref,
                "Loud Quote",
                &[
                    PipelineStepInput {
                        operation_ref: "builtin:uppercase".to_string(),
                        config_json: None,
                        failure_policy: "stop".to_string(),
                    },
                    PipelineStepInput {
                        operation_ref: "builtin:quote_text".to_string(),
                        config_json: None,
                        failure_policy: "skip".to_string(),
                    },
                ],
                Some("Alt+L"),
            )
            .unwrap();
        assert_eq!(updated.stable_ref, created.stable_ref);
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.revision, 2);
        assert_eq!(updated.name, "Loud Quote");
        assert_eq!(updated.shortcut.as_deref(), Some("Alt+L"));
        assert_eq!(
            updated
                .steps
                .iter()
                .map(|step| (step.position, step.operation_ref.as_str()))
                .collect::<Vec<_>>(),
            vec![(0, "builtin:uppercase"), (1, "builtin:quote_text")]
        );

        let invalid = db.update_pipeline(
            &created.stable_ref,
            "Must Roll Back",
            &[PipelineStepInput {
                operation_ref: "builtin:not-real".to_string(),
                config_json: None,
                failure_policy: "stop".to_string(),
            }],
            None,
        );
        assert!(invalid.is_err());
        let after_failure = db
            .get_pipelines()
            .unwrap()
            .into_iter()
            .find(|pipeline| pipeline.stable_ref == created.stable_ref)
            .unwrap();
        assert_eq!(after_failure.name, "Loud Quote");
        assert_eq!(after_failure.revision, 2);
        assert_eq!(after_failure.steps, updated.steps);
    }

    #[test]
    fn test_pipeline_update_and_delete_report_not_found() {
        let db = setup_test_db();
        let steps = [PipelineStepInput {
            operation_ref: "builtin:trim".to_string(),
            config_json: None,
            failure_policy: "stop".to_string(),
        }];
        assert!(db
            .update_pipeline("pipeline:missing", "Missing", &steps, None)
            .is_err());
        assert!(db.delete_pipeline("pipeline:missing").is_err());
        assert!(db
            .update_pipeline_shortcut("pipeline:missing", Some("Alt+M"))
            .is_err());
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
        assert!(index_names.contains(&"idx_clips_bin_created".to_string()));
        assert!(index_names.contains(&"idx_clips_hash".to_string()));
        assert!(index_names.contains(&"idx_clips_active_timeline".to_string()));
    }

    #[test]
    fn test_fts5_search_indexing() {
        let db = setup_test_db();

        let clip1 = db
            .save_clip(
                "text",
                Some("Supercalifragilisticexpialidocious secret token"),
                None,
                None,
                "HashFTS1",
                "IntelliJ",
            )
            .unwrap();
        let _clip2 = db
            .save_clip(
                "text",
                Some("Unrelated standard content text"),
                None,
                None,
                "HashFTS2",
                "Safari",
            )
            .unwrap();

        let search_res = db
            .get_clips(Some("Supercalifragilisticexpialidocious"), None, false)
            .unwrap();
        assert_eq!(search_res.len(), 1);
        assert_eq!(search_res[0].id, clip1.id);

        db.delete_clip(clip1.id).unwrap();
        let search_after_delete = db
            .get_clips(Some("Supercalifragilisticexpialidocious"), None, false)
            .unwrap();
        assert_eq!(search_after_delete.len(), 0);
    }

    #[test]
    fn test_startup_rebuilds_fts_before_clip_updates() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "text",
                Some("Recoverable noted clip"),
                None,
                None,
                "HashFTSRecovery",
                "Notes",
            )
            .unwrap();
        db.update_clip_note(clip.id, Some("Keep this note"))
            .unwrap();

        {
            let conn = db.conn.lock();
            conn.execute("INSERT INTO clips_fts(clips_fts) VALUES('delete-all')", [])
                .unwrap();
        }

        db.init_tables().unwrap();
        let search_results = db.get_clips(Some("Recoverable"), None, false).unwrap();
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0].id, clip.id);

        assert!(db.toggle_pin(clip.id).unwrap());
        db.update_clip_note(clip.id, Some("Updated note")).unwrap();
        db.delete_clip(clip.id).unwrap();
        assert!(db.get_clips(None, None, false).unwrap().is_empty());
    }

    #[test]
    fn test_unified_taxonomy_and_tags() {
        let db = setup_test_db();
        let tag = db
            .create_bin_with_type("CodeSnippet", "Tag", "#06b6d4", None, "tag")
            .unwrap();
        assert_eq!(tag.bin_type, "tag");

        let bins = db.get_bins().unwrap();
        assert!(bins.iter().any(|b| b.id == tag.id && b.bin_type == "tag"));
    }

    #[test]
    fn test_pin_reordering() {
        let db = setup_test_db();
        let clip1 = db
            .save_clip("text", Some("First Pinned"), None, None, "HashP1", "App")
            .unwrap();
        let clip2 = db
            .save_clip("text", Some("Second Pinned"), None, None, "HashP2", "App")
            .unwrap();
        db.toggle_pin(clip1.id).unwrap();
        db.toggle_pin(clip2.id).unwrap();

        let newly_pinned_first = db.get_clips(None, None, true).unwrap();
        assert_eq!(newly_pinned_first[0].id, clip2.id);
        assert_eq!(newly_pinned_first[1].id, clip1.id);

        db.reorder_pinned_clips(vec![clip1.id, clip2.id]).unwrap();
        let clips = db.get_clips(None, None, true).unwrap();
        assert_eq!(clips[0].id, clip1.id);
        assert_eq!(clips[1].id, clip2.id);
    }

    #[test]
    fn test_clip_version_history() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "text",
                Some("Original Content"),
                None,
                None,
                "HashV1",
                "App",
            )
            .unwrap();

        db.update_clip_text(clip.id, "Transformed Uppercase Content")
            .unwrap();
        db.update_clip_text(clip.id, "Transformed Uppercase Content")
            .unwrap();
        db.update_clip_text(clip.id, "Final Content").unwrap();

        let versions = db.get_clip_versions(clip.id).unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 2);
        assert_eq!(versions[0].text_content, "Transformed Uppercase Content");
        assert_eq!(versions[1].text_content, "Original Content");

        let updated = db.get_clips(None, None, false).unwrap();
        assert_eq!(updated[0].text_content.as_deref(), Some("Final Content"));

        for index in 0..55 {
            db.update_clip_text(clip.id, &format!("Revision {index}"))
                .unwrap();
        }
        assert_eq!(db.get_clip_versions(clip.id).unwrap().len(), 50);
        assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 50);

        db.purge_clip_permanently(clip.id).unwrap();
        assert!(db.get_clip_versions(clip.id).unwrap().is_empty());
        assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 0);
    }

    #[test]
    fn revision_retention_is_configurable_and_can_be_unlimited() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "text",
                Some("Original"),
                None,
                None,
                "revision-policy",
                "App",
            )
            .unwrap();

        db.enforce_revision_retention(10).unwrap();
        for index in 0..18 {
            db.update_clip_text(clip.id, &format!("Limited {index}"))
                .unwrap();
        }
        assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 10);

        db.enforce_revision_retention(0).unwrap();
        for index in 0..60 {
            db.update_clip_text(clip.id, &format!("Unlimited {index}"))
                .unwrap();
        }
        assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 70);

        db.enforce_revision_retention(25).unwrap();
        assert_eq!(db.get_clip_version_count(clip.id).unwrap(), 25);
        let newest = db.get_clip_versions_page(clip.id, 10, 0).unwrap();
        let middle = db.get_clip_versions_page(clip.id, 10, 10).unwrap();
        let oldest = db.get_clip_versions_page(clip.id, 10, 20).unwrap();
        assert_eq!((newest.len(), middle.len(), oldest.len()), (10, 10, 5));
        assert_ne!(newest[0].id, middle[0].id);
    }

    #[test]
    fn test_batch_operations() {
        let db = setup_test_db();
        let clip1 = db
            .save_clip("text", Some("Batch 1"), None, None, "HashB1", "App")
            .unwrap();
        let clip2 = db
            .save_clip("text", Some("Batch 2"), None, None, "HashB2", "App")
            .unwrap();

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
        let clip1 = db
            .save_clip("text", Some("Exclusive 1"), None, None, "HashE1", "App")
            .unwrap();
        let clip2 = db
            .save_clip("text", Some("Exclusive 2"), None, None, "HashE2", "App")
            .unwrap();
        let first_bin = db
            .create_bin("First Bin", "Folder", "#3b82f6", None)
            .unwrap();
        let second_bin = db
            .create_bin("Second Bin", "Folder", "#10b981", None)
            .unwrap();
        let tag = db
            .create_bin_with_type("Important", "Tag", "#f59e0b", None, "tag")
            .unwrap();

        assert!(db.toggle_pin(clip1.id).unwrap());
        assert!(db.toggle_protected(clip1.id).unwrap());
        db.assign_to_bin(clip1.id, Some(first_bin.id)).unwrap();
        db.add_clip_to_bin(clip1.id, tag.id).unwrap();
        db.assign_to_bin(clip1.id, Some(second_bin.id)).unwrap();

        assert!(db
            .get_clips(None, Some(first_bin.id), false)
            .unwrap()
            .is_empty());
        let second_bin_clips = db.get_clips(None, Some(second_bin.id), false).unwrap();
        assert_eq!(second_bin_clips.len(), 1);
        assert_eq!(second_bin_clips[0].id, clip1.id);
        assert!(second_bin_clips[0].is_pinned);
        assert!(second_bin_clips[0].is_protected);
        assert!(second_bin_clips[0]
            .bin_ids
            .as_ref()
            .unwrap()
            .contains(&tag.id));

        db.assign_to_bin(clip1.id, None).unwrap();
        let unassigned = db.get_clips(None, None, false).unwrap();
        let clip1_after_unassign = unassigned.iter().find(|clip| clip.id == clip1.id).unwrap();
        assert_eq!(clip1_after_unassign.bin_id, None);
        assert!(clip1_after_unassign.is_pinned);
        assert!(clip1_after_unassign.is_protected);
        assert_eq!(
            clip1_after_unassign.bin_ids.as_ref().unwrap(),
            &vec![tag.id]
        );

        db.batch_assign_bin_clips(vec![clip1.id, clip2.id], Some(first_bin.id))
            .unwrap();
        db.batch_assign_bin_clips(vec![clip1.id, clip2.id], Some(second_bin.id))
            .unwrap();
        assert!(db
            .get_clips(None, Some(first_bin.id), false)
            .unwrap()
            .is_empty());
        let batch_assigned = db.get_clips(None, Some(second_bin.id), false).unwrap();
        assert_eq!(batch_assigned.len(), 2);
        let protected_pinned = batch_assigned
            .iter()
            .find(|clip| clip.id == clip1.id)
            .unwrap();
        assert!(protected_pinned.is_pinned);
        assert!(protected_pinned.is_protected);
    }

    #[test]
    fn test_backup_export_import() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "text",
                Some("Backup Test Item"),
                Some("<strong>Backup Test Item</strong>"),
                None,
                "HashBK1",
                "VSCode",
            )
            .unwrap();
        let trashed = db
            .save_clip("text", Some("In Trash"), None, None, "HashBK2", "Notes")
            .unwrap();
        let bin = db.create_bin("DevBin", "Code", "#3b82f6", None).unwrap();
        let tag = db
            .create_bin_with_type("BackupTag", "Tag", "#f59e0b", None, "tag")
            .unwrap();
        db.assign_to_bin(clip.id, Some(bin.id)).unwrap();
        db.add_clip_to_bin(clip.id, tag.id).unwrap();
        db.update_clip_note(clip.id, Some("Restore this note"))
            .unwrap();
        db.toggle_pin(clip.id).unwrap();
        db.toggle_protected(clip.id).unwrap();
        db.delete_clip(trashed.id).unwrap();
        let backup_pipeline = db
            .create_pipeline(
                "Backup Pipeline",
                &[
                    PipelineStepInput {
                        operation_ref: "builtin:trim".to_string(),
                        config_json: None,
                        failure_policy: "stop".to_string(),
                    },
                    PipelineStepInput {
                        operation_ref: "builtin:uppercase".to_string(),
                        config_json: None,
                        failure_policy: "stop".to_string(),
                    },
                ],
                Some("Alt+B"),
            )
            .unwrap();
        db.create_operation(
            "Backup Operation",
            "uppercase",
            Some("{}"),
            Some("Backup Tools"),
        )
        .unwrap();
        let transform_plan = crate::transformation_intent::TransformationPlan {
            schema_version: crate::transformation_intent::TRANSFORMATION_PLAN_SCHEMA_VERSION,
            intent: "Uppercase".to_string(),
            summary: "Uppercase".to_string(),
            planning_mode: crate::transformation_intent::IntentPlanningMode::Pinned,
            steps: vec![crate::transformation_intent::PlannedTransformationStep {
                name: "Uppercase".to_string(),
                rationale: "Replayable".to_string(),
                scope: crate::transformation_intent::StepExecutionScope::WholeInput,
                executor: crate::transformation_intent::PlannedExecutor::Deterministic {
                    operation_ref: "builtin:uppercase".to_string(),
                    config_json: None,
                },
            }],
        };
        let saved_transform = db
            .create_saved_transform("Backup Transform", &transform_plan, None)
            .unwrap();
        db.set_bin_transform_ref(bin.id, Some(&saved_transform.stable_ref))
            .unwrap();

        let json = db.export_backup_json().unwrap();
        assert!(json.contains("Backup Test Item"));
        assert!(json.contains("DevBin"));

        let db2 = setup_test_db();
        let imported_count = db2.import_backup_json(&json).unwrap();
        assert_eq!(imported_count, 2);

        let restored = db2.get_all_clips_for_backup().unwrap();
        let restored_clip = restored
            .iter()
            .find(|item| item.content_hash == "HashBK1")
            .unwrap();
        assert_eq!(
            restored_clip.text_content.as_deref(),
            Some("Backup Test Item")
        );
        assert_eq!(
            restored_clip.html_content.as_deref(),
            Some("<strong>Backup Test Item</strong>")
        );
        assert_eq!(restored_clip.note.as_deref(), Some("Restore this note"));
        assert!(restored_clip.is_pinned);
        assert!(restored_clip.is_protected);
        assert!(!restored_clip.is_trashed);

        let restored_trashed = restored
            .iter()
            .find(|item| item.content_hash == "HashBK2")
            .unwrap();
        assert!(restored_trashed.is_trashed);
        assert!(restored_trashed.trashed_at.is_some());

        let restored_bins = db2.get_bins().unwrap();
        let restored_bin = restored_bins
            .iter()
            .find(|item| item.name == "DevBin")
            .unwrap();
        let restored_tag = restored_bins
            .iter()
            .find(|item| item.name == "BackupTag")
            .unwrap();
        let restored_bin_ids = restored_clip.bin_ids.as_ref().unwrap();
        assert!(restored_bin_ids.contains(&restored_bin.id));
        assert!(restored_bin_ids.contains(&restored_tag.id));
        let restored_pipeline = db2
            .get_pipelines()
            .unwrap()
            .into_iter()
            .find(|item| item.name == "Backup Pipeline")
            .unwrap();
        assert_eq!(restored_pipeline.stable_ref, backup_pipeline.stable_ref);
        assert_eq!(restored_pipeline.shortcut.as_deref(), Some("Alt+B"));
        assert_eq!(restored_pipeline.steps.len(), 2);
        assert_eq!(
            restored_pipeline.steps[1].operation_ref,
            "builtin:uppercase"
        );
        assert_eq!(
            db2.get_saved_transforms().unwrap()[0].stable_ref,
            saved_transform.stable_ref
        );
        assert_eq!(
            db2.get_bin_transform_ref(restored_bin.id)
                .unwrap()
                .as_deref(),
            Some(saved_transform.stable_ref.as_str())
        );
        assert!(db2
            .get_operations()
            .unwrap()
            .iter()
            .any(|item| item.name == "Backup Operation" && item.category == "Backup Tools"));
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
            )
            .unwrap();
        }

        let json = db.export_backup_json().unwrap();
        let payload: BackupPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(payload.version, BACKUP_SCHEMA_VERSION);
        assert_eq!(payload.clips.len(), 501);
        assert_eq!(db.get_clips(None, None, false).unwrap().len(), 501);
    }

    #[test]
    fn backup_import_rejects_unknown_schema_without_mutating_data() {
        let source = setup_test_db();
        source
            .save_clip(
                "text",
                Some("future data"),
                None,
                None,
                "future-backup-item",
                "Test",
            )
            .unwrap();
        let mut payload: serde_json::Value =
            serde_json::from_str(&source.export_backup_json().unwrap()).unwrap();
        payload["version"] = serde_json::json!(BACKUP_SCHEMA_VERSION + 1);

        let destination = setup_test_db();
        let error = destination
            .import_backup_json(&serde_json::to_string(&payload).unwrap())
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("unsupported backup schema version"));
        assert!(destination.get_clips(None, None, false).unwrap().is_empty());
    }

    #[test]
    fn test_saved_transform_roundtrip_and_delete() {
        let db = setup_test_db();
        let connection = db
            .create_intelligence_connection(
                "Codex CLI",
                "cli",
                Some("/usr/local/bin/codex"),
                None,
                None,
            )
            .unwrap();
        let plan = crate::transformation_intent::TransformationPlan {
            schema_version: crate::transformation_intent::TRANSFORMATION_PLAN_SCHEMA_VERSION,
            intent: "Convert this text to Markdown".to_string(),
            summary: "Convert text to Markdown".to_string(),
            planning_mode: crate::transformation_intent::IntentPlanningMode::Pinned,
            steps: vec![crate::transformation_intent::PlannedTransformationStep {
                name: "Convert to Markdown".to_string(),
                rationale: "Structure requires interpretation".to_string(),
                scope: crate::transformation_intent::StepExecutionScope::WholeInput,
                executor: crate::transformation_intent::PlannedExecutor::Semantic {
                    instructions: "Return clean Markdown".to_string(),
                    output_schema: None,
                    model_policy: crate::transformation_intent::ModelPolicy::Balanced,
                },
            }],
        };
        let transform = db
            .create_saved_transform("Markdown", &plan, Some(connection.id.as_str()))
            .unwrap();
        assert!(transform.stable_ref.starts_with("transform:"));
        assert_eq!(
            transform.connection_id.as_deref(),
            Some(connection.id.as_str())
        );
        assert_eq!(transform.plan, plan);
        assert_eq!(db.get_saved_transforms().unwrap().len(), 1);
        assert_eq!(
            db.resolve_saved_transform(&transform.stable_ref)
                .unwrap()
                .unwrap()
                .name,
            "Markdown"
        );
        let mut updated_plan = plan.clone();
        updated_plan.summary = "Convert text to concise Markdown".to_string();
        let updated = db
            .update_saved_transform(
                &transform.stable_ref,
                "Concise Markdown",
                &updated_plan,
                Some(connection.id.as_str()),
            )
            .unwrap();
        assert_eq!(updated.stable_ref, transform.stable_ref);
        assert_eq!(updated.name, "Concise Markdown");
        assert_eq!(updated.revision, transform.revision + 1);
        assert_eq!(updated.plan, updated_plan);
        db.delete_saved_transform(&transform.stable_ref).unwrap();
        assert!(db.get_saved_transforms().unwrap().is_empty());
    }

    #[test]
    fn test_transform_preview_applies_atomically_with_revision_and_provenance() {
        let db = setup_test_db();
        let clip = db
            .save_clip("text", Some("hello"), None, None, "transform-clip", "Test")
            .unwrap();
        let plan = crate::transformation_intent::TransformationPlan {
            schema_version: crate::transformation_intent::TRANSFORMATION_PLAN_SCHEMA_VERSION,
            intent: "Uppercase".to_string(),
            summary: "Uppercase text".to_string(),
            planning_mode: crate::transformation_intent::IntentPlanningMode::Pinned,
            steps: vec![crate::transformation_intent::PlannedTransformationStep {
                name: "Uppercase".to_string(),
                rationale: "Replayable".to_string(),
                scope: crate::transformation_intent::StepExecutionScope::WholeInput,
                executor: crate::transformation_intent::PlannedExecutor::Deterministic {
                    operation_ref: "builtin:uppercase".to_string(),
                    config_json: None,
                },
            }],
        };
        let transform = db.create_saved_transform("Uppercase", &plan, None).unwrap();
        let provenance = db
            .apply_transform_output_to_clip(TransformClipApplication {
                clip_id: clip.id,
                transform_ref: &transform.stable_ref,
                expected_input: "hello",
                output: "HELLO",
                connection_id: None,
                duration_ms: 12,
                bin_move: None,
            })
            .unwrap();
        assert_eq!(provenance.transform_name, "Uppercase");
        assert_eq!(provenance.duration_ms, 12);
        assert_eq!(
            db.get_clip_versions(clip.id).unwrap()[0].text_content,
            "hello"
        );
        assert_eq!(
            db.get_clip_transformation_provenance(clip.id)
                .unwrap()
                .unwrap()
                .transform_ref,
            transform.stable_ref
        );
        let current = db
            .get_clips(None, None, false)
            .unwrap()
            .into_iter()
            .find(|item| item.id == clip.id)
            .unwrap();
        assert_eq!(current.text_content.as_deref(), Some("HELLO"));

        let stale = db.apply_transform_output_to_clip(TransformClipApplication {
            clip_id: clip.id,
            transform_ref: &transform.stable_ref,
            expected_input: "hello",
            output: "ANOTHER RESULT",
            connection_id: None,
            duration_ms: 5,
            bin_move: None,
        });
        assert!(stale
            .unwrap_err()
            .to_string()
            .contains("changed after this preview"));
        assert_eq!(db.get_clip_versions(clip.id).unwrap().len(), 1);
    }

    #[test]
    fn transform_bin_drop_revision_restores_content_and_previous_bin_only() {
        let db = setup_test_db();
        let source_bin = db.create_bin("Source", "📥", "#111111", None).unwrap();
        let destination_bin = db.create_bin("Markdown", "📝", "#222222", None).unwrap();
        let tag = db
            .create_bin_with_type("Important", "⭐", "#333333", None, "tag")
            .unwrap();
        let clip = db
            .save_clip("text", Some("hello"), None, None, "compound-undo", "Test")
            .unwrap();
        db.add_clip_to_bin(clip.id, tag.id).unwrap();
        db.assign_to_bin(clip.id, Some(source_bin.id)).unwrap();
        let plan = crate::transformation_intent::TransformationPlan {
            schema_version: crate::transformation_intent::TRANSFORMATION_PLAN_SCHEMA_VERSION,
            intent: "Uppercase".to_string(),
            summary: "Uppercase text".to_string(),
            planning_mode: crate::transformation_intent::IntentPlanningMode::Pinned,
            steps: vec![crate::transformation_intent::PlannedTransformationStep {
                name: "Uppercase".to_string(),
                rationale: "Replayable".to_string(),
                scope: crate::transformation_intent::StepExecutionScope::WholeInput,
                executor: crate::transformation_intent::PlannedExecutor::Deterministic {
                    operation_ref: "builtin:uppercase".to_string(),
                    config_json: None,
                },
            }],
        };
        let transform = db.create_saved_transform("Uppercase", &plan, None).unwrap();

        db.assign_to_bin(clip.id, Some(destination_bin.id)).unwrap();
        db.apply_transform_output_to_clip(TransformClipApplication {
            clip_id: clip.id,
            transform_ref: &transform.stable_ref,
            expected_input: "hello",
            output: "HELLO",
            connection_id: None,
            duration_ms: 3,
            bin_move: Some((Some(source_bin.id), destination_bin.id)),
        })
        .unwrap();
        let version = db.get_clip_versions(clip.id).unwrap().remove(0);
        assert_eq!(
            version.action_label.as_deref(),
            Some("Moved to Markdown · Applied Uppercase")
        );
        assert!(version.restores_organization);

        let restored = db.restore_clip_version(clip.id, version.id).unwrap();
        assert_eq!(restored.text_content.as_deref(), Some("hello"));
        assert_eq!(restored.bin_id, Some(source_bin.id));
        assert!(!restored.is_transformed);
        assert!(db
            .get_clip_transformation_provenance(clip.id)
            .unwrap()
            .is_none());
        assert!(restored.bin_ids.unwrap_or_default().contains(&tag.id));

        let inverse = db.get_clip_versions(clip.id).unwrap().remove(0);
        assert_eq!(inverse.text_content, "HELLO");
        assert!(inverse.restores_organization);
        let redone = db.restore_clip_version(clip.id, inverse.id).unwrap();
        assert_eq!(redone.text_content.as_deref(), Some("HELLO"));
        assert_eq!(redone.bin_id, Some(destination_bin.id));
        assert!(redone.is_transformed);
        assert_eq!(
            db.get_clip_transformation_provenance(clip.id)
                .unwrap()
                .unwrap()
                .transform_name,
            "Uppercase"
        );
    }
}
