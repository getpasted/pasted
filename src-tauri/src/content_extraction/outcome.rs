use super::ExtractionOutcome;

pub(super) fn normalize(outcome: ExtractionOutcome) -> ExtractionOutcome {
    match outcome {
        ExtractionOutcome::Produced { text, labels } => {
            super::visual_labels::into_outcome(text, labels)
        }
        outcome => outcome,
    }
}
