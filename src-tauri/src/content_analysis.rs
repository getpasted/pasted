use crate::content_detection::{detect_match_with_detectors, Detector};
use crate::content_extraction::{ExtractionOutcome, Extractor, ExtractorEngineRegistry};

pub use crate::analysis_contract::{
    AnalysisFailure, AnalysisPass, AnalysisPolicy, AnalysisTargetKind, ParticipantContract,
    ParticipantOutcome, ParticipantRun, RepresentationKind, ANALYSIS_CONTRACT_VERSION,
    MAX_ANALYSIS_PASSES,
};
pub const DETECTOR_PARTICIPANT_REF: &str = "analysis:content-detectors";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AnalysisContext {
    pub clip_kind: String,
    pub capture_source: Option<String>,
    pub original_text: Option<String>,
    pub file_references: Option<Vec<String>>,
    pub image_bytes: Option<Vec<u8>>,
    pub searchable_text: Option<String>,
    pub detected_type: Option<String>,
    pub matched_detector_ref: Option<String>,
    pub structural_metadata: Option<crate::content_inspection::StructuralMetadata>,
    pub media_metadata: Option<crate::content_inspection::MediaMetadata>,
    pub recommendations: Option<crate::content_enrichment::SmartActionRecommendations>,
}

impl AnalysisContext {
    pub fn for_text(text: impl Into<String>) -> Self {
        Self {
            clip_kind: "text".into(),
            capture_source: None,
            original_text: Some(text.into()),
            file_references: None,
            image_bytes: None,
            searchable_text: None,
            detected_type: None,
            matched_detector_ref: None,
            structural_metadata: None,
            media_metadata: None,
            recommendations: None,
        }
    }

    pub fn for_image(image_bytes: Vec<u8>) -> Self {
        Self {
            clip_kind: "image".into(),
            capture_source: None,
            original_text: None,
            file_references: None,
            image_bytes: Some(image_bytes),
            searchable_text: None,
            detected_type: None,
            matched_detector_ref: None,
            structural_metadata: None,
            media_metadata: None,
            recommendations: None,
        }
    }

    pub fn with_searchable_text(mut self, searchable_text: Option<String>) -> Self {
        self.searchable_text = searchable_text;
        self
    }

    pub fn with_capture_source(mut self, capture_source: Option<String>) -> Self {
        self.capture_source = capture_source;
        self
    }

    pub fn has(&self, kind: RepresentationKind) -> bool {
        match kind {
            RepresentationKind::ClipKind => !self.clip_kind.is_empty(),
            RepresentationKind::CaptureSource => self.capture_source.is_some(),
            RepresentationKind::OriginalText => self.original_text.is_some(),
            RepresentationKind::FileReferences => self.file_references.is_some(),
            RepresentationKind::ImageBytes => self.image_bytes.is_some(),
            RepresentationKind::SearchableText => self.searchable_text.is_some(),
            RepresentationKind::AnalyzableText => self.analysis_text().is_some(),
            RepresentationKind::Classification => self.detected_type.is_some(),
            RepresentationKind::StructuralMetadata => self.structural_metadata.is_some(),
            RepresentationKind::MediaMetadata => self.media_metadata.is_some(),
            RepresentationKind::Recommendations => self.recommendations.is_some(),
        }
    }

    pub fn analysis_text(&self) -> Option<&str> {
        self.original_text
            .as_deref()
            .or(self.searchable_text.as_deref())
    }
}

#[derive(Clone)]
pub(crate) enum AnalysisInput {
    Text {
        text: String,
        source: Option<String>,
    },
    Image {
        image_bytes: Vec<u8>,
        searchable_text: Option<String>,
        source: Option<String>,
    },
    Files {
        paths: Vec<String>,
        source: Option<String>,
    },
}

impl AnalysisInput {
    fn into_context(self) -> AnalysisContext {
        match self {
            Self::Text { text, source } => {
                AnalysisContext::for_text(text).with_capture_source(source)
            }
            Self::Image {
                image_bytes,
                searchable_text,
                source,
            } => AnalysisContext::for_image(image_bytes)
                .with_searchable_text(searchable_text)
                .with_capture_source(source),
            Self::Files { paths, source } => AnalysisContext {
                clip_kind: "file".into(),
                capture_source: source,
                original_text: None,
                file_references: Some(paths),
                image_bytes: None,
                searchable_text: None,
                detected_type: None,
                matched_detector_ref: None,
                structural_metadata: None,
                media_metadata: None,
                recommendations: None,
            },
        }
    }
}

pub(crate) struct ExtractorParticipantSource<'a> {
    pub extractor: &'a Extractor,
    pub registry: &'a ExtractorEngineRegistry<'a>,
}

