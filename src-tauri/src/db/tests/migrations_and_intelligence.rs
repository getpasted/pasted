use super::*;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchGrammarFixture {
    query: String,
    sources: Vec<String>,
    clip_types: Vec<String>,
    content_types: Vec<String>,
    file_formats: Vec<String>,
    terms: Vec<String>,
    requires_note: bool,
    requires_named: bool,
    requires_pinned: bool,
    requires_protected: bool,
    requires_trashed: bool,
    incomplete: bool,
    regex: Option<String>,
    regex_fallback: Option<String>,
}
#[test]
fn native_and_frontend_search_grammar_share_public_fixtures() {
    let fixtures: Vec<SearchGrammarFixture> =
        serde_json::from_str(include_str!("../../../../contracts/search/v1/grammar.json")).unwrap();
    for fixture in fixtures {
        let parsed = parse_clip_search(&fixture.query);
        assert_eq!(parsed.sources, fixture.sources, "{}", fixture.query);
        assert_eq!(parsed.clip_types, fixture.clip_types, "{}", fixture.query);
        assert_eq!(
            parsed.content_types, fixture.content_types,
            "{}",
            fixture.query
        );
        assert_eq!(
            parsed.file_formats, fixture.file_formats,
            "{}",
            fixture.query
        );
        assert_eq!(parsed.terms, fixture.terms, "{}", fixture.query);
        assert_eq!(
            parsed.requires_note, fixture.requires_note,
            "{}",
            fixture.query
        );
        assert_eq!(
            parsed.requires_named, fixture.requires_named,
            "{}",
            fixture.query
        );
        assert_eq!(
            parsed.requires_pinned, fixture.requires_pinned,
            "{}",
            fixture.query
        );
        assert_eq!(
            parsed.requires_protected, fixture.requires_protected,
            "{}",
            fixture.query
        );
        assert_eq!(
            parsed.requires_trashed, fixture.requires_trashed,
            "{}",
            fixture.query
        );
        assert_eq!(parsed.incomplete, fixture.incomplete, "{}", fixture.query);
        assert_eq!(parsed.regex, fixture.regex, "{}", fixture.query);
        assert_eq!(
            parsed.regex_fallback, fixture.regex_fallback,
            "{}",
            fixture.query
        );
    }
}

#[test]
#[ignore = "run explicitly against a disposable copy of a real Pasted database"]
fn real_database_library_item_migration_smoke_test() {
    let path = std::env::var("PASTED_MIGRATION_TEST_DB")
        .expect("PASTED_MIGRATION_TEST_DB must point to a disposable database copy");
    let db = DbState::new(PathBuf::from(path)).unwrap();
    let items = db.get_library_items(None, true).unwrap();
    assert!(items.iter().any(|item| item.item.kind == "classifier"));
    assert!(items.iter().any(|item| item.item.kind == "extractor"));
    assert!(items.iter().any(|item| item.item.kind == "operation"));
    assert!(items.iter().any(|item| item.item.kind == "capture"));
}

#[test]
fn capture_capabilities_are_exposed_through_the_shared_registry() {
    let db = setup_test_db();
    let items = db.get_library_items(Some("capture"), false).unwrap();
    assert_eq!(items.len(), 2);
    let clip_type = items
        .iter()
        .find(|item| item.item.stable_ref == "capture:clip-type-v1")
        .unwrap();
    assert_eq!(clip_type.item.input_contract, "clipboard_representation");
    assert_eq!(clip_type.item.output_contract, "clip_type");
    assert_eq!(clip_type.analysis_pass, None);
    assert_eq!(clip_type.participant_contract, None);
    let source = items
        .iter()
        .find(|item| item.item.stable_ref == "capture:source-attribution-v1")
        .unwrap();
    assert_eq!(source.item.stable_ref, "capture:source-attribution-v1");
    assert_eq!(source.item.input_contract, "clipboard_event");
    assert_eq!(source.item.output_contract, "source_attribution");
    assert_eq!(source.analysis_pass, None);
    assert_eq!(source.participant_contract, None);
}

