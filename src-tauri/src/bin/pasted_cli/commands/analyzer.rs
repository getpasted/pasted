use super::super::*;
use super::*;

pub(crate) fn run_analyzer(args: Vec<String>, db_path: PathBuf, conn: Connection) -> Result<()> {
    drop(conn);
    let db = DbState::new(db_path.clone())?;
    let subcommand = args.get(2).map(String::as_str).unwrap_or("run");
    if !matches!(subcommand, "run" | "preview") {
        eprintln!("Usage: pasted analyzer run [--text TEXT | --clip ID | --stdin] [--policy POLICY] [--extract] [--json]");
        std::process::exit(2);
    }
    let clip_id = argument_value(&args, "--clip").map(|value| {
        value.parse::<i64>().unwrap_or_else(|_| {
            eprintln!("--clip requires a numeric clip ID.");
            std::process::exit(2);
        })
    });
    let explicit_text = argument_value(&args, "--text");
    if clip_id.is_some() && explicit_text.is_some() {
        eprintln!("Provide only one of --text or --clip ID.");
        std::process::exit(2);
    }
    let policy = argument_value(&args, "--policy")
        .unwrap_or_else(|| "interactive".into())
        .parse::<pasted_lib::analysis_contract::AnalysisPolicy>()
        .unwrap_or_else(|error| {
            eprintln!("{error}");
            std::process::exit(2);
        });
    let options = pasted_lib::analysis_execution::AnalyzerOptions {
        policy,
        include_extractor: args.iter().any(|argument| argument == "--extract"),
        include_classifiers: pasted_lib::features::is_enabled(&db, Feature::ContentClassification),
        include_suggestions: pasted_lib::features::is_enabled(&db, Feature::Transformations),
    };
    let result = if let Some(clip_id) = clip_id {
        pasted_lib::analysis_execution::analyze_clip(&db, clip_id, options)
    } else {
        let text = explicit_text.unwrap_or_else(|| {
            read_stdin_bounded(pasted_lib::resource_limits::MAX_CLIP_TEXT_BYTES).unwrap_or_else(
                |error| {
                    eprintln!("Could not read analysis input: {error}");
                    std::process::exit(2);
                },
            )
        });
        if text.is_empty() {
            eprintln!("Provide input with --text, --clip, or stdin.");
            std::process::exit(2);
        }
        pasted_lib::analysis_execution::analyze_text(&db, &text, Some("Pasted CLI"), options)
    }
    .map_err(rusqlite::Error::InvalidParameterName)?;
    if args.iter().any(|argument| argument == "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(json_error)?
        );
    } else {
        println!("Kind: {}", result.analysis.result.clip_kind);
        println!(
            "Content types: {}",
            if result.analysis.result.classification_matches.is_empty() {
                "—".to_string()
            } else {
                result
                    .analysis
                    .result
                    .classification_matches
                    .iter()
                    .map(|matched| matched.content_type.as_str())
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        );
        println!("Participants: {}", result.analysis.participants.len());
        if let Some(suggestions) = result.analysis.result.suggestions.as_ref() {
            println!("Smart Actions: {}", suggestions.actions.len());
        }
    }
    Ok(())
}
