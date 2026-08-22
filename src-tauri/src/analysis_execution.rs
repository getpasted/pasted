use crate::analysis_contract::{AnalysisEnvelope, AnalysisPolicy, ClipApplication};
use crate::content_analysis::{
    AnalysisInput, AnalysisRequest, ExtractorParticipantSource, SuggestionParticipantSource,
};
use crate::content_inspection::{FileFormatInspection, FileObservations, StructuralMetadata};
use crate::content_suggestions::SmartActionSuggestions;
use crate::db::DbState;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnalyzerOptions {
    pub policy: AnalysisPolicy,
    pub include_extractor: bool,
    pub include_classifiers: bool,
    pub include_suggestions: bool,
}

impl Default for AnalyzerOptions {
    fn default() -> Self {
        Self {
            policy: AnalysisPolicy::Interactive,
            include_extractor: false,
            include_classifiers: true,
            include_suggestions: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzerSnapshot {
    pub clip_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structure: Option<StructuralMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_formats: Option<FileFormatInspection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_metadata: Option<crate::content_inspection::MediaMetadata>,
    pub classification_matches: Vec<crate::content_classification::ClassificationMatch>,
    pub searchable_text_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestions: Option<SmartActionSuggestions>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzerPreview {
    #[serde(flatten)]
    pub analysis: AnalysisEnvelope<AnalyzerSnapshot>,
    #[serde(flatten)]
    pub application: ClipApplication,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_file_observations: Option<FileObservations>,
}

fn validate_text_input(text: &str, source: Option<&str>) -> Result<(), String> {
    if text.len() > crate::resource_limits::MAX_CLIP_TEXT_BYTES {
        return Err("Analysis input exceeds Pasted's safety limit".into());
    }
    if source.is_some_and(|value| value.len() > crate::analysis_contract::MAX_ANALYSIS_SOURCE_BYTES)
    {
        return Err("Analysis source metadata exceeds Pasted's safety limit".into());
    }
    Ok(())
}

fn execute(
    db: &DbState,
    input: AnalysisInput,
    clip_kind: &str,
    options: AnalyzerOptions,
    allow_text_participants: bool,
) -> Result<AnalyzerPreview, String> {
    let run_classifiers = allow_text_participants && options.include_classifiers;
    let classifiers = if run_classifiers {
        db.get_content_classifiers()
            .map_err(|error| error.to_string())?
            .into_iter()
            .filter(|classifier| classifier.enabled)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let transforms = if allow_text_participants
        && options.include_suggestions
        && options
            .policy
            .includes(crate::analysis_contract::AnalysisPass::Suggest)
    {
        db.get_transform_definitions()
            .map_err(|error| error.to_string())?
    } else {
        Vec::new()
    };
    let extractors = if options.include_extractor {
        match clip_kind {
            "image" => db.active_image_text_extractors_for_features(crate::features::is_enabled(
                db,
                crate::features::Feature::Ocr,
            )),
            "file" => db.active_file_text_extractors_for_features(
                crate::features::is_enabled(db, crate::features::Feature::Ocr),
                crate::features::is_enabled(db, crate::features::Feature::Transcriptions),
            ),
            _ => Ok(Vec::new()),
        }
        .map_err(|error| error.to_string())?
    } else {
        Vec::new()
    };
    let registry = crate::content_extraction::system_engine_registry();
    let extractor_sources = extractors
        .iter()
        .map(|extractor| ExtractorParticipantSource {
            extractor,
            registry: &registry,
        })
        .collect();
    let report = crate::content_analysis::analyze(AnalysisRequest {
        input,
        policy: options.policy,
        inspector: true,
        file_format_inspector: clip_kind == "file"
            && (options.include_extractor
                || crate::features::is_enabled(db, crate::features::Feature::FileFormats)),
        extractors: extractor_sources,
        classifiers: run_classifiers.then_some(classifiers.as_slice()),
        suggestion: (allow_text_participants && options.include_suggestions).then_some(
            SuggestionParticipantSource {
                transforms: transforms.as_slice(),
            },
        ),
    });
    let snapshot = AnalyzerSnapshot {
        clip_kind: report.context.clip_kind.clone(),
        structure: report.context.structural_metadata,
        file_formats: report.context.file_formats,
        media_metadata: report.context.media_metadata,
        classification_matches: report.context.classification_matches,
        searchable_text_available: report.context.searchable_text.is_some(),
        suggestions: report.context.suggestions,
    };
    Ok(AnalyzerPreview {
        analysis: AnalysisEnvelope::new(options.policy, snapshot, report.runs),
        application: ClipApplication::preview(),
        live_file_observations: None,
    })
}

pub fn analyze_text(
    db: &DbState,
    text: &str,
    source: Option<&str>,
    options: AnalyzerOptions,
) -> Result<AnalyzerPreview, String> {
    validate_text_input(text, source)?;
    execute(
        db,
        AnalysisInput::Text {
            text: text.into(),
            source: source.map(str::to_owned),
        },
        "text",
        options,
        true,
    )
}

pub fn analyze_clip(
    db: &DbState,
    clip_id: i64,
    options: AnalyzerOptions,
) -> Result<AnalyzerPreview, String> {
    let clip = db
        .get_clip_by_id(clip_id)
        .map_err(|error| error.to_string())?;
    validate_text_input("", Some(&clip.source))?;
    match clip.content_type.as_str() {
        "image" => {
            let image_bytes = clip
                .image_base64
                .as_deref()
                .and_then(crate::ocr::decode_stored_image)
                .ok_or_else(|| "Clip has no analyzable image data".to_string())?;
            let allow_text_participants = options.include_extractor;
            execute(
                db,
                AnalysisInput::Image {
                    image_bytes,
                    searchable_text: None,
                    source: Some(clip.source),
                },
                "image",
                options,
                allow_text_participants,
            )
        }
        "file" => {
            let paths = clip
                .text_content
                .as_deref()
                .map(crate::content_inspection::parse_file_paths)
                .filter(|paths| !paths.is_empty())
                .ok_or_else(|| "File clip has no valid path metadata".to_string())?;
            if !crate::resource_limits::file_list_within_limit(&paths) {
                return Err("File list exceeds Pasted's safety limit".into());
            }
            let observations = crate::content_inspection::observe_files(&paths);
            let allow_text_participants = options.include_extractor;
            let mut result = execute(
                db,
                AnalysisInput::Files {
                    paths,
                    source: Some(clip.source),
                },
                "file",
                options,
                allow_text_participants,
            )?;
            result.live_file_observations = Some(observations);
            Ok(result)
        }
        _ => {
            let text = clip
                .text_content
                .as_deref()
                .ok_or_else(|| "Clip has no analyzable text".to_string())?;
            analyze_text(db, text, Some(&clip.source), options)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_DATABASE: AtomicU64 = AtomicU64::new(0);

    fn db() -> DbState {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = NEXT_DATABASE.fetch_add(1, Ordering::Relaxed);
        DbState::new(std::env::temp_dir().join(format!(
            "pasted-analysis-execution-{}-{nanos}-{sequence}.db",
            std::process::id()
        )))
        .unwrap()
    }

    #[test]
    fn interactive_text_runs_one_ordered_control_plane() {
        let result = analyze_text(
            &db(),
            "agent@example.com",
            Some("Pasted CLI"),
            AnalyzerOptions::default(),
        )
        .unwrap();
        assert_eq!(
            result.analysis.result.classification_matches[0].content_type,
            "email"
        );
        assert!(result.analysis.result.structure.is_some());
        assert!(result.analysis.result.suggestions.is_some());
        assert_eq!(result.analysis.participants.len(), 3);
        assert_eq!(
            result.analysis.participants[0].pass,
            crate::analysis_contract::AnalysisPass::Inspect
        );
        assert_eq!(
            result.analysis.participants[1].pass,
            crate::analysis_contract::AnalysisPass::Classify
        );
        assert_eq!(
            result.analysis.participants[2].pass,
            crate::analysis_contract::AnalysisPass::Suggest
        );
    }

    #[test]
    fn interactive_text_matches_the_public_json_fixture() {
        let result = analyze_text(
            &db(),
            "ordinary words",
            Some("Pasted CLI"),
            AnalyzerOptions::default(),
        )
        .unwrap();
        let expected = serde_json::from_str::<serde_json::Value>(include_str!(
            "../../contracts/analysis/v1/analyzer-interactive-text.json"
        ))
        .unwrap();
        assert_eq!(serde_json::to_value(result).unwrap(), expected);
    }

    #[test]
    fn capture_policy_omits_suggestion() {
        let result = analyze_text(
            &db(),
            "ordinary words",
            Some("Pasted CLI"),
            AnalyzerOptions {
                policy: AnalysisPolicy::Capture,
                ..AnalyzerOptions::default()
            },
        )
        .unwrap();
        assert!(result.analysis.result.suggestions.is_none());
        assert_eq!(result.analysis.participants.len(), 2);
        let expected = serde_json::from_str::<serde_json::Value>(include_str!(
            "../../contracts/analysis/v1/analyzer-capture-text.json"
        ))
        .unwrap();
        assert_eq!(serde_json::to_value(result).unwrap(), expected);
    }

    #[test]
    fn inspection_only_preview_skips_unused_text_passes() {
        let result = analyze_text(
            &db(),
            "agent@example.com",
            None,
            AnalyzerOptions {
                include_classifiers: false,
                include_suggestions: false,
                ..AnalyzerOptions::default()
            },
        )
        .unwrap();
        assert!(result.analysis.result.classification_matches.is_empty());
        assert!(result.analysis.result.suggestions.is_none());
        assert_eq!(result.analysis.participants.len(), 1);
        assert_eq!(
            result.analysis.participants[0].pass,
            crate::analysis_contract::AnalysisPass::Inspect
        );
    }

    #[test]
    fn suggestion_does_not_implicitly_enable_classifiers() {
        let result = analyze_text(
            &db(),
            "agent@example.com",
            None,
            AnalyzerOptions {
                include_classifiers: false,
                include_suggestions: true,
                ..AnalyzerOptions::default()
            },
        )
        .unwrap();
        assert!(result.analysis.result.classification_matches.is_empty());
        assert!(result.analysis.result.suggestions.is_some());
        assert_eq!(result.analysis.participants.len(), 2);
        assert!(result
            .analysis
            .participants
            .iter()
            .all(|run| run.pass != crate::analysis_contract::AnalysisPass::Classify));
    }

    #[test]
    fn file_paths_never_reach_text_participants_or_serialized_results() {
        let db = db();
        let secret_path = "/private/secret/customer.txt";
        let clip = db
            .save_clip(
                "file",
                Some(&serde_json::to_string(&[secret_path]).unwrap()),
                None,
                None,
                "analyzer-file-test",
                "Finder",
            )
            .unwrap();
        let result = analyze_clip(&db, clip.id, AnalyzerOptions::default()).unwrap();
        assert_eq!(result.analysis.participants.len(), 3);
        assert!(result
            .analysis
            .participants
            .iter()
            .any(|run| { run.stable_ref == crate::content_inspection::FILE_FORMAT_INSPECTOR_REF }));
        assert!(result
            .analysis
            .participants
            .iter()
            .any(|run| run.stable_ref == crate::content_inspection::MEDIA_INSPECTOR_REF));
        assert!(result.analysis.result.classification_matches.is_empty());
        assert!(result.analysis.result.suggestions.is_none());
        assert!(!serde_json::to_string(&result)
            .unwrap()
            .contains(secret_path));
    }

    #[test]
    fn serialized_snapshot_never_contains_analyzed_text() {
        let secret = "private-token-123";
        let result = analyze_text(&db(), secret, None, AnalyzerOptions::default()).unwrap();
        assert!(!serde_json::to_string(&result).unwrap().contains(secret));
    }

    #[test]
    fn oversized_text_and_source_fail_before_analysis() {
        let db = db();
        let text = "x".repeat(crate::resource_limits::MAX_CLIP_TEXT_BYTES + 1);
        assert!(analyze_text(&db, &text, None, AnalyzerOptions::default())
            .unwrap_err()
            .contains("safety limit"));

        let source = "x".repeat(crate::analysis_contract::MAX_ANALYSIS_SOURCE_BYTES + 1);
        assert!(
            analyze_text(&db, "hello", Some(&source), AnalyzerOptions::default())
                .unwrap_err()
                .contains("source metadata")
        );
    }
}