pub(crate) struct EnricherParticipantSource<'a> {
    pub transforms: &'a [crate::db::TransformDefinition],
}

pub(crate) struct AnalysisRequest<'a> {
    pub input: AnalysisInput,
    pub policy: AnalysisPolicy,
    pub inspector: bool,
    pub extractor: Option<ExtractorParticipantSource<'a>>,
    pub detectors: Option<&'a [Detector]>,
    pub enricher: Option<EnricherParticipantSource<'a>>,
}

fn inspector_participant(input: AnalysisInput) -> AnalysisParticipant<'static> {
    AnalysisParticipant::new(
        ParticipantContract {
            stable_ref: crate::content_inspection::STRUCTURE_INSPECTOR_REF.into(),
            name: "Structure".into(),
            pass: AnalysisPass::Inspect,
            priority: 0,
            requires: vec![RepresentationKind::ClipKind],
            provides: vec![RepresentationKind::StructuralMetadata],
        },
        move |context| match crate::content_inspection::inspect_input(&input) {
            Ok(metadata) => {
                context.structural_metadata = Some(metadata);
                Ok(ParticipantOutcome::Produced)
            }
            Err(failure) => Err(failure),
        },
    )
}

fn media_inspector_participant(paths: Vec<String>) -> AnalysisParticipant<'static> {
    AnalysisParticipant::new(
        ParticipantContract {
            stable_ref: crate::content_inspection::MEDIA_INSPECTOR_REF.into(),
            name: "Media Metadata".into(),
            pass: AnalysisPass::Inspect,
            priority: 10,
            requires: vec![RepresentationKind::FileReferences],
            provides: vec![RepresentationKind::MediaMetadata],
        },
        move |context| match crate::content_inspection::inspect_media_paths(&paths)? {
            Some(metadata) => {
                context.media_metadata = Some(metadata);
                Ok(ParticipantOutcome::Produced)
            }
            None => Ok(ParticipantOutcome::NoOutput),
        },
    )
}

pub(crate) struct AnalysisReport {
    pub context: AnalysisContext,
    pub runs: Vec<ParticipantRun>,
}

pub(crate) struct ParticipantResolution {
    pub outcome: ParticipantOutcome,
    pub failure: Option<AnalysisFailure>,
}

impl AnalysisReport {
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

struct AnalysisParticipant<'a> {
    pub contract: ParticipantContract,
    execute: ParticipantExecutor<'a>,
}

impl<'a> AnalysisParticipant<'a> {
    fn new(
        contract: ParticipantContract,
        execute: impl FnMut(&mut AnalysisContext) -> Result<ParticipantOutcome, AnalysisFailure> + 'a,
    ) -> Self {
        Self {
            contract,
            execute: Box::new(execute),
        }
    }
}

fn schedule(
    context: AnalysisContext,
    mut participants: Vec<AnalysisParticipant<'_>>,
    through: AnalysisPass,
) -> AnalysisReport {
    participants.retain(|participant| through.includes(participant.contract.pass));
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

fn extractor_participant<'a>(
    extractor: &'a Extractor,
    registry: &'a ExtractorEngineRegistry<'a>,
) -> AnalysisParticipant<'a> {
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
    AnalysisParticipant::new(
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
    )
}

fn enricher_participant<'a>(
    transforms: &'a [crate::db::TransformDefinition],
) -> AnalysisParticipant<'a> {
    AnalysisParticipant::new(
        crate::content_enrichment::smart_actions_enricher_definition().participant_contract(),
        move |context| {
            let Some(text) = context.analysis_text() else {
                return Ok(ParticipantOutcome::NoOutput);
            };
            let Some(structure) = context.structural_metadata.as_ref() else {
                return Ok(ParticipantOutcome::NoOutput);
            };
            let recommendations = crate::content_enrichment::recommend_smart_actions(
                text,
                context.detected_type.as_deref(),
                structure,
                transforms,
            );
            context.recommendations = Some(recommendations);
            Ok(ParticipantOutcome::Produced)
        },
    )
}

