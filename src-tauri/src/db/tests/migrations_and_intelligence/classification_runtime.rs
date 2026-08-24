use super::super::*;

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
    let card = save_plain_test_clip(&db, "text", "4242-4242-4242-4242", "card-hash", "Test");
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
    let empty = save_plain_test_clip(&db, "code", "", "empty-hash", "Test");
    let whitespace = save_plain_test_clip(&db, "code", " \n\t", "whitespace-hash", "Test");

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
