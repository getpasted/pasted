use super::{argument_value, json_error};
use pasted_lib::db::DbState;
use pasted_lib::features::{self, Feature};
use rusqlite::{Connection, Result};
use std::path::PathBuf;

const DEFAULT_PAGE_SIZE: usize = 100;
const MAX_PAGE_SIZE: usize = 500;
const MAX_OFFSET: usize = 10_000_000;

pub(crate) fn run(args: &[String], db_path: PathBuf, conn: Connection) -> Result<()> {
    drop(conn);
    let db = DbState::new(db_path)?;
    require_search(&db);
    let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
    let json = args.iter().any(|argument| argument == "--json");

    match subcommand {
        "list" => run_list(args, &db, json),
        "delete" | "remove" => run_delete(args, &db, json),
        "clear" => run_clear(args, &db, json),
        _ => usage(),
    }
}

fn run_list(args: &[String], db: &DbState, json: bool) -> Result<()> {
    let limit = parse_bounded_usize(args, "--limit", DEFAULT_PAGE_SIZE, 1, MAX_PAGE_SIZE)?;
    let offset = parse_bounded_usize(args, "--offset", 0, 0, MAX_OFFSET)?;
    let page = db.list_search_history(limit, offset)?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&page).map_err(json_error)?
        );
    } else if page.items.is_empty() {
        println!("No Search history.");
    } else {
        println!(
            "{:<6} | {:<20} | {:<7} | {:<7} | QUERY",
            "ID", "LAST USED", "USES", "RESULTS"
        );
        println!(
            "{:-<6}-+-{:-<20}-+-{:-<7}-+-{:-<7}-+-{:-<30}",
            "", "", "", "", ""
        );
        for entry in page.items {
            println!(
                "{:<6} | {:<20} | {:<7} | {:<7} | {}",
                entry.id,
                entry.last_used_at,
                entry.use_count,
                entry.result_count,
                entry.request.query.replace(['\r', '\n'], " "),
            );
        }
    }
    Ok(())
}

fn run_delete(args: &[String], db: &DbState, json: bool) -> Result<()> {
    let id = args
        .get(3)
        .filter(|value| !value.starts_with("--"))
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|id| *id > 0)
        .ok_or_else(|| {
            rusqlite::Error::InvalidParameterName(
                "search-history delete requires a positive entry ID".into(),
            )
        })?;
    let deleted = db.delete_search_history(id)?;
    if json {
        println!("{}", serde_json::json!({ "id": id, "deleted": deleted }));
    } else if deleted {
        println!("Deleted Search history entry #{id}.");
    } else {
        println!("Search history entry #{id} was not found.");
    }
    Ok(())
}

fn run_clear(args: &[String], db: &DbState, json: bool) -> Result<()> {
    if !args.iter().any(|argument| argument == "--yes") {
        eprintln!("Clearing Search history is permanent. Re-run with --yes to continue.");
        std::process::exit(2);
    }
    let cleared_count = db.clear_search_history()?;
    if json {
        println!("{}", serde_json::json!({ "clearedCount": cleared_count }));
    } else {
        println!("Cleared {cleared_count} Search history entries.");
    }
    Ok(())
}

fn parse_bounded_usize(
    args: &[String],
    name: &str,
    default: usize,
    minimum: usize,
    maximum: usize,
) -> Result<usize> {
    let Some(value) = argument_value(args, name) else {
        return Ok(default);
    };
    value
        .parse::<usize>()
        .ok()
        .filter(|value| (*value >= minimum) && (*value <= maximum))
        .ok_or_else(|| {
            rusqlite::Error::InvalidParameterName(format!(
                "{name} must be between {minimum} and {maximum}"
            ))
        })
}

fn require_search(db: &DbState) {
    if let Err(error) = features::require(db, Feature::Search) {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

fn usage() -> ! {
    eprintln!(
        "Usage: pasted search-history list [--limit N] [--offset N] [--json] | delete <id> [--json] | clear --yes [--json]"
    );
    std::process::exit(2);
}
