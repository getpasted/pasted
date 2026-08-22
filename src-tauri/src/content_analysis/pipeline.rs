use super::*;

mod file_extraction;
mod file_inspection;

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
            priority: 20,
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

pub(super) struct AnalysisParticipant<'a> {
    pub contract: ParticipantContract,
    execute: ParticipantExecutor<'a>,
}

impl<'a> AnalysisParticipant<'a> {
    pub(super) fn new(
        contract: ParticipantContract,
        execute: impl FnMut(&mut AnalysisContext) -> Result<ParticipantOutcome, AnalysisFailure> + 'a,
    ) -> Self {
        Self {
            contract,
            execute: Box::new(execute),
        }
    }
}

pub(super) fn schedule(
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

fn classifier_participant<'a>(classifiers: &'a [Classifier]) -> AnalysisParticipant<'a> {
    AnalysisParticipant::new(
        ParticipantContract {
            stable_ref: CLASSIFIER_PARTICIPANT_REF.into(),
            name: "Content Classifiers".into(),
            pass: AnalysisPass::Classify,
            priority: 0,
            requires: vec![RepresentationKind::AnalyzableText],
            provides: vec![RepresentationKind::Classification],
        },
        move |context| {
            let Some(text) = context.analysis_text() else {
                return Ok(ParticipantOutcome::NoOutput);
            };
            context.classification_matches = classify_with_classifiers(text, classifiers);
            context.classification_complete = true;
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
            requires: file_extraction::requirements(representation_contract.input),
            provides: provided_representations,
        },
        move |context| {
            let outcome = match representation_contract.input {
                RepresentationKind::ImageBytes => {
                    let Some(image_bytes) = context.image_bytes.as_deref() else {
                        return Ok(ParticipantOutcome::NoOutput);
                    };
                    registry.execute(extractor, image_bytes)
                }
                RepresentationKind::FileReferences => {
                    let Some(paths) = context.file_references.as_deref() else {
                        return Ok(ParticipantOutcome::NoOutput);
                    };
                    file_extraction::execute(
                        extractor,
                        registry,
                        paths,
                        context.file_formats.as_ref(),
                    )
                }
                _ => ExtractionOutcome::Failed {
                    failure: crate::content_extraction::ExtractionFailure {
                        code: "invalid_contract".into(),
                        message: "This extraction input is not supported.".into(),
                    },
                },
            };
            let duplicate_of = match &outcome {
                ExtractionOutcome::Produced { text } => context
                    .extraction_observations
                    .iter()
                    .find_map(|observation| {
                        matches!(
                            &observation.outcome,
                            ExtractionOutcome::Produced { text: existing } if existing == text
                        )
                        .then(|| observation.extractor_ref.clone())
                    }),
                _ => None,
            };
            let outcome = match outcome {
                ExtractionOutcome::Produced { text }
                    if duplicate_of.is_none()
                        && context.searchable_text.as_ref().is_some_and(|current| {
                            current.len().saturating_add(text.len()).saturating_add(1)
                                > crate::resource_limits::MAX_OCR_TEXT_BYTES
                        }) =>
                {
                    ExtractionOutcome::Failed {
                        failure: crate::content_extraction::ExtractionFailure {
                            code: "combined_output_too_large".into(),
                            message: "Combined Extractor output exceeds the supported size limit."
                                .into(),
                        },
                    }
                }
                outcome => outcome,
            };
            context.extraction_observations.push(ExtractionObservation {
                extractor_ref: extractor.stable_ref.clone(),
                extractor_name: extractor.name.clone(),
                engine: extractor.engine.clone(),
                priority: extractor.priority,
                duplicate_of: duplicate_of.clone(),
                outcome: outcome.clone(),
            });
            match outcome {
                ExtractionOutcome::Produced { text } => {
                    if duplicate_of.is_none() {
                        if let Some(current) = context.searchable_text.as_mut() {
                            if !current.is_empty() {
                                current.push('\n');
                            }
                            current.push_str(&text);
                        } else {
                            context.searchable_text = Some(text);
                        }
                    }
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

fn suggestion_participant<'a>(
    transforms: &'a [crate::db::TransformDefinition],
) -> AnalysisParticipant<'a> {
    AnalysisParticipant::new(
        crate::content_suggestions::smart_actions_suggestion_definition().participant_contract(),
        move |context| {
            let Some(text) = context.analysis_text() else {
                return Ok(ParticipantOutcome::NoOutput);
            };
            let Some(structure) = context.structural_metadata.as_ref() else {
                return Ok(ParticipantOutcome::NoOutput);
            };
            let suggestions = crate::content_suggestions::suggest_smart_actions(
                text,
                &context
                    .classification_matches
                    .iter()
                    .map(|matched| matched.content_type.clone())
                    .collect::<Vec<_>>(),
                structure,
                transforms,
            );
            context.suggestions = Some(suggestions);
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
    if request.file_format_inspector {
        if let AnalysisInput::Files { paths, .. } = &request.input {
            participants.push(file_inspection::participant(paths.clone()));
        }
    }
    for source in request.extractors {
        participants.push(extractor_participant(source.extractor, source.registry));
    }
    if let Some(classifiers) = request.classifiers {
        participants.push(classifier_participant(classifiers));
    }
    if let Some(source) = request.suggestion {
        participants.push(suggestion_participant(source.transforms));
    }
    schedule(
        request.input.into_context(),
        participants,
        request.policy.through(),
    )
}
