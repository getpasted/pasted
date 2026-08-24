use super::support::*;

#[test]
fn structural_inspector_has_registry_preview_and_apply_parity() {
    let database = temporary_path("inspector", "db");
    let clip = success_json(&database, &["copy", "alpha beta\ngamma", "--json"]);
    let clip_id = clip["id"].as_i64().expect("clip ID");
    let clip_id_text = clip_id.to_string();

    let inspectors = success_json(&database, &["inspector", "list", "--json"]);
    assert_eq!(inspectors[0]["stableRef"], "inspector:structure-v1");
    assert_eq!(inspectors[0]["outputContract"], "structural_metadata");
    let media = inspectors
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|item| item["stableRef"] == "inspector:media-metadata-v1")
        })
        .expect("shipped Media Metadata Inspector");
    assert!(matches!(
        media["engine"].as_str(),
        Some("ffprobe-cli-v1" | "mediainfo-cli-v1")
    ));
    assert_eq!(media["inputContract"], "file_references");
    assert_eq!(media["outputContract"], "media_metadata");
    assert!(media["isAvailable"].is_boolean());
    let legacy_media = success_json(
        &database,
        &["inspector", "get", "inspector:ffprobe-media-v1", "--json"],
    );
    assert_eq!(legacy_media["stableRef"], "inspector:media-metadata-v1");

    let registry = success_json(
        &database,
        &["registry", "list", "--kind", "inspector", "--json"],
    );
    assert_eq!(registry[0]["analysisPass"], "inspect");
    assert_eq!(registry[0]["participantContract"]["pass"], "inspect");
    assert_eq!(
        registry[0]["participantContract"]["requires"][0],
        "clip_kind"
    );
    assert_eq!(registry[0]["capabilities"]["canDisable"], false);
    assert!(registry
        .as_array()
        .is_some_and(|items| items.iter().any(|item| {
            item["stableRef"] == "inspector:media-metadata-v1"
                && item["outputContract"] == "media_metadata"
                && item["typeRelations"][0]["kind"] == "accepts"
                && item["typeRelations"][0]["typeId"] == "file"
        })));

    let preview = success_json(
        &database,
        &["inspector", "run", "--clip", &clip_id_text, "--json"],
    );
    assert_eq!(preview["formatVersion"], 1);
    assert_eq!(preview["result"]["text"]["characterCount"], 16);
    assert_eq!(preview["result"]["text"]["wordCount"], 3);
    assert_eq!(preview["result"]["text"]["lineCount"], 2);
    assert_eq!(preview["appliedClipId"], Value::Null);
    assert!(!preview.to_string().contains("alpha beta"));
    assert_eq!(preview, analysis_fixture("inspector-interactive-text"));

    let unicode = success_json(
        &database,
        &["inspector", "run", "--text", "é 😀\n", "--json"],
    );
    assert_eq!(unicode["result"]["byteCount"], 8);
    assert_eq!(unicode["result"]["text"]["characterCount"], 4);
    assert_eq!(unicode["result"]["text"]["wordCount"], 2);
    assert_eq!(unicode["result"]["text"]["lineCount"], 1);

    let applied = success_json(
        &database,
        &[
            "inspector",
            "run",
            "--clip",
            &clip_id_text,
            "--apply",
            "--json",
        ],
    );
    assert_eq!(applied["appliedClipId"], clip_id);
    clean_database(&database);
}

