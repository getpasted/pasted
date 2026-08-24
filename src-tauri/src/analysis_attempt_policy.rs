use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};

use crate::content_analysis::ExtractionObservation;
use crate::content_extraction::{ExtractionOutcome, Extractor};
use crate::db::{AnalysisFailureClass, ExtractionAttemptContext, StoredExtractionAttempt};

const FINGERPRINT_VERSION: &str = "analysis-attempt-v1";
const MAX_RETRY_SECONDS: i64 = 300;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReuseAction {
    Run,
    Reuse,
    Defer,
}

fn hash_field(hasher: &mut Sha256, value: impl AsRef<[u8]>) {
    hasher.update(value.as_ref());
    hasher.update([0]);
}

fn hash_extractor(hasher: &mut Sha256, extractor: &Extractor) {
    hash_field(hasher, FINGERPRINT_VERSION);
    hash_field(hasher, &extractor.stable_ref);
    hash_field(hasher, extractor.revision.to_le_bytes());
    hash_field(hasher, &extractor.recipe_hash);
    hash_field(hasher, &extractor.engine);
    hash_field(hasher, extractor.executable_path.as_deref().unwrap_or(""));
    hash_field(hasher, extractor.model_path.as_deref().unwrap_or(""));
    hash_field(hasher, extractor.priority.to_le_bytes());
    hash_field(
        hasher,
        serde_json::to_vec(&extractor.runtime).unwrap_or_default(),
    );
}

fn finish_context(extractor: &Extractor, hasher: Sha256) -> ExtractionAttemptContext {
    ExtractionAttemptContext {
        participant_ref: extractor.stable_ref.clone(),
        input_fingerprint: format!("{:x}", hasher.finalize()),
    }
}

pub fn image_contexts(bytes: &[u8], extractors: &[Extractor]) -> Vec<ExtractionAttemptContext> {
    extractors
        .iter()
        .map(|extractor| {
            let mut hasher = Sha256::new();
            hash_extractor(&mut hasher, extractor);
            hash_field(&mut hasher, bytes);
            finish_context(extractor, hasher)
        })
        .collect()
}

pub fn file_contexts(paths: &[String], extractors: &[Extractor]) -> Vec<ExtractionAttemptContext> {
    extractors
        .iter()
        .map(|extractor| {
            let mut hasher = Sha256::new();
            hash_extractor(&mut hasher, extractor);
            for path in paths {
                hash_field(&mut hasher, path);
                match std::fs::metadata(path) {
                    Ok(metadata) => {
                        hash_field(&mut hasher, b"available");
                        hash_field(&mut hasher, metadata.len().to_le_bytes());
                        hash_field(&mut hasher, [metadata.is_file() as u8]);
                        let modified = metadata
                            .modified()
                            .ok()
                            .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|value| value.as_nanos())
                            .unwrap_or_default();
                        hash_field(&mut hasher, modified.to_le_bytes());
                    }
                    Err(error) => hash_field(&mut hasher, format!("missing:{:?}", error.kind())),
                }
            }
            finish_context(extractor, hasher)
        })
        .collect()
}

pub fn legacy_contexts(
    content_hash: &str,
    observations: &[ExtractionObservation],
) -> Vec<ExtractionAttemptContext> {
    observations
        .iter()
        .map(|observation| {
            let mut hasher = Sha256::new();
            hash_field(&mut hasher, "legacy-analysis-attempt-v1");
            hash_field(&mut hasher, content_hash);
            hash_field(&mut hasher, &observation.extractor_ref);
            hash_field(&mut hasher, &observation.engine);
            ExtractionAttemptContext {
                participant_ref: observation.extractor_ref.clone(),
                input_fingerprint: format!("{:x}", hasher.finalize()),
            }
        })
        .collect()
}

pub fn failure_class(outcome: &ExtractionOutcome) -> Option<AnalysisFailureClass> {
    let ExtractionOutcome::Failed { failure } = outcome else {
        return None;
    };
    Some(match failure.code.as_str() {
        "engine_unavailable" | "engine_not_installed" | "preparation_unavailable" => {
            AnalysisFailureClass::Dependency
        }
        "invalid_contract"
        | "invalid_recipe"
        | "invalid_image_data"
        | "missing_input"
        | "missing_participant"
        | "output_too_large"
        | "unsupported_input" => AnalysisFailureClass::Terminal,
        _ => AnalysisFailureClass::Transient,
    })
}

pub fn retry_after(run_at: &str, consecutive_failures: usize) -> Option<String> {
    let run_at = DateTime::parse_from_rfc3339(run_at)
        .ok()?
        .with_timezone(&Utc);
    let exponent = consecutive_failures.saturating_sub(1).min(6) as u32;
    let seconds = (5_i64.saturating_mul(2_i64.pow(exponent))).min(MAX_RETRY_SECONDS);
    Some((run_at + Duration::seconds(seconds)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}

pub fn reuse_action(
    attempt: &StoredExtractionAttempt,
    manual: bool,
    now: DateTime<Utc>,
) -> ReuseAction {
    if manual {
        return ReuseAction::Run;
    }
    match (&attempt.observation.outcome, &attempt.failure_class) {
        (ExtractionOutcome::Produced { .. } | ExtractionOutcome::NoOutput, _) => ReuseAction::Reuse,
        (ExtractionOutcome::Failed { .. }, Some(AnalysisFailureClass::Terminal))
        | (ExtractionOutcome::Failed { .. }, Some(AnalysisFailureClass::Dependency)) => {
            ReuseAction::Reuse
        }
        (ExtractionOutcome::Failed { .. }, Some(AnalysisFailureClass::Transient)) => {
            if attempt
                .retry_after
                .as_deref()
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                .is_some_and(|retry| retry.with_timezone(&Utc) > now)
            {
                ReuseAction::Defer
            } else {
                ReuseAction::Run
            }
        }
        _ => ReuseAction::Run,
    }
}

#[cfg(test)]
mod tests;
