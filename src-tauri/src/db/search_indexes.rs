use rusqlite::{Connection, OptionalExtension, Result};
use serde::Serialize;

use super::DbState;

pub const CAPTURED_CLIPS_INDEX_REF: &str = "index:captured-clips-v1";
pub const EXTRACTED_TEXT_INDEX_REF: &str = "index:extracted-text-v1";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchIndexEntry {
    pub stable_ref: String,
    pub canonical_count: usize,
    pub indexed_count: usize,
    pub healthy: bool,
    pub engine: String,
    pub included_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchIndexStatus {
    pub schema_version: u32,
    pub indexes: Vec<SearchIndexEntry>,
}

fn table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table],
        |_| Ok(()),
    )
    .optional()
    .ok()
    .flatten()
    .is_some()
}

fn index_uses_trigram(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT INSTR(LOWER(sql), 'trigram') > 0 FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table],
        |row| row.get(0),
    )
    .unwrap_or(false)
}

fn clips_index_is_current(conn: &Connection) -> bool {
    conn.prepare("PRAGMA table_info(clips_fts)")
        .and_then(|mut statement| {
            let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
            columns.collect::<Result<Vec<_>>>()
        })
        .is_ok_and(|columns| columns.iter().any(|column| column == "name"))
        && index_uses_trigram(conn, "clips_fts")
}

pub(super) fn ensure_search_indexes(conn: &Connection) {
    if table_exists(conn, "clips_fts") && !clips_index_is_current(conn) {
        let _ = conn.execute_batch(
            "DROP TRIGGER IF EXISTS clips_ai;
             DROP TRIGGER IF EXISTS clips_ad;
             DROP TRIGGER IF EXISTS clips_au;
             DROP TABLE IF EXISTS clips_fts;",
        );
    }

    if conn
        .execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS clips_fts USING fts5(
                text_content, note, name, source,
                content='clips', content_rowid='id', tokenize='trigram'
            )",
            [],
        )
        .is_ok()
    {
        let _ = conn.execute_batch(
            "CREATE TRIGGER IF NOT EXISTS clips_ai AFTER INSERT ON clips BEGIN
                INSERT INTO clips_fts(rowid, text_content, note, name, source)
                VALUES (new.id, new.text_content, new.note, new.name, new.source);
             END;
             CREATE TRIGGER IF NOT EXISTS clips_ad AFTER DELETE ON clips BEGIN
                INSERT INTO clips_fts(clips_fts, rowid, text_content, note, name, source)
                VALUES ('delete', old.id, old.text_content, old.note, old.name, old.source);
             END;
             CREATE TRIGGER IF NOT EXISTS clips_au AFTER UPDATE ON clips BEGIN
                INSERT INTO clips_fts(clips_fts, rowid, text_content, note, name, source)
                VALUES ('delete', old.id, old.text_content, old.note, old.name, old.source);
                INSERT INTO clips_fts(rowid, text_content, note, name, source)
                VALUES (new.id, new.text_content, new.note, new.name, new.source);
             END;",
        );
        let _ = rebuild(conn, CAPTURED_CLIPS_INDEX_REF);
    }

    if table_exists(conn, "clip_searchable_text_fts")
        && !index_uses_trigram(conn, "clip_searchable_text_fts")
    {
        let _ = conn.execute_batch(
            "DROP TRIGGER IF EXISTS clip_searchable_text_ai;
             DROP TRIGGER IF EXISTS clip_searchable_text_ad;
             DROP TRIGGER IF EXISTS clip_searchable_text_au;
             DROP TABLE IF EXISTS clip_searchable_text_fts;",
        );
    }

    if conn
        .execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS clip_searchable_text_fts USING fts5(
                searchable_text,
                content='clip_searchable_text', content_rowid='clip_id', tokenize='trigram'
            )",
            [],
        )
        .is_ok()
    {
        let _ = conn.execute_batch(
            "CREATE TRIGGER IF NOT EXISTS clip_searchable_text_ai
                AFTER INSERT ON clip_searchable_text BEGIN
                    INSERT INTO clip_searchable_text_fts(rowid, searchable_text)
                    VALUES (new.clip_id, new.searchable_text);
                END;
             CREATE TRIGGER IF NOT EXISTS clip_searchable_text_ad
                AFTER DELETE ON clip_searchable_text BEGIN
                    INSERT INTO clip_searchable_text_fts(
                        clip_searchable_text_fts, rowid, searchable_text
                    ) VALUES ('delete', old.clip_id, old.searchable_text);
                END;
             CREATE TRIGGER IF NOT EXISTS clip_searchable_text_au
                AFTER UPDATE ON clip_searchable_text BEGIN
                    INSERT INTO clip_searchable_text_fts(
                        clip_searchable_text_fts, rowid, searchable_text
                    ) VALUES ('delete', old.clip_id, old.searchable_text);
                    INSERT INTO clip_searchable_text_fts(rowid, searchable_text)
                    VALUES (new.clip_id, new.searchable_text);
                END;",
        );
        let _ = rebuild(conn, EXTRACTED_TEXT_INDEX_REF);
    }
}

