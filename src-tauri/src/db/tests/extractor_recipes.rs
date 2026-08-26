use super::*;

#[test]
fn extractor_recipes_preserve_multi_input_authoring_history() {
    let db = setup_test_db();
    let executable = std::env::current_exe().unwrap();
    let timestamp = "2026-08-17T12:34:56-05:00";
    let created = db
        .create_content_extractor_recipe(&crate::extractor_recipe::ExtractorRecipeDefinitionInput {
            name: "Portable text reader".into(),
            description: "Extracts text from supported local content.".into(),
            enabled: false,
            priority: 100,
            recipe: crate::extractor_recipe::ExtractorRecipe {
                definition_version: crate::extractor_recipe::EXTRACTOR_RECIPE_VERSION,
                accepts: vec![
                    crate::extractor_recipe::ExtractorInputKind::Image,
                    crate::extractor_recipe::ExtractorInputKind::FileReferences,
                ],
                accepted_file_formats: vec!["pdf".into(), "png".into()],
                minimum_visual_label_confidence:
                    crate::extractor_recipe::DEFAULT_MINIMUM_VISUAL_LABEL_CONFIDENCE,
                output: crate::extractor_recipe::ExtractorOutputKind::SearchableText,
                steps: vec![crate::extractor_recipe::ExtractorCommandStep {
                    id: "extract".into(),
                    executable: crate::extractor_recipe::ExtractorExecutable {
                        path: Some(executable.to_string_lossy().into_owned()),
                        discover: Vec::new(),
                        version_arguments: Vec::new(),
                    },
                    arguments: vec!["--pasted-extract-v1".into(), "{request.path}".into()],
                    mode: crate::extractor_recipe::ExtractorStepMode::Once,
                    capture: crate::extractor_recipe::ExtractorCapture::PastedJsonV1,
                    output_extension: None,
                    no_output_exit_codes: Vec::new(),
                    timeout_seconds: 60,
                }],
                resources: Vec::new(),
            },
            authoring: Some(crate::extractor_recipe::ExtractorAuthoringManifest {
                manifest_version: crate::extractor_recipe::EXTRACTOR_AUTHORING_VERSION,
                source: crate::extractor_recipe::ExtractorAuthoringSource::Ai,
                original_prompt: Some("Read text locally".into()),
                provider: Some("Test Provider".into()),
                model: Some("test-model".into()),
                messages: vec![crate::extractor_recipe::ExtractorAuthoringMessage {
                    role: crate::extractor_recipe::ExtractorAuthoringRole::User,
                    content: "Read text locally".into(),
                    created_at: timestamp.into(),
                    structured_content: None,
                }],
            }),
        })
        .unwrap();

    assert_eq!(created.engine, crate::content_extraction::RECIPE_ENGINE);
    assert!(created
        .recipe
        .accepts(crate::extractor_recipe::ExtractorInputKind::Image));
    assert!(created
        .recipe
        .accepts(crate::extractor_recipe::ExtractorInputKind::FileReferences));
    assert_eq!(created.recipe.accepted_file_formats, ["pdf", "png"]);
    let history = db
        .get_extractor_authoring_sessions(&created.stable_ref)
        .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(
        history[0].original_prompt.as_deref(),
        Some("Read text locally")
    );
    assert_eq!(history[0].messages[0].created_at, "2026-08-17T17:34:56Z");
    db.set_library_item_enabled("extractor", &created.stable_ref, true)
        .unwrap();
    assert!(db
        .active_image_text_extractors_for_features(true)
        .unwrap()
        .iter()
        .any(|extractor| extractor.stable_ref == created.stable_ref));
    assert!(db
        .active_file_text_extractors_for_features(true, true)
        .unwrap()
        .iter()
        .any(|extractor| extractor.stable_ref == created.stable_ref));
    let revision_count: i64 = db
        .conn
        .lock()
        .query_row(
            "SELECT COUNT(*) FROM extractor_recipe_revisions WHERE extractor_id = ?1",
            params![created.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(revision_count, 1);
}