#[test]
fn content_extractors_are_versioned_available_and_restorable() {
    let db = setup_test_db();
    let extractors = db.get_content_extractors().unwrap();
    assert_eq!(
        extractors.len(),
        crate::content_extraction::EXTRACTOR_PRESETS.len()
    );
    let apple = extractors
        .iter()
        .find(|extractor| extractor.stable_ref == crate::content_extraction::APPLE_VISION_OCR_REF)
        .unwrap();
    assert_eq!(apple.input_contract, "image");
    assert_eq!(apple.output_contract, "searchable_text");
    assert_eq!(apple.is_available, cfg!(target_os = "macos"));
    assert_eq!(
        apple.unavailable_reason.is_some(),
        !cfg!(target_os = "macos")
    );
    let tesseract = extractors
        .iter()
        .find(|extractor| extractor.stable_ref == crate::content_extraction::TESSERACT_OCR_REF)
        .unwrap();
    assert_eq!(tesseract.engine, crate::content_extraction::RECIPE_ENGINE);
    assert_eq!(tesseract.input_contract, "image");
    assert_eq!(tesseract.output_contract, "searchable_text");
    assert_eq!(
        tesseract.is_available,
        crate::extractor_recipe::availability(&tesseract.recipe).is_available
    );
    let whisper = extractors
        .iter()
        .find(|extractor| {
            extractor.stable_ref == crate::content_extraction::WHISPER_TRANSCRIPTION_REF
        })
        .unwrap();
    assert_eq!(whisper.engine, crate::content_extraction::RECIPE_ENGINE);
    assert_eq!(whisper.input_contract, "file_references");
    assert_eq!(whisper.output_contract, "searchable_text");
    assert_eq!(whisper.model_path, None);
    let mut whisper_recipe = whisper.recipe.clone();
    whisper_recipe
        .resources
        .iter_mut()
        .find(|resource| resource.id == "model")
        .unwrap()
        .path = Some("/tmp/pasted-missing-whisper-model.bin".into());
    db.update_content_extractor_recipe(
        whisper.id,
        &crate::extractor_recipe::ExtractorRecipeDefinitionInput {
            name: whisper.name.clone(),
            description: whisper.description.clone(),
            enabled: whisper.enabled,
            priority: whisper.priority,
            recipe: whisper_recipe,
            authoring: None,
        },
    )
    .unwrap();
    let configured_whisper = db.get_content_extractor(&whisper.stable_ref).unwrap();
    assert_eq!(
        configured_whisper.model_path.as_deref(),
        Some("/tmp/pasted-missing-whisper-model.bin")
    );
    assert!(!configured_whisper.is_available);
    assert_eq!(configured_whisper.revision, whisper.revision + 1);

    db.update_content_extractor(
        apple.id,
        &crate::content_extraction::ExtractorInput {
            name: "Local Image Text".into(),
            description: "Customized label".into(),
            enabled: false,
            priority: 42,
        },
    )
    .unwrap();
    let updated = db.get_content_extractor(&apple.stable_ref).unwrap();
    assert_eq!(updated.name, "Local Image Text");
    assert!(!updated.enabled);
    let active = db.active_image_text_extractor().unwrap();
    if tesseract.is_available {
        assert_eq!(
            active
                .as_ref()
                .map(|extractor| extractor.stable_ref.as_str()),
            Some(crate::content_extraction::TESSERACT_OCR_REF)
        );
    } else {
        assert!(active.is_none());
    }
    assert!(db
        .get_library_items(Some("extractor"), false)
        .unwrap()
        .iter()
        .any(|item| {
            item.item.stable_ref == apple.stable_ref
                && item.item.enabled == Some(false)
                && item.analysis_pass.as_deref() == Some("extract")
        }));

    let custom = db
        .create_content_extractor(&crate::content_extraction::ExtractorDefinitionInput {
            name: "Project OCR".into(),
            description: "Extracts project screenshots".into(),
            engine: crate::content_extraction::APPLE_VISION_ENGINE.into(),
            executable_path: None,
            model_path: None,
            input_contract: "image".into(),
            output_contract: "searchable_text".into(),
            enabled: true,
            priority: 80,
        })
        .unwrap();
    assert!(!custom.is_builtin);
    assert_eq!(
        db.get_content_extractor(&custom.stable_ref).unwrap().id,
        custom.id
    );
    let duplicate = db
        .duplicate_content_extractor(&custom.stable_ref, Some("Project OCR Copy"))
        .unwrap();
    assert_eq!(duplicate.priority, 81);
    assert_eq!(duplicate.revision, 1);
    db.update_content_extractor_definition(
        duplicate.id,
        &crate::content_extraction::ExtractorDefinitionInput {
            name: "Project OCR Revised".into(),
            description: duplicate.description.clone(),
            engine: duplicate.engine.clone(),
            executable_path: duplicate.executable_path.clone(),
            model_path: duplicate.model_path.clone(),
            input_contract: duplicate.input_contract.clone(),
            output_contract: duplicate.output_contract.clone(),
            enabled: false,
            priority: duplicate.priority,
        },
    )
    .unwrap();
    db.delete_content_extractor(custom.id).unwrap();
    assert!(db.get_content_extractor(&custom.stable_ref).is_err());

    db.restore_default_content_extractors().unwrap();
    let restored_extractors = db.get_content_extractors().unwrap();
    let restored = restored_extractors
        .iter()
        .find(|extractor| extractor.stable_ref == apple.stable_ref)
        .unwrap();
    assert_eq!(restored.name, "Apple Vision OCR");
    assert!(restored.enabled);
    assert_eq!(restored.priority, 10);
    assert!(restored.revision > updated.revision);
    assert_eq!(
        db.get_content_extractor(crate::content_extraction::WHISPER_TRANSCRIPTION_REF)
            .unwrap()
            .model_path
            .as_deref(),
        Some("/tmp/pasted-missing-whisper-model.bin")
    );
    assert!(restored_extractors.iter().any(|extractor| {
        extractor.stable_ref == duplicate.stable_ref
            && extractor.name == "Project OCR Revised"
            && !extractor.enabled
    }));
}

