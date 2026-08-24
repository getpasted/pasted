use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisClassification {
    pub id: i64,
    pub clip_id: i64,
    pub content_type: String,
    pub classifier_ref: String,
    pub classifier_name: String,
    pub priority: i64,
    pub source_representation: String,
    pub input_hash: String,
    pub start_offset: Option<usize>,
    pub end_offset: Option<usize>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClipSearchableText {
    pub clip_id: i64,
    pub extractor_ref: String,
    pub extractor_name: String,
    pub engine: String,
    pub input_hash: String,
    pub searchable_text: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisFailureClass {
    Terminal,
    Dependency,
    Transient,
}

impl AnalysisFailureClass {
    pub(crate) const fn as_str(&self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Dependency => "dependency",
            Self::Transient => "transient",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoredExtractionObservation {
    #[serde(flatten)]
    pub observation: crate::content_analysis::ExtractionObservation,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoredExtractionAttempt {
    #[serde(flatten)]
    pub observation: crate::content_analysis::ExtractionObservation,
    pub run_id: String,
    pub run_at: String,
    pub input_fingerprint: String,
    pub failure_class: Option<AnalysisFailureClass>,
    pub retry_after: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionAttemptContext {
    pub participant_ref: String,
    pub input_fingerprint: String,
}
