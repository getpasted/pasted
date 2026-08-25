use pasted_lib::db::DbState;
use rusqlite::Result;

use super::super::{argument_value, json_error, parse_i64_argument};

pub(super) fn run(subcommand: &str, args: &[String], db: &DbState, json: bool) -> Result<()> {
    match subcommand {
        "revisions" | "versions" => list(args, db, json),
        "restore-revision" | "restore-version" => restore(args, db, json),
        "delete-revision" | "delete-version" => delete(args, db, json),
        _ => unreachable!("version command dispatch is exhaustive"),
    }
}

fn list(args: &[String], db: &DbState, json: bool) -> Result<()> {
    let clip_id = parse_i64_argument(
        args,
        3,
        "Usage: pasted clip versions <clip-id> [--limit N] [--offset N] [--json]",
    );
    let limit = argument_value(args, "--limit")
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(50)
        .clamp(1, 1_000);
    let offset = argument_value(args, "--offset")
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0)
        .max(0);
    let versions = db.get_clip_version_timeline_page(clip_id, limit, offset)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&versions).map_err(json_error)?
        );
    } else if versions.is_empty() {
        println!("No versions for clip #{clip_id}.");
    } else {
        for version in versions {
            println!(
                "{}\t{}\t{}",
                version.id,
                version.created_at,
                version.action_label.as_deref().unwrap_or("Version")
            );
        }
    }
    Ok(())
}

fn restore(args: &[String], db: &DbState, json: bool) -> Result<()> {
    let usage = "Usage: pasted clip restore-version <clip-id> <version-id> [--json]";
    let clip_id = parse_i64_argument(args, 3, usage);
    let version_id = parse_i64_argument(args, 4, usage);
    let clip = db.restore_clip_version(clip_id, version_id)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&clip).map_err(json_error)?
        );
    } else {
        println!("Restored version #{version_id} for clip #{clip_id}.");
    }
    Ok(())
}

fn delete(args: &[String], db: &DbState, json: bool) -> Result<()> {
    if !args.iter().any(|argument| argument == "--yes") {
        eprintln!("Deleting a version is permanent. Re-run with --yes.");
        std::process::exit(2);
    }
    let usage = "Usage: pasted clip delete-version <clip-id> <version-id> --yes [--json]";
    let clip_id = parse_i64_argument(args, 3, usage);
    let version_id = parse_i64_argument(args, 4, usage);
    db.delete_clip_version(clip_id, version_id)?;
    if json {
        println!(
            "{}",
            serde_json::json!({
                "clipId": clip_id,
                "versionId": version_id,
                "deleted": true,
            })
        );
    } else {
        println!("Deleted version #{version_id} from clip #{clip_id}.");
    }
    Ok(())
}