#[test]
fn extractor_and_classifier_toggles_record_explicit_activity_across_shared_paths() {
    let db = setup_test_db();
    let extractor = db
        .get_content_extractor(crate::content_extraction::APPLE_VISION_OCR_REF)
        .unwrap();
    db.update_content_extractor_definition(
        extractor.id,
        &crate::content_extraction::ExtractorDefinitionInput {
            name: extractor.name.clone(),
            description: extractor.description.clone(),
            engine: extractor.engine.clone(),
            executable_path: extractor.executable_path.clone(),
            model_path: extractor.model_path.clone(),
            input_contract: extractor.input_contract.clone(),
            output_contract: extractor.output_contract.clone(),
            enabled: false,
            priority: extractor.priority,
        },
    )
    .unwrap();
    db.set_library_item_enabled("extractor", &extractor.stable_ref, true)
        .unwrap();

    let classifier = db
        .get_content_classifiers()
        .unwrap()
        .into_iter()
        .find(|classifier| classifier.stable_ref == "email")
        .unwrap();
    db.update_content_classifier(
        classifier.id,
        &crate::content_classification::ClassifierInput {
            name: classifier.name.clone(),
            content_type: classifier.content_type.clone(),
            description: classifier.description.clone(),
            patterns: classifier.patterns.clone(),
            validator: classifier.validator.clone(),
            enabled: false,
            priority: classifier.priority,
        },
    )
    .unwrap();
    db.set_library_item_enabled("classifier", &classifier.stable_ref, true)
        .unwrap();
    db.set_library_item_enabled("classifier", &classifier.stable_ref, true)
        .unwrap();

    let analysis_logs = db
        .get_activity_logs(Some(100), None)
        .unwrap()
        .into_iter()
        .filter(|log| {
            log.event_type.starts_with("content_extractor_")
                || log.event_type.starts_with("content_classifier_")
        })
        .collect::<Vec<_>>();
    assert!(analysis_logs.iter().all(|log| {
        log.attributes["analysis.participant.kind"].is_string()
            && log.attributes["analysis.participant.ref"].is_string()
            && log.attributes["analysis.participant.enabled"].is_boolean()
    }));
    let events = analysis_logs
        .into_iter()
        .map(|log| (log.event_type, log.description))
        .collect::<Vec<_>>();
    assert_eq!(
        events,
        vec![
            (
                "content_classifier_enabled".to_string(),
                "Enabled Classifier \"Email Addresses\"".to_string(),
            ),
            (
                "content_classifier_disabled".to_string(),
                "Disabled Classifier \"Email Addresses\"".to_string(),
            ),
            (
                "content_extractor_enabled".to_string(),
                "Enabled Extractor \"Apple Vision OCR\"".to_string(),
            ),
            (
                "content_extractor_disabled".to_string(),
                "Disabled Extractor \"Apple Vision OCR\"".to_string(),
            ),
        ]
    );
}

#[test]
fn derived_analysis_classification_is_hash_safe_and_non_destructive() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "image",
            None,
            None,
            Some(crate::resource_limits::TEST_PNG_DATA_URL),
            "analysis-image-hash",
            "Screenshot",
        )
        .unwrap();

    assert!(db
        .replace_analysis_classifications(
            clip.id,
            &clip.content_hash,
            &[crate::content_classification::ClassificationMatch {
                classifier_ref: "email".into(),
                classifier_name: "Email".into(),
                content_type: "email".into(),
                priority: 10,
                start_offset: 0,
                end_offset: 5,
            }],
            "searchable_text",
        )
        .unwrap());
    let classification = db.get_analysis_classifications(clip.id).unwrap().remove(0);
    assert_eq!(classification.content_type, "email");
    assert_eq!(classification.source_representation, "searchable_text");
    assert_eq!(db.get_clip_by_id(clip.id).unwrap().content_type, "image");

    assert!(!db
        .replace_analysis_classifications(
            clip.id,
            "stale-hash",
            &[crate::content_classification::ClassificationMatch {
                classifier_ref: "credential".into(),
                classifier_name: "Credential".into(),
                content_type: "credential".into(),
                priority: 10,
                start_offset: 0,
                end_offset: 5,
            }],
            "searchable_text",
        )
        .unwrap());
    assert_eq!(
        db.get_analysis_classifications(clip.id).unwrap()[0].content_type,
        "email"
    );

    db.replace_analysis_classifications(clip.id, &clip.content_hash, &[], "searchable_text")
        .unwrap();
    assert!(db.get_analysis_classifications(clip.id).unwrap().is_empty());
}

