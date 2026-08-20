use super::super::*;
use super::common::{argument_value, argument_values, optional_argument_update};

pub(crate) fn extractor_definition_from_args(
    args: &[String],
    current: Option<&pasted_lib::content_extraction::Extractor>,
) -> ExtractorDefinitionInput {
    ExtractorDefinitionInput {
        name: argument_value(args, "--name").unwrap_or_else(|| {
            current
                .map(|item| item.name.clone())
                .unwrap_or_else(|| "Custom Extractor".into())
        }),
        description: argument_value(args, "--description").unwrap_or_else(|| {
            current
                .map(|item| item.description.clone())
                .unwrap_or_else(|| "Extracts searchable text with a local command.".into())
        }),
        engine: current
            .map(|item| item.engine.clone())
            .unwrap_or_else(|| CUSTOM_COMMAND_ENGINE.into()),
        executable_path: optional_argument_update(
            args,
            "--executable",
            "--automatic-discovery",
            current.and_then(|item| item.executable_path.clone()),
        ),
        model_path: optional_argument_update(
            args,
            "--model",
            "--no-model",
            current.and_then(|item| item.model_path.clone()),
        ),
        input_contract: argument_value(args, "--input").unwrap_or_else(|| {
            current
                .map(|item| item.input_contract.clone())
                .unwrap_or_else(|| "image".into())
        }),
        output_contract: argument_value(args, "--output").unwrap_or_else(|| {
            current
                .map(|item| item.output_contract.clone())
                .unwrap_or_else(|| "searchable_text".into())
        }),
        enabled: if args.iter().any(|argument| argument == "--disabled") {
            false
        } else if args.iter().any(|argument| argument == "--enabled") {
            true
        } else {
            current.map(|item| item.enabled).unwrap_or(false)
        },
        priority: argument_value(args, "--priority")
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or_else(|| current.map(|item| item.priority).unwrap_or(100)),
    }
}

pub(crate) fn read_extractor_recipe(path: &Path) -> Result<ExtractorRecipe> {
    let metadata =
        fs::metadata(path).map_err(|_| rusqlite::Error::InvalidPath(path.to_path_buf()))?;
    if !metadata.is_file() || metadata.len() > 1024 * 1024 {
        return Err(rusqlite::Error::InvalidParameterName(
            "Extractor recipe must be a regular JSON file no larger than 1 MB".into(),
        ));
    }
    let bytes = fs::read(path).map_err(|_| rusqlite::Error::InvalidPath(path.to_path_buf()))?;
    let recipe = serde_json::from_slice::<ExtractorRecipe>(&bytes).map_err(json_error)?;
    pasted_lib::extractor_recipe::validate_recipe(&recipe)
        .map_err(rusqlite::Error::InvalidParameterName)?;
    Ok(recipe)
}

pub(crate) fn extractor_recipe_definition_from_args(
    args: &[String],
    recipe: ExtractorRecipe,
    current: Option<&pasted_lib::content_extraction::Extractor>,
    authoring: Option<ExtractorAuthoringManifest>,
) -> ExtractorRecipeDefinitionInput {
    ExtractorRecipeDefinitionInput {
        name: argument_value(args, "--name").unwrap_or_else(|| {
            current
                .map(|item| item.name.clone())
                .unwrap_or_else(|| "Custom Extractor".into())
        }),
        description: argument_value(args, "--description").unwrap_or_else(|| {
            current
                .map(|item| item.description.clone())
                .unwrap_or_else(|| "Extracts searchable text with local commands.".into())
        }),
        enabled: if args.iter().any(|argument| argument == "--disabled") {
            false
        } else if args.iter().any(|argument| argument == "--enabled") {
            true
        } else {
            current.map(|item| item.enabled).unwrap_or(false)
        },
        priority: argument_value(args, "--priority")
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or_else(|| current.map(|item| item.priority).unwrap_or(100)),
        recipe,
        authoring: Some(authoring.unwrap_or(ExtractorAuthoringManifest {
            manifest_version: EXTRACTOR_AUTHORING_VERSION,
            source: ExtractorAuthoringSource::Manual,
            original_prompt: None,
            provider: None,
            model: None,
            messages: Vec::new(),
        })),
    }
}

pub(crate) fn print_extractor(
    extractor: &pasted_lib::content_extraction::Extractor,
    json: bool,
) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(extractor).map_err(json_error)?
        );
    } else {
        println!(
            "{}\t{}\t{} → {}\t{}",
            extractor.stable_ref,
            extractor.engine,
            extractor.input_contract,
            extractor.output_contract,
            extractor.name
        );
        if let Some(model_path) = extractor.model_path.as_deref() {
            println!("Model: {model_path}");
        }
        if let Some(executable_path) = extractor.executable_path.as_deref() {
            println!("Executable: {executable_path}");
        } else if let Some(location) = extractor.runtime.location.as_deref() {
            println!("Runtime: {location}");
        }
        if let Some(version) = extractor.runtime.version.as_deref() {
            println!("Version: {version}");
        }
        println!("Revision: {}", extractor.revision);
    }
    Ok(())
}

