use super::super::*;

#[test]
fn transfer_import_rejects_unknown_schema_without_mutating_data() {
    let source = setup_test_db();
    save_plain_test_clip(&source, "text", "future data", "future-backup-item", "Test");
    let mut payload: serde_json::Value =
        serde_json::from_str(&source.export_backup_json().unwrap()).unwrap();
    payload["version"] = serde_json::json!(BACKUP_SCHEMA_VERSION + 1);

    let destination = setup_test_db();
    let error = destination
        .import_backup_json(&serde_json::to_string(&payload).unwrap())
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("unsupported transfer schema version"));
    assert!(destination.get_clips(None, false).unwrap().is_empty());
}

#[test]
fn transfer_import_rolls_back_earlier_writes_when_valid_payload_fails_midway() {
    let source = setup_test_db();
    source
        .create_bin("Imported Bin", "Folder", "default", None)
        .unwrap();
    source
        .create_operation(
            "Imported Operation",
            "uppercase",
            Some("{}"),
            Some("Import Test"),
        )
        .unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(&source.export_backup_json().unwrap()).unwrap();
    let custom_operation = payload["operations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|operation| {
            operation["stable_id"]
                .as_str()
                .is_some_and(|stable_id| stable_id.starts_with("custom:"))
        })
        .unwrap();
    custom_operation["stable_id"] = serde_json::json!("invalid-operation-reference");

    let destination = setup_test_db();
    let existing = save_plain_test_clip(
        &destination,
        "text",
        "Destination must survive",
        "backup-rollback-existing",
        "Test",
    );
    destination.save_setting("themeMode", "warm").unwrap();
    let bins_before = destination
        .get_bins()
        .unwrap()
        .into_iter()
        .map(|bin| (bin.id, bin.name))
        .collect::<Vec<_>>();

    let error = destination
        .import_backup_json(&serde_json::to_string(&payload).unwrap())
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("custom operation in transfer file is missing a stable reference"));
    assert_eq!(
        destination
            .get_clip_by_id(existing.id)
            .unwrap()
            .text_content
            .as_deref(),
        Some("Destination must survive")
    );
    assert_eq!(
        destination.get_setting("themeMode").unwrap().as_deref(),
        Some("warm")
    );
    assert_eq!(
        destination
            .get_bins()
            .unwrap()
            .into_iter()
            .map(|bin| (bin.id, bin.name))
            .collect::<Vec<_>>(),
        bins_before
    );
    assert!(!destination
        .get_operations()
        .unwrap()
        .iter()
        .any(|operation| operation.name == "Imported Operation"));
}
