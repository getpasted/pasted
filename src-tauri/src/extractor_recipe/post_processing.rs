use serde::{Deserialize, Serialize};

use crate::content_extraction::VisualLabel;

pub const DEFAULT_LABEL_CONFIDENCE_PERCENT: u8 = 80;
const MAX_POST_PROCESSING_OPERATIONS: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExtractorPostProcessing {
    FilterLabelsByConfidence {
        #[serde(rename = "minimumPercent")]
        minimum_percent: u8,
    },
}

impl ExtractorPostProcessing {
    const fn stable_name(&self) -> &'static str {
        match self {
            Self::FilterLabelsByConfidence { .. } => "filter_labels_by_confidence",
        }
    }
}

pub(super) fn validate(operations: &[ExtractorPostProcessing]) -> Result<(), String> {
    if operations.len() > MAX_POST_PROCESSING_OPERATIONS {
        return Err(format!(
            "Extractor recipes support up to {MAX_POST_PROCESSING_OPERATIONS} post-processing operations"
        ));
    }
    let mut kinds = std::collections::HashSet::new();
    for operation in operations {
        if !kinds.insert(operation.stable_name()) {
            return Err("Extractor post-processing operations must be unique".into());
        }
        match operation {
            ExtractorPostProcessing::FilterLabelsByConfidence { minimum_percent }
                if *minimum_percent > 100 =>
            {
                return Err("Minimum label confidence must be between 0 and 100".into());
            }
            ExtractorPostProcessing::FilterLabelsByConfidence { .. } => {}
        }
    }
    Ok(())
}

pub(super) fn apply(
    operations: &[ExtractorPostProcessing],
    labels: Vec<VisualLabel>,
) -> Vec<VisualLabel> {
    let mut labels = crate::content_extraction::normalize_visual_labels(labels);
    for operation in operations {
        match operation {
            ExtractorPostProcessing::FilterLabelsByConfidence { minimum_percent } => {
                retain_labels_at_or_above(&mut labels, *minimum_percent);
            }
        }
    }
    labels
}

fn retain_labels_at_or_above(labels: &mut Vec<VisualLabel>, minimum_percent: u8) {
    let minimum_basis_points = u16::from(minimum_percent) * 100;
    labels.retain(|label| {
        label
            .confidence_basis_points
            .is_none_or(|confidence| confidence >= minimum_basis_points)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels() -> Vec<VisualLabel> {
        vec![
            VisualLabel {
                value: "dog".into(),
                confidence_basis_points: Some(8_000),
            },
            VisualLabel {
                value: "terrier".into(),
                confidence_basis_points: Some(7_999),
            },
            VisualLabel {
                value: "favorite".into(),
                confidence_basis_points: None,
            },
        ]
    }

    #[test]
    fn declared_confidence_filter_is_available_to_any_recipe() {
        let accepted = apply(
            &[ExtractorPostProcessing::FilterLabelsByConfidence {
                minimum_percent: 80,
            }],
            labels(),
        );
        assert_eq!(
            accepted
                .iter()
                .map(|label| label.value.as_str())
                .collect::<Vec<_>>(),
            ["dog", "favorite"]
        );
    }

    #[test]
    fn recipes_without_a_filter_keep_scored_labels() {
        assert_eq!(apply(&[], labels()).len(), 3);
    }
}
