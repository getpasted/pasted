use crate::content_detection::{detect_match_with_detectors, Detector};
use crate::content_extraction::{ExtractionOutcome, Extractor, ExtractorEngineRegistry};

pub use crate::analysis_contract::{
    AnalysisFailure, AnalysisPass, AnalysisTargetKind, ParticipantContract, ParticipantOutcome,
    ParticipantRun, RepresentationKind, MAX_ANALYSIS_PASSES,
};
pub const DETECTOR_PARTICIPANT_REF: &str = "analysis:content-detectors";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisContext {
    pub clip_kind: String,
    pub original_text: Option<String>,
    pub image_bytes: Option<Vec<u8>>,
    pub searchable_text: Option<String>,
    pub detected_type: Option<String>,
    pub matched_detector_ref: Option<String>,
}

impl AnalysisContext {
    pub fn for_text(text: impl Into<String>) -> Self {
        Self {
            clip_kind: "text".into(),
            original_text: Some(text.into()),
            image_bytes: None,
            searchable_text: None,
            detected_type: None,
            matched_detector_ref: None,
        }
    }

    pub fn for_image(image_bytes: Vec<u8>) -> Self {
        Self {
            clip_kind: "image".into(),
            original_text: None,
            image_bytes: Some(image_bytes),
            searchable_text: None,
            detected_type: None,
            matched_detector_ref: None,
        }
    }

    pub fn has(&self, kind: RepresentationKind) -> bool {
        match kind {
            RepresentationKind::ClipKind => !self.clip_kind.is_empty(),
            RepresentationKind::OriginalText => self.original_text.is_some(),
            RepresentationKind::ImageBytes => self.image_bytes.is_some(),
            RepresentationKind::SearchableText => self.searchable_text.is_some(),
            RepresentationKind::AnalyzableText => self.analysis_text().is_some(),
            RepresentationKind::Classification => self.detected_type.is_some(),
        }
    }

    pub fn analysis_text(&self) -> Option<&str> {
        self.original_text
            .as_deref()
            .or(self.searchable_text.as_deref())
    }
}

pub struct AnalysisReport {
    pub context: AnalysisContext,
    pub runs: Vec<ParticipantRun>,
}

pub(crate) struct ParticipantResolution {
    pub outcome: ParticipantOutcome,
    pub failure: Option<AnalysisFailure>,
}

impl AnalysisReport {
    pub fn failure_for(&self, stable_ref: &str) -> Option<&AnalysisFailure> {
        self.runs
            .iter()
            .find(|run| run.stable_ref == stable_ref)
            .and_then(|run| run.failure.as_ref())
    }

    pub(crate) fn resolve_participant(
        &self,
        stable_ref: &str,
        target_kind: AnalysisTargetKind,
    ) -> ParticipantResolution {
        let subject = target_kind.failure_subject();
        let run = self.runs.iter().find(|run| run.stable_ref == stable_ref);
        match run {
            Some(run) if run.outcome == ParticipantOutcome::Failed => ParticipantResolution {
                outcome: ParticipantOutcome::Failed,
                failure: run.failure.clone().or_else(|| {
                    Some(AnalysisFailure {
                        code: "analysis_failed".into(),
                        message: format!("{subject} failed without a structured reason."),
                    })
                }),
            },
            Some(run) if run.outcome == ParticipantOutcome::MissingInput => ParticipantResolution {
                outcome: ParticipantOutcome::MissingInput,
                failure: Some(AnalysisFailure {
                    code: "missing_input".into(),
                    message: format!("{subject}'s required input was not available."),
                }),
            },
            Some(run) => ParticipantResolution {
                outcome: run.outcome,
                failure: None,
            },
            None => ParticipantResolution {
                outcome: ParticipantOutcome::Failed,
                failure: Some(AnalysisFailure {
                    code: "missing_participant".into(),
                    message: format!("{subject} did not participate in Analysis."),
                }),
            },
        }
    }
}

type ParticipantExecutor<'a> =
    Box<dyn FnMut(&mut AnalysisContext) -> Result<ParticipantOutcome, AnalysisFailure> + 'a>;

pub struct AnalysisParticipant<'a> {
    pub contract: ParticipantContract,
    execute: ParticipantExecutor<'a>,
}

impl<'a> AnalysisParticipant<'a> {
    pub fn new(
        contract: ParticipantContract,
        execute: impl FnMut(&mut AnalysisContext) -> Result<ParticipantOutcome, AnalysisFailure> + 'a,
    ) -> Self {
        Self {
            contract,
            execute: Box::new(execute),
        }
    }
}