#[test]
fn content_classifiers_are_editable_deletable_restorable_and_backed_up() {
    let source = setup_test_db();
    let shipped = source.get_content_classifiers().unwrap();
    assert_eq!(
        shipped.len(),
        crate::content_classification::CLASSIFIER_PRESETS.len()
    );

    let email = shipped
        .iter()
        .find(|classifier| classifier.stable_ref == "email")
        .unwrap();
    assert_eq!(
        email
            .defaults
            .as_ref()
            .map(|defaults| defaults.name.as_str()),
        Some("Email Addresses")
    );
    let custom_pattern = r"(?i)^[a-z0-9._%+-]+@example\.test$".to_string();
    source
        .update_content_classifier(
            email.id,
            &crate::content_classification::ClassifierInput {
                name: "Example Mail".into(),
                content_type: "email".into(),
                description: "Project-specific addresses".into(),
                patterns: vec![custom_pattern.clone()],
                validator: None,
                enabled: true,
                priority: 7,
            },
        )
        .unwrap();
    source
        .create_content_type(&crate::content_types::ContentTypeInput {
            id: "ticket_id".into(),
            label: "Ticket ID".into(),
            icon: "Hash".into(),
            group: "custom".into(),
            conceal_clips: true,
        })
        .unwrap();
    let custom = source
        .create_content_classifier(&crate::content_classification::ClassifierInput {
            name: "Ticket IDs".into(),
            content_type: "ticket_id".into(),
            description: "Internal issue identifiers".into(),
            patterns: vec![r"^PASTE-[0-9]+$".into()],
            validator: None,
            enabled: true,
            priority: 8,
        })
        .unwrap();
    assert!(custom.defaults.is_none());
    source.delete_content_classifier(custom.id).unwrap();

    let backup = source.export_backup_json().unwrap();
    assert!(backup.contains("\"content_classifiers\""));
    assert!(!backup.contains("\"content_detectors\""));
    let mut legacy_backup: serde_json::Value = serde_json::from_str(&backup).unwrap();
    let classifiers = legacy_backup
        .as_object_mut()
        .unwrap()
        .remove("content_classifiers")
        .unwrap();
    legacy_backup
        .as_object_mut()
        .unwrap()
        .insert("content_detectors".into(), classifiers);
    let destination = setup_test_db();
    destination
        .import_backup_json(&serde_json::to_string(&legacy_backup).unwrap())
        .unwrap();
    let restored = destination.get_content_classifiers().unwrap();
    let restored_email = restored
        .iter()
        .find(|classifier| classifier.stable_ref == "email")
        .unwrap();
    assert_eq!(restored_email.name, "Example Mail");
    assert_eq!(restored_email.patterns, vec![custom_pattern]);
    assert!(!restored
        .iter()
        .any(|classifier| classifier.stable_ref == custom.stable_ref));

    destination.restore_default_content_classifiers().unwrap();
    let defaults = destination.get_content_classifiers().unwrap();
    assert_eq!(
        defaults
            .iter()
            .find(|classifier| classifier.stable_ref == "email")
            .unwrap()
            .name,
        "Email Addresses"
    );
    assert!(!defaults
        .iter()
        .any(|classifier| classifier.stable_ref == custom.stable_ref));
}

#[test]
fn a_single_classifier_can_be_resolved_duplicated_and_applied() {
    let db = setup_test_db();
    let email = db.get_content_classifier("email").unwrap();
    assert_eq!(
        db.get_content_classifier(&email.id.to_string()).unwrap().id,
        email.id
    );
    let duplicate = db
        .duplicate_content_classifier("email", Some("Email Copy"))
        .unwrap();
    assert_eq!(duplicate.name, "Email Copy");
    assert!(!duplicate.is_builtin);

    let matching = db
        .save_clip(
            "text",
            Some("person@example.com"),
            None,
            None,
            "classifier-apply-match",
            "Test",
        )
        .unwrap();
    let applied = db.apply_content_classifier(matching.id, "email").unwrap();
    assert!(applied.analysis.matched);
    assert_eq!(applied.application.applied_clip_id, Some(matching.id));
    assert_eq!(db.get_clip_by_id(matching.id).unwrap().content_type, "text");
    assert_eq!(
        db.get_clip_by_id(matching.id).unwrap().content_types,
        vec!["email"]
    );

    let nonmatching = db
        .save_clip(
            "text",
            Some("plain prose"),
            None,
            None,
            "classifier-apply-no-match",
            "Test",
        )
        .unwrap();
    let not_applied = db
        .apply_content_classifier(nonmatching.id, "email")
        .unwrap();
    assert!(!not_applied.analysis.matched);
    assert_eq!(not_applied.application.applied_clip_id, None);
    assert_eq!(
        db.get_clip_by_id(nonmatching.id).unwrap().content_type,
        "text"
    );

    let empty = db
        .save_clip(
            "text",
            Some(""),
            None,
            None,
            "classifier-apply-empty",
            "Test",
        )
        .unwrap();
    assert!(db
        .apply_content_classifier(empty.id, "email")
        .unwrap_err()
        .to_string()
        .contains("no analyzable text"));
    let whitespace = db
        .save_clip(
            "text",
            Some(" \n\t"),
            None,
            None,
            "classifier-apply-whitespace",
            "Test",
        )
        .unwrap();
    assert!(db
        .apply_content_classifier(whitespace.id, "email")
        .unwrap_err()
        .to_string()
        .contains("no analyzable text"));

    db.delete_content_classifier(duplicate.id).unwrap();
    assert!(db.get_content_classifier(&duplicate.stable_ref).is_err());
}

