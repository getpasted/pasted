use super::{ExtractionFailure, ExtractionOutcome};

pub(super) fn normalize(outcome: ExtractionOutcome) -> ExtractionOutcome {
    match outcome {
        ExtractionOutcome::Produced { text } if text.trim().is_empty() => {
            ExtractionOutcome::NoOutput
        }
        ExtractionOutcome::Produced { text }
            if text.len() > crate::resource_limits::MAX_OCR_TEXT_BYTES =>
        {
            ExtractionOutcome::Failed {
                failure: ExtractionFailure {
                    code: "output_too_large".into(),
                    message: "Extracted text exceeds the supported size limit.".into(),
                },
            }
        }
        outcome => outcome,
    }
}