pub fn schedule(
    context: AnalysisContext,
    mut participants: Vec<AnalysisParticipant<'_>>,
) -> AnalysisReport {
    participants.sort_by(|left, right| {
        left.contract
            .pass
            .cmp(&right.contract.pass)
            .then(left.contract.priority.cmp(&right.contract.priority))
            .then(left.contract.stable_ref.cmp(&right.contract.stable_ref))
    });

    let mut context = context;
    let mut runs = Vec::with_capacity(participants.len());
    let mut pending = participants.into_iter().map(Some).collect::<Vec<_>>();

    for pass in AnalysisPass::ORDERED {
        loop {
            let next_ready = pending.iter().position(|slot| {
                slot.as_ref().is_some_and(|participant| {
                    participant.contract.pass == pass
                        && participant
                            .contract
                            .requires
                            .iter()
                            .all(|requirement| context.has(*requirement))
                })
            });
            let Some(index) = next_ready else {
                break;
            };

            let mut participant = pending[index].take().expect("ready participant exists");
            let stable_ref = participant.contract.stable_ref.clone();
            let (outcome, failure) = match (participant.execute)(&mut context) {
                Ok(ParticipantOutcome::Produced)
                    if !participant
                        .contract
                        .provides
                        .iter()
                        .all(|provided| context.has(*provided)) =>
                {
                    (
                        ParticipantOutcome::Failed,
                        Some(AnalysisFailure {
                            code: "contract_violation".into(),
                            message: "Participant did not provide its declared representations."
                                .into(),
                        }),
                    )
                }
                Ok(outcome) => (outcome, None),
                Err(failure) => (ParticipantOutcome::Failed, Some(failure)),
            };
            runs.push(ParticipantRun {
                stable_ref,
                pass,
                outcome,
                failure,
            });
        }

        for slot in &mut pending {
            let is_blocked = slot
                .as_ref()
                .is_some_and(|participant| participant.contract.pass == pass);
            if !is_blocked {
                continue;
            }
            let participant = slot.take().expect("blocked participant exists");
            runs.push(ParticipantRun {
                stable_ref: participant.contract.stable_ref,
                pass,
                outcome: ParticipantOutcome::MissingInput,
                failure: None,
            });
        }
    }

    AnalysisReport { context, runs }
}

fn detector_participant<'a>(detectors: &'a [Detector]) -> AnalysisParticipant<'a> {
    AnalysisParticipant::new(
        ParticipantContract {
            stable_ref: DETECTOR_PARTICIPANT_REF.into(),
            name: "Content Detectors".into(),
            pass: AnalysisPass::Classify,
            priority: 0,
            requires: vec![RepresentationKind::AnalyzableText],
            provides: vec![RepresentationKind::Classification],
        },
        move |context| {
            let Some(text) = context.analysis_text() else {
                return Ok(ParticipantOutcome::NoOutput);
            };
            let detection = detect_match_with_detectors(text, detectors);
            context.detected_type = Some(detection.as_ref().map_or_else(
                || "text".to_string(),
                |matched| matched.content_type.clone(),
            ));
            context.matched_detector_ref = detection.map(|matched| matched.detector_ref);
            Ok(ParticipantOutcome::Produced)
        },
    )
}

pub(crate) fn analyze_text(text: &str, detectors: &[Detector]) -> AnalysisReport {
    schedule(
        AnalysisContext::for_text(text),
        vec![detector_participant(detectors)],
    )
}