#[test]
fn shared_text_capture_hashes_deduplicates_and_classifies() {
    let db = setup_test_db();
    let first = db
        .save_text_clip("person@example.com", "CLI Terminal")
        .unwrap();
    assert_eq!(first.content_type, "text");
    assert_eq!(first.content_types, vec!["email"]);
    let email_bin = db
        .create_bin(
            "Email",
            "Mail",
            "default",
            Some(r#"{"type":"content_type","value":"email"}"#),
        )
        .unwrap();
    assert_eq!(
        db.get_clips(Some(email_bin.id), false).unwrap()[0].id,
        first.id
    );
    db.set_bin_transform_ref(email_bin.id, Some("transform:test-email"))
        .unwrap();
    assert_eq!(
        db.matching_smart_bin_transforms(
            &first.content_type,
            &first.file_formats,
            &first.content_types,
            first.text_content.as_deref().unwrap(),
            &first.source,
        )
        .unwrap(),
        vec![(email_bin.id, "transform:test-email".to_string())]
    );
    assert_eq!(first.source, "CLI Terminal");
    assert!(!first.content_hash.is_empty());
    let structure = db
        .get_structural_inspection(
            first.id,
            &crate::inspection_execution::inspection_input_hash(&first),
        )
        .unwrap()
        .expect("capture should persist its Analyzer structure");
    assert_eq!(structure.text.unwrap().word_count, 1);

    let duplicate = db
        .save_text_clip("person@example.com", "CLI Terminal")
        .unwrap();
    assert_eq!(duplicate.id, first.id);
    assert_eq!(db.get_clips(None, false).unwrap().len(), 1);
}

#[test]
fn extractor_observations_round_trip_per_clip_in_priority_order() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "image",
            None,
            Some("test-image"),
            None,
            "extractor-observations",
            "Tests",
        )
        .unwrap();
    let observations = vec![
        crate::content_analysis::ExtractionObservation {
            extractor_ref: "extractor:second".into(),
            extractor_name: "Second".into(),
            engine: "second-v1".into(),
            priority: 20,
            duplicate_of: None,
            outcome: crate::content_extraction::ExtractionOutcome::Produced {
                text: "Hello World!".into(),
            },
        },
        crate::content_analysis::ExtractionObservation {
            extractor_ref: "extractor:first".into(),
            extractor_name: "First".into(),
            engine: "first-v1".into(),
            priority: 10,
            duplicate_of: None,
            outcome: crate::content_extraction::ExtractionOutcome::NoOutput,
        },
    ];

    assert!(db
        .record_extraction_observations(clip.id, &clip.content_hash, &observations)
        .unwrap());
    let stored = db.get_extraction_observations(clip.id).unwrap();
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[0].observation.extractor_ref, "extractor:first");
    assert_eq!(stored[1].observation.extractor_ref, "extractor:second");
    assert!(matches!(
        stored[1].observation.outcome,
        crate::content_extraction::ExtractionOutcome::Produced { ref text }
            if text == "Hello World!"
    ));
    let second_run = vec![crate::content_analysis::ExtractionObservation {
        extractor_ref: "extractor:first".into(),
        extractor_name: "First".into(),
        engine: "first-v1".into(),
        priority: 10,
        duplicate_of: None,
        outcome: crate::content_extraction::ExtractionOutcome::Failed {
            failure: crate::content_extraction::ExtractionFailure {
                code: "test_failure".into(),
                message: "The Extractor failed.".into(),
            },
        },
    }];
    assert!(db
        .record_extraction_observations(clip.id, &clip.content_hash, &second_run)
        .unwrap());
    let history = db.get_extraction_history(clip.id, 101, 0).unwrap();
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].observation.extractor_ref, "extractor:first");
    assert_ne!(history[0].run_id, history[1].run_id);
}

#[test]
fn duplicate_text_capture_inspects_using_the_stored_source() {
    let db = setup_test_db();
    let first = db.save_text_clip("person@example.com", "Safari").unwrap();
    let duplicate = db
        .save_text_clip("person@example.com", "CLI Terminal")
        .unwrap();
    assert_eq!(duplicate.id, first.id);
    assert_eq!(duplicate.source, "Safari");

    let structure = db
        .get_structural_inspection(
            duplicate.id,
            &crate::inspection_execution::inspection_input_hash(&duplicate),
        )
        .unwrap()
        .expect("duplicate capture should persist structure for the stored clip");
    assert_eq!(
        structure.origin,
        crate::content_inspection::OriginKind::ClipboardContent
    );
}

#[test]
fn text_capture_still_inspects_when_content_classification_is_disabled() {
    let db = setup_test_db();
    db.save_settings(&std::collections::HashMap::from([(
        crate::features::Feature::ContentClassification
            .setting_key()
            .to_string(),
        "false".to_string(),
    )]))
    .unwrap();
    let clip = db
        .save_text_clip("person@example.com", "CLI Terminal")
        .unwrap();
    assert_eq!(clip.content_type, "text");
    assert!(db
        .get_structural_inspection(
            clip.id,
            &crate::inspection_execution::inspection_input_hash(&clip),
        )
        .unwrap()
        .is_some());
}

