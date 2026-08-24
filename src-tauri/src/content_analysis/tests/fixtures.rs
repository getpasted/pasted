use super::*;

pub(super) struct StaticEngine {
    pub(super) id: &'static str,
    pub(super) outcome: ExtractionOutcome,
}

pub(super) fn engine(outcome: ExtractionOutcome) -> StaticEngine {
    StaticEngine {
        id: "test-v1",
        outcome,
    }
}

pub(super) fn failing_engine() -> StaticEngine {
    engine(ExtractionOutcome::Failed {
        failure: crate::content_extraction::ExtractionFailure {
            code: "test_failure".into(),
            message: "The test engine failed.".into(),
        },
    })
}

impl crate::content_extraction::ExtractorEngine for StaticEngine {
    fn id(&self) -> &'static str {
        self.id
    }

    fn availability(&self) -> crate::content_extraction::EngineAvailability {
        crate::content_extraction::EngineAvailability {
            is_available: true,
            unavailable_reason: None,
        }
    }

    fn extract(&self, _image_bytes: &[u8]) -> ExtractionOutcome {
        self.outcome.clone()
    }
}

pub(super) fn classifier(pattern: &str, content_type: &str) -> Classifier {
    Classifier {
        id: 1,
        stable_ref: format!("test:{content_type}"),
        name: content_type.into(),
        content_type: content_type.into(),
        description: String::new(),
        patterns: vec![pattern.into()],
        validator: None,
        enabled: true,
        priority: 10,
        is_builtin: false,
        defaults: None,
        is_deleted: false,
    }
}

pub(super) fn extractor() -> Extractor {
    Extractor {
        id: 1,
        stable_ref: "extractor:test".into(),
        name: "Test OCR".into(),
        description: String::new(),
        engine: "test-v1".into(),
        executable_path: None,
        model_path: None,
        input_contract: "image".into(),
        output_contract: "searchable_text".into(),
        enabled: true,
        priority: 10,
        revision: 1,
        is_builtin: false,
        is_available: true,
        unavailable_reason: None,
        runtime: crate::content_extraction::runtime_status_for("test-v1", None),
        recipe: crate::content_extraction::test_recipe("image"),
        recipe_hash: "test".into(),
        default_recipe: None,
        defaults: None,
    }
}

pub(super) fn analyze_test_image(
    image_bytes: Vec<u8>,
    extractor: &Extractor,
    classifiers: Option<&[Classifier]>,
    registry: &ExtractorEngineRegistry<'_>,
) -> AnalysisReport {
    analyze(AnalysisRequest {
        input: AnalysisInput::Image {
            image_bytes,
            searchable_text: None,
            source: None,
        },
        policy: AnalysisPolicy::Interactive,
        inspector: false,
        file_format_inspector: false,
        extractors: vec![ExtractorParticipantSource {
            extractor,
            registry,
        }],
        classifiers,
        suggestion: None,
    })
}

pub(super) fn analyze_test_text(text: &str, classifiers: &[Classifier]) -> AnalysisReport {
    analyze(AnalysisRequest {
        input: AnalysisInput::Text {
            text: text.into(),
            source: None,
        },
        policy: AnalysisPolicy::Capture,
        inspector: false,
        file_format_inspector: false,
        extractors: Vec::new(),
        classifiers: Some(classifiers),
        suggestion: None,
    })
}