pub(crate) fn analyze_image_with_registry(
    image_bytes: Vec<u8>,
    extractor: &Extractor,
    detectors: Option<&[Detector]>,
    registry: &ExtractorEngineRegistry<'_>,
) -> AnalysisReport {
    let extractor_ref = extractor.stable_ref.clone();
    let extractor_name = extractor.name.clone();
    let extractor_priority = extractor.priority;
    let representation_contract = extractor.representation_contract().unwrap_or(
        crate::analysis_contract::RepresentationContract {
            input: RepresentationKind::ImageBytes,
            output: RepresentationKind::SearchableText,
        },
    );
    let mut provided_representations = vec![representation_contract.output];
    if representation_contract.output == RepresentationKind::SearchableText {
        provided_representations.push(RepresentationKind::AnalyzableText);
    }
    let mut participants = vec![AnalysisParticipant::new(
        ParticipantContract {
            stable_ref: extractor_ref,
            name: extractor_name,
            pass: AnalysisPass::Extract,
            priority: extractor_priority,
            requires: vec![representation_contract.input],
            provides: provided_representations,
        },
        move |context| {
            let Some(image_bytes) = context.image_bytes.as_deref() else {
                return Ok(ParticipantOutcome::NoOutput);
            };
            match registry.execute(extractor, image_bytes) {
                ExtractionOutcome::Produced { text } => {
                    context.searchable_text = Some(text);
                    Ok(ParticipantOutcome::Produced)
                }
                ExtractionOutcome::NoOutput => Ok(ParticipantOutcome::NoOutput),
                ExtractionOutcome::Failed { failure } => Err(AnalysisFailure {
                    code: failure.code,
                    message: failure.message,
                }),
            }
        },
    )];
    if let Some(detectors) = detectors {
        participants.push(detector_participant(detectors));
    }
    schedule(AnalysisContext::for_image(image_bytes), participants)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestEngine;

    struct FailingEngine;

    impl crate::content_extraction::ExtractorEngine for TestEngine {
        fn id(&self) -> &'static str {
            "test-v1"
        }

        fn availability(&self) -> crate::content_extraction::EngineAvailability {
            crate::content_extraction::EngineAvailability {
                is_available: true,
                unavailable_reason: None,
            }
        }

        fn extract(&self, _image_bytes: &[u8]) -> ExtractionOutcome {
            ExtractionOutcome::Produced {
                text: "agent@example.com".into(),
            }
        }
    }

    impl crate::content_extraction::ExtractorEngine for FailingEngine {
        fn id(&self) -> &'static str {
            "test-v1"
        }

        fn availability(&self) -> crate::content_extraction::EngineAvailability {
            crate::content_extraction::EngineAvailability {
                is_available: true,
                unavailable_reason: None,
            }
        }

        fn extract(&self, _image_bytes: &[u8]) -> ExtractionOutcome {
            ExtractionOutcome::Failed {
                failure: crate::content_extraction::ExtractionFailure {
                    code: "test_failure".into(),
                    message: "The test engine failed.".into(),
                },
            }
        }
    }

    fn detector(pattern: &str, content_type: &str) -> Detector {
        Detector {
            id: 1,
            stable_ref: format!("test:{content_type}"),
            name: content_type.into(),
            content_type: content_type.into(),
            description: String::new(),
            patterns: vec![pattern.into()],
            validator: None,
            enabled: true,
            priority: 10,
            is_builtin: false,
            defaults: None,
            is_deleted: false,
        }
    }

    fn extractor() -> Extractor {
        Extractor {
            id: 1,
            stable_ref: "extractor:test".into(),
            name: "Test OCR".into(),
            description: String::new(),
            engine: "test-v1".into(),
            input_contract: "image".into(),
            output_contract: "searchable_text".into(),
            enabled: true,
            priority: 10,
            is_builtin: false,
            is_available: true,
            unavailable_reason: None,
            defaults: None,
        }
    }

    #[test]
    fn extraction_makes_text_available_to_later_detection() {
        let detectors = vec![detector(r"^[^@]+@[^@]+\.[^@]+$", "email")];
        let engine = TestEngine;
        let engines: [&dyn crate::content_extraction::ExtractorEngine; 1] = [&engine];
        let registry = ExtractorEngineRegistry::new(&engines);
        let report =
            analyze_image_with_registry(vec![1, 2, 3], &extractor(), Some(&detectors), &registry);

        assert_eq!(
            report.context.searchable_text.as_deref(),
            Some("agent@example.com")
        );
        assert_eq!(report.context.detected_type.as_deref(), Some("email"));
        assert_eq!(report.runs.len(), 2);
        assert_eq!(report.runs[0].pass, AnalysisPass::Extract);
        assert_eq!(report.runs[1].pass, AnalysisPass::Classify);
    }

    #[test]
    fn unresolved_missing_representations_skip_without_execution() {
        let runs = schedule(
            AnalysisContext::for_text("hello"),
            vec![AnalysisParticipant::new(
                ParticipantContract {
                    stable_ref: "needs-image".into(),
                    name: "Needs Image".into(),
                    pass: AnalysisPass::Enrich,
                    priority: 1,
                    requires: vec![RepresentationKind::ImageBytes],
                    provides: vec![RepresentationKind::SearchableText],
                },
                |_| panic!("a participant with missing inputs must not execute"),
            )],
        )
        .runs;

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].outcome, ParticipantOutcome::MissingInput);
    }

    #[test]
    fn participant_resolution_normalizes_scheduler_failures_for_every_surface() {
        let failed = AnalysisReport {
            context: AnalysisContext::for_text("text"),
            runs: vec![ParticipantRun {
                stable_ref: "participant:test".into(),
                pass: AnalysisPass::Enrich,
                outcome: ParticipantOutcome::Failed,
                failure: None,
            }],
        }
        .resolve_participant("participant:test", AnalysisTargetKind::Enricher);
        assert_eq!(failed.outcome, ParticipantOutcome::Failed);
        assert_eq!(failed.failure.unwrap().code, "analysis_failed");

        let missing_input = AnalysisReport {
            context: AnalysisContext::for_text("text"),
            runs: vec![ParticipantRun {
                stable_ref: "participant:test".into(),
                pass: AnalysisPass::Enrich,
                outcome: ParticipantOutcome::MissingInput,
                failure: None,
            }],
        }
        .resolve_participant("participant:test", AnalysisTargetKind::Enricher);
        assert_eq!(missing_input.failure.unwrap().code, "missing_input");

        let missing_participant = AnalysisReport {
            context: AnalysisContext::for_text("text"),
            runs: Vec::new(),
        }
        .resolve_participant("participant:test", AnalysisTargetKind::Enricher);
        assert_eq!(
            missing_participant.failure.unwrap().code,
            "missing_participant"
        );
    }

    #[test]
    fn same_pass_consumers_run_after_their_inputs_become_available() {
        let report = schedule(
            AnalysisContext::for_image(vec![1]),
            vec![
                AnalysisParticipant::new(
                    ParticipantContract {
                        stable_ref: "consumer".into(),
                        name: "Consumer".into(),
                        pass: AnalysisPass::Extract,
                        priority: 1,
                        requires: vec![RepresentationKind::SearchableText],
                        provides: vec![RepresentationKind::Classification],
                    },
                    |context| {
                        context.detected_type = Some("derived".into());
                        Ok(ParticipantOutcome::Produced)
                    },
                ),
                AnalysisParticipant::new(
                    ParticipantContract {
                        stable_ref: "producer".into(),
                        name: "Producer".into(),
                        pass: AnalysisPass::Extract,
                        priority: 2,
                        requires: vec![RepresentationKind::ImageBytes],
                        provides: vec![RepresentationKind::SearchableText],
                    },
                    |context| {
                        context.searchable_text = Some("derived text".into());
                        Ok(ParticipantOutcome::Produced)
                    },
                ),
                AnalysisParticipant::new(
                    ParticipantContract {
                        stable_ref: "independent".into(),
                        name: "Independent".into(),
                        pass: AnalysisPass::Extract,
                        priority: 3,
                        requires: vec![RepresentationKind::ImageBytes],
                        provides: vec![],
                    },
                    |_| Ok(ParticipantOutcome::Produced),
                ),
            ],
        );

        assert_eq!(report.context.detected_type.as_deref(), Some("derived"));
        assert_eq!(report.runs.len(), 3);
        assert_eq!(report.runs[0].stable_ref, "producer");
        assert_eq!(report.runs[1].stable_ref, "consumer");
        assert_eq!(report.runs[2].stable_ref, "independent");
        assert!(report
            .runs
            .iter()
            .all(|run| run.outcome == ParticipantOutcome::Produced));
    }

    #[test]
    fn typed_engine_failures_fail_the_extractor_participant_closed() {
        let engine = FailingEngine;
        let engines: [&dyn crate::content_extraction::ExtractorEngine; 1] = [&engine];
        let registry = ExtractorEngineRegistry::new(&engines);
        let report = analyze_image_with_registry(vec![1], &extractor(), None, &registry);

        assert_eq!(report.context.searchable_text, None);
        assert_eq!(report.runs[0].outcome, ParticipantOutcome::Failed);
        assert_eq!(
            report.runs[0].failure.as_ref(),
            Some(&AnalysisFailure {
                code: "test_failure".into(),
                message: "The test engine failed.".into(),
            })
        );
        assert_eq!(
            report.failure_for("extractor:test"),
            report.runs[0].failure.as_ref()
        );
    }

    #[test]
    fn participants_fail_closed_when_declared_outputs_are_missing() {
        let runs = schedule(
            AnalysisContext::for_text("hello"),
            vec![AnalysisParticipant::new(
                ParticipantContract {
                    stable_ref: "broken-extractor".into(),
                    name: "Broken Extractor".into(),
                    pass: AnalysisPass::Extract,
                    priority: 1,
                    requires: vec![RepresentationKind::OriginalText],
                    provides: vec![RepresentationKind::SearchableText],
                },
                |_| Ok(ParticipantOutcome::Produced),
            )],
        )
        .runs;

        assert_eq!(runs[0].outcome, ParticipantOutcome::Failed);
        assert_eq!(
            runs[0]
                .failure
                .as_ref()
                .map(|failure| failure.code.as_str()),
            Some("contract_violation")
        );
    }

    #[test]
    fn text_classification_uses_the_same_scheduler_contract() {
        let detectors = vec![detector(r"^#[0-9a-fA-F]{6}$", "color")];
        let report = analyze_text("#112233", &detectors);
        assert_eq!(report.context.detected_type.as_deref(), Some("color"));
        assert_eq!(report.runs[0].stable_ref, "analysis:content-detectors");
    }
}
