use super::super::*;

mod expectations;
mod identity_migrations;

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
    let visual_labels = extractors
        .iter()
        .find(|extractor| {
            extractor.stable_ref == crate::content_extraction::APPLE_VISION_LABELS_REF
        })
        .unwrap();
    assert_eq!(visual_labels.is_available, cfg!(target_os = "macos"));
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
    expectations::assert_active_image_extractor(
        active
            .as_ref()
            .map(|extractor| extractor.stable_ref.as_str()),
        tesseract.is_available,
    );
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

    let matching = save_plain_test_clip(
        &db,
        "text",
        "person@example.com",
        "classifier-apply-match",
        "Test",
    );
    let applied = db.apply_content_classifier(matching.id, "email").unwrap();
    assert!(applied.analysis.matched);
    assert_eq!(applied.application.applied_clip_id, Some(matching.id));
    assert_eq!(db.get_clip_by_id(matching.id).unwrap().content_type, "text");
    assert_eq!(
        db.get_clip_by_id(matching.id).unwrap().content_types,
        vec!["email"]
    );

    let nonmatching = save_plain_test_clip(
        &db,
        "text",
        "plain prose",
        "classifier-apply-no-match",
        "Test",
    );
    let not_applied = db
        .apply_content_classifier(nonmatching.id, "email")
        .unwrap();
    assert!(!not_applied.analysis.matched);
    assert_eq!(not_applied.application.applied_clip_id, None);
    assert_eq!(
        db.get_clip_by_id(nonmatching.id).unwrap().content_type,
        "text"
    );

    let empty = save_plain_test_clip(&db, "text", "", "classifier-apply-empty", "Test");
    assert!(db
        .apply_content_classifier(empty.id, "email")
        .unwrap_err()
        .to_string()
        .contains("no analyzable text"));
    let whitespace =
        save_plain_test_clip(&db, "text", " \n\t", "classifier-apply-whitespace", "Test");
    assert!(db
        .apply_content_classifier(whitespace.id, "email")
        .unwrap_err()
        .to_string()
        .contains("no analyzable text"));

    db.delete_content_classifier(duplicate.id).unwrap();
    assert!(db.get_content_classifier(&duplicate.stable_ref).is_err());
}
