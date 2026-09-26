use serde::{Deserialize, Serialize};

use crate::db::DbState;
pub mod analyzer;
pub mod parts;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartPasteError {
    pub code: String,
    pub message: String,
}

impl SmartPasteError {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartPasteContext {
    pub application: String,
    pub role: Option<String>,
    pub label: Option<String>,
    pub description: Option<String>,
    pub help: Option<String>,
    pub placeholder: Option<String>,
}

impl SmartPasteContext {
    pub fn has_field_hint(&self) -> bool {
        [
            self.label.as_deref(),
            self.description.as_deref(),
            self.help.as_deref(),
            self.placeholder.as_deref(),
        ]
        .into_iter()
        .flatten()
        .any(|value| !value.trim().is_empty())
    }

    pub fn is_verification_code_field(&self) -> bool {
        [
            self.role.as_deref(),
            self.label.as_deref(),
            self.description.as_deref(),
            self.help.as_deref(),
            self.placeholder.as_deref(),
        ]
        .into_iter()
        .flatten()
        .map(str::to_ascii_lowercase)
        .any(|value| {
            [
                "passcode",
                "one-time code",
                "verification code",
                "security code",
            ]
            .iter()
            .any(|marker| value.contains(marker))
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartPasteOutcome {
    pub value: String,
}

pub fn select_value(
    db: &DbState,
    source: &str,
    context: &SmartPasteContext,
) -> Result<SmartPasteOutcome, SmartPasteError> {
    crate::features::require(db, crate::features::Feature::Transformations)
        .map_err(|message| SmartPasteError::new("feature_disabled", message))?;
    if source.len() > crate::resource_limits::MAX_TRANSFORM_TEXT_BYTES {
        return Err(SmartPasteError::new(
            "smart_paste_input_too_large",
            "Smart Paste input exceeds Pasted's 8 MB safety limit",
        ));
    }
    if source.trim().is_empty() {
        return Err(SmartPasteError::new(
            "empty_clipboard",
            "Copy text before using Smart Paste",
        ));
    }
    if !context.has_field_hint() {
        return Err(SmartPasteError::new(
            "field_context_unavailable",
            "The focused field does not expose enough context for Smart Paste",
        ));
    }
    if context.is_verification_code_field() {
        return Err(SmartPasteError::new(
            "verification_code_field",
            "Smart Paste is unavailable for passcode and verification-code fields",
        ));
    }

    let content_hash = crate::clipboard_fingerprint::text(source);
    let analysis =
        db.get_paste_parts_by_hash(&content_hash)
            .map_err(|error| SmartPasteError::new("database_error", error.to_string()))?
            .unwrap_or_else(|| {
                let classifiers = db.get_content_classifiers().unwrap_or_default();
                parts::from_classifications(
                    &crate::content_classification::classify_with_classifiers(source, &classifiers),
                )
            });
    let part = parts::best_match(&analysis.parts, context).ok_or_else(|| {
        SmartPasteError::new(
            "no_smart_paste_match",
            "Smart Paste could not find a confident value for the focused field",
        )
    })?;
    let value = parts::value_at_offsets(source, part).ok_or_else(|| {
        SmartPasteError::new(
            "invalid_smart_paste_part",
            "The saved Smart Paste value no longer matches the copied text",
        )
    })?;
    Ok(SmartPasteOutcome {
        value: value.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_context_requires_a_semantic_hint() {
        let context = SmartPasteContext {
            application: "Browser".into(),
            role: Some("text field".into()),
            ..Default::default()
        };
        assert!(!context.has_field_hint());
        assert!(SmartPasteContext {
            label: Some("Email address".into()),
            ..context
        }
        .has_field_hint());
    }

    #[test]
    fn password_fields_are_allowed_but_verification_fields_are_detected() {
        assert!(!SmartPasteContext {
            application: "Browser".into(),
            role: Some("secure text field".into()),
            label: Some("Password".into()),
            ..Default::default()
        }
        .is_verification_code_field());
        assert!(SmartPasteContext {
            application: "Browser".into(),
            label: Some("One-time code".into()),
            ..Default::default()
        }
        .is_verification_code_field());
    }
}
