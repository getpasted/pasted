use super::super::support::*;

#[test]
fn smart_actions_have_registry_and_non_mutating_cli_parity() {
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
