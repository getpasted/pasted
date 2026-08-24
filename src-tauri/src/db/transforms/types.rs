use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Pipeline {
    pub id: i64,
    pub stable_ref: String,
    pub name: String,
    #[serde(rename = "hotkey", alias = "shortcut")]
    pub shortcut: Option<String>,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
    pub steps: Vec<PipelineStep>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SavedTransform {
    pub id: i64,
    pub stable_ref: String,
    pub name: String,
    pub plan: crate::transformation_intent::TransformationPlan,
    pub connection_id: Option<String>,
    #[serde(default)]
    #[serde(rename = "hotkey", alias = "shortcut")]
    pub shortcut: Option<String>,
    #[serde(default = "default_transform_authoring_kind")]
    pub authoring_kind: String,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
}

fn default_transform_authoring_kind() -> String {
    "intent".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransformAuthoringKind {
    Intent,
    Manual,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransformDefinition {
    pub id: i64,
    pub stable_ref: String,
    pub name: String,
    pub authoring_kind: TransformAuthoringKind,
    pub execution_character: String,
    pub connection_id: Option<String>,
    #[serde(rename = "hotkey", alias = "shortcut")]
    pub shortcut: Option<String>,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
    pub plan: Option<crate::transformation_intent::TransformationPlan>,
    pub steps: Vec<PipelineStep>,
}

impl From<SavedTransform> for TransformDefinition {
    fn from(transform: SavedTransform) -> Self {
        let execution_character = match transform.plan.execution_character() {
            crate::transformation_intent::ExecutionCharacter::Replayable => "replayable",
            crate::transformation_intent::ExecutionCharacter::Interpretive => "interpretive",
            crate::transformation_intent::ExecutionCharacter::Mixed => "mixed",
        }
        .to_string();
        let is_manual = transform.authoring_kind == "manual";
        let manual_steps = if is_manual {
            transform
                .plan
                .steps
                .iter()
                .enumerate()
                .filter_map(|(position, step)| match &step.executor {
                    crate::transformation_intent::PlannedExecutor::Deterministic {
                        operation_ref,
                        config_json,
                    } => Some(PipelineStep {
                        position: position as i64,
                        operation_ref: operation_ref.clone(),
                        config_json: config_json.clone(),
                        failure_policy: match step.failure_policy {
                            crate::transformation_intent::StepFailurePolicy::Stop => "stop",
                            crate::transformation_intent::StepFailurePolicy::Skip => "skip",
                        }
                        .to_string(),
                    }),
                    crate::transformation_intent::PlannedExecutor::Semantic { .. } => None,
                })
                .collect()
        } else {
            Vec::new()
        };
        Self {
            id: transform.id,
            stable_ref: transform.stable_ref,
            name: transform.name,
            authoring_kind: if is_manual {
                TransformAuthoringKind::Manual
            } else {
                TransformAuthoringKind::Intent
            },
            execution_character,
            connection_id: transform.connection_id,
            shortcut: transform.shortcut,
            revision: transform.revision,
            created_at: transform.created_at,
            updated_at: transform.updated_at,
            plan: (!is_manual).then_some(transform.plan),
            steps: manual_steps,
        }
    }
}

impl From<Pipeline> for TransformDefinition {
    fn from(pipeline: Pipeline) -> Self {
        Self {
            id: pipeline.id,
            stable_ref: pipeline.stable_ref,
            name: pipeline.name,
            authoring_kind: TransformAuthoringKind::Manual,
            execution_character: "replayable".to_string(),
            connection_id: None,
            shortcut: pipeline.shortcut,
            revision: pipeline.revision,
            created_at: pipeline.created_at,
            updated_at: pipeline.updated_at,
            plan: None,
            steps: pipeline.steps,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClipTransformationProvenance {
    pub transform_ref: String,
    pub transform_name: String,
    pub transform_revision: i64,
    pub connection_id: Option<String>,
    pub duration_ms: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransformationExecution {
    pub id: String,
    pub target_kind: String,
    pub target_ref: String,
    pub target_revision: Option<i64>,
    pub source_clip_id: Option<i64>,
    pub trigger_kind: String,
    pub destination_kind: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub status: String,
    pub error_summary: Option<String>,
}

pub struct TransformationExecutionStart<'a> {
    pub target_kind: &'a str,
    pub target_ref: &'a str,
    pub target_revision: Option<i64>,
    pub source_clip_id: Option<i64>,
    pub trigger_kind: &'a str,
    pub destination_kind: &'a str,
    pub input_hash: &'a str,
}

pub struct TransformClipApplication<'a> {
    pub clip_id: i64,
    pub transform_ref: &'a str,
    pub expected_input: &'a str,
    pub output: &'a str,
    pub connection_id: Option<&'a str>,
    pub duration_ms: i64,
    pub bin_move: Option<(Option<i64>, i64)>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStepInput {
    pub operation_ref: String,
    pub config_json: Option<String>,
    #[serde(default = "default_pipeline_failure_policy")]
    pub failure_policy: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStep {
    pub position: i64,
    pub operation_ref: String,
    pub config_json: Option<String>,
    pub failure_policy: String,
}

fn default_pipeline_failure_policy() -> String {
    "stop".to_string()
}
