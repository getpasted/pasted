use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

pub const ANALYSIS_CONTRACT_VERSION: u32 = 1;
pub const MAX_ANALYSIS_PASSES: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisPass {
    Inspect,
    Extract,
    Classify,
    Enrich,
}

impl AnalysisPass {
    pub(crate) const ORDERED: [Self; MAX_ANALYSIS_PASSES] =
        [Self::Inspect, Self::Extract, Self::Classify, Self::Enrich];

    pub const fn includes(self, pass: Self) -> bool {
        match self {
            Self::Inspect => matches!(pass, Self::Inspect),
            Self::Extract => matches!(pass, Self::Inspect | Self::Extract),
            Self::Classify => matches!(pass, Self::Inspect | Self::Extract | Self::Classify),
            Self::Enrich => true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisPolicy {
    Capture,
    Background,
    Interactive,
    Rescan,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisEnvelope<T> {
    pub format_version: u32,
    pub policy: AnalysisPolicy,
    pub through: AnalysisPass,
    pub result: T,
    pub participants: Vec<ParticipantRun>,
}

impl<T> AnalysisEnvelope<T> {
    pub fn new(policy: AnalysisPolicy, result: T, participants: Vec<ParticipantRun>) -> Self {
        Self {
            format_version: ANALYSIS_CONTRACT_VERSION,
            policy,
            through: policy.through(),
            result,
            participants,
        }
    }
}

impl AnalysisPolicy {
    pub const fn through(self) -> AnalysisPass {
        match self {
            Self::Capture | Self::Background | Self::Rescan => AnalysisPass::Classify,
            Self::Interactive => AnalysisPass::Enrich,
        }
    }

    pub const fn includes(self, pass: AnalysisPass) -> bool {
        self.through().includes(pass)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisTargetKind {
    Inspector,
    InspectorSet,
    Extractor,
    Detector,
    DetectorSet,
    Enricher,
    EnricherSet,
}

impl AnalysisTargetKind {
    pub(crate) const fn failure_subject(self) -> &'static str {
        match self {
            Self::Inspector | Self::InspectorSet => "Inspection",
            Self::Extractor => "The Extractor",
            Self::Detector | Self::DetectorSet => "Detection",
            Self::Enricher | Self::EnricherSet => "The Enricher",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepresentationKind {
    ClipKind,
    CaptureSource,
    OriginalText,
    #[serde(rename = "image")]
    ImageBytes,
    SearchableText,
    AnalyzableText,
    Classification,
    StructuralMetadata,
    Recommendations,
}

impl RepresentationKind {
    pub const fn stable_name(self) -> &'static str {
        match self {
            Self::ClipKind => "clip_kind",
            Self::CaptureSource => "capture_source",
            Self::OriginalText => "original_text",
            Self::ImageBytes => "image",
            Self::SearchableText => "searchable_text",
            Self::AnalyzableText => "analyzable_text",
            Self::Classification => "classification",
            Self::StructuralMetadata => "structural_metadata",
            Self::Recommendations => "recommendations",
        }
    }
}

impl fmt::Display for RepresentationKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.stable_name())
    }
}

impl FromStr for RepresentationKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "clip_kind" => Ok(Self::ClipKind),
            "capture_source" => Ok(Self::CaptureSource),
            "original_text" => Ok(Self::OriginalText),
            "image" => Ok(Self::ImageBytes),
            "searchable_text" => Ok(Self::SearchableText),
            "analyzable_text" => Ok(Self::AnalyzableText),
            "classification" => Ok(Self::Classification),
            "structural_metadata" => Ok(Self::StructuralMetadata),
            "recommendations" => Ok(Self::Recommendations),
            _ => Err(format!("Unknown analysis representation \"{value}\"")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepresentationContract {
    pub input: RepresentationKind,
    pub output: RepresentationKind,
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipApplication {
    pub applied_clip_id: Option<i64>,
}

impl ClipApplication {
    pub const fn preview() -> Self {
        Self {
            applied_clip_id: None,
        }
    }

    pub const fn applied(clip_id: i64) -> Self {
        Self {
            applied_clip_id: Some(clip_id),
        }
    }
}

impl RepresentationContract {
    pub fn parse(input: &str, output: &str) -> Result<Self, String> {
        Ok(Self {
            input: input.parse()?,
            output: output.parse()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn participant_passes_and_target_kinds_have_stable_names() {
        assert_eq!(
            AnalysisPass::ORDERED,
            [
                AnalysisPass::Inspect,
                AnalysisPass::Extract,
                AnalysisPass::Classify,
                AnalysisPass::Enrich,
            ]
        );
        for (kind, name) in [
            (AnalysisTargetKind::Inspector, "inspector"),
            (AnalysisTargetKind::InspectorSet, "inspector_set"),
            (AnalysisTargetKind::Extractor, "extractor"),
            (AnalysisTargetKind::Detector, "detector"),
            (AnalysisTargetKind::DetectorSet, "detector_set"),
            (AnalysisTargetKind::Enricher, "enricher"),
            (AnalysisTargetKind::EnricherSet, "enricher_set"),
        ] {
            assert_eq!(serde_json::to_string(&kind).unwrap(), format!("\"{name}\""));
        }
    }

    #[test]
    fn analysis_policies_bound_work_at_the_expected_pass() {
        assert_eq!(ANALYSIS_CONTRACT_VERSION, 1);
        for policy in [
            AnalysisPolicy::Capture,
            AnalysisPolicy::Background,
            AnalysisPolicy::Rescan,
        ] {
            assert!(policy.includes(AnalysisPass::Inspect));
            assert!(policy.includes(AnalysisPass::Classify));
            assert!(!policy.includes(AnalysisPass::Enrich));
        }
        assert!(AnalysisPolicy::Interactive.includes(AnalysisPass::Enrich));
    }

    #[test]
    fn public_analysis_envelopes_are_explicitly_versioned() {
        let envelope = AnalysisEnvelope::new(
            AnalysisPolicy::Interactive,
            serde_json::json!({ "kind": "test" }),
            Vec::new(),
        );
        assert_eq!(
            serde_json::to_value(envelope).unwrap(),
            serde_json::json!({
                "formatVersion": 1,
                "policy": "interactive",
                "through": "enrich",
                "result": { "kind": "test" },
                "participants": []
            })
        );
    }

    #[test]
    fn clip_application_flattens_to_the_shared_json_field() {
        assert_eq!(
            serde_json::to_value(ClipApplication::preview()).unwrap(),
            serde_json::json!({ "appliedClipId": null })
        );
        assert_eq!(
            serde_json::to_value(ClipApplication::applied(42)).unwrap(),
            serde_json::json!({ "appliedClipId": 42 })
        );
    }

    #[test]
    fn representation_names_round_trip_through_the_shared_contract() {
        for kind in [
            RepresentationKind::ClipKind,
            RepresentationKind::CaptureSource,
            RepresentationKind::OriginalText,
            RepresentationKind::ImageBytes,
            RepresentationKind::SearchableText,
            RepresentationKind::AnalyzableText,
            RepresentationKind::Classification,
            RepresentationKind::StructuralMetadata,
            RepresentationKind::Recommendations,
        ] {
            assert_eq!(kind.stable_name().parse::<RepresentationKind>(), Ok(kind));
            assert_eq!(
                serde_json::to_string(&kind).unwrap(),
                format!("\"{}\"", kind.stable_name())
            );
            assert_eq!(
                serde_json::from_str::<RepresentationKind>(&format!("\"{}\"", kind.stable_name()))
                    .unwrap(),
                kind
            );
        }
    }

    #[test]
    fn unknown_representation_names_fail_closed() {
        assert_eq!(
            RepresentationContract::parse("image", "mystery").unwrap_err(),
            "Unknown analysis representation \"mystery\""
        );
    }
}
