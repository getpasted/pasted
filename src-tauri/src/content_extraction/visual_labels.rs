use serde::{Deserialize, Serialize};

pub const MAX_VISUAL_LABELS: usize = 64;
pub const MAX_VISUAL_LABEL_BYTES: usize = 120;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VisualLabel {
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_basis_points: Option<u16>,
}

impl super::ExtractionOutcome {
    pub fn produced(text: impl Into<String>) -> Self {
        Self::Produced {
            text: text.into(),
            labels: Vec::new(),
        }
    }
}

pub(crate) fn normalize(labels: Vec<VisualLabel>) -> Vec<VisualLabel> {
    let mut normalized = Vec::new();
    for mut label in labels {
        label.value = label.value.trim().to_string();
        if label.value.is_empty()
            || label.value.len() > MAX_VISUAL_LABEL_BYTES
            || label
                .confidence_basis_points
                .is_some_and(|value| value > 10_000)
            || normalized
                .iter()
                .any(|existing: &VisualLabel| existing.value.eq_ignore_ascii_case(&label.value))
        {
            continue;
        }
        normalized.push(label);
        if normalized.len() == MAX_VISUAL_LABELS {
            break;
        }
    }
    normalized
}

pub(crate) fn into_outcome(text: String, labels: Vec<VisualLabel>) -> super::ExtractionOutcome {
    let labels = normalize(labels);
    if text.trim().is_empty() && labels.is_empty() {
        super::ExtractionOutcome::NoOutput
    } else if text.len() > crate::resource_limits::MAX_OCR_TEXT_BYTES {
        super::ExtractionOutcome::Failed {
            failure: super::ExtractionFailure {
                code: "output_too_large".into(),
                message: "Extracted text exceeds the supported size limit.".into(),
            },
        }
    } else {
        super::ExtractionOutcome::Produced { text, labels }
    }
}

pub(crate) fn parse_json_fields(
    value: &serde_json::Value,
) -> Result<(Option<String>, Vec<VisualLabel>), String> {
    let text = match value.get("text") {
        Some(serde_json::Value::String(text)) if !text.trim().is_empty() => {
            Some(text.trim().to_string())
        }
        Some(serde_json::Value::String(_) | serde_json::Value::Null) | None => None,
        _ => return Err("Extractor output requires a string or null text field.".into()),
    };
    let labels = match value.get("labels") {
        Some(value) => serde_json::from_value::<Vec<VisualLabel>>(value.clone())
            .map(normalize)
            .map_err(|_| {
                "Extractor output labels require an array of Visual Labels.".to_string()
            })?,
        None => Vec::new(),
    };
    Ok((text, labels))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_bounds_and_deduplicates_labels() {
        let labels = normalize(vec![
            VisualLabel {
                value: " Dog ".into(),
                confidence_basis_points: Some(9_500),
            },
            VisualLabel {
                value: "dog".into(),
                confidence_basis_points: None,
            },
            VisualLabel {
                value: "cat".into(),
                confidence_basis_points: Some(10_001),
            },
        ]);

        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].value, "Dog");
    }

    #[test]
    fn old_text_only_results_remain_compatible() {
        let outcome: super::super::ExtractionOutcome =
            serde_json::from_str(r#"{"outcome":"produced","text":"legacy"}"#).unwrap();
        assert_eq!(outcome, super::super::ExtractionOutcome::produced("legacy"));
    }

    #[test]
    fn protocol_labels_remain_structured_metadata() {
        let value = serde_json::json!({
            "text": "A happy pet",
            "labels": [{ "value": "dog", "confidenceBasisPoints": 9600 }],
        });
        let (text, labels) = parse_json_fields(&value).unwrap();
        let outcome = into_outcome(text.unwrap(), labels);
        let super::super::ExtractionOutcome::Produced { text, labels } = outcome else {
            panic!("expected a produced result");
        };
        assert_eq!(text, "A happy pet");
        assert_eq!(labels[0].confidence_basis_points, Some(9_600));
    }
}
