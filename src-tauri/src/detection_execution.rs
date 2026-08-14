use crate::analysis_contract::{
    AnalysisFailure, AnalysisMetadata, AnalysisPolicy, AnalysisTargetKind, ClipApplication,
    ParticipantRun,
};
use crate::content_analysis::{AnalysisReport, DETECTOR_PARTICIPANT_REF};
use crate::content_detection::Detector;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectionOutcome {
    Matched,
    NoMatch,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionResult {
    #[serde(flatten)]
    pub metadata: AnalysisMetadata,
    pub target_kind: AnalysisTargetKind,
    pub target_ref: String,
    pub outcome: DetectionOutcome,
    pub matched: bool,
    pub detected_type: Option<String>,
    pub matched_detector_ref: Option<String>,
    pub failure: Option<AnalysisFailure>,
    pub participants: Vec<ParticipantRun>,
}

impl DetectionResult {
    fn from_report(
        policy: AnalysisPolicy,
        target_kind: AnalysisTargetKind,
        target_ref: String,
        analysis: AnalysisReport,
    ) -> Self {
        let resolution = analysis.resolve_participant(DETECTOR_PARTICIPANT_REF, target_kind);
        let failure = resolution.failure;
        let matched = failure.is_none()
            && analysis.context.matched_detector_ref.is_some()
            && analysis.context.detected_type.is_some();
        let outcome = if failure.is_some() {
            DetectionOutcome::Failed
        } else if matched {
            DetectionOutcome::Matched
        } else {
            DetectionOutcome::NoMatch
        };
        Self {
            metadata: AnalysisMetadata::new(policy),
            target_kind,
            target_ref,
            outcome,
            matched,
            detected_type: matched.then_some(analysis.context.detected_type).flatten(),
            matched_detector_ref: matched
                .then_some(analysis.context.matched_detector_ref)
                .flatten(),
            failure,
            participants: analysis.runs,
        }
    }

    pub fn classification(&self) -> &str {
        self.detected_type.as_deref().unwrap_or("text")
    }

    pub fn failed(&self) -> bool {
        self.outcome == DetectionOutcome::Failed
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionApplicationResult {
    #[serde(flatten)]
    pub analysis: DetectionResult,
    #[serde(flatten)]
    pub application: ClipApplication,
}

impl DetectionApplicationResult {
    pub fn preview(analysis: DetectionResult) -> Self {
        Self {
            analysis,
            application: ClipApplication::preview(),
        }
    }

    pub fn applied(analysis: DetectionResult, clip_id: i64) -> Self {
        Self {
            analysis,
            application: ClipApplication::applied(clip_id),
        }
    }
}

pub fn analyze_detectors(text: &str, detectors: &[Detector]) -> DetectionResult {
    analyze_detectors_with_policy(
        text,
        detectors,
        crate::analysis_contract::AnalysisPolicy::Interactive,
        None,
    )
}

pub(crate) fn analyze_detectors_with_policy(
    text: &str,
    detectors: &[Detector],
    policy: crate::analysis_contract::AnalysisPolicy,
    source: Option<&str>,
) -> DetectionResult {
    DetectionResult::from_report(
        policy,
        AnalysisTargetKind::DetectorSet,
        DETECTOR_PARTICIPANT_REF.into(),
        crate::content_analysis::analyze(crate::content_analysis::AnalysisRequest {
            input: crate::content_analysis::AnalysisInput::Text {
                text: text.into(),
                source: source.map(str::to_owned),
            },
            policy,
            inspector: false,
            extractor: None,
            detectors: Some(detectors),
            enricher: None,
        }),
    )
}

pub fn analyze_detector(text: &str, detector: &Detector) -> DetectionResult {
    DetectionResult::from_report(
        AnalysisPolicy::Interactive,
        AnalysisTargetKind::Detector,
        detector.stable_ref.clone(),
        crate::content_analysis::analyze(crate::content_analysis::AnalysisRequest {
            input: crate::content_analysis::AnalysisInput::Text {
                text: text.into(),
                source: None,
            },
            policy: crate::analysis_contract::AnalysisPolicy::Interactive,
            inspector: false,
            extractor: None,
            detectors: Some(std::slice::from_ref(detector)),
            enricher: None,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content_analysis::{AnalysisContext, AnalysisPass, ParticipantOutcome};

    fn detector() -> Detector {
        Detector {
            id: 1,
            stable_ref: "detector:email".into(),
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
    fn detector_results_have_a_stable_matched_json_contract() {
        let result = analyze_detector("agent@example.com", &detector());

        assert!(result.matched);
        assert_eq!(result.classification(), "email");
        assert_eq!(result.participants.len(), 1);
        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            serde_json::json!({
                "formatVersion": 1,
                "policy": "interactive",
                "through": "enrich",
                "targetKind": "detector",
                "targetRef": "detector:email",
                "outcome": "matched",
                "matched": true,
                "detectedType": "email",
                "matchedDetectorRef": "detector:email",
                "failure": null,
                "participants": [{
                    "stableRef": DETECTOR_PARTICIPANT_REF,
                    "pass": "classify",
                    "outcome": "produced"
                }]
            })
        );
    }

    #[test]
    fn no_match_is_distinct_and_classifies_as_plain_text() {
        let result = analyze_detector("ordinary prose", &detector());

        assert_eq!(result.outcome, DetectionOutcome::NoMatch);
        assert!(!result.matched);
        assert_eq!(result.classification(), "text");
        assert_eq!(result.detected_type, None);
        assert_eq!(result.failure, None);
    }

    #[test]
    fn bounded_detection_preserves_its_execution_policy() {
        let result = analyze_detectors_with_policy(
            "agent@example.com",
            &[detector()],
            AnalysisPolicy::Rescan,
            Some("Pasted CLI"),
        );

        assert_eq!(
            result.metadata,
            AnalysisMetadata::new(AnalysisPolicy::Rescan)
        );
    }

    #[test]
    fn failed_detection_discards_partial_classification_context() {
        let report = AnalysisReport {
            context: AnalysisContext {
                clip_kind: "text".into(),
                capture_source: None,
                original_text: None,
                image_bytes: None,
                searchable_text: None,
                detected_type: Some("credential".into()),
                matched_detector_ref: Some("detector:credential".into()),
                structural_metadata: None,
                recommendations: None,
            },
            runs: vec![ParticipantRun {
                stable_ref: DETECTOR_PARTICIPANT_REF.into(),
                pass: AnalysisPass::Classify,
                outcome: ParticipantOutcome::Failed,
                failure: Some(AnalysisFailure {
                    code: "contract_violation".into(),
                    message: "Detection violated its contract.".into(),
                }),
            }],
        };

        let result = DetectionResult::from_report(
            AnalysisPolicy::Interactive,
            AnalysisTargetKind::DetectorSet,
            DETECTOR_PARTICIPANT_REF.into(),
            report,
        );

        assert!(result.failed());
        assert!(!result.matched);
        assert_eq!(result.detected_type, None);
        assert_eq!(result.matched_detector_ref, None);
        assert_eq!(result.classification(), "text");
    }
}
