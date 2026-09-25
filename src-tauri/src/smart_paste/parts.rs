use serde::{Deserialize, Serialize};

pub const PARTICIPANT_REF: &str = "analysis:paste_parts";
pub const FORMAT_VERSION: u32 = 1;
pub const MAX_PARTS_PER_CLIP: usize = 128;
pub const MAX_ALIASES_PER_PART: usize = 12;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PastePart {
    pub kind: String,
    pub role: Option<String>,
    pub aliases: Vec<String>,
    pub start_offset: usize,
    pub end_offset: usize,
    pub confidence: f32,
    pub analyzer_ref: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PastePartsAnalysis {
    pub format_version: u32,
    pub parts: Vec<PastePart>,
}

impl PastePartsAnalysis {
    pub fn empty() -> Self {
        Self {
            format_version: FORMAT_VERSION,
            parts: Vec::new(),
        }
    }
}

pub fn from_classifications(
    matches: &[crate::content_classification::ClassificationMatch],
) -> PastePartsAnalysis {
    let mut parts = matches
        .iter()
        .map(|matched| PastePart {
            kind: matched.content_type.clone(),
            role: None,
            aliases: vec![
                matched.content_type.replace('_', " "),
                matched.classifier_name.clone(),
            ],
            start_offset: matched.start_offset,
            end_offset: matched.end_offset,
            confidence: 1.0,
            analyzer_ref: matched.classifier_ref.clone(),
        })
        .collect::<Vec<_>>();
    parts.sort_by(|left, right| {
        left.start_offset
            .cmp(&right.start_offset)
            .then_with(|| left.end_offset.cmp(&right.end_offset))
            .then_with(|| left.kind.cmp(&right.kind))
    });
    parts.dedup_by(|left, right| {
        left.start_offset == right.start_offset
            && left.end_offset == right.end_offset
            && left.kind == right.kind
    });
    PastePartsAnalysis {
        format_version: FORMAT_VERSION,
        parts,
    }
}

pub fn value_at_offsets<'a>(source: &'a str, part: &PastePart) -> Option<&'a str> {
    let start = source
        .char_indices()
        .nth(part.start_offset)
        .map_or(source.len(), |(index, _)| index);
    let end = source
        .char_indices()
        .nth(part.end_offset)
        .map_or(source.len(), |(index, _)| index);
    (start < end).then(|| &source[start..end])
}

pub fn best_match<'a>(
    parts: &'a [PastePart],
    context: &super::SmartPasteContext,
) -> Option<&'a PastePart> {
    let context = normalize(
        &[
            context.role.as_deref(),
            context.label.as_deref(),
            context.description.as_deref(),
            context.help.as_deref(),
            context.placeholder.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" "),
    );
    let mut scored = parts
        .iter()
        .map(|part| (score(part, &context), part))
        .filter(|(score, _)| *score > 0)
        .collect::<Vec<_>>();
    scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    match scored.as_slice() {
        [] => None,
        [(_, part), ..] if scored.get(1).is_none_or(|next| next.0 < scored[0].0) => Some(part),
        _ => None,
    }
}

fn score(part: &PastePart, context: &str) -> u32 {
    let role_score = part
        .role
        .as_deref()
        .map(normalize)
        .filter(|role| !role.is_empty() && context.contains(role))
        .map_or(0, |_| 8);
    let alias_score = part
        .aliases
        .iter()
        .map(|alias| normalize(alias))
        .filter(|alias| !alias.is_empty() && context.contains(alias))
        .map(|_| 5)
        .max()
        .unwrap_or(0);
    role_score + alias_score + u32::from(kind_matches(&part.kind, context)) * 3
}

fn kind_matches(kind: &str, context: &str) -> bool {
    let phrases: &[&str] = match kind {
        "email" => &["email", "e mail"],
        "phone" => &["phone", "mobile", "telephone", "tel"],
        "link" => &["url", "website", "web site", "link"],
        "credential" => &["password", "credential", "secret"],
        "payment_card" => &["card number", "credit card", "debit card"],
        "person_name" => &["full name", "name"],
        "organization" => &["company", "employer", "organization"],
        "job_title" => &["job title", "position", "role"],
        "location" => &["location", "address", "city", "state", "country"],
        _ => return context.contains(&normalize(&kind.replace('_', " "))),
    };
    phrases.iter().any(|phrase| context.contains(phrase))
}

fn normalize(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_matches_become_deduplicated_exact_parts() {
        let matched = crate::content_classification::ClassificationMatch {
            classifier_ref: "classifier:email".into(),
            classifier_name: "Email address".into(),
            content_type: "email".into(),
            priority: 1,
            start_offset: 2,
            end_offset: 5,
        };
        let analysis = from_classifications(&[matched.clone(), matched]);
        assert_eq!(analysis.parts.len(), 1);
        assert_eq!(analysis.parts[0].aliases, ["email", "Email address"]);
        assert_eq!(value_at_offsets("A😀bcZ", &analysis.parts[0]), Some("bcZ"));
    }

    #[test]
    fn matching_prefers_semantic_roles_and_rejects_ambiguous_parts() {
        let part = |role: Option<&str>, start_offset| PastePart {
            kind: "email".into(),
            role: role.map(str::to_string),
            aliases: vec!["email address".into()],
            start_offset,
            end_offset: start_offset + 3,
            confidence: 1.0,
            analyzer_ref: "test".into(),
        };
        let parts = [part(Some("personal"), 0), part(Some("work"), 4)];
        assert_eq!(
            best_match(
                &parts,
                &super::super::SmartPasteContext {
                    application: "Browser".into(),
                    label: Some("Work email".into()),
                    ..Default::default()
                },
            )
            .map(|matched| matched.start_offset),
            Some(4)
        );
        assert!(best_match(
            &parts,
            &super::super::SmartPasteContext {
                application: "Browser".into(),
                label: Some("Email".into()),
                ..Default::default()
            },
        )
        .is_none());
    }
}
