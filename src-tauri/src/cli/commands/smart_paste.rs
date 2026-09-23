use super::super::*;
use super::*;

pub(crate) fn run(args: Vec<String>, db_path: PathBuf, _conn: Connection) -> Result<()> {
    if args.get(2).is_some_and(|value| value == "parts") {
        return list_parts(&args, db_path);
    }
    let label = argument_value(&args, "--context").unwrap_or_else(|| {
        eprintln!("Usage: pasted smart-paste --context TEXT [--application APP] [--role ROLE] [--description TEXT] [--placeholder TEXT] [--text TEXT | --stdin] [--json]");
        std::process::exit(2);
    });
    let source = match argument_value(&args, "--text") {
        Some(value) => value,
        None => read_stdin_bounded(pasted_lib::resource_limits::MAX_TRANSFORM_TEXT_BYTES)?,
    };
    let db = DbState::new(db_path)?;
    let context = pasted_lib::smart_paste::SmartPasteContext {
        application: argument_value(&args, "--application").unwrap_or_else(|| "CLI".into()),
        role: argument_value(&args, "--role"),
        label: Some(label),
        description: argument_value(&args, "--description"),
        help: None,
        placeholder: argument_value(&args, "--placeholder"),
    };
    match pasted_lib::smart_paste::select_value(&db, &source, &context) {
        Ok(outcome) => {
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&outcome).map_err(json_error)?
                );
            } else {
                print!("{}", outcome.value);
            }
        }
        Err(error) => {
            eprintln!("Smart Paste failed ({}): {}", error.code, error.message);
            std::process::exit(1);
        }
    }
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PastePartOutput<'a> {
    kind: &'a str,
    role: Option<&'a str>,
    aliases: &'a [String],
    value: &'a str,
    start_offset: usize,
    end_offset: usize,
    confidence: f32,
    analyzer_ref: &'a str,
}

fn list_parts(args: &[String], db_path: PathBuf) -> Result<()> {
    let clip_id = argument_value(args, "--clip")
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or_else(|| {
            eprintln!("Usage: pasted smart-paste parts --clip ID [--json]");
            std::process::exit(2);
        });
    let db = DbState::new(db_path)?;
    let clip = db.get_clip_by_id(clip_id).unwrap_or_else(|_| {
        eprintln!("Clip not found.");
        std::process::exit(1);
    });
    let source = clip.text_content.as_deref().unwrap_or_else(|| {
        eprintln!("Clip does not contain text.");
        std::process::exit(1);
    });
    let analysis = db
        .get_paste_parts(clip_id)?
        .unwrap_or_else(pasted_lib::smart_paste::parts::PastePartsAnalysis::empty);
    let output = analysis
        .parts
        .iter()
        .filter_map(|part| {
            pasted_lib::smart_paste::parts::value_at_offsets(source, part).map(|value| {
                PastePartOutput {
                    kind: &part.kind,
                    role: part.role.as_deref(),
                    aliases: &part.aliases,
                    value,
                    start_offset: part.start_offset,
                    end_offset: part.end_offset,
                    confidence: part.confidence,
                    analyzer_ref: &part.analyzer_ref,
                }
            })
        })
        .collect::<Vec<_>>();
    if args.iter().any(|argument| argument == "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&output).map_err(json_error)?
        );
    } else {
        for part in output {
            let label = part.role.unwrap_or(part.kind);
            println!("{label}: {}", part.value);
        }
    }
    Ok(())
}
