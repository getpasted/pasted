use super::super::*;
use super::*;

pub(crate) fn run_copy(args: Vec<String>, db_path: PathBuf, conn: Connection) -> Result<()> {
    let capture_limit = configured_capture_bytes(&conn);
    let text = if let Some(arg_text) = args.get(2) {
        arg_text.clone()
    } else {
        read_stdin_bounded(capture_limit)?
    };

    let trimmed = text.trim().to_string();
    if trimmed.is_empty() {
        eprintln!("Error: Cannot copy empty content.");
        std::process::exit(1);
    }
    if trimmed.len() > capture_limit {
        eprintln!(
            "Error: Content exceeds the configured {} MB clip limit.",
            capture_limit / 1024 / 1024
        );
        std::process::exit(1);
    }

    drop(conn);
    let db = DbState::new(db_path.clone())?;
    let clip = db.save_text_clip(&trimmed, "CLI Terminal")?;
    if setting_value_is_enabled(db.get_setting(Feature::Bins.setting_key())?.as_deref())
        && setting_value_is_enabled(
            db.get_setting(Feature::Transformations.setting_key())?
                .as_deref(),
        )
    {
        pasted_lib::intelligence_executor::apply_smart_bin_transforms_for_clip(
            &db,
            clip.id,
            &clip.content_type,
            &clip.content_types,
            &trimmed,
            "CLI Terminal",
        );
    }
    let clip = db.get_clip_by_id(clip.id)?;

    if args.iter().any(|argument| argument == "--json") {
        println!(
            "{}",
            serde_json::json!({
                "id": clip.id,
                "contentType": clip.content_type,
                "contentTypes": clip.content_types,
            })
        );
    } else {
        println!("Saved {} clip #{} to History.", clip.content_type, clip.id);
    }
    Ok(())
}

pub(crate) fn run_list(args: Vec<String>, db_path: PathBuf, conn: Connection) -> Result<()> {
    let limit = argument_value(&args, "--limit")
        .as_ref()
        .or_else(|| args.get(2).filter(|value| !value.starts_with("--")))
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(10)
        .clamp(1, 10_000);
    let offset = argument_value(&args, "--offset")
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0)
        .max(0);
    let bin_id = argument_value(&args, "--bin").and_then(|value| value.parse::<i64>().ok());
    let pinned = args.iter().any(|argument| argument == "--pinned");
    let named = args.iter().any(|argument| argument == "--named");
    let trash = args.iter().any(|argument| argument == "--trash");
    if [bin_id.is_some(), pinned, named, trash]
        .into_iter()
        .filter(|selected| *selected)
        .count()
        > 1
    {
        eprintln!("--bin, --pinned, --named, and --trash cannot be combined.");
        std::process::exit(2);
    }
    drop(conn);
    let db = DbState::new(db_path.clone())?;
    let clips = if named {
        require_feature(&db, Feature::Naming);
        db.search_clips(&pasted_lib::db::ClipSearchRequest {
            query: "is:named".to_string(),
            limit: usize::try_from(limit).unwrap_or(10),
            offset: usize::try_from(offset).unwrap_or(0),
            ..Default::default()
        })?
        .items
    } else if trash {
        db.get_trashed_clips_page(Some(limit), Some(offset))?
    } else {
        db.get_clips_page(bin_id, pinned, Some(limit), Some(offset))?
    };
    if args.iter().any(|argument| argument == "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&clips).map_err(json_error)?
        );
        return Ok(());
    }
    println!(
        "{:<5} | {:<8} | {:<15} | {:<20} | {:<20} | CONTENT",
        "ID", "TYPE", "SOURCE", "DATE", "NAME"
    );
    println!(
        "{:-<5}-+-{:-<8}-+-{:-<15}-+-{:-<20}-+-{:-<20}-+-{:-<30}",
        "", "", "", "", "", ""
    );
    for clip in clips {
        let snippet: String = clip
            .text_content
            .as_deref()
            .unwrap_or("")
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(40)
            .collect();
        println!(
            "{:<5} | {:<8} | {:<15} | {:<20} | {:<20} | {}",
            clip.id,
            clip.content_type,
            clip.source,
            clip.created_at,
            clip.name.as_deref().unwrap_or(""),
            snippet
        );
    }
    Ok(())
}

pub(crate) fn run_search(args: Vec<String>, db_path: PathBuf, _conn: Connection) -> Result<()> {
    let db = DbState::new(db_path.clone())?;
    require_feature(&db, Feature::Search);
    let option_value = |name: &str| {
        args.iter()
            .position(|argument| argument == name)
            .and_then(|index| args.get(index + 1))
            .cloned()
    };
    let clip_type = option_value("--clip");
    let content_type = option_value("--content");
    let file_format = option_value("--format");
    let source = option_value("--source");
    let clip_ids = option_value("--ids")
        .map(|value| {
            value
                .split(',')
                .map(|id| id.parse::<i64>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| {
                    rusqlite::Error::InvalidParameterName(
                        "--ids must be a comma-separated list of clip IDs".into(),
                    )
                })
        })
        .transpose()?
        .unwrap_or_default();
    let json = args.iter().any(|argument| argument == "--json");
    let trash = args.iter().any(|argument| argument == "--trash");
    let limit = match option_value("--limit") {
        Some(value) => match value.parse::<usize>() {
            Ok(value) if (1..=pasted_lib::db::MAX_CLIP_SEARCH_PAGE_SIZE).contains(&value) => value,
            _ => {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "--limit must be between 1 and {}",
                    pasted_lib::db::MAX_CLIP_SEARCH_PAGE_SIZE
                )))
            }
        },
        None => 20,
    };
    let offset = option_value("--offset")
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0)
        .max(0);
    let query = args
        .iter()
        .skip(2)
        .take_while(|argument| !argument.starts_with("--"))
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    let result = db.search_clips(&pasted_lib::db::ClipSearchRequest {
        query,
        clip_ids,
        clip_types: clip_type.into_iter().collect(),
        content_types: content_type.into_iter().collect(),
        file_formats: file_format.into_iter().collect(),
        sources: source.into_iter().collect(),
        trash,
        limit,
        offset: usize::try_from(offset).unwrap_or(0),
    })?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(json_error)?
        );
    } else {
        for clip in result.items {
            let detected = if clip.content_types.is_empty() {
                String::new()
            } else {
                format!("; {}", clip.content_types.join(", "))
            };
            let formats = if clip.file_formats.is_empty() {
                String::new()
            } else {
                format!("; {}", clip.file_formats.join(", "))
            };
            let content = clip.text_content.unwrap_or_default();
            println!(
                "[#{id}] ({clip_type}{detected}{formats} from {source} @ {date}):\n{content}\n---",
                id = clip.id,
                clip_type = clip.content_type,
                source = clip.source,
                date = clip.created_at,
            );
        }
    }
    Ok(())
}
