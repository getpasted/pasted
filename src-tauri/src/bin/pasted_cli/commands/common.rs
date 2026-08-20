use super::super::*;

pub(crate) fn json_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

pub(crate) fn require_feature(db: &DbState, feature: Feature) {
    let enabled = db.get_setting(feature.setting_key()).ok().flatten();
    if !setting_value_is_enabled(enabled.as_deref()) {
        eprintln!(
            "{} is disabled in Settings → Functionality.",
            feature.label()
        );
        std::process::exit(1);
    }
}

pub(crate) fn parse_clip_ids(args: &[String], start: usize) -> Vec<i64> {
    let ids = args
        .iter()
        .skip(start)
        .filter(|argument| !matches!(argument.as_str(), "--json" | "--yes"))
        .map(|value| value.parse::<i64>())
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|_| {
            eprintln!("Every clip ID must be an integer.");
            std::process::exit(2);
        });
    if ids.is_empty() {
        eprintln!("Provide at least one clip ID.");
        std::process::exit(2);
    }
    ids
}

pub(crate) fn print_mutation_summary(summary: &ClipMutationSummary, json: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(summary).map_err(json_error)?
        );
    } else {
        println!(
            "{}: {} changed, {} skipped.",
            summary.action, summary.changed_count, summary.skipped_count
        );
    }
    Ok(())
}

pub(crate) fn configured_capture_bytes(conn: &Connection) -> usize {
    let configured = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'maxClipSizeMb'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok();
    pasted_lib::resource_limits::configured_clip_capture_bytes(configured.as_deref())
}

pub(crate) fn argument_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|argument| argument == flag)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

pub(crate) fn parse_i64_argument(args: &[String], index: usize, usage: &str) -> i64 {
    args.get(index)
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or_else(|| {
            eprintln!("{usage}");
            std::process::exit(2);
        })
}

pub(crate) fn optional_argument_update(
    args: &[String],
    value_flag: &str,
    clear_flag: &str,
    current: Option<String>,
) -> Option<String> {
    if args.iter().any(|argument| argument == clear_flag) {
        None
    } else {
        argument_value(args, value_flag).or(current)
    }
}

pub(crate) fn validate_json_or_exit(value: Option<&str>, label: &str) {
    if let Some(value) = value {
        if let Err(error) = serde_json::from_str::<serde_json::Value>(value) {
            eprintln!("{label} must be valid JSON: {error}");
            std::process::exit(2);
        }
    }
}

pub(crate) fn validate_smart_bin_rule_or_exit(value: Option<&str>) {
    if let Some(value) = value {
        if let Err(error) = pasted_lib::smart_bins::parse_rule_json(value) {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}

pub(crate) fn read_file_bounded(path: &Path, maximum_bytes: usize) -> Result<Vec<u8>> {
    let metadata = fs::metadata(path).map_err(|_| rusqlite::Error::InvalidPath(path.into()))?;
    if !metadata.is_file() || metadata.len() > maximum_bytes as u64 {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "Input file must be a regular file no larger than {} MB",
            maximum_bytes / 1024 / 1024
        )));
    }
    let bytes = fs::read(path).map_err(|_| rusqlite::Error::InvalidPath(path.into()))?;
    if bytes.len() > maximum_bytes {
        return Err(rusqlite::Error::InvalidParameterName(
            "Input file exceeded the extraction safety limit".into(),
        ));
    }
    Ok(bytes)
}

pub(crate) fn argument_values(args: &[String], flag: &str) -> Vec<String> {
    args.iter()
        .enumerate()
        .filter_map(|(index, argument)| {
            (argument == flag)
                .then(|| args.get(index + 1).cloned())
                .flatten()
        })
        .collect()
}

pub(crate) fn print_bin(bin: &pasted_lib::db::Bin, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(bin).map_err(json_error)?);
    } else {
        println!(
            "#{}\t{}\t{}\t{} clips",
            bin.id,
            bin.icon,
            bin.name,
            bin.clip_count.unwrap_or(0)
        );
    }
    Ok(())
}

pub(crate) fn send_live_or_exit(action: pasted_lib::live_app::LiveAppAction) -> serde_json::Value {
    pasted_lib::live_app::send(action).unwrap_or_else(|error| {
        eprintln!("Live-app command failed: {error}");
        std::process::exit(1);
    })
}

pub(crate) fn print_live_result(result: &serde_json::Value, json: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(result).map_err(json_error)?
        );
    } else if let Some(paused) = result.get("paused").and_then(serde_json::Value::as_bool) {
        println!(
            "Clipboard recording is {}.",
            if paused { "paused" } else { "active" }
        );
    } else if let Some(total) = result
        .get("total_count")
        .and_then(serde_json::Value::as_u64)
    {
        println!(
            "Queue contains {total} item{}.",
            if total == 1 { "" } else { "s" }
        );
    } else if let Some(status) = result.get("status") {
        let total = status
            .get("total_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        println!(
            "Queue command completed; {total} item{} remain.",
            if total == 1 { "" } else { "s" }
        );
    } else {
        println!("Command completed.");
    }
    Ok(())
}

pub(crate) fn print_content_type(
    content_type: &pasted_lib::content_types::ContentTypeDefinition,
    json: bool,
) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(content_type).map_err(json_error)?
        );
    } else {
        println!(
            "Saved content type {}: {}",
            content_type.id, content_type.label
        );
    }
    Ok(())
}

pub(crate) fn read_stdin_bounded(maximum: usize) -> Result<String> {
    let mut buffer = String::new();
    io::stdin()
        .take((maximum + 1) as u64)
        .read_to_string(&mut buffer)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    if buffer.len() > maximum {
        return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "stdin exceeds Pasted's {} MB safety limit",
                    maximum / 1024 / 1024
                ),
            ),
        )));
    }
    Ok(buffer)
}

pub(crate) fn cli_input_error(error: String) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(io::Error::new(
        io::ErrorKind::InvalidInput,
        error,
    )))
}
