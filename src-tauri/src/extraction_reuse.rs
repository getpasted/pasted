use chrono::Utc;

use crate::analysis_attempt_policy::ReuseAction;
use crate::analysis_contract::{
    AnalysisFailure, AnalysisMetadata, AnalysisPass, AnalysisPolicy, AnalysisTargetKind,
    ParticipantOutcome, ParticipantRun,
};
use crate::content_analysis::ExtractionObservation;
use crate::content_extraction::{ExtractionOutcome, Extractor, ExtractorEngineRegistry};
use crate::db::{DbState, ExtractionAttemptContext};
use crate::extraction_execution::{ExtractionResult, ExtractionResultOutcome};

pub(crate) fn analyze_background_image(
    db: &DbState,
    clip_id: i64,
    image_bytes: Vec<u8>,
    extractors: &[Extractor],
    classifiers: Option<&[crate::content_classification::Classifier]>,
    registry: &ExtractorEngineRegistry<'_>,
    manual: bool,
) -> ExtractionResult {
    let contexts = crate::analysis_attempt_policy::image_contexts(&image_bytes, extractors);
    let mut cached = Vec::new();
    let mut fresh_extractors = Vec::new();
    for (extractor, context) in extractors.iter().zip(&contexts) {
        let latest = if manual {
            None
        } else {
            db.get_latest_extraction_attempt(
                clip_id,
                &context.participant_ref,
                &context.input_fingerprint,
            )
            .ok()
            .flatten()
        };
        match latest
            .as_ref()
            .map(|attempt| crate::analysis_attempt_policy::reuse_action(attempt, false, Utc::now()))
        {
            Some(ReuseAction::Reuse | ReuseAction::Defer) => {
                cached.push(latest.expect("cached attempt exists").observation)
            }
            _ => fresh_extractors.push(extractor.clone()),
        }
    }

    let mut result = if fresh_extractors.is_empty() {
        cached_result(cached.clone(), classifiers)
    } else {
        crate::extraction_execution::analyze_images_with_registry_and_policy(
            image_bytes,
            &fresh_extractors,
            classifiers,
            registry,
            AnalysisPolicy::Background,
        )
    };
    if !cached.is_empty() && !fresh_extractors.is_empty() {
        result.observations.extend(cached);
        normalize_result(&mut result, classifiers);
    }
    result
}

fn cached_result(
    observations: Vec<ExtractionObservation>,
    classifiers: Option<&[crate::content_classification::Classifier]>,
) -> ExtractionResult {
    let mut result = ExtractionResult {
        metadata: AnalysisMetadata::new(AnalysisPolicy::Background),
        target_kind: AnalysisTargetKind::Extractor,
        target_ref: observations
            .last()
            .map(|observation| observation.extractor_ref.clone())
            .unwrap_or_default(),
        outcome: ExtractionResultOutcome::NoOutput,
        output: None,
        classification_matches: Vec::new(),
        failure: None,
        participants: observations.iter().map(cached_participant).collect(),
        observations,
        attempt_observations: Vec::new(),
        attempt_contexts: Vec::<ExtractionAttemptContext>::new(),
    };
    normalize_result(&mut result, classifiers);
    result
}

fn cached_participant(observation: &ExtractionObservation) -> ParticipantRun {
    let (outcome, failure) = match &observation.outcome {
        ExtractionOutcome::Produced { .. } => (ParticipantOutcome::Produced, None),
        ExtractionOutcome::NoOutput => (ParticipantOutcome::NoOutput, None),
        ExtractionOutcome::Failed { failure } => (
            ParticipantOutcome::Failed,
            Some(AnalysisFailure {
                code: failure.code.clone(),
                message: failure.message.clone(),
            }),
        ),
    };
    ParticipantRun {
        stable_ref: observation.extractor_ref.clone(),
        pass: AnalysisPass::Extract,
        outcome,
        failure,
    }
}

fn normalize_result(
    result: &mut ExtractionResult,
    classifiers: Option<&[crate::content_classification::Classifier]>,
) {
    result.observations.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.extractor_ref.cmp(&right.extractor_ref))
    });
    let selected = result
        .observations
        .iter()
        .rev()
        .find(|observation| matches!(observation.outcome, ExtractionOutcome::Produced { .. }))
        .or_else(|| result.observations.last());
    if let Some(selected) = selected {
        result.target_ref = selected.extractor_ref.clone();
        result.failure = match &selected.outcome {
            ExtractionOutcome::Failed { failure } => Some(AnalysisFailure {
                code: failure.code.clone(),
                message: failure.message.clone(),
            }),
            _ => None,
        };
    }
    let mut outputs = Vec::new();
    for observation in &result.observations {
        if let ExtractionOutcome::Produced { text, .. } = &observation.outcome {
            if !outputs.contains(text) {
                outputs.push(text.clone());
            }
        }
    }
    result.output = (!outputs.is_empty()).then(|| outputs.join("\n"));
    result.outcome = if result.output.is_some() {
        ExtractionResultOutcome::Produced
    } else if result.failure.is_some() {
        ExtractionResultOutcome::Failed
    } else {
        ExtractionResultOutcome::NoOutput
    };
    result.classification_matches = result
        .output
        .as_deref()
        .zip(classifiers)
        .map(|(text, classifiers)| {
            crate::content_classification::classify_with_classifiers(text, classifiers)
        })
        .unwrap_or_default();
}

#[cfg(test)]
mod tests;