#[test]
fn smart_actions_suggestion_has_registry_and_non_mutating_cli_parity() {
    let database = temporary_path("suggestion", "db");
    let transform = success_json(
        &database,
        &[
            "transform",
            "create",
            "--name",
            "Clean URL",
            "--steps-json",
            r#"[{"operationRef":"builtin:clean_url_tracking","configJson":null,"failurePolicy":"stop"}]"#,
            "--json",
        ],
    );
    let transform_ref = transform["stableRef"].as_str().expect("Transform ref");
    assert!(transform["createdAt"].as_str().unwrap().ends_with('Z'));
    assert!(transform["updatedAt"].as_str().unwrap().ends_with('Z'));
    let secret_url = "https://example.com/private-token-0123456789?utm_source=test";
    let clip = success_json(&database, &["copy", secret_url, "--json"]);
    let clip_id = clip["id"].as_i64().expect("clip ID").to_string();

    let suggestions = success_json(&database, &["suggestion", "list", "--json"]);
    assert_eq!(suggestions[0]["stableRef"], "suggestion:smart-actions-v1");
    assert_eq!(suggestions[0]["outputContract"], "suggestions");

    let registry = success_json(
        &database,
        &["registry", "list", "--kind", "suggestion", "--json"],
    );
    assert_eq!(registry[0]["analysisPass"], "suggest");
    assert_eq!(
        registry[0]["participantContract"]["requires"],
        serde_json::json!(["analyzable_text", "structural_metadata"])
    );
    assert_eq!(
        registry[0]["inputContract"],
        "analyzable_text+structural_metadata"
    );
    assert_eq!(registry[0]["capabilities"]["canDisable"], false);

    let result = success_json(
        &database,
        &["suggestion", "run", "--clip", &clip_id, "--json"],
    );
    assert_eq!(result["formatVersion"], 1);
    assert_eq!(result["policy"], "interactive");
    assert_eq!(result["through"], "suggest");
    assert_eq!(result["result"]["signals"][0], "url");
    assert_eq!(
        result["result"]["actions"][0]["transformRef"],
        transform_ref
    );
    assert_eq!(result["appliedClipId"], Value::Null);
    assert!(!result.to_string().contains("private-token-0123456789"));

    let empty = success_json(
        &database,
        &["suggestion", "run", "--text", "ordinary words", "--json"],
    );
    assert_eq!(empty, analysis_fixture("suggestion-interactive-empty"));
    clean_database(&database);
}

#[test]
fn whole_analyzer_has_one_versioned_privacy_safe_cli_contract() {
    let database = temporary_path("analyzer", "db");
    let secret = "agent@example.com private-token-0123456789";
    let interactive = success_json(&database, &["analyzer", "run", "--text", secret, "--json"]);
    assert_eq!(interactive["formatVersion"], 1);
    assert_eq!(interactive["policy"], "interactive");
    assert_eq!(interactive["through"], "suggest");
    assert_eq!(interactive["result"]["clipKind"], "text");
    assert_eq!(
        interactive["result"]["classificationMatches"][0]["contentType"],
        "email"
    );
    assert!(interactive["result"]["structure"].is_object());
    assert!(interactive["result"]["suggestions"].is_object());
    assert_eq!(interactive["participants"][0]["pass"], "inspect");
    assert_eq!(interactive["participants"][1]["pass"], "classify");
    assert_eq!(interactive["participants"][2]["pass"], "suggest");
    assert!(!interactive.to_string().contains("private-token-0123456789"));
    assert_eq!(
        success_json(
            &database,
            &["analyzer", "run", "--text", "ordinary words", "--json"],
        ),
        analysis_fixture("analyzer-interactive-text")
    );

    let capture = success_json(
        &database,
        &[
            "analyzer",
            "run",
            "--text",
            "ordinary words",
            "--policy",
            "capture",
            "--json",
        ],
    );
    assert_eq!(capture["through"], "classify");
    assert!(capture["result"].get("suggestions").is_none());
    assert_eq!(capture["participants"].as_array().map(Vec::len), Some(2));
    assert!(!capture.to_string().contains("ordinary words"));
    assert_eq!(capture, analysis_fixture("analyzer-capture-text"));
    clean_database(&database);
}

