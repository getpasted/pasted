use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::content_extraction::{EngineAvailability, ExtractionOutcome, ExtractorEngine};

struct CountingEngine(AtomicUsize);

impl ExtractorEngine for CountingEngine {
    fn id(&self) -> &'static str {
        "counting-v1"
    }

    fn availability(&self) -> EngineAvailability {
        EngineAvailability {
            is_available: true,
            unavailable_reason: None,
        }
    }

    fn extract(&self, _image_bytes: &[u8]) -> ExtractionOutcome {
        self.0.fetch_add(1, Ordering::SeqCst);
        ExtractionOutcome::Produced {
            text: "searchable".into(),
        }
    }
}

fn extractor() -> Extractor {
    Extractor {
        id: 1,
        stable_ref: "extractor:counting".into(),
        name: "Counting OCR".into(),
        description: String::new(),
        engine: "counting-v1".into(),
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
        runtime: crate::content_extraction::runtime_status_for("counting-v1", None),
        recipe: crate::content_extraction::test_recipe("image"),
        recipe_hash: "recipe-v1".into(),
        default_recipe: None,
        defaults: None,
    }
}

#[test]
fn background_reuses_identical_attempts_while_manual_and_changed_input_run() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db = DbState::new(std::env::temp_dir().join(format!("pasted_extraction_reuse_{nonce}.db")))
        .unwrap();
    let clip = db
        .save_clip("image", None, Some("image"), None, "reuse-clip", "Tests")
        .unwrap();
    let engine = CountingEngine(AtomicUsize::new(0));
    let engines: [&dyn ExtractorEngine; 1] = [&engine];
    let registry = ExtractorEngineRegistry::new(&engines);
    let extractors = vec![extractor()];
    let first = analyze_background_image(
        &db,
        clip.id,
        b"same".to_vec(),
        &extractors,
        None,
        &registry,
        false,
    );
    db.record_extraction_observations_with_context(
        clip.id,
        &clip.content_hash,
        &first.observations,
        &first.attempt_observations,
        &first.attempt_contexts,
    )
    .unwrap();
    let reused = analyze_background_image(
        &db,
        clip.id,
        b"same".to_vec(),
        &extractors,
        None,
        &registry,
        false,
    );
    assert!(reused.attempt_observations.is_empty());
    assert_eq!(reused.output.as_deref(), Some("searchable"));
    assert_eq!(engine.0.load(Ordering::SeqCst), 1);

    analyze_background_image(
        &db,
        clip.id,
        b"same".to_vec(),
        &extractors,
        None,
        &registry,
        true,
    );
    analyze_background_image(
        &db,
        clip.id,
        b"changed".to_vec(),
        &extractors,
        None,
        &registry,
        false,
    );
    assert_eq!(engine.0.load(Ordering::SeqCst), 3);
}
