use crate::content_detection::{detect_match_with_detectors, Detector};
use crate::content_extraction::{ExtractionOutcome, Extractor, ExtractorEngineRegistry};
use serde::Serialize;

pub const MAX_ANALYSIS_PASSES: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisPass {
    Inspect,
    Extract,
    Classify,
    Enrich,
}

impl AnalysisPass {
    const ORDERED: [Self; MAX_ANALYSIS_PASSES] =
        [Self::Inspect, Self::Extract, Self::Classify, Self::Enrich];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepresentationKind {
    ClipKind,
    OriginalText,
    ImageBytes,
    SearchableText,
    AnalyzableText,
    Classification,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantContract {
    pub stable_ref: String,
    pub name: String,
    pub pass: AnalysisPass,
    pub priority: i64,
    pub requires: Vec<RepresentationKind>,
    pub provides: Vec<RepresentationKind>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantOutcome {
    Produced,
    NoOutput,
    MissingInput,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisFailure {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantRun {
    pub stable_ref: String,
    pub pass: AnalysisPass,
    pub outcome: ParticipantOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<AnalysisFailure>,
}

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

impl AnalysisReport {
    pub fn failure_for(&self, stable_ref: &str) -> Option<&AnalysisFailure> {
        self.runs
            .iter()
            .find(|run| run.stable_ref == stable_ref)
            .and_then(|run| run.failure.as_ref())
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
            let mut progressed = false;
            for slot in &mut pending {
                let is_ready = slot.as_ref().is_some_and(|participant| {
                    participant.contract.pass == pass
                        && participant
                            .contract
                            .requires
                            .iter()
                            .all(|requirement| context.has(*requirement))
                });
                if !is_ready {
                    continue;
                }

                let mut participant = slot.take().expect("ready participant exists");
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
                                message:
                                    "Participant did not provide its declared representations."
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
                progressed = true;
            }
            if !progressed {
                break;
            }
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
            stable_ref: "analysis:content-detectors".into(),
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

pub fn analyze_text(text: &str, detectors: &[Detector]) -> AnalysisReport {
    schedule(
        AnalysisContext::for_text(text),
        vec![detector_participant(detectors)],
    )
}

pub fn classify_text(text: &str, detectors: &[Detector]) -> String {
    analyze_text(text, detectors)
        .context
        .detected_type
        .unwrap_or_else(|| "text".into())
}

pub fn analyze_image(
    image_bytes: Vec<u8>,
    extractor: &Extractor,
    detectors: Option<&[Detector]>,
) -> AnalysisReport {
    let registry = crate::content_extraction::system_engine_registry();
    analyze_image_with_registry(image_bytes, extractor, detectors, &registry)
}

pub fn analyze_image_with_registry(
    image_bytes: Vec<u8>,
    extractor: &Extractor,
    detectors: Option<&[Detector]>,
    registry: &ExtractorEngineRegistry<'_>,
) -> AnalysisReport {
    let extractor_ref = extractor.stable_ref.clone();
    let extractor_name = extractor.name.clone();
    let extractor_priority = extractor.priority;
    let mut participants = vec![AnalysisParticipant::new(
        ParticipantContract {
            stable_ref: extractor_ref,
            name: extractor_name,
            pass: AnalysisPass::Extract,
            priority: extractor_priority,
            requires: vec![RepresentationKind::ImageBytes],
            provides: vec![
                RepresentationKind::SearchableText,
                RepresentationKind::AnalyzableText,
            ],
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
            ],
        );

        assert_eq!(report.context.detected_type.as_deref(), Some("derived"));
        assert_eq!(report.runs.len(), 2);
        assert_eq!(report.runs[0].stable_ref, "producer");
        assert_eq!(report.runs[1].stable_ref, "consumer");
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
