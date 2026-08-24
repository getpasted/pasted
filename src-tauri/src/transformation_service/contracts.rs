use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionTrigger {
    Manual,
    Shortcut,
    Bin,
    Automation,
    Cli,
}

impl ExecutionTrigger {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Shortcut => "shortcut",
            Self::Bin => "bin",
            Self::Automation => "automation",
            Self::Cli => "cli",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionDestination {
    #[default]
    Preview,
    Replace,
    Copy,
    Paste,
    Route,
}

impl ExecutionDestination {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Preview => "preview",
            Self::Replace => "replace",
            Self::Copy => "copy",
            Self::Paste => "paste",
            Self::Route => "route",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ExecutionTarget {
    Transform {
        transform_ref: String,
    },
    Operation {
        operation_ref: String,
    },
    #[serde(alias = "pipeline")]
    ManualTransform {
        #[serde(alias = "pipelineRef")]
        transform_ref: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionRequest {
    pub input: String,
    pub target: ExecutionTarget,
    pub source_clip_id: Option<i64>,
    pub trigger: ExecutionTrigger,
    #[serde(default)]
    pub destination: ExecutionDestination,
    #[serde(default)]
    pub client_request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionOutcome {
    pub execution_id: String,
    pub output: String,
    pub connection_id: Option<String>,
    pub connection_name: Option<String>,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionError {
    pub code: &'static str,
    pub message: String,
    pub step: Option<usize>,
    pub operation_ref: Option<String>,
}

impl ExecutionError {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            step: None,
            operation_ref: None,
        }
    }

    pub(crate) fn at_step(mut self, step: usize, operation_ref: &str) -> Self {
        self.step = Some(step);
        self.operation_ref = Some(operation_ref.to_string());
        self
    }

    pub(crate) fn safe_summary(&self) -> String {
        let summary = match (self.step, self.operation_ref.as_deref()) {
            (Some(step), Some(operation_ref)) => format!(
                "{} at manual Transform step {} ({}): {}",
                self.code, step, operation_ref, self.message
            ),
            _ => format!("{}: {}", self.code, self.message),
        };
        summary.chars().take(512).collect()
    }
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.safe_summary())
    }
}

pub(crate) fn database_error(error: impl fmt::Display) -> ExecutionError {
    ExecutionError::new("database_error", error.to_string())
}
