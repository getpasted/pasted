use crate::content_analysis::{
    AnalysisFailure, AnalysisReport, ParticipantOutcome, ParticipantRun, DETECTOR_PARTICIPANT_REF,
};
use crate::content_detection::Detector;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectionTargetKind {
    Detector,
    DetectorSet,
}

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
    pub target_kind: DetectionTargetKind,
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
        target_kind: DetectionTargetKind,
        target_ref: String,
        analysis: AnalysisReport,
    ) -> Self {
        let run = analysis
            .runs
            .iter()
            .find(|run| run.stable_ref == DETECTOR_PARTICIPANT_REF);
        let failure = match run {
            Some(run) if run.outcome == ParticipantOutcome::Failed => {
                run.failure.clone().or_else(|| {
                    Some(AnalysisFailure {
                        code: "analysis_failed".into(),
                        message: "Detection failed without a structured reason.".into(),
                    })
                })
            }
            Some(run) if run.outcome == ParticipantOutcome::MissingInput => Some(AnalysisFailure {
                code: "missing_input".into(),
                message: "Detection's required input was not available.".into(),
            }),
            None => Some(AnalysisFailure {
                code: "missing_participant".into(),
                message: "Detection did not participate in Analysis.".into(),
            }),
            _ => None,
        };
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
    pub applied_clip_id: Option<i64>,
}

impl DetectionApplicationResult {
    pub fn preview(analysis: DetectionResult) -> Self {
        Self {
            analysis,
            applied_clip_id: None,
        }
    }
}

pub fn analyze_detectors(text: &str, detectors: &[Detector]) -> DetectionResult {
    DetectionResult::from_report(
        DetectionTargetKind::DetectorSet,
        DETECTOR_PARTICIPANT_REF.into(),
        crate::content_analysis::analyze_text(text, detectors),
    )
}

pub fn analyze_detector(text: &str, detector: &Detector) -> DetectionResult {
    DetectionResult::from_report(
        DetectionTargetKind::Detector,
        detector.stable_ref.clone(),
        crate::content_analysis::analyze_text(text, std::slice::from_ref(detector)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content_analysis::{AnalysisContext, AnalysisPass};

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
    fn failed_detection_discards_partial_classification_context() {
        let report = AnalysisReport {
            context: AnalysisContext {
                clip_kind: "text".into(),
                original_text: None,
                image_bytes: None,
                searchable_text: None,
                detected_type: Some("credential".into()),
                matched_detector_ref: Some("detector:credential".into()),
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
            DetectionTargetKind::DetectorSet,
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
