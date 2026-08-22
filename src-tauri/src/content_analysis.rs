use crate::content_classification::{classify_with_classifiers, Classifier};
use crate::content_extraction::{ExtractionOutcome, Extractor, ExtractorEngineRegistry};
use serde::{Deserialize, Serialize};

pub use crate::analysis_contract::{
    AnalysisFailure, AnalysisPass, AnalysisPolicy, AnalysisTargetKind, ParticipantContract,
    ParticipantOutcome, ParticipantRun, RepresentationKind, ANALYSIS_CONTRACT_VERSION,
    MAX_ANALYSIS_PASSES,
};
pub const CLASSIFIER_PARTICIPANT_REF: &str = "analysis:content-classifiers";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionObservation {
    pub extractor_ref: String,
    pub extractor_name: String,
    pub engine: String,
    pub priority: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_of: Option<String>,
    #[serde(flatten)]
    pub outcome: ExtractionOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AnalysisContext {
    pub clip_kind: String,
    pub capture_source: Option<String>,
    pub original_text: Option<String>,
    pub file_references: Option<Vec<String>>,
    pub image_bytes: Option<Vec<u8>>,
    pub searchable_text: Option<String>,
    pub extraction_observations: Vec<ExtractionObservation>,
    pub classification_matches: Vec<crate::content_classification::ClassificationMatch>,
    pub classification_complete: bool,
    pub structural_metadata: Option<crate::content_inspection::StructuralMetadata>,
    pub file_formats: Option<crate::content_inspection::FileFormatInspection>,
    pub media_metadata: Option<crate::content_inspection::MediaMetadata>,
    pub suggestions: Option<crate::content_suggestions::SmartActionSuggestions>,
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
            extraction_observations: Vec::new(),
            classification_matches: Vec::new(),
            classification_complete: false,
            structural_metadata: None,
            file_formats: None,
            media_metadata: None,
            suggestions: None,
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
            extraction_observations: Vec::new(),
            classification_matches: Vec::new(),
            classification_complete: false,
            structural_metadata: None,
            file_formats: None,
            media_metadata: None,
            suggestions: None,
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
            RepresentationKind::Classification => self.classification_complete,
            RepresentationKind::StructuralMetadata => self.structural_metadata.is_some(),
            RepresentationKind::FileFormats => self.file_formats.is_some(),
            RepresentationKind::MediaMetadata => self.media_metadata.is_some(),
            RepresentationKind::Suggestions => self.suggestions.is_some(),
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
                extraction_observations: Vec::new(),
                classification_matches: Vec::new(),
                classification_complete: false,
                structural_metadata: None,
                file_formats: None,
                media_metadata: None,
                suggestions: None,
            },
        }
    }
}

pub(crate) struct ExtractorParticipantSource<'a> {
    pub extractor: &'a Extractor,
    pub registry: &'a ExtractorEngineRegistry<'a>,
}

pub(crate) struct SuggestionParticipantSource<'a> {
    pub transforms: &'a [crate::db::TransformDefinition],
}

pub(crate) struct AnalysisRequest<'a> {
    pub input: AnalysisInput,
    pub policy: AnalysisPolicy,
    pub inspector: bool,
    pub file_format_inspector: bool,
    pub extractors: Vec<ExtractorParticipantSource<'a>>,
    pub classifiers: Option<&'a [Classifier]>,
    pub suggestion: Option<SuggestionParticipantSource<'a>>,
}

mod pipeline;
pub(crate) use pipeline::{analyze, AnalysisReport};
#[cfg(test)]
use pipeline::{schedule, AnalysisParticipant};
#[cfg(test)]
mod tests;
