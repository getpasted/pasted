use super::{
    json_error, parse_retention_argument, retention_age_label, retention_count_label, setting_i64,
};
use pasted_lib::db::DbState;
use rusqlite::{Connection, Result};
use std::path::PathBuf;

pub(crate) fn run(args: &[String], db_path: PathBuf, conn: Connection) -> Result<()> {
    drop(conn);
    let db = DbState::new(db_path.clone())?;
    let current_count = db
        .get_setting("keepClipCount")?
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(1000);
    let current_age_days = db
        .get_setting("keepClipAgeDays")?
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0);
    let count =
        parse_retention_argument(args, "--count", "unlimited", 100_000).unwrap_or(current_count);
    let age_days =
        parse_retention_argument(args, "--days", "forever", 36_500).unwrap_or(current_age_days);
    let trash_count = parse_retention_argument(args, "--trash-count", "unlimited", 100_000)
        .unwrap_or(setting_i64(&db, "trashCapacityCount", 500)?);
    let trash_age_days = parse_retention_argument(args, "--trash-days", "forever", 36_500)
        .unwrap_or(setting_i64(&db, "trashAgeDays", 0)?);
    let activity_count = parse_retention_argument(args, "--log-count", "unlimited", 100_000)
        .unwrap_or(setting_i64(&db, "activityLogCapacity", 1000)?);
    let activity_age_days = parse_retention_argument(args, "--log-days", "forever", 36_500)
        .unwrap_or(setting_i64(&db, "activityLogAgeDays", 0)?);
    let revision_count = parse_retention_argument(args, "--revision-count", "unlimited", 10_000)
        .unwrap_or(setting_i64(&db, "revisionHistoryLimit", 10)?);
    let history_changed = args
        .iter()
        .any(|argument| argument == "--count" || argument == "--days");
    let trash_changed = args
        .iter()
        .any(|argument| argument == "--trash-count" || argument == "--trash-days");
    let activity_changed = args
        .iter()
        .any(|argument| argument == "--log-count" || argument == "--log-days");
    let revisions_changed = args.iter().any(|argument| argument == "--revision-count");
    if history_changed {
        db.configure_clip_retention(count, age_days)?;
    }
    if trash_changed {
        db.configure_trash_retention(trash_count, trash_age_days)?;
    }
    if activity_changed {
        db.configure_activity_retention(activity_count, activity_age_days)?;
    }
    if revisions_changed {
        db.enforce_revision_retention(revision_count)?;
    }
    if args.iter().any(|argument| argument == "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "maximumClips": count,
                "maximumAgeDays": age_days,
                "maximumClipsUnlimited": count == 0,
                "maximumAgeForever": age_days == 0,
                "trashMaximumClips": trash_count,
                "trashMaximumAgeDays": trash_age_days,
                "trashMaximumClipsUnlimited": trash_count == 0,
                "trashMaximumAgeForever": trash_age_days == 0,
                "activityMaximumEntries": activity_count,
                "activityMaximumAgeDays": activity_age_days,
                "activityMaximumEntriesUnlimited": activity_count == 0,
                "activityMaximumAgeForever": activity_age_days == 0,
                "revisionsPerClip": revision_count,
                "revisionsUnlimited": revision_count == 0,
            }))
            .map_err(json_error)?
        );
    } else {
        let count_label = if count == 0 {
            "Unlimited".to_string()
        } else {
            format!("{count} clips")
        };
        let age_label = if age_days == 0 {
            "Forever".to_string()
        } else {
            format!("{age_days} days")
        };
        println!(
            "History: {count_label}; {age_label}\nTrash: {}; {}\nActivity: {}; {}\nRevisions: {}",
            retention_count_label(trash_count, "clips"),
            retention_age_label(trash_age_days),
            retention_count_label(activity_count, "entries"),
            retention_age_label(activity_age_days),
            retention_count_label(revision_count, "per clip"),
        );
    }
    Ok(())
}