pub(crate) fn analyze(request: AnalysisRequest<'_>) -> AnalysisReport {
    let mut participants = Vec::new();
    if request.inspector {
        participants.push(inspector_participant(request.input.clone()));
        if request.policy == AnalysisPolicy::Interactive {
            if let AnalysisInput::Files { paths, .. } = &request.input {
                let probe_paths = paths
                    .iter()
                    .take(crate::resource_limits::MAX_MEDIA_PROBE_FILES)
                    .cloned()
                    .collect();
                participants.push(media_inspector_participant(probe_paths));
            }
        }
    }
    if let Some(source) = request.extractor {
        participants.push(extractor_participant(source.extractor, source.registry));
    }
    if let Some(detectors) = request.detectors {
        participants.push(detector_participant(detectors));
    }
    if let Some(source) = request.enricher {
        participants.push(enricher_participant(source.transforms));
    }
    schedule(
        request.input.into_context(),
        participants,
        request.policy.through(),
    )
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

    fn analyze_test_image(
        image_bytes: Vec<u8>,
        extractor: &Extractor,
        detectors: Option<&[Detector]>,
        registry: &ExtractorEngineRegistry<'_>,
    ) -> AnalysisReport {
        analyze(AnalysisRequest {
            input: AnalysisInput::Image {
                image_bytes,
                searchable_text: None,
                source: None,
            },
            policy: AnalysisPolicy::Interactive,
            inspector: false,
            extractor: Some(ExtractorParticipantSource {
                extractor,
                registry,
            }),
            detectors,
            enricher: None,
        })
    }

    fn analyze_test_text(text: &str, detectors: &[Detector]) -> AnalysisReport {
        analyze(AnalysisRequest {
            input: AnalysisInput::Text {
                text: text.into(),
                source: None,
            },
            policy: AnalysisPolicy::Capture,
            inspector: false,
            extractor: None,
            detectors: Some(detectors),
            enricher: None,
        })
    }

    #[test]
    fn media_inspection_is_interactive_only() {
        let request = |policy| AnalysisRequest {
            input: AnalysisInput::Files {
                paths: vec!["/missing/private-recording.wav".into()],
                source: Some("Finder".into()),
            },
            policy,
            inspector: true,
            extractor: None,
            detectors: None,
            enricher: None,
        };
        let capture = analyze(request(AnalysisPolicy::Capture));
        assert_eq!(capture.runs.len(), 1);
        assert_eq!(
            capture.runs[0].stable_ref,
            crate::content_inspection::STRUCTURE_INSPECTOR_REF
        );

        let interactive = analyze(request(AnalysisPolicy::Interactive));
        assert_eq!(interactive.runs.len(), 2);
        assert!(interactive
            .runs
            .iter()
            .any(|run| run.stable_ref == crate::content_inspection::MEDIA_INSPECTOR_REF));
    }

    #[test]
    fn extraction_makes_text_available_to_later_detection() {
        let detectors = vec![detector(r"^[^@]+@[^@]+\.[^@]+$", "email")];
        let engine = TestEngine;
        let engines: [&dyn crate::content_extraction::ExtractorEngine; 1] = [&engine];
        let registry = ExtractorEngineRegistry::new(&engines);
        let report = analyze_test_image(vec![1, 2, 3], &extractor(), Some(&detectors), &registry);

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
            AnalysisPass::Enrich,
        )
        .runs;

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].outcome, ParticipantOutcome::MissingInput);
    }

    #[test]
    fn bounded_policies_exclude_later_participants_without_reporting_fake_runs() {
        let report = schedule(
            AnalysisContext::for_text("hello"),
            vec![AnalysisParticipant::new(
                ParticipantContract {
                    stable_ref: "enricher:test".into(),
                    name: "Test Enricher".into(),
                    pass: AnalysisPass::Enrich,
                    priority: 1,
                    requires: vec![RepresentationKind::AnalyzableText],
                    provides: vec![],
                },
                |_| panic!("capture policy must not execute Enrich participants"),
            )],
            AnalysisPolicy::Capture.through(),
        );

        assert!(report.runs.is_empty());
    }

    #[test]
    fn typed_inputs_carry_capture_source_without_changing_analyzable_text() {
        let report = analyze(AnalysisRequest {
            input: AnalysisInput::Text {
                text: "hello".into(),
                source: Some("Terminal".into()),
            },
            policy: AnalysisPolicy::Capture,
            inspector: false,
            extractor: None,
            detectors: None,
            enricher: None,
        });

        assert!(report.context.has(RepresentationKind::CaptureSource));
        assert_eq!(report.context.analysis_text(), Some("hello"));
        assert!(report.runs.is_empty());
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
            AnalysisPass::Enrich,
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
        let report = analyze_test_image(vec![1], &extractor(), None, &registry);

        assert_eq!(report.context.searchable_text, None);
        assert_eq!(report.runs[0].outcome, ParticipantOutcome::Failed);
        assert_eq!(
            report.runs[0].failure.as_ref(),
            Some(&AnalysisFailure {
                code: "test_failure".into(),
                message: "The test engine failed.".into(),
            })
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
            AnalysisPass::Enrich,
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
        let report = analyze_test_text("#112233", &detectors);
        assert_eq!(report.context.detected_type.as_deref(), Some("color"));
        assert_eq!(report.runs[0].stable_ref, "analysis:content-detectors");
    }
}