#[test]
fn classifier_preview_and_apply_share_the_safe_execution_contract() {
    let database = temporary_path("classifiers", "db");
    let clip = success_json(&database, &["copy", "ticket-123", "--json"]);
    let clip_id = clip["id"].as_i64().expect("clip ID");
    let clip_id_text = clip_id.to_string();
    let classifier = success_json(
        &database,
        &[
            "classifier",
            "create",
            "--name",
            "Ticket IDs",
            "--type",
            "code",
            "--regex",
            "^ticket-[0-9]+$",
            "--json",
        ],
    );
    let stable_ref = classifier["stable_ref"]
        .as_str()
        .expect("Classifier stable ref");
    let fetched = success_json(&database, &["classifier", "get", stable_ref, "--json"]);
    assert_eq!(fetched["name"], "Ticket IDs");
    let duplicate = success_json(
        &database,
        &[
            "classifier",
            "duplicate",
            stable_ref,
            "--name",
            "Ticket IDs Copy",
            "--json",
        ],
    );
    assert_eq!(duplicate["name"], "Ticket IDs Copy");

    let registry = success_json(
        &database,
        &["registry", "list", "--kind", "classifier", "--json"],
    );
    let registry_item = registry
        .as_array()
        .and_then(|items| items.iter().find(|item| item["stableRef"] == stable_ref))
        .expect("Classifier registry item");
    assert_eq!(registry_item["analysisPass"], "classify");
    assert_eq!(
        registry_item["participantContract"]["requires"],
        serde_json::json!(["analyzable_text"])
    );
    assert_eq!(registry_item["typeRelations"][0]["kind"], "classifies_as");
    assert_eq!(registry_item["typeRelations"][0]["typeId"], "code");
    assert_eq!(registry_item["capabilities"]["canDuplicate"], true);
    assert_eq!(registry_item["capabilities"]["canDelete"], true);

    success_json(
        &database,
        &[
            "registry",
            "disable",
            "--kind",
            "classifier",
            "--ref",
            stable_ref,
            "--json",
        ],
    );
    success_json(
        &database,
        &[
            "registry",
            "enable",
            "--kind",
            "classifier",
            "--ref",
            stable_ref,
            "--json",
        ],
    );
    let activity = success_json(&database, &["activity", "list", "--all", "--json"]);
    assert!(activity.as_array().is_some_and(|logs| {
        logs.iter()
            .any(|log| log["event_type"] == "content_classifier_disabled")
            && logs
                .iter()
                .any(|log| log["event_type"] == "content_classifier_enabled")
    }));

    let mut no_match = success_json(
        &database,
        &[
            "classifier",
            "run",
            stable_ref,
            "--text",
            "ordinary words",
            "--json",
        ],
    );
    assert_eq!(no_match["targetRef"], stable_ref);
    no_match["targetRef"] = Value::String("classifier:email".into());
    assert_eq!(
        no_match,
        analysis_fixture("classifier-interactive-no-match")
    );

    let preview = success_json(
        &database,
        &[
            "classifier",
            "run",
            stable_ref,
            "--text",
            "ticket-123",
            "--json",
        ],
    );
    assert_eq!(preview["formatVersion"], 1);
    assert_eq!(preview["policy"], "interactive");
    assert_eq!(preview["through"], "suggest");
    assert_eq!(preview["targetKind"], "classifier");
    assert_eq!(preview["targetRef"], stable_ref);
    assert_eq!(preview["outcome"], "matched");
    assert_eq!(preview["matched"], true);
    assert_eq!(preview["contentTypes"][0], "code");
    assert_eq!(preview["matches"][0]["classifierRef"], stable_ref);
    assert_eq!(preview["appliedClipId"], Value::Null);
    assert_eq!(preview["participants"][0]["pass"], "classify");
    assert!(!preview.to_string().contains("ticket-123"));

    let applied = success_json(
        &database,
        &[
            "classifier",
            "run",
            stable_ref,
            "--clip",
            &clip_id_text,
            "--apply",
            "--json",
        ],
    );
    assert_eq!(applied["outcome"], "matched");
    assert_eq!(applied["appliedClipId"], clip_id);

    let clips = success_json(&database, &["list", "--limit", "5", "--json"]);
    let updated = clips
        .as_array()
        .and_then(|items| items.iter().find(|item| item["id"] == clip_id))
        .expect("updated clip");
    assert_eq!(updated["content_type"], "text");
    assert_eq!(updated["content_types"][0], "code");

    let deleted = success_json(&database, &["classifier", "delete", stable_ref, "--json"]);
    assert_eq!(deleted["deleted"], true);
    clean_database(&database);
}
