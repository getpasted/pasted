use rusqlite::{params, Connection, Result};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

fn get_db_path() -> PathBuf {
    if let Some(mut dir) = dirs::data_dir() {
        dir.push("com.tauri.dev");
        if dir.exists() {
            return dir.join("pasted.db");
        }
    }
    if let Some(mut dir) = dirs::data_dir() {
        dir.push("tauri-app");
        if dir.exists() {
            return dir.join("pasted.db");
        }
    }
    let local_dir = PathBuf::from("./pasted_data");
    let _ = fs::create_dir_all(&local_dir);
    local_dir.join("pasted.db")
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    let db_path = get_db_path();
    let conn = match Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error opening Pasted database at '{:?}': {}", db_path, e);
            std::process::exit(1);
        }
    };

    // Ensure database table schema exists
    let _ = conn.execute(
        "CREATE TABLE IF NOT EXISTS clips (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content_type TEXT NOT NULL,
            text_content TEXT,
            html_content TEXT,
            image_base64 TEXT,
            content_hash TEXT,
            source_app TEXT DEFAULT 'System Clipboard',
            is_pinned INTEGER DEFAULT 0,
            bin_id INTEGER,
            note TEXT,
            is_trashed INTEGER DEFAULT 0,
            trashed_at TEXT,
            created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
        )",
        [],
    );

    match command {
        "copy" | "add" => {
            let text = if let Some(arg_text) = args.get(2) {
                arg_text.clone()
            } else {
                let mut buffer = String::new();
                let _ = io::stdin().read_to_string(&mut buffer);
                buffer
            };

            let trimmed = text.trim().to_string();
            if trimmed.is_empty() {
                eprintln!("Error: Cannot copy empty content.");
                std::process::exit(1);
            }

            conn.execute(
                "INSERT INTO clips (content_type, text_content, source_app, created_at) VALUES ('text', ?1, 'CLI Terminal', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
                params![trimmed],
            )?;

            println!("✓ Successfully copied clip to Pasted history!");
        }
        "list" | "ls" => {
            let limit: i64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);
            let mut stmt = conn.prepare(
                "SELECT id, content_type, text_content, source_app, created_at FROM clips WHERE is_trashed = 0 ORDER BY created_at DESC LIMIT ?1"
            )?;
            let rows = stmt.query_map(params![limit], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?;

            println!(
                "{:<5} | {:<8} | {:<15} | {:<20} | CONTENT",
                "ID", "TYPE", "SOURCE", "DATE"
            );
            println!(
                "{:-<5}-+-{:-<8}-+-{:-<15}-+-{:-<20}-+-{:-<30}",
                "", "", "", "", ""
            );

            for r in rows {
                let (id, c_type, content, source, date) = r?;
                let snippet: String = content
                    .lines()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(40)
                    .collect();
                println!(
                    "{:<5} | {:<8} | {:<15} | {:<20} | {}",
                    id, c_type, source, date, snippet
                );
            }
        }
        "search" | "find" => {
            let query = args.get(2).cloned().unwrap_or_default();
            let pattern = format!("%{}%", query);
            let mut stmt = conn.prepare(
                "SELECT id, content_type, text_content, source_app, created_at FROM clips WHERE is_trashed = 0 AND text_content LIKE ?1 ORDER BY created_at DESC LIMIT 20"
            )?;
            let rows = stmt.query_map(params![pattern], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?;

            for r in rows {
                let (id, c_type, content, source, date) = r?;
                println!("[#{id}] ({c_type} from {source} @ {date}):\n{content}\n---");
            }
        }
        "clear" => {
            conn.execute("DELETE FROM clips WHERE is_pinned = 0", [])?;
            println!("✓ Cleared unpinned clipboard history via CLI.");
        }
        _ => {
            println!("Pasted CLI Tool (v1.0.0)");
            println!("Usage:");
            println!("  pasted-cli copy <text>       Save text or pipe stdin (cat file.txt | pasted-cli copy)");
            println!("  pasted-cli list [limit]      List N recent clipboard items (default: 10)");
            println!("  pasted-cli search <query>    Search clips for keyword query");
            println!("  pasted-cli clear             Clear unpinned clipboard history");
        }
    }

    Ok(())
}