#[test]
fn content_type_registry_protects_builtin_ids_and_archives_custom_types_safely() {
    let db = setup_test_db();
    let registered = db.get_content_types(false).unwrap();
    assert!(registered.iter().all(|content_type| {
        !crate::content_types::is_structural_clip_type_id(&content_type.id)
    }));
    assert!(db
        .create_content_type(&crate::content_types::ContentTypeInput {
            id: "text".into(),
            label: "Text".into(),
            icon: "Type".into(),
            group: "general".into(),
            conceal_clips: false,
        })
        .is_err());
    let mut payment = db
        .get_content_types(false)
        .unwrap()
        .into_iter()
        .find(|item| item.id == "payment_card")
        .unwrap();
    assert_eq!(payment.conceal_clips, Some(true));
    assert_eq!(
        payment
            .defaults
            .as_ref()
            .map(|defaults| defaults.conceal_clips),
        Some(true)
    );
    assert_eq!(
        payment
            .defaults
            .as_ref()
            .map(|defaults| defaults.label.as_str()),
        Some("Payment Card")
    );
    payment.label = "Cards".into();
    payment.icon = "ShieldKeyhole".into();
    db.update_content_type(
        "payment_card",
        &crate::content_types::ContentTypeInput {
            id: payment.id.clone(),
            label: payment.label.clone(),
            icon: payment.icon.clone(),
            group: payment.group.clone(),
            conceal_clips: false,
        },
    )
    .unwrap();
    assert_eq!(
        db.get_content_types(false)
            .unwrap()
            .into_iter()
            .find(|item| item.id == "payment_card")
            .unwrap()
            .conceal_clips,
        Some(false)
    );
    assert!(db.set_content_type_archived("payment_card", true).is_err());

    let custom_type = db
        .create_content_type(&crate::content_types::ContentTypeInput {
            id: "ticket_id".into(),
            label: "Ticket ID".into(),
            icon: "Hash".into(),
            group: "custom".into(),
            conceal_clips: false,
        })
        .unwrap();
    assert!(custom_type.defaults.is_none());
    let classifier = db
        .create_content_classifier(&crate::content_classification::ClassifierInput {
            name: "Tickets".into(),
            content_type: "ticket_id".into(),
            description: String::new(),
            patterns: vec![r"^T-[0-9]+$".into()],
            validator: None,
            enabled: true,
            priority: 5,
        })
        .unwrap();
    db.set_content_type_archived("ticket_id", true).unwrap();
    assert!(db
        .get_content_types(false)
        .unwrap()
        .iter()
        .all(|item| item.id != "ticket_id"));
    assert!(
        !db.get_content_classifiers()
            .unwrap()
            .into_iter()
            .find(|item| item.id == classifier.id)
            .unwrap()
            .enabled
    );

    db.restore_default_content_types().unwrap();
    let restored_payment = db
        .get_content_types(false)
        .unwrap()
        .into_iter()
        .find(|item| item.id == "payment_card")
        .unwrap();
    assert_eq!(restored_payment.label, "Payment Card");
    assert_eq!(restored_payment.conceal_clips, Some(true));
}

#[test]
fn content_type_groups_are_editable_but_cannot_be_archived_while_in_use() {
    let db = setup_test_db();
    let general = db
        .get_content_type_groups(false)
        .unwrap()
        .into_iter()
        .find(|group| group.id == "general")
        .unwrap();
    assert_eq!(
        general
            .defaults
            .as_ref()
            .map(|defaults| defaults.label.as_str()),
        Some("General")
    );
    let custom_group = db
        .create_content_type_group(&crate::content_types::ContentTypeGroupInput {
            id: "work".into(),
            label: "Work".into(),
            sort_order: 15,
        })
        .unwrap();
    assert!(custom_group.defaults.is_none());
    db.create_content_type(&crate::content_types::ContentTypeInput {
        id: "ticket".into(),
        label: "Ticket".into(),
        icon: "Tag".into(),
        group: "work".into(),
        conceal_clips: false,
    })
    .unwrap();
    assert!(db.set_content_type_group_archived("work", true).is_err());
    db.update_content_type(
        "ticket",
        &crate::content_types::ContentTypeInput {
            id: "ticket".into(),
            label: "Ticket".into(),
            icon: "Tag".into(),
            group: "custom".into(),
            conceal_clips: false,
        },
    )
    .unwrap();
    db.set_content_type_group_archived("work", true).unwrap();
    assert!(db
        .get_content_type_groups(false)
        .unwrap()
        .iter()
        .all(|group| group.id != "work"));
    assert!(db.set_content_type_group_archived("general", true).is_err());
    let destination = setup_test_db();
    destination
        .import_backup_json(&db.export_backup_json().unwrap())
        .unwrap();
    assert!(destination
        .get_content_type_groups(true)
        .unwrap()
        .iter()
        .any(|group| group.id == "work" && group.is_archived));
    db.delete_content_type_group("work").unwrap();
    assert!(db
        .get_content_type_groups(true)
        .unwrap()
        .iter()
        .all(|group| group.id != "work"));
    assert!(db.delete_content_type_group("general").is_err());
}

