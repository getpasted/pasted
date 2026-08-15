use crate::analysis_contract::{
    AnalysisEnvelope, AnalysisPass, ParticipantContract, RepresentationKind,
};
use crate::content_inspection::StructuralMetadata;
use crate::db::TransformDefinition;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use std::collections::BTreeSet;

pub const SMART_ACTIONS_ENRICHER_REF: &str = "enricher:smart-actions-v1";
const MAX_RECOMMENDATIONS: usize = 12;
const MAX_TRANSFORM_CANDIDATES: usize = 256;

static URL_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)https?://[^\s]+").expect("valid URL pattern"));
static EMAIL_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)[a-z0-9._%+\-]+@[a-z0-9.\-]+\.[a-z]{2,}").expect("valid email pattern")
});
static PHONE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?x)(?:^|[^0-9])(?:\+?[0-9]{1,3}[-.\x20]?)?\(?[0-9]{3}\)?[-.\x20]?[0-9]{3}[-.\x20]?[0-9]{4}(?:$|[^0-9])")
        .expect("valid phone pattern")
});
static HTML_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?is)<[a-z][^>]*>.*</[a-z][^>]*>|<[a-z][^>]*/?>").expect("valid HTML pattern")
});
static MARKDOWN_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)(^|\s)(#{1,6}\s|\*\*|__|```|\[[^\]]+\]\([^\)]+\))")
        .expect("valid Markdown pattern")
});

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnricherDefinition {
    pub stable_ref: String,
    pub name: String,
    pub description: String,
    pub priority: i64,
    pub input_contracts: Vec<String>,
    pub output_contract: String,
}

impl EnricherDefinition {
    pub(crate) fn participant_contract(&self) -> ParticipantContract {
        ParticipantContract {
            stable_ref: self.stable_ref.clone(),
            name: self.name.clone(),
            pass: AnalysisPass::Enrich,
            priority: self.priority,
            requires: vec![
                RepresentationKind::AnalyzableText,
                RepresentationKind::StructuralMetadata,
            ],
            provides: vec![RepresentationKind::Recommendations],
        }
    }
}

