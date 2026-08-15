use crate::analysis_contract::{
    AnalysisFailure, AnalysisMetadata, AnalysisPolicy, AnalysisTargetKind, ClipApplication,
    ParticipantRun,
};
use crate::content_analysis::{AnalysisReport, CLASSIFIER_PARTICIPANT_REF};
use crate::content_classification::Classifier;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassificationOutcome {
    Matched,
    NoMatch,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationResult {
    #[serde(flatten)]
    pub metadata: AnalysisMetadata,
    pub target_kind: AnalysisTargetKind,
    pub target_ref: String,
    pub outcome: ClassificationOutcome,
    pub matched: bool,
    pub classified_type: Option<String>,
    pub matched_classifier_ref: Option<String>,
    pub failure: Option<AnalysisFailure>,
    pub participants: Vec<ParticipantRun>,
}

impl ClassificationResult {
    fn from_report(
        policy: AnalysisPolicy,
        target_kind: AnalysisTargetKind,
        target_ref: String,
        analysis: AnalysisReport,
    ) -> Self {
        let resolution = analysis.resolve_participant(CLASSIFIER_PARTICIPANT_REF, target_kind);
        let failure = resolution.failure;
        let matched = failure.is_none()
            && analysis.context.matched_classifier_ref.is_some()
            && analysis.context.classified_type.is_some();
        let outcome = if failure.is_some() {
            ClassificationOutcome::Failed
        } else if matched {
            ClassificationOutcome::Matched
        } else {
            ClassificationOutcome::NoMatch
        };
        Self {
            metadata: AnalysisMetadata::new(policy),
            target_kind,
            target_ref,
            outcome,
            matched,
            classified_type: matched
                .then_some(analysis.context.classified_type)
                .flatten(),
            matched_classifier_ref: matched
                .then_some(analysis.context.matched_classifier_ref)
                .flatten(),
            failure,
            participants: analysis.runs,
        }
    }

    pub fn classification(&self) -> &str {
        self.classified_type.as_deref().unwrap_or("text")
    }

    pub fn failed(&self) -> bool {
        self.outcome == ClassificationOutcome::Failed
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationApplicationResult {
    #[serde(flatten)]
    pub analysis: ClassificationResult,
    #[serde(flatten)]
    pub application: ClipApplication,
}

impl ClassificationApplicationResult {
    pub fn preview(analysis: ClassificationResult) -> Self {
        Self {
            analysis,
            application: ClipApplication::preview(),
        }
    }

    pub fn applied(analysis: ClassificationResult, clip_id: i64) -> Self {
        Self {
            analysis,
            application: ClipApplication::applied(clip_id),
        }
    }
}

pub fn analyze_classifiers(text: &str, classifiers: &[Classifier]) -> ClassificationResult {
    analyze_classifiers_with_policy(
        text,
        classifiers,
        crate::analysis_contract::AnalysisPolicy::Interactive,
        None,
    )
}

pub(crate) fn analyze_classifiers_with_policy(
    text: &str,
    classifiers: &[Classifier],
    policy: crate::analysis_contract::AnalysisPolicy,
    source: Option<&str>,
) -> ClassificationResult {
    ClassificationResult::from_report(
        policy,
        AnalysisTargetKind::ClassifierSet,
        CLASSIFIER_PARTICIPANT_REF.into(),
        crate::content_analysis::analyze(crate::content_analysis::AnalysisRequest {
            input: crate::content_analysis::AnalysisInput::Text {
                text: text.into(),
                source: source.map(str::to_owned),
            },
            policy,
            inspector: false,
            extractor: None,
            classifiers: Some(classifiers),
            suggestion: None,
        }),
    )
}

pub fn analyze_classifier(text: &str, classifier: &Classifier) -> ClassificationResult {
    ClassificationResult::from_report(
        AnalysisPolicy::Interactive,
        AnalysisTargetKind::Classifier,
        classifier.stable_ref.clone(),
        crate::content_analysis::analyze(crate::content_analysis::AnalysisRequest {
            input: crate::content_analysis::AnalysisInput::Text {
                text: text.into(),
                source: None,
            },
            policy: crate::analysis_contract::AnalysisPolicy::Interactive,
            inspector: false,
            extractor: None,
            classifiers: Some(std::slice::from_ref(classifier)),
            suggestion: None,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content_analysis::{AnalysisContext, AnalysisPass, ParticipantOutcome};

    fn classifier() -> Classifier {
        Classifier {
            id: 1,
            stable_ref: "classifier:email".into(),
            name: "Email".into(),
            content_type: "email".into(),
            description: String::new(),
            patterns: vec![r"^[^@]+@[^@]+$".into()],
            validator: None,
            enabled: true,
            priority: 10,
            is_builtin: false,
            defaults: None,
            is_deleted: false,
        }
    }

    #[test]
    fn classifier_results_have_a_stable_matched_json_contract() {
        let result = analyze_classifier("agent@example.com", &classifier());

        assert!(result.matched);
        assert_eq!(result.classification(), "email");
        assert_eq!(result.participants.len(), 1);
        assert_eq!(
            serde_json::to_value(ClassificationApplicationResult::preview(result)).unwrap(),
            serde_json::from_str::<serde_json::Value>(include_str!(
                "../../contracts/analysis/v1/classifier-interactive-matched.json"
            ))
            .unwrap()
        );
    }

    #[test]
    fn no_match_is_distinct_and_classifies_as_plain_text() {
        let result = analyze_classifier("ordinary prose", &classifier());

        assert_eq!(result.outcome, ClassificationOutcome::NoMatch);
        assert!(!result.matched);
        assert_eq!(result.classification(), "text");
        assert_eq!(result.classified_type, None);
        assert_eq!(result.failure, None);
        let expected = serde_json::from_str::<serde_json::Value>(include_str!(
            "../../contracts/analysis/v1/classifier-interactive-no-match.json"
        ))
        .unwrap();
        assert_eq!(
            serde_json::to_value(ClassificationApplicationResult::preview(result)).unwrap(),
            expected
        );
    }

    #[test]
    fn bounded_classification_preserves_its_execution_policy() {
        let result = analyze_classifiers_with_policy(
            "agent@example.com",
            &[classifier()],
            AnalysisPolicy::Rescan,
            Some("Pasted CLI"),
        );

        assert_eq!(
            result.metadata,
            AnalysisMetadata::new(AnalysisPolicy::Rescan)
        );
    }

    #[test]
    fn failed_classification_discards_partial_classification_context() {
        let report = AnalysisReport {
            context: AnalysisContext {
                clip_kind: "text".into(),
                capture_source: None,
                original_text: None,
                file_references: None,
                image_bytes: None,
                searchable_text: None,
                classified_type: Some("credential".into()),
                matched_classifier_ref: Some("classifier:credential".into()),
                structural_metadata: None,
                media_metadata: None,
                suggestions: None,
            },
            runs: vec![ParticipantRun {
                stable_ref: CLASSIFIER_PARTICIPANT_REF.into(),
                pass: AnalysisPass::Classify,
                outcome: ParticipantOutcome::Failed,
                failure: Some(AnalysisFailure {
                    code: "contract_violation".into(),
                    message: "Classification violated its contract.".into(),
                }),
            }],
        };

        let result = ClassificationResult::from_report(
            AnalysisPolicy::Interactive,
            AnalysisTargetKind::ClassifierSet,
            CLASSIFIER_PARTICIPANT_REF.into(),
            report,
        );

        assert!(result.failed());
        assert!(!result.matched);
        assert_eq!(result.classified_type, None);
        assert_eq!(result.matched_classifier_ref, None);
        assert_eq!(result.classification(), "text");
    }
}
