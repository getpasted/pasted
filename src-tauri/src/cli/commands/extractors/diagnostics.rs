use super::*;

pub(super) fn run_preflight(args: &[String], db: &DbState) -> Result<()> {
    let reference = args.get(3).unwrap_or_else(|| {
        eprintln!("Usage: pasted extractor preflight <ref> [--json]");
        std::process::exit(2);
    });
    let extractor = db.get_content_extractor(reference)?;
    let report = pasted_lib::extractor_recipe::diagnose(&extractor.recipe);
    if args.iter().any(|argument| argument == "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(json_error)?
        );
    } else if report.is_available {
        println!("Extractor preflight passed.");
    } else {
        for issue in report.issues {
            println!(
                "{}\t{}\t{}",
                issue.code.stable_name(),
                issue.subject_id,
                issue.label
            );
        }
    }
    Ok(())
}

pub(super) fn run_diagnose(args: &[String], db: &DbState) -> Result<()> {
    let reference = args.get(3).unwrap_or_else(|| {
        eprintln!(
            "Usage: pasted extractor diagnose <ref> [--prompt TEXT] [--connection ID] [--json]"
        );
        std::process::exit(2);
    });
    let extractor = db.get_content_extractor(reference)?;
    let outcome = pasted_lib::intelligence_executor::repair_extractor_recipe(
        db,
        pasted_lib::intelligence_executor::RepairExtractorRecipeRequest {
            name: extractor.name,
            description: extractor.description,
            recipe: extractor.recipe,
            prompt: argument_value(args, "--prompt"),
            connection_id: argument_value(args, "--connection"),
            max_attempts: Some(3),
        },
        None,
    )
    .map_err(|error| rusqlite::Error::InvalidParameterName(error.message))?;
    if args.iter().any(|argument| argument == "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&outcome).map_err(json_error)?
        );
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&outcome.recipe).map_err(json_error)?
        );
        eprintln!("Status: {}", outcome.status.stable_name());
        for item in outcome.setup_guidance {
            eprintln!("Setup: {item}");
        }
    }
    Ok(())
}
