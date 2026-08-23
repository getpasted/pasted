use super::super::*;
use super::*;

pub(crate) fn run_extractor(args: Vec<String>, db_path: PathBuf, conn: Connection) -> Result<()> {
    drop(conn);
    let db = DbState::new(db_path.clone())?;
    let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
    match subcommand {
        "list" | "ls" => {
            let extractors = db.get_content_extractors()?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&extractors).map_err(json_error)?
                );
            } else {
                for extractor in extractors {
                    println!(
                        "{}\t{}\t{}\t{}\t{} → {}\t{}",
                        extractor.stable_ref,
                        extractor.priority,
                        if extractor.enabled { "on" } else { "off" },
                        if extractor.is_available {
                            "available"
                        } else {
                            "unavailable"
                        },
                        extractor.input_contract,
                        extractor.output_contract,
                        extractor.name
                    );
                }
            }
        }
        "get" => {
            let reference = args.get(3).unwrap_or_else(|| {
                eprintln!("Usage: pasted extractor get <ref> [--json]");
                std::process::exit(2);
            });
            let mut extractor = db.get_content_extractor(reference)?;
            extractor.runtime =
                pasted_lib::content_extraction::inspect_extractor_runtime(&extractor);
            print_extractor(&extractor, args.iter().any(|argument| argument == "--json"))?;
        }
        "create" | "new" => {
            let extractor = if let Some(prompt) = argument_value(&args, "--prompt") {
                let proposal = pasted_lib::intelligence_executor::propose_extractor_recipe(
                    &db,
                    ProposeExtractorRecipeRequest {
                        prompt,
                        connection_id: argument_value(&args, "--connection"),
                    },
                    None,
                )
                .map_err(|error| rusqlite::Error::InvalidParameterName(error.message))?;
                let mut input = extractor_recipe_definition_from_args(
                    &args,
                    proposal.recipe,
                    None,
                    Some(proposal.authoring),
                );
                if argument_value(&args, "--name").is_none() {
                    input.name = proposal.name;
                }
                if argument_value(&args, "--description").is_none() {
                    input.description = proposal.description;
                }
                db.create_content_extractor_recipe(&input)?
            } else if let Some(recipe_path) = argument_value(&args, "--recipe") {
                let recipe = read_extractor_recipe(Path::new(&recipe_path))?;
                let input = extractor_recipe_definition_from_args(&args, recipe, None, None);
                db.create_content_extractor_recipe(&input)?
            } else {
                let legacy = extractor_definition_from_args(&args, None);
                let recipe = pasted_lib::content_extraction::recipe_for_legacy_definition(&legacy);
                let input = extractor_recipe_definition_from_args(&args, recipe, None, None);
                db.create_content_extractor_recipe(&input)?
            };
            print_extractor(&extractor, args.iter().any(|argument| argument == "--json"))?;
        }
        "update" => {
            let reference = args.get(3).unwrap_or_else(|| {
            eprintln!("Usage: pasted extractor update <ref> --recipe FILE [--format FORMAT]... [--name NAME] [--description TEXT] [--priority N] [--enabled|--disabled] [--json]");
            std::process::exit(2);
        });
            let current = db.get_content_extractor(reference)?;
            let updated = if let Some(recipe_path) = argument_value(&args, "--recipe") {
                let recipe = read_extractor_recipe(Path::new(&recipe_path))?;
                let input =
                    extractor_recipe_definition_from_args(&args, recipe, Some(&current), None);
                db.update_content_extractor_recipe(current.id, &input)?
            } else if args.iter().any(|argument| argument == "--format") {
                let input = extractor_recipe_definition_from_args(
                    &args,
                    current.recipe.clone(),
                    Some(&current),
                    None,
                );
                db.update_content_extractor_recipe(current.id, &input)?
            } else {
                let input = extractor_definition_from_args(&args, Some(&current));
                db.update_content_extractor_definition(current.id, &input)?
            };
            print_extractor(&updated, args.iter().any(|argument| argument == "--json"))?;
        }
        "propose" | "draft" => {
            let prompt = argument_value(&args, "--prompt").unwrap_or_else(|| {
                eprintln!(
                    "Usage: pasted extractor propose --prompt TEXT [--connection ID] [--json]"
                );
                std::process::exit(2);
            });
            let proposal = pasted_lib::intelligence_executor::propose_extractor_recipe(
                &db,
                ProposeExtractorRecipeRequest {
                    prompt,
                    connection_id: argument_value(&args, "--connection"),
                },
                None,
            )
            .map_err(|error| rusqlite::Error::InvalidParameterName(error.message))?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&proposal).map_err(json_error)?
                );
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&proposal.recipe).map_err(json_error)?
                );
                for item in proposal.setup_guidance {
                    eprintln!("Setup: {item}");
                }
            }
        }
        "history" => {
            let reference = args.get(3).unwrap_or_else(|| {
                eprintln!("Usage: pasted extractor history <ref> [--json]");
                std::process::exit(2);
            });
            let sessions = db.get_extractor_authoring_sessions(reference)?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&sessions).map_err(json_error)?
                );
            } else if sessions.is_empty() {
                println!("No authoring history.");
            } else {
                for session in sessions {
                    println!(
                        "{}\t{}\t{}\t{}",
                        session.created_at,
                        session.source.stable_name(),
                        session.provider.as_deref().unwrap_or("local"),
                        session.model.as_deref().unwrap_or("-")
                    );
                }
            }
        }
        "duplicate" | "copy" => {
            let reference = args.get(3).unwrap_or_else(|| {
                eprintln!("Usage: pasted extractor duplicate <ref> [--name NAME] [--json]");
                std::process::exit(2);
            });
            let duplicate = db.duplicate_content_extractor(
                reference,
                argument_value(&args, "--name").as_deref(),
            )?;
            print_extractor(&duplicate, args.iter().any(|argument| argument == "--json"))?;
        }
        "delete" | "remove" => {
            let reference = args.get(3).unwrap_or_else(|| {
                eprintln!("Usage: pasted extractor delete <ref> [--json]");
                std::process::exit(2);
            });
            let extractor = db.get_content_extractor(reference)?;
            db.delete_content_extractor(extractor.id)?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::json!({ "deleted": true, "stableRef": extractor.stable_ref })
                );
            } else {
                println!("Deleted Extractor {}.", extractor.name);
            }
        }
        "run" | "test" => {
            let reference = args.get(3).unwrap_or_else(|| {
            eprintln!("Usage: pasted extractor run <ref> (--clip ID | --file PATH) [--apply] [--json]");
            std::process::exit(2);
        });
            let extractor = db.get_content_extractor(reference)?;
            let clip_id =
                argument_value(&args, "--clip").and_then(|value| value.parse::<i64>().ok());
            let file_path = argument_value(&args, "--file");
            if clip_id.is_some() == file_path.is_some() {
                eprintln!("Provide exactly one of --clip ID or --file PATH.");
                std::process::exit(2);
            }
            let apply = args.iter().any(|argument| argument == "--apply");
            if apply && clip_id.is_none() {
                eprintln!("--apply requires --clip ID.");
                std::process::exit(2);
            }
            let classifiers = setting_value_is_enabled(
                db.get_setting(Feature::ContentClassification.setting_key())?
                    .as_deref(),
            )
            .then(|| db.get_content_classifiers())
            .transpose()?;
            let image_contract = extractor.supports_contract(
                pasted_lib::analysis_contract::RepresentationKind::ImageBytes,
                pasted_lib::analysis_contract::RepresentationKind::SearchableText,
            );
            let file_contract = extractor.supports_contract(
                pasted_lib::analysis_contract::RepresentationKind::FileReferences,
                pasted_lib::analysis_contract::RepresentationKind::SearchableText,
            );
            if !image_contract && !file_contract {
                eprintln!("This Extractor does not have a runnable input contract.");
                std::process::exit(2);
            }
            if image_contract
                && matches!(
                    extractor.stable_ref.as_str(),
                    pasted_lib::content_extraction::APPLE_VISION_OCR_REF
                        | pasted_lib::content_extraction::TESSERACT_OCR_REF
                )
            {
                require_feature(&db, Feature::Ocr);
            }
            if file_contract
                && extractor.stable_ref == pasted_lib::content_extraction::WHISPER_TRANSCRIPTION_REF
            {
                require_feature(&db, Feature::Transcriptions);
            }
            let mut content_hash = None;
            let analysis = if image_contract {
                let image_bytes = if let Some(clip_id) = clip_id {
                    let clip = db.get_clip_by_id(clip_id)?;
                    content_hash = Some(clip.content_hash);
                    clip.image_base64
                        .as_deref()
                        .and_then(pasted_lib::ocr::decode_stored_image)
                        .unwrap_or_else(|| {
                            eprintln!("Clip #{clip_id} has no extractable image data.");
                            std::process::exit(2);
                        })
                } else {
                    read_file_bounded(
                        Path::new(file_path.as_deref().expect("checked above")),
                        pasted_lib::resource_limits::MAX_ENCODED_IMAGE_BYTES,
                    )?
                };
                pasted_lib::extraction_execution::analyze_image(
                    image_bytes,
                    &extractor,
                    classifiers.as_deref(),
                )
            } else {
                let paths = if let Some(clip_id) = clip_id {
                    let clip = db.get_clip_by_id(clip_id)?;
                    content_hash = Some(clip.content_hash.clone());
                    clip.text_content
                        .as_deref()
                        .map(pasted_lib::content_inspection::parse_file_paths)
                        .filter(|paths| !paths.is_empty())
                        .unwrap_or_else(|| {
                            eprintln!("Clip #{clip_id} has no extractable file references.");
                            std::process::exit(2);
                        })
                } else {
                    vec![file_path.expect("checked above")]
                };
                if !pasted_lib::resource_limits::file_list_within_limit(&paths) {
                    eprintln!("File references exceed the extraction safety limit.");
                    std::process::exit(2);
                }
                pasted_lib::extraction_execution::analyze_files(
                    paths,
                    &extractor,
                    classifiers.as_deref(),
                )
            };
            let result = if apply {
                let clip_id = clip_id.expect("validated apply target");
                let content_hash = content_hash.as_deref().expect("clip input has a hash");
                if image_contract {
                    pasted_lib::extraction_execution::apply_image_analysis(
                        &db,
                        clip_id,
                        content_hash,
                        &extractor,
                        classifiers.is_some(),
                        analysis,
                    )?
                } else {
                    pasted_lib::extraction_execution::apply_file_analysis(
                        &db,
                        clip_id,
                        content_hash,
                        &extractor,
                        classifiers.is_some(),
                        analysis,
                    )?
                }
            } else {
                pasted_lib::extraction_execution::ExtractionApplicationResult::preview(analysis)
            };
            if args.iter().any(|argument| argument == "--json") {
                println!("{}", serde_json::json!(&result));
            } else if let Some(failure) = result.analysis.failure.as_ref() {
                eprintln!("Extractor failed ({}): {}", failure.code, failure.message);
            } else if let Some(text) = result.analysis.output.as_deref() {
                print!("{text}");
            } else {
                println!("No text extracted.");
            }
            if result.analysis.failed() {
                let _ = io::stdout().flush();
                let _ = io::stderr().flush();
                std::process::exit(1);
            }
        }
        "restore-defaults" => {
            db.restore_default_content_extractors()?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::json!({ "restoredDefaults": true, "kind": "extractors" })
                );
            } else {
                println!("Restored shipped Extractor defaults.");
            }
        }
        _ => {
            eprintln!("Usage: pasted extractor list|get|create|update|propose|history|duplicate|delete|run|restore-defaults [options] [--json]");
            std::process::exit(2);
        }
    }
    Ok(())
}