pub(crate) fn classifier_input_from_args(
    args: &[String],
    current: Option<&pasted_lib::content_classification::Classifier>,
) -> ClassifierInput {
    let patterns = argument_values(args, "--regex");
    ClassifierInput {
        name: argument_value(args, "--name").unwrap_or_else(|| {
            current
                .map(|item| item.name.clone())
                .unwrap_or_else(|| "Custom Classifier".into())
        }),
        content_type: argument_value(args, "--type").unwrap_or_else(|| {
            current
                .map(|item| item.content_type.clone())
                .unwrap_or_else(|| "text".into())
        }),
        description: argument_value(args, "--description").unwrap_or_else(|| {
            current
                .map(|item| item.description.clone())
                .unwrap_or_default()
        }),
        patterns: if patterns.is_empty() {
            current
                .map(|item| item.patterns.clone())
                .unwrap_or_else(|| vec!["^.+$".into()])
        } else {
            patterns
        },
        validator: argument_value(args, "--validator")
            .map(|value| (value != "none").then_some(value))
            .unwrap_or_else(|| current.and_then(|item| item.validator.clone())),
        enabled: if args.iter().any(|argument| argument == "--disabled") {
            false
        } else if args.iter().any(|argument| argument == "--enabled") {
            true
        } else {
            current.map(|item| item.enabled).unwrap_or(true)
        },
        priority: argument_value(args, "--priority")
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| current.map(|item| item.priority).unwrap_or(200)),
    }
}

pub(crate) fn print_classifier(
    classifier: &pasted_lib::content_classification::Classifier,
    json: bool,
) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(classifier).map_err(json_error)?
        );
    } else {
        println!(
            "{}\t{}\t{}\t{}",
            classifier.stable_ref, classifier.priority, classifier.content_type, classifier.name
        );
    }
    Ok(())
}

pub(crate) fn scan_existing_images(db: &DbState, clip_id: Option<i64>) -> Result<usize> {
    let extractors = db.active_image_text_extractors_for_features(true)?;
    if extractors.is_empty() {
        eprintln!("No available image text Extractor is enabled.");
        std::process::exit(1);
    }
    let classifiers = setting_value_is_enabled(
        db.get_setting(Feature::ContentClassification.setting_key())?
            .as_deref(),
    )
    .then(|| db.get_content_classifiers())
    .transpose()?;
    let mut pending = Vec::new();
    if let Some(clip_id) = clip_id {
        let clip = db.get_clip_by_id(clip_id)?;
        let image_base64 = clip.image_base64.clone().unwrap_or_else(|| {
            eprintln!("Clip #{clip_id} has no image data.");
            std::process::exit(2);
        });
        if !db.force_ocr_running(clip_id, &clip.content_hash)? {
            eprintln!("Clip #{clip_id} is not an active image clip.");
            std::process::exit(2);
        }
        pending.push(pasted_lib::db::OcrCandidate {
            clip_id,
            content_hash: clip.content_hash,
            image_base64,
        });
    }

    let mut scanned = 0usize;
    loop {
        let candidate = if !pending.is_empty() {
            Some(pending.remove(0))
        } else if clip_id.is_none() {
            db.claim_next_ocr_candidate()?
        } else {
            None
        };
        let Some(candidate) = candidate else {
            break;
        };
        let Some(bytes) = pasted_lib::ocr::decode_stored_image(&candidate.image_base64) else {
            db.complete_or_reset_ocr_attempt_with_extractor(
                candidate.clip_id,
                &candidate.content_hash,
                None,
                pasted_lib::db::OcrExtractorProvenance::identified(
                    &extractors[0].engine,
                    &extractors[0].stable_ref,
                    &extractors[0].name,
                ),
                Some("invalid_image_data"),
            )?;
            scanned += 1;
            continue;
        };
        let registry = pasted_lib::content_extraction::system_engine_registry();
        let analysis = pasted_lib::extraction_execution::analyze_images_with_registry(
            bytes,
            &extractors,
            classifiers.as_deref(),
            &registry,
        );
        let extractor = extractors
            .iter()
            .find(|extractor| extractor.stable_ref == analysis.target_ref)
            .expect("analysis target must identify an active Extractor");
        pasted_lib::extraction_execution::persist_claimed_image_analysis(
            db,
            candidate.clip_id,
            &candidate.content_hash,
            extractor,
            classifiers.is_some(),
            analysis,
        )?;
        scanned += 1;
    }
    Ok(scanned)
}