pub fn smart_actions_enricher_definition() -> EnricherDefinition {
    EnricherDefinition {
        stable_ref: SMART_ACTIONS_ENRICHER_REF.into(),
        name: "Smart Actions".into(),
        description: "Recommends saved Transforms from content-free analysis signals.".into(),
        priority: 0,
        input_contracts: vec!["analyzable_text".into(), "structural_metadata".into()],
        output_contract: "recommendations".into(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SmartActionSignal {
    Url,
    Json,
    Html,
    Markdown,
    MultiLine,
    Email,
    Phone,
}

impl SmartActionSignal {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Url => "URL Link",
            Self::Json => "JSON Data",
            Self::Html => "HTML Markup",
            Self::Markdown => "Markdown Text",
            Self::MultiLine => "Multiple Lines",
            Self::Email => "Email Address",
            Self::Phone => "Phone Number",
        }
    }

    const fn keywords(self) -> &'static [&'static str] {
        match self {
            Self::Url => &[
                "url",
                "link",
                "tracking",
                "clean_url_tracking",
                "extract_urls",
            ],
            Self::Json => &["json", "json_format", "json_minify"],
            Self::Html => &["html", "markup", "tag", "strip_html", "wrap_tags"],
            Self::Markdown => &["markdown", "strip_markdown"],
            Self::MultiLine => &[
                "line",
                "list",
                "sort",
                "dedupe",
                "sort_lines",
                "dedupe_lines",
            ],
            Self::Email => &["email", "extract_emails"],
            Self::Phone => &["phone", "extract_phones"],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartActionRecommendation {
    pub transform_ref: String,
    pub transform_name: String,
    pub transform_revision: i64,
    pub reasons: Vec<SmartActionSignal>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartActionRecommendations {
    pub signals: Vec<SmartActionSignal>,
    pub signal_labels: Vec<String>,
    pub actions: Vec<SmartActionRecommendation>,
}

pub type EnrichmentResult = AnalysisEnvelope<SmartActionRecommendations>;

fn detect_signals(
    text: &str,
    classification: Option<&str>,
    structure: &StructuralMetadata,
) -> Vec<SmartActionSignal> {
    let trimmed = text.trim();
    let mut signals = BTreeSet::new();
    if URL_PATTERN.is_match(text) || classification == Some("url") {
        signals.insert(SmartActionSignal::Url);
    }
    let is_json = ((trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']')))
        && serde_json::from_str::<serde_json::Value>(trimmed).is_ok();
    if is_json {
        signals.insert(SmartActionSignal::Json);
    }
    let has_html = HTML_PATTERN.is_match(text);
    if has_html && !is_json {
        signals.insert(SmartActionSignal::Html);
    }
    if MARKDOWN_PATTERN.is_match(text) && !has_html && !is_json {
        signals.insert(SmartActionSignal::Markdown);
    }
    if structure
        .text
        .as_ref()
        .is_some_and(|metrics| metrics.line_count > 1)
    {
        signals.insert(SmartActionSignal::MultiLine);
    }
    if EMAIL_PATTERN.is_match(text) || classification == Some("email") {
        signals.insert(SmartActionSignal::Email);
    }
    if PHONE_PATTERN.is_match(text) || classification == Some("phone") {
        signals.insert(SmartActionSignal::Phone);
    }
    signals.into_iter().collect()
}

fn searchable_transform(transform: &TransformDefinition) -> String {
    let mut parts = vec![transform.name.as_str()];
    for step in &transform.steps {
        parts.push(step.operation_ref.as_str());
        if let Some(config) = step.config_json.as_deref() {
            parts.push(config);
        }
    }
    if let Some(plan) = transform.plan.as_ref() {
        parts.push(plan.intent.as_str());
        parts.push(plan.summary.as_str());
        for step in &plan.steps {
            parts.push(step.name.as_str());
            parts.push(step.rationale.as_str());
            match &step.executor {
                crate::transformation_intent::PlannedExecutor::Deterministic {
                    operation_ref,
                    config_json,
                } => {
                    parts.push(operation_ref.as_str());
                    if let Some(config) = config_json.as_deref() {
                        parts.push(config);
                    }
                }
                crate::transformation_intent::PlannedExecutor::Semantic {
                    instructions, ..
                } => parts.push(instructions.as_str()),
            }
        }
    }
    parts.join(" ").to_lowercase()
}

pub fn recommend_smart_actions(
    text: &str,
    classification: Option<&str>,
    structure: &StructuralMetadata,
    transforms: &[TransformDefinition],
) -> SmartActionRecommendations {
    let signals = detect_signals(text, classification, structure);
    let signal_labels = signals
        .iter()
        .map(|signal| signal.label().to_string())
        .collect();
    if signals.is_empty() {
        return SmartActionRecommendations {
            signals,
            signal_labels,
            actions: Vec::new(),
        };
    }
    let actions = transforms
        .iter()
        .take(MAX_TRANSFORM_CANDIDATES)
        .filter_map(|transform| {
            let searchable = searchable_transform(transform);
            let reasons = signals
                .iter()
                .copied()
                .filter(|signal| {
                    signal
                        .keywords()
                        .iter()
                        .any(|keyword| searchable.contains(keyword))
                })
                .collect::<Vec<_>>();
            (!reasons.is_empty()).then(|| SmartActionRecommendation {
                transform_ref: transform.stable_ref.clone(),
                transform_name: transform.name.clone(),
                transform_revision: transform.revision,
                reasons,
            })
        })
        .take(MAX_RECOMMENDATIONS)
        .collect();
    SmartActionRecommendations {
        signals,
        signal_labels,
        actions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content_inspection::{OriginKind, TextStructure};
    use crate::db::{PipelineStep, TransformAuthoringKind};

    fn structure(line_count: usize) -> StructuralMetadata {
        StructuralMetadata {
            origin: OriginKind::ClipboardContent,
            byte_count: 20,
            text: Some(TextStructure {
                character_count: 20,
                word_count: 2,
                line_count,
            }),
            image: None,
            files: None,
        }
    }

    fn transform(name: &str, operation_ref: &str) -> TransformDefinition {
        TransformDefinition {
            id: 1,
            stable_ref: "transform:test".into(),
            name: name.into(),
            authoring_kind: TransformAuthoringKind::Manual,
            execution_character: "replayable".into(),
            connection_id: None,
            shortcut: None,
            revision: 3,
            created_at: String::new(),
            updated_at: String::new(),
            plan: None,
            steps: vec![PipelineStep {
                position: 0,
                operation_ref: operation_ref.into(),
                config_json: None,
                failure_policy: "stop".into(),
            }],
        }
    }

    #[test]
    fn recommends_stable_transform_references_from_shared_signals() {
        let result = recommend_smart_actions(
            "https://example.com?a=1\nhttps://example.com?a=2",
            Some("url"),
            &structure(2),
            &[transform("Clean links", "builtin:clean_url_tracking")],
        );
        assert_eq!(
            result.signals,
            vec![SmartActionSignal::Url, SmartActionSignal::MultiLine]
        );
        assert_eq!(result.actions[0].transform_ref, "transform:test");
        assert_eq!(result.actions[0].transform_revision, 3);
    }

    #[test]
    fn recommendations_never_serialize_clipboard_content() {
        let secret = "private-token-0123456789";
        let result = recommend_smart_actions(
            &format!("{{\"value\":\"{secret}\"}}"),
            Some("code"),
            &structure(1),
            &[transform("Format JSON", "builtin:json_format")],
        );
        let json = serde_json::to_string(&result).unwrap();
        assert!(!json.contains(secret));
        assert!(json.contains("transform:test"));
    }

    #[test]
    fn transform_candidate_work_is_bounded_before_matching() {
        let mut transforms = (0..MAX_TRANSFORM_CANDIDATES)
            .map(|_| transform("Uppercase", "builtin:uppercase"))
            .collect::<Vec<_>>();
        transforms.push(transform("Format JSON", "builtin:json_format"));
        let result = recommend_smart_actions(
            "{\"hello\":\"world\"}",
            Some("code"),
            &structure(1),
            &transforms,
        );
        assert!(result.actions.is_empty());
    }
}