#[test]
fn content_classification_rescan_reclassifies_text_but_preserves_structural_types() {
    let db = setup_test_db();
    let card = db
        .save_clip(
            "text",
            Some("4242-4242-4242-4242"),
            None,
            None,
            "card-hash",
            "Test",
        )
        .unwrap();
    let image = db
        .save_clip(
            "image",
            Some("4242-4242-4242-4242"),
            None,
            Some(crate::resource_limits::TEST_PNG_DATA_URL),
            "image-hash",
            "Test",
        )
        .unwrap();
    let empty = db
        .save_clip("code", Some(""), None, None, "empty-hash", "Test")
        .unwrap();
    let whitespace = db
        .save_clip("code", Some(" \n\t"), None, None, "whitespace-hash", "Test")
        .unwrap();

    let report = db.rescan_content_classification().unwrap();
    assert_eq!(report.scanned_count, 4);
    assert_eq!(report.changed_count, 2);
    assert_eq!(report.unchanged_count, 0);
    assert_eq!(report.failed_count, 2);
    assert_eq!(db.get_clip_by_id(card.id).unwrap().content_type, "text");
    assert_eq!(
        db.get_clip_by_id(card.id).unwrap().content_types,
        vec!["payment_card"]
    );
    assert_eq!(db.get_clip_by_id(image.id).unwrap().content_type, "image");
    assert_eq!(
        db.get_clip_by_id(image.id).unwrap().content_types,
        vec!["payment_card"]
    );
    assert_eq!(db.get_clip_by_id(empty.id).unwrap().content_type, "text");
    assert_eq!(
        db.get_clip_by_id(whitespace.id).unwrap().content_type,
        "text"
    );
}

#[test]
fn file_format_rescan_reports_missing_external_references() {
    let db = setup_test_db();
    let workspace = crate::external_tools::PrivateWorkspace::create("missing-format").unwrap();
    let missing_path = workspace.join("moved.png");
    let payload =
        serde_json::to_string(&vec![missing_path.to_string_lossy().into_owned()]).unwrap();
    db.save_clip(
        "file",
        Some(&payload),
        None,
        None,
        "missing-format-hash",
        "Finder",
    )
    .unwrap();

    let report = db.rescan_file_formats().unwrap();
    assert_eq!(report.scanned_count, 1);
    assert_eq!(report.changed_count, 0);
    assert_eq!(report.unchanged_count, 0);
    assert_eq!(report.missing_count, 1);
    assert_eq!(report.failed_count, 0);
}

#[test]
fn legacy_semantic_clip_types_become_preserved_content_type_matches() {
    let db = setup_test_db();
    let clip = db
        .save_clip(
            "link",
            Some("https://example.com"),
            None,
            None,
            "legacy-link-hash",
            "Test",
        )
        .unwrap();
    {
        let conn = db.conn.lock();
        migrate_legacy_semantic_clip_types(&conn).unwrap();
    }

    let migrated = db.get_clip_by_id(clip.id).unwrap();
    assert_eq!(migrated.content_type, "text");
    assert_eq!(migrated.content_types, vec!["link"]);
    let matches = db.get_analysis_classifications(clip.id).unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].classifier_ref, "url");
    assert_eq!(matches[0].start_offset, None);
}

#[test]
fn legacy_source_app_column_migrates_without_losing_filters_or_search() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("pasted_source_migration_{nanos}.db"));
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE clips (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    content_type TEXT NOT NULL,
                    text_content TEXT,
                    html_content TEXT,
                    image_base64 TEXT,
                    content_hash TEXT UNIQUE NOT NULL,
                    source_app TEXT DEFAULT 'Unknown',
                    is_pinned INTEGER DEFAULT 0,
                    bin_id INTEGER,
                    note TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                 );
                 CREATE TABLE bins (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    icon TEXT DEFAULT 'Folder',
                    color TEXT DEFAULT 'default',
                    smart_rule TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                 );
                 INSERT INTO clips
                    (content_type, text_content, content_hash, source_app)
                 VALUES ('text', 'migration-search-token', 'legacy-source-hash', 'Safari');
                 INSERT INTO bins (name, smart_rule)
                 VALUES ('Safari', '{\"type\":\"source_app\",\"value\":\"Safari\"}');",
        )
        .unwrap();
    drop(connection);

    let db = DbState::new(path).unwrap();
    let conn = db.conn.lock();
    assert!(column_exists(&conn, "clips", "source").unwrap());
    assert!(!column_exists(&conn, "clips", "source_app").unwrap());
    let migrated_rule: String = conn
        .query_row(
            "SELECT smart_rule FROM bins WHERE name = 'Safari'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(migrated_rule, r#"{"type":"source","value":"Safari"}"#);
    drop(conn);

    let clips = search_test_clips(&db, "migration-search-token");
    assert_eq!(clips.len(), 1);
    assert_eq!(clips[0].source, "Safari");
    assert_eq!(db.get_clips(Some(1), false).unwrap().len(), 1);

    let backup = db.export_backup_json().unwrap();
    assert!(backup.contains("\"source\": \"Safari\""));
    assert!(!backup.contains("\"source_app\""));

    let mut legacy_backup: serde_json::Value = serde_json::from_str(&backup).unwrap();
    for clip in legacy_backup["clips"].as_array_mut().unwrap() {
        let object = clip.as_object_mut().unwrap();
        let source = object.remove("source").unwrap();
        object.insert("source_app".to_string(), source);
    }
    let destination = setup_test_db();
    destination
        .import_backup_json(&serde_json::to_string(&legacy_backup).unwrap())
        .unwrap();
    assert!(destination
        .get_clips(None, false)
        .unwrap()
        .iter()
        .any(|clip| clip.source == "Safari"));
}

