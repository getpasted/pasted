use super::*;

#[test]
fn portable_transfer_preserves_external_references_without_copying_external_state() {
    let db = setup_test_db();
    let workspace = crate::external_tools::PrivateWorkspace::create("transfer-boundaries").unwrap();
    let external_path = workspace.join("external-source.bin");
    let external_bytes = "external-byte-marker-not-owned-by-pasted";
    std::fs::write(&external_path, external_bytes).unwrap();
    let external_path = external_path.to_string_lossy().into_owned();
    let file_payload = serde_json::to_string(&vec![external_path.clone()]).unwrap();
    db.save_clip(
        "file",
        Some(&file_payload),
        None,
        None,
        "portable-external-file-reference",
        "Tests",
    )
    .unwrap();
    db.create_intelligence_connection(
        "Transfer-excluded connection marker",
        "openai_compatible",
        Some("http://127.0.0.1:1234/transfer-excluded"),
        Some("transfer-excluded-model"),
        Some("env:TRANSFER_EXCLUDED_CREDENTIAL"),
    )
    .unwrap();

    let transfer = db.export_backup_json().unwrap();
    let payload: BackupPayload = serde_json::from_str(&transfer).unwrap();
    assert_eq!(payload.clips.len(), 1);
    assert_eq!(
        payload.clips[0].text_content.as_deref(),
        Some(file_payload.as_str())
    );
    assert!(transfer.contains(&external_path));
    assert!(!transfer.contains(external_bytes));
    assert!(!transfer.contains("Transfer-excluded connection marker"));
    assert!(!transfer.contains("env:TRANSFER_EXCLUDED_CREDENTIAL"));
}