fn count(conn: &Connection, table: &str) -> Result<usize> {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    let value = conn.query_row(&sql, [], |row| row.get::<_, i64>(0))?;
    usize::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}

fn entry(
    conn: &Connection,
    stable_ref: &str,
    canonical_table: &str,
    index_table: &str,
    included_fields: &[&str],
) -> Result<SearchIndexEntry> {
    let canonical_count = count(conn, canonical_table)?;
    let docsize_table = format!("{index_table}_docsize");
    let available = table_exists(conn, index_table) && table_exists(conn, &docsize_table);
    let indexed_count = if available {
        count(conn, &docsize_table)?
    } else {
        0
    };
    Ok(SearchIndexEntry {
        stable_ref: stable_ref.to_string(),
        canonical_count,
        indexed_count,
        healthy: available && canonical_count == indexed_count,
        engine: "SQLite FTS5".to_string(),
        included_fields: included_fields
            .iter()
            .map(|field| (*field).to_string())
            .collect(),
    })
}

fn status(conn: &Connection) -> Result<SearchIndexStatus> {
    Ok(SearchIndexStatus {
        schema_version: 1,
        indexes: vec![
            entry(
                conn,
                CAPTURED_CLIPS_INDEX_REF,
                "clips",
                "clips_fts",
                &["content", "name", "note", "source"],
            )?,
            entry(
                conn,
                EXTRACTED_TEXT_INDEX_REF,
                "clip_searchable_text",
                "clip_searchable_text_fts",
                &["extractedText"],
            )?,
        ],
    })
}

fn rebuild(conn: &Connection, stable_ref: &str) -> Result<()> {
    match stable_ref {
        CAPTURED_CLIPS_INDEX_REF => {
            conn.execute("INSERT INTO clips_fts(clips_fts) VALUES('rebuild')", [])?;
        }
        EXTRACTED_TEXT_INDEX_REF => {
            conn.execute(
                "INSERT INTO clip_searchable_text_fts(clip_searchable_text_fts) VALUES('rebuild')",
                [],
            )?;
        }
        "all" => {
            rebuild(conn, CAPTURED_CLIPS_INDEX_REF)?;
            rebuild(conn, EXTRACTED_TEXT_INDEX_REF)?;
        }
        _ => {
            return Err(rusqlite::Error::InvalidParameterName(
                "stableRef".to_string(),
            ))
        }
    }
    Ok(())
}

impl DbState {
    pub fn get_search_index_status(&self) -> Result<SearchIndexStatus> {
        status(&self.conn.lock())
    }

    pub fn rebuild_search_index(&self, stable_ref: &str) -> Result<SearchIndexStatus> {
        let conn = self.conn.lock();
        rebuild(&conn, stable_ref)?;
        status(&conn)
    }
}
