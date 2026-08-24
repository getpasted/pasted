use super::{DbState, IntelligenceConnectionUpdate};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn policy_reset_disables_and_reorders_without_discarding_connection_details() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("pasted_intelligence_reset_{nonce}.db"));
    let db = DbState::new(path.clone()).unwrap();
    let first = db
        .create_intelligence_connection(
            "Remote",
            "openai_compatible",
            Some("https://example.test"),
            Some("model"),
            Some("env:TEST_KEY"),
        )
        .unwrap();
    let second = db
        .create_intelligence_connection(
            "Local",
            "ollama",
            Some("http://127.0.0.1:11434"),
            None,
            None,
        )
        .unwrap();
    db.update_intelligence_connection(IntelligenceConnectionUpdate {
        id: &first.id,
        name: &first.name,
        provider_kind: &first.provider_kind,
        endpoint: first.endpoint.as_deref(),
        model: first.model.as_deref(),
        credential_ref: first.credential_ref.as_deref(),
        enabled: true,
    })
    .unwrap();

    let reset = db
        .reset_intelligence_connection_policy(&[(
            "ollama".into(),
            Some("http://127.0.0.1:11434".into()),
        )])
        .unwrap();
    assert_eq!(
        reset
            .iter()
            .map(|connection| connection.id.as_str())
            .collect::<Vec<_>>(),
        vec![second.id.as_str(), first.id.as_str()]
    );
    assert!(reset.iter().all(|connection| !connection.enabled));
    let remote = reset
        .iter()
        .find(|connection| connection.name == "Remote")
        .unwrap();
    assert_eq!(remote.model.as_deref(), Some("model"));
    assert_eq!(remote.credential_ref.as_deref(), Some("env:TEST_KEY"));
    drop(db);
    let _ = std::fs::remove_file(path);
}
