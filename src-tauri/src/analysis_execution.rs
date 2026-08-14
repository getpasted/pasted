use crate::content_analysis::AnalysisReport;
use crate::content_extraction::Extractor;
use crate::db::DbState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageAnalysisPersistence {
    pub ocr_updated: bool,
    pub classification_updated: bool,
}

pub fn persist_image_analysis(
    db: &DbState,
    clip_id: i64,
    content_hash: &str,
    extractor: &Extractor,
    classification_enabled: bool,
    analysis: &AnalysisReport,
) -> rusqlite::Result<ImageAnalysisPersistence> {
    let extraction_error = analysis
        .failure_for(&extractor.stable_ref)
        .map(|failure| failure.code.as_str());
    let ocr_updated = db.complete_or_reset_ocr_attempt(
        clip_id,
        content_hash,
        analysis.context.searchable_text.as_deref(),
        &extractor.engine,
        extraction_error,
    )?;
    if !ocr_updated {
        return Ok(ImageAnalysisPersistence {
            ocr_updated: false,
            classification_updated: false,
        });
    }

    let classification_updated =
        if classification_enabled && analysis.context.searchable_text.is_some() {
            db.record_analysis_classification(
                clip_id,
                content_hash,
                analysis.context.detected_type.as_deref(),
                analysis.context.matched_detector_ref.as_deref(),
                "searchable_text",
            )
            .unwrap_or(false)
        } else {
            false
        };

    Ok(ImageAnalysisPersistence {
        ocr_updated,
        classification_updated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content_analysis::{AnalysisContext, ParticipantRun};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn setup_test_db() -> DbState {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        DbState::new(std::env::temp_dir().join(format!("pasted_analysis_execution_{nonce}.db")))
            .unwrap()
    }

    fn extractor() -> Extractor {
        Extractor {
            id: 1,
            stable_ref: "extractor:test".into(),
            name: "Test OCR".into(),
            description: String::new(),
            engine: "test-engine-v1".into(),
            input_contract: "image".into(),
            output_contract: "searchable_text".into(),
            enabled: true,
            priority: 10,
            is_builtin: false,
            is_available: true,
            unavailable_reason: None,
            defaults: None,
        }
    }

    #[test]
    fn shared_persistence_applies_ocr_and_derived_classification() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "image",
                None,
                None,
                Some(crate::resource_limits::TEST_PNG_DATA_URL),
                "shared-analysis-persistence",
                "Screenshot",
            )
            .unwrap();
        assert!(db.force_ocr_running(clip.id, &clip.content_hash).unwrap());
        let analysis = AnalysisReport {
            context: AnalysisContext {
                clip_kind: "image".into(),
                original_text: None,
                image_bytes: None,
                searchable_text: Some("agent@example.com".into()),
                detected_type: Some("email".into()),
                matched_detector_ref: Some("detector:email".into()),
            },
            runs: Vec::<ParticipantRun>::new(),
        };

        let persisted = persist_image_analysis(
            &db,
            clip.id,
            &clip.content_hash,
            &extractor(),
            true,
            &analysis,
        )
        .unwrap();

        assert_eq!(
            persisted,
            ImageAnalysisPersistence {
                ocr_updated: true,
                classification_updated: true,
            }
        );
        assert_eq!(
            db.get_clip_by_id(clip.id).unwrap().text_content.as_deref(),
            Some("agent@example.com")
        );
        let classification = db.get_analysis_classification(clip.id).unwrap().unwrap();
        assert_eq!(classification.content_type, "email");
        assert_eq!(classification.detector_ref, "detector:email");
        assert_eq!(classification.source_representation, "searchable_text");
    }

    #[test]
    fn derived_classification_failure_does_not_fail_ocr_completion() {
        let db = setup_test_db();
        let clip = db
            .save_clip(
                "image",
                None,
                None,
                Some(crate::resource_limits::TEST_PNG_DATA_URL),
                "best-effort-classification",
                "Screenshot",
            )
            .unwrap();
        assert!(db.force_ocr_running(clip.id, &clip.content_hash).unwrap());
        let analysis = AnalysisReport {
            context: AnalysisContext {
                clip_kind: "image".into(),
                original_text: None,
                image_bytes: None,
                searchable_text: Some("recognized text".into()),
                detected_type: Some("x".repeat(81)),
                matched_detector_ref: Some("detector:test".into()),
            },
            runs: Vec::<ParticipantRun>::new(),
        };

        let persisted = persist_image_analysis(
            &db,
            clip.id,
            &clip.content_hash,
            &extractor(),
            true,
            &analysis,
        )
        .unwrap();

        assert_eq!(
            persisted,
            ImageAnalysisPersistence {
                ocr_updated: true,
                classification_updated: false,
            }
        );
        assert_eq!(
            db.get_clip_by_id(clip.id).unwrap().text_content.as_deref(),
            Some("recognized text")
        );
        assert!(db.get_analysis_classification(clip.id).unwrap().is_none());
    }
}