#[test]
fn legacy_classification_preferences_migrate_once_into_classifier_records() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("pasted_classifier_migration_{nanos}.db"));
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO settings (key, value) VALUES ('detectColors', 'false');",
        )
        .unwrap();
    drop(connection);

    let db = DbState::new(path).unwrap();
    let classifiers = db.get_content_classifiers().unwrap();
    assert!(
        !classifiers
            .iter()
            .find(|classifier| classifier.stable_ref == "color")
            .unwrap()
            .enabled
    );
    assert!(
        classifiers
            .iter()
            .find(|classifier| classifier.stable_ref == "url")
            .unwrap()
            .enabled
    );

    let color = classifiers
        .iter()
        .find(|classifier| classifier.stable_ref == "color")
        .unwrap();
    db.update_content_classifier(
        color.id,
        &crate::content_classification::ClassifierInput {
            name: color.name.clone(),
            content_type: color.content_type.clone(),
            description: color.description.clone(),
            patterns: color.patterns.clone(),
            validator: color.validator.clone(),
            enabled: true,
            priority: color.priority,
        },
    )
    .unwrap();
    let reopened = DbState::new(db.database_path()).unwrap();
    assert!(
        reopened
            .get_content_classifiers()
            .unwrap()
            .iter()
            .find(|classifier| classifier.stable_ref == "color")
            .unwrap()
            .enabled
    );
}

#[test]
fn legacy_analysis_terminology_migrates_without_losing_classifier_configuration() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("pasted_analysis_terms_{nanos}.db"));
    let connection = Connection::open(&path).unwrap();
    connection
            .execute_batch(
                "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 INSERT INTO settings (key, value) VALUES ('enableContentDetection', 'false');
                 CREATE TABLE schema_migrations (key TEXT PRIMARY KEY, applied_at DATETIME DEFAULT CURRENT_TIMESTAMP);
                 INSERT INTO schema_migrations (key) VALUES ('contentDetectorRegistryV1');
                 CREATE TABLE content_detectors (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    stable_ref TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    content_type TEXT NOT NULL,
                    description TEXT NOT NULL DEFAULT '',
                    patterns_json TEXT NOT NULL,
                    validator TEXT,
                    enabled INTEGER NOT NULL DEFAULT 1,
                    priority INTEGER NOT NULL DEFAULT 100,
                    is_builtin INTEGER NOT NULL DEFAULT 0,
                    is_deleted INTEGER NOT NULL DEFAULT 0,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
                 );
                 INSERT INTO content_detectors
                    (stable_ref, name, content_type, patterns_json, enabled, priority)
                 VALUES ('custom:legacy-classifier', 'Legacy Classifier', 'prose', '[\"legacy\"]', 0, 42);
                 CREATE TABLE content_classifiers (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    stable_ref TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    content_type TEXT NOT NULL,
                    description TEXT NOT NULL DEFAULT '',
                    patterns_json TEXT NOT NULL,
                    validator TEXT,
                    enabled INTEGER NOT NULL DEFAULT 1,
                    priority INTEGER NOT NULL DEFAULT 100,
                    is_builtin INTEGER NOT NULL DEFAULT 0,
                    is_deleted INTEGER NOT NULL DEFAULT 0,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
                 );
                 INSERT INTO content_classifiers
                    (stable_ref, name, content_type, patterns_json, enabled, priority)
                 VALUES ('custom:current-classifier', 'Current Classifier', 'prose', '[\"current\"]', 1, 41);",
            )
            .unwrap();
    drop(connection);

    let db = DbState::new(path).unwrap();
    let classifiers = db.get_content_classifiers().unwrap();
    let migrated = classifiers
        .iter()
        .find(|classifier| classifier.stable_ref == "custom:legacy-classifier")
        .unwrap();
    assert_eq!(migrated.name, "Legacy Classifier");
    assert!(!migrated.enabled);
    assert_eq!(migrated.priority, 42);
    assert!(classifiers
        .iter()
        .any(|classifier| classifier.stable_ref == "custom:current-classifier"));
    assert_eq!(
        db.get_setting("enableContentClassification")
            .unwrap()
            .as_deref(),
        Some("false")
    );
    assert_eq!(db.get_setting("enableContentDetection").unwrap(), None);

    let conn = db.conn.lock();
    assert!(table_exists(&conn, "content_classifiers").unwrap());
    assert!(!table_exists(&conn, "content_detectors").unwrap());
    let migrated_key: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE key = 'contentClassifierRegistryV1')",
                [],
                |row| row.get(0),
            )
            .unwrap();
    assert!(migrated_key);
}
