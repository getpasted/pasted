use super::*;

pub(super) fn participant(paths: Vec<String>) -> AnalysisParticipant<'static> {
    AnalysisParticipant::new(
        ParticipantContract {
            stable_ref: crate::content_inspection::FILE_FORMAT_INSPECTOR_REF.into(),
            name: "File Format".into(),
            pass: AnalysisPass::Inspect,
            priority: 10,
            requires: vec![RepresentationKind::FileReferences],
            provides: vec![RepresentationKind::FileFormats],
        },
        move |context| {
            let inspection = crate::content_inspection::inspect_file_formats(&paths);
            let outcome = if inspection.formats.is_empty() {
                ParticipantOutcome::NoOutput
            } else {
                ParticipantOutcome::Produced
            };
            context.file_formats = Some(inspection);
            Ok(outcome)
        },
    )
}
