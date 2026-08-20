use super::super::*;
use super::*;

pub(crate) fn run_inspector(args: Vec<String>, db_path: PathBuf, conn: Connection) -> Result<()> {
    drop(conn);
    let db = DbState::new(db_path.clone())?;
    let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
    match subcommand {
        "list" | "ls" => {
            let inspectors = pasted_lib::content_inspection::inspector_definitions();
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&inspectors).map_err(json_error)?
                );
            } else {
                for inspector in inspectors {
                    println!(
                        "{}\t{}\t{} → {}\t{}{}",
                        inspector.stable_ref,
                        inspector.priority,
                        inspector.input_contract,
                        inspector.output_contract,
                        inspector.name,
                        if inspector.is_available {
                            ""
                        } else {
                            " (unavailable)"
                        }
                    );
                }
            }
        }
        "get" => {
            let reference = args.get(3).unwrap_or_else(|| {
                eprintln!("Usage: pasted inspector get <ref> [--json]");
                std::process::exit(2);
            });
            let inspector = pasted_lib::content_inspection::inspector_definitions()
                .into_iter()
                .find(|inspector| {
                    pasted_lib::content_inspection::canonical_inspector_ref(reference)
                        == inspector.stable_ref
                })
                .unwrap_or_else(|| {
                    eprintln!("Inspector {reference} was not found.");
                    std::process::exit(1);
                });
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&inspector).map_err(json_error)?
                );
            } else {
                println!("{}\t{}", inspector.stable_ref, inspector.name);
            }
        }
        "run" | "test" => {
            let clip_id =
                argument_value(&args, "--clip").and_then(|value| value.parse::<i64>().ok());
            let explicit_text = argument_value(&args, "--text");
            if clip_id.is_some() && explicit_text.is_some() {
                eprintln!("Provide only one of --text or --clip ID.");
                std::process::exit(2);
            }
            let apply = args.iter().any(|argument| argument == "--apply");
            if apply && clip_id.is_none() {
                eprintln!("--apply requires --clip ID.");
                std::process::exit(2);
            }
            let result = if let Some(clip_id) = clip_id {
                pasted_lib::inspection_execution::inspect_clip(&db, clip_id, apply)?
            } else {
                let text = explicit_text.unwrap_or_else(|| {
                    read_stdin_bounded(pasted_lib::resource_limits::MAX_CLIP_TEXT_BYTES)
                        .unwrap_or_else(|error| {
                            eprintln!("Could not read inspection input: {error}");
                            std::process::exit(2);
                        })
                });
                if text.is_empty() {
                    eprintln!("Provide input with --text, --clip, or stdin.");
                    std::process::exit(2);
                }
                let analysis =
                    pasted_lib::inspection_execution::inspect_text(&text, Some("Pasted CLI"))
                        .map_err(|failure| {
                            rusqlite::Error::InvalidParameterName(failure.message)
                        })?;
                pasted_lib::inspection_execution::ClipInspectionResult {
                    analysis,
                    application: pasted_lib::analysis_contract::ClipApplication::preview(),
                    live_file_observations: None,
                    file_formats: None,
                    media_metadata: None,
                }
            };
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).map_err(json_error)?
                );
            } else {
                let metadata = &result.analysis.result;
                println!("Origin: {}", metadata.origin.stable_name());
                println!("Bytes: {}", metadata.byte_count);
                if let Some(text) = metadata.text.as_ref() {
                    println!(
                        "Characters: {}; words: {}; lines: {}",
                        text.character_count, text.word_count, text.line_count
                    );
                }
                if let Some(image) = metadata.image.as_ref() {
                    println!("Dimensions: {} × {}", image.width, image.height);
                }
                if let Some(files) = metadata.files.as_ref() {
                    println!(
                        "Items: {}; extensions: {}",
                        files.item_count,
                        files.extensions.join(", ")
                    );
                }
                if let Some(file_formats) = result.file_formats.as_ref() {
                    println!(
                        "File formats: {}",
                        file_formats
                            .formats
                            .iter()
                            .map(|detected| detected.format.to_uppercase())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                if let Some(media) = result.media_metadata.as_ref() {
                    println!(
                        "Media: {} file(s); audio streams: {}; video streams: {}; duration: {} ms",
                        media.media_file_count,
                        media.audio_stream_count,
                        media.video_stream_count,
                        media.total_duration_ms
                    );
                    if !media.codecs.is_empty() {
                        println!("Codecs: {}", media.codecs.join(", "));
                    }
                }
            }
        }
        "rescan" => {
            require_feature(&db, Feature::FileFormats);
            if !args.iter().any(|argument| argument == "--yes") {
                eprintln!("File Format rescans refresh derived metadata and Smart Bin membership. Re-run with --yes to continue.");
                std::process::exit(2);
            }
            let report = db.rescan_file_formats()?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).map_err(json_error)?
                );
            } else {
                let mut details = Vec::new();
                if report.changed_count > 0 {
                    details.push(format!("{} updated", report.changed_count));
                }
                if report.unchanged_count > 0 {
                    details.push(format!("{} unchanged", report.unchanged_count));
                }
                if report.missing_count > 0 {
                    details.push(format!("{} missing", report.missing_count));
                }
                if report.failed_count > 0 {
                    details.push(format!("{} failed", report.failed_count));
                }
                if details.is_empty() {
                    println!("Rescanned {} file clips.", report.scanned_count);
                } else {
                    println!(
                        "Rescanned {} file clips: {}.",
                        report.scanned_count,
                        details.join(", ")
                    );
                }
            }
        }
        _ => {
            eprintln!("Usage: pasted inspector list|get|run|rescan [options] [--json]");
            std::process::exit(2);
        }
    }
    Ok(())
}
