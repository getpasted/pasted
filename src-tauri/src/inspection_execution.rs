use crate::analysis_contract::{
    AnalysisEnvelope, AnalysisFailure, AnalysisPass, AnalysisPolicy, AnalysisTargetKind,
    ClipApplication, ParticipantOutcome, ParticipantRun, ANALYSIS_CONTRACT_VERSION,
};
use crate::content_analysis::{AnalysisInput, AnalysisRequest};
use crate::content_inspection::{
    FileObservations, InspectionResult, MediaMetadata, StructuralMetadata, STRUCTURE_INSPECTOR_REF,
};
use crate::db::{ClipItem, DbState};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipInspectionResult {
    #[serde(flatten)]
    pub analysis: InspectionResult,
    #[serde(flatten)]
    pub application: ClipApplication,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_file_observations: Option<FileObservations>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_metadata: Option<MediaMetadata>,
}

fn completed_envelope(metadata: StructuralMetadata, policy: AnalysisPolicy) -> InspectionResult {
    AnalysisEnvelope::new(
        policy,
        metadata,
        vec![ParticipantRun {
            stable_ref: STRUCTURE_INSPECTOR_REF.into(),
            pass: AnalysisPass::Inspect,
            outcome: ParticipantOutcome::Produced,
            failure: None,
        }],
    )
}

fn completed_file_envelope(
    metadata: StructuralMetadata,
    policy: AnalysisPolicy,
    paths: &[String],
) -> (InspectionResult, Option<MediaMetadata>) {
    let mut result = completed_envelope(metadata, policy);
    if policy != AnalysisPolicy::Interactive {
        return (result, None);
    }
    let (outcome, failure, media_metadata) =
        match crate::content_inspection::inspect_media_paths(paths) {
            Ok(Some(media)) => (ParticipantOutcome::Produced, None, Some(media)),
            Ok(None) => (ParticipantOutcome::NoOutput, None, None),
            Err(failure) => (ParticipantOutcome::Failed, Some(failure), None),
        };
    result.participants.push(ParticipantRun {
        stable_ref: crate::content_inspection::MEDIA_INSPECTOR_REF.into(),
        pass: AnalysisPass::Inspect,
        outcome,
        failure,
    });
    (result, media_metadata)
}

fn inspect_with_media(
    input: AnalysisInput,
    policy: AnalysisPolicy,
) -> Result<(InspectionResult, Option<MediaMetadata>), AnalysisFailure> {
    let source_within_limit = match &input {
        AnalysisInput::Text { source, .. }
        | AnalysisInput::Image { source, .. }
        | AnalysisInput::Files { source, .. } => source.as_ref().is_none_or(|source| {
            source.len() <= crate::analysis_contract::MAX_ANALYSIS_SOURCE_BYTES
        }),
    };
    if !source_within_limit {
        return Err(AnalysisFailure {
            code: "input_too_large".into(),
            message: "Inspection source metadata exceeds the supported safety limit.".into(),
        });
    }
    let within_limit = match &input {
        AnalysisInput::Text { text, .. } => {
            text.len() <= crate::resource_limits::MAX_CLIP_TEXT_BYTES
        }
        AnalysisInput::Image { image_bytes, .. } => {
            image_bytes.len() <= crate::resource_limits::MAX_ENCODED_IMAGE_BYTES
        }
        AnalysisInput::Files { paths, .. } => crate::resource_limits::file_list_within_limit(paths),
    };
    if !within_limit {
        return Err(AnalysisFailure {
            code: "input_too_large".into(),
            message: "Inspection input exceeds the supported safety limit.".into(),
        });
    }
    let report = crate::content_analysis::analyze(AnalysisRequest {
        input,
        policy,
        inspector: true,
        extractor: None,
        detectors: None,
        enricher: None,
    });
    let resolution =
        report.resolve_participant(STRUCTURE_INSPECTOR_REF, AnalysisTargetKind::Inspector);
    if let Some(failure) = resolution.failure {
        return Err(failure);
    }
    let metadata = report
        .context
        .structural_metadata
        .ok_or_else(|| AnalysisFailure {
            code: "missing_output".into(),
            message: "Inspection completed without structural metadata.".into(),
        })?;
    Ok((
        AnalysisEnvelope::new(policy, metadata, report.runs),
        report.context.media_metadata,
    ))
}

