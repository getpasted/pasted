use super::super::*;

#[test]
fn transfer_roundtrip_preserves_completed_ocr_lifecycle_state() {
    let source = setup_test_db();
    let clip = source
        .save_clip(
            "image",
            None,
            None,
            Some(crate::resource_limits::TEST_PNG_DATA_URL),
            "ocr-backup-hash",
            "Screenshot",
        )
        .unwrap();
    assert!(source
        .complete_ocr_attempt_with_extractor(
            clip.id,
            "ocr-backup-hash",
            Some("Recovered words"),
            OcrExtractorProvenance::identified(
                "vision-test-v1",
                "extractor:test-vision",
                "Test Vision OCR",
            ),
            None,
        )
        .unwrap());

    let backup = source.export_backup_json().unwrap();
    let destination = setup_test_db();
    assert_eq!(destination.import_backup_json(&backup).unwrap(), 1);

    let status = destination.get_ocr_backfill_status().unwrap();
    assert_eq!(status.total_images, 1);
    assert_eq!(status.completed_count, 1);
    assert_eq!(status.eligible_count, 0);

    let restored_payload: BackupPayload =
        serde_json::from_str(&destination.export_backup_json().unwrap()).unwrap();
    assert_eq!(restored_payload.ocr_metadata.len(), 1);
    assert_eq!(restored_payload.ocr_metadata[0].status, "complete");
    assert_eq!(
        restored_payload.ocr_metadata[0].engine_version.as_deref(),
        Some("vision-test-v1")
    );
    assert_eq!(
        restored_payload.ocr_metadata[0].extractor_ref.as_deref(),
        Some("extractor:test-vision")
    );
    assert_eq!(
        restored_payload.ocr_metadata[0].extractor_name.as_deref(),
        Some("Test Vision OCR")
    );
}
