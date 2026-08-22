use crate::analysis_contract::RepresentationKind;
use crate::content_extraction::{ExtractionOutcome, Extractor, ExtractorEngineRegistry};
use crate::content_inspection::FileFormatInspection;

pub(super) fn requirements(input: RepresentationKind) -> Vec<RepresentationKind> {
    if input == RepresentationKind::FileReferences {
        vec![input, RepresentationKind::FileFormats]
    } else {
        vec![input]
    }
}

pub(super) fn execute(
    extractor: &Extractor,
    registry: &ExtractorEngineRegistry<'_>,
    paths: &[String],
    inspection: Option<&FileFormatInspection>,
) -> ExtractionOutcome {
    let Some(routes) = inspection.map(|inspection| inspection.routes.as_slice()) else {
        return ExtractionOutcome::NoOutput;
    };
    let eligible = crate::content_extraction::file_routing::eligible_paths(
        &extractor.recipe.accepted_file_formats,
        paths,
        routes,
    );
    if eligible.is_empty() {
        ExtractionOutcome::NoOutput
    } else {
        registry.execute_files(extractor, &eligible)
    }
}