fn inspect(
    input: AnalysisInput,
    policy: AnalysisPolicy,
) -> Result<InspectionResult, AnalysisFailure> {
    inspect_with_media(input, policy).map(|(analysis, _)| analysis)
}

pub fn inspect_text(text: &str, source: Option<&str>) -> Result<InspectionResult, AnalysisFailure> {
    inspect_text_with_policy(text, source, AnalysisPolicy::Interactive)
}

pub(crate) fn inspect_text_with_policy(
    text: &str,
    source: Option<&str>,
    policy: AnalysisPolicy,
) -> Result<InspectionResult, AnalysisFailure> {
    inspect(
        AnalysisInput::Text {
            text: text.into(),
            source: source.map(str::to_owned),
        },
        policy,
    )
}

pub fn inspect_image(
    image_bytes: Vec<u8>,
    source: Option<&str>,
) -> Result<InspectionResult, AnalysisFailure> {
    inspect_image_with_policy(image_bytes, source, AnalysisPolicy::Interactive)
}

pub(crate) fn inspect_image_with_policy(
    image_bytes: Vec<u8>,
    source: Option<&str>,
    policy: AnalysisPolicy,
) -> Result<InspectionResult, AnalysisFailure> {
    inspect(
        AnalysisInput::Image {
            image_bytes,
            searchable_text: None,
            source: source.map(str::to_owned),
        },
        policy,
    )
}

pub fn inspect_files(
    paths: Vec<String>,
    source: Option<&str>,
) -> Result<InspectionResult, AnalysisFailure> {
    inspect_files_with_policy(paths, source, AnalysisPolicy::Interactive)
}

pub(crate) fn inspect_files_with_policy(
    paths: Vec<String>,
    source: Option<&str>,
    policy: AnalysisPolicy,
) -> Result<InspectionResult, AnalysisFailure> {
    inspect(
        AnalysisInput::Files {
            paths,
            source: source.map(str::to_owned),
        },
        policy,
    )
}

pub(crate) fn inspection_input_hash(clip: &ClipItem) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"pasted-structural-inspection-v1\0");
    hasher.update(clip.content_type.as_bytes());
    hasher.update([0]);
    hasher.update(clip.source.as_bytes());
    hasher.update([0]);
    if clip.content_type == "image" {
        hasher.update(clip.content_hash.as_bytes());
    } else if let Some(text) = clip.text_content.as_deref() {
        hasher.update(text.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn analyze_clip(
    clip: &ClipItem,
    policy: AnalysisPolicy,
) -> Result<(InspectionResult, Option<Vec<String>>, Option<MediaMetadata>), String> {
    match clip.content_type.as_str() {
        "image" => {
            let bytes = clip
                .image_base64
                .as_deref()
                .and_then(crate::ocr::decode_stored_image)
                .ok_or_else(|| "Clip has no inspectable image data".to_string())?;
            inspect_image_with_policy(bytes, Some(&clip.source), policy)
                .map(|analysis| (analysis, None, None))
                .map_err(|failure| failure.message)
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
            inspect_with_media(
                AnalysisInput::Files {
                    paths: paths.clone(),
                    source: Some(clip.source.clone()),
                },
                policy,
            )
            .map(|(analysis, media)| (analysis, Some(paths), media))
            .map_err(|failure| failure.message)
        }
        _ => {
            let text = clip
                .text_content
                .as_deref()
                .ok_or_else(|| "Clip has no inspectable text".to_string())?;
            inspect_text_with_policy(text, Some(&clip.source), policy)
                .map(|analysis| (analysis, None, None))
                .map_err(|failure| failure.message)
        }
    }
}

pub fn inspect_clip(
    db: &DbState,
    clip_id: i64,
    apply: bool,
) -> rusqlite::Result<ClipInspectionResult> {
    inspect_clip_with_policy(db, clip_id, apply, AnalysisPolicy::Interactive)
}

pub(crate) fn inspect_clip_with_policy(
    db: &DbState,
    clip_id: i64,
    apply: bool,
    policy: AnalysisPolicy,
) -> rusqlite::Result<ClipInspectionResult> {
    let clip = db.get_clip_by_id(clip_id)?;
    let input_hash = inspection_input_hash(&clip);
    let cached = db.get_structural_inspection(clip_id, &input_hash)?;
    let (analysis, file_paths, media_metadata) = if let Some(metadata) = cached {
        let paths = (clip.content_type == "file").then(|| {
            clip.text_content
                .as_deref()
                .map(crate::content_inspection::parse_file_paths)
                .unwrap_or_default()
        });
        let (analysis, media_metadata) = match paths.as_deref() {
            Some(paths) => completed_file_envelope(metadata, policy, paths),
            None => (completed_envelope(metadata, policy), None),
        };
        (analysis, paths, media_metadata)
    } else {
        analyze_clip(&clip, policy).map_err(rusqlite::Error::InvalidParameterName)?
    };
    debug_assert_eq!(analysis.metadata.format_version, ANALYSIS_CONTRACT_VERSION);
    let applied = if apply {
        db.record_structural_inspection(clip.id, &clip.content_hash, &input_hash, &analysis.result)?
    } else {
        false
    };
    let live_file_observations = file_paths
        .as_deref()
        .map(crate::content_inspection::observe_files);
    Ok(ClipInspectionResult {
        analysis,
        application: if applied {
            ClipApplication::applied(clip.id)
        } else {
            ClipApplication::preview()
        },
        live_file_observations,
        media_metadata,
    })
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
            "pasted-inspection-execution-{}-{nanos}-{sequence}.db",
            std::process::id()
        )))
        .unwrap()
    }

    #[test]
    fn interactive_text_matches_the_public_json_fixture() {
        let analysis = inspect_text("alpha beta\ngamma", Some("Pasted CLI")).unwrap();
        let result = ClipInspectionResult {
            analysis,
            application: ClipApplication::preview(),
            live_file_observations: None,
            media_metadata: None,
        };
        let expected = serde_json::from_str::<serde_json::Value>(include_str!(
            "../../contracts/analysis/v1/inspector-interactive-text.json"
        ))
        .unwrap();
        assert_eq!(serde_json::to_value(result).unwrap(), expected);
    }

    #[test]
    fn oversized_text_and_source_return_the_same_bounded_failure() {
        let text = "x".repeat(crate::resource_limits::MAX_CLIP_TEXT_BYTES + 1);
        let text_failure = inspect_text(&text, None).unwrap_err();
        assert_eq!(text_failure.code, "input_too_large");

        let source = "x".repeat(crate::analysis_contract::MAX_ANALYSIS_SOURCE_BYTES + 1);
        let source_failure = inspect_text("hello", Some(&source)).unwrap_err();
        assert_eq!(source_failure.code, "input_too_large");
        assert!(!source_failure.message.contains(&source));
    }

    #[test]
    fn captures_persist_inspection_and_previews_remain_non_mutating() {
        let db = db();
        let clip = db
            .save_clip(
                "text",
                Some("alpha beta"),
                None,
                None,
                "inspection-test-hash",
                "Pasted CLI",
            )
            .unwrap();

        let preview = inspect_clip(&db, clip.id, false).unwrap();
        assert_eq!(preview.application, ClipApplication::preview());
        assert_eq!(preview.analysis.result.text.unwrap().word_count, 2);
        assert!(db
            .get_structural_inspection(clip.id, &inspection_input_hash(&clip))
            .unwrap()
            .is_some());

        let applied = inspect_clip(&db, clip.id, true).unwrap();
        assert_eq!(applied.application, ClipApplication::applied(clip.id));
        assert!(db
            .get_structural_inspection(clip.id, &inspection_input_hash(&clip))
            .unwrap()
            .is_some());
    }

    #[test]
    fn persisted_results_are_invalidated_by_structural_input_changes() {
        let db = db();
        let clip = db
            .save_clip(
                "text",
                Some("one"),
                None,
                None,
                "inspection-edit-hash",
                "Safari",
            )
            .unwrap();
        inspect_clip(&db, clip.id, true).unwrap();
        db.update_clip_text(clip.id, "one two").unwrap();
        let refreshed = inspect_clip(&db, clip.id, false).unwrap();
        assert_eq!(refreshed.analysis.result.text.unwrap().word_count, 2);
    }
}
