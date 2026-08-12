use serde::{Deserialize, Serialize};

use crate::operation_registry::is_builtin_operation;

pub const TRANSFORMATION_PLAN_SCHEMA_VERSION: u32 = 1;
const MAX_PLAN_STEPS: usize = 32;
const MAX_INTENT_LENGTH: usize = 8_000;
const MAX_STEP_INSTRUCTION_LENGTH: usize = 12_000;
const MAX_SERIALIZED_PLAN_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentPlanningMode {
    Pinned,
    Adaptive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionCharacter {
    Replayable,
    Interpretive,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepExecutionScope {
    WholeInput,
    EachLine,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepFailurePolicy {
    #[default]
    Stop,
    Skip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelPolicy {
    Fast,
    Balanced,
    Deep,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlannedExecutor {
    Deterministic {
        operation_ref: String,
        #[serde(default)]
        config_json: Option<String>,
    },
    Semantic {
        instructions: String,
        #[serde(default)]
        output_schema: Option<serde_json::Value>,
        model_policy: ModelPolicy,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedTransformationStep {
    pub name: String,
    pub rationale: String,
    pub scope: StepExecutionScope,
    #[serde(default)]
    pub failure_policy: StepFailurePolicy,
    pub executor: PlannedExecutor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransformationPlan {
    pub schema_version: u32,
    pub intent: String,
    pub summary: String,
    pub planning_mode: IntentPlanningMode,
    pub steps: Vec<PlannedTransformationStep>,
}

impl TransformationPlan {
    pub fn execution_character(&self) -> ExecutionCharacter {
        let deterministic_count = self
            .steps
            .iter()
            .filter(|step| matches!(step.executor, PlannedExecutor::Deterministic { .. }))
            .count();
        match deterministic_count {
            0 => ExecutionCharacter::Interpretive,
            count if count == self.steps.len() => ExecutionCharacter::Replayable,
            _ => ExecutionCharacter::Mixed,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if serde_json::to_vec(self)
            .map(|plan| plan.len() > MAX_SERIALIZED_PLAN_BYTES)
            .unwrap_or(true)
        {
            return Err("Transformation plan exceeds Pasted's 256 KB safety limit".to_string());
        }
        if self.schema_version != TRANSFORMATION_PLAN_SCHEMA_VERSION {
            return Err(format!(
                "Unsupported transformation plan schema version: {}",
                self.schema_version
            ));
        }
        let intent = self.intent.trim();
        if intent.is_empty() {
            return Err("Transformation intent cannot be empty".to_string());
        }
        if intent.chars().count() > MAX_INTENT_LENGTH {
            return Err("Transformation intent is too long".to_string());
        }
        if self.summary.trim().is_empty() {
            return Err("Transformation plan summary cannot be empty".to_string());
        }
        if self.steps.is_empty() {
            return Err("Transformation plan must contain at least one step".to_string());
        }
        if self.steps.len() > MAX_PLAN_STEPS {
            return Err(format!(
                "Transformation plans may contain at most {MAX_PLAN_STEPS} steps"
            ));
        }

        for (index, step) in self.steps.iter().enumerate() {
            if step.name.trim().is_empty() {
                return Err(format!("Plan step {} has no name", index + 1));
            }
            if step.rationale.trim().is_empty() {
                return Err(format!("Plan step {} has no rationale", index + 1));
            }
            match &step.executor {
                PlannedExecutor::Deterministic {
                    operation_ref,
                    config_json,
                } => {
                    let valid_reference = operation_ref
                        .strip_prefix("builtin:")
                        .is_some_and(is_builtin_operation)
                        || operation_ref
                            .strip_prefix("custom:")
                            .is_some_and(|id| !id.trim().is_empty());
                    if !valid_reference {
                        return Err(format!(
                            "Plan step {} references an unknown Operation: {}",
                            index + 1,
                            operation_ref
                        ));
                    }
                    if let Some(config) = config_json {
                        serde_json::from_str::<serde_json::Value>(config).map_err(|error| {
                            format!("Plan step {} has invalid JSON config: {error}", index + 1)
                        })?;
                    }
                }
                PlannedExecutor::Semantic {
                    instructions,
                    output_schema,
                    ..
                } => {
                    let instructions = instructions.trim();
                    if instructions.is_empty() {
                        return Err(format!(
                            "Semantic plan step {} has no instructions",
                            index + 1
                        ));
                    }
                    if instructions.chars().count() > MAX_STEP_INSTRUCTION_LENGTH {
                        return Err(format!(
                            "Semantic plan step {} instructions are too long",
                            index + 1
                        ));
                    }
                    if let Some(schema) = output_schema {
                        if !schema.is_object() {
                            return Err(format!(
                                "Semantic plan step {} output schema must be a JSON object",
                                index + 1
                            ));
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deterministic_plan() -> TransformationPlan {
        TransformationPlan {
            schema_version: TRANSFORMATION_PLAN_SCHEMA_VERSION,
            intent: "Remove tracking parameters and uppercase the result".to_string(),
            summary: "Clean and capitalize a URL".to_string(),
            planning_mode: IntentPlanningMode::Pinned,
            steps: vec![
                PlannedTransformationStep {
                    name: "Clean URL".to_string(),
                    rationale: "Tracking parameters are mechanical and replayable".to_string(),
                    scope: StepExecutionScope::WholeInput,
                    failure_policy: Default::default(),
                    executor: PlannedExecutor::Deterministic {
                        operation_ref: "builtin:clean_url_tracking".to_string(),
                        config_json: None,
                    },
                },
                PlannedTransformationStep {
                    name: "Uppercase".to_string(),
                    rationale: "Casing is deterministic".to_string(),
                    scope: StepExecutionScope::WholeInput,
                    failure_policy: Default::default(),
                    executor: PlannedExecutor::Deterministic {
                        operation_ref: "builtin:uppercase".to_string(),
                        config_json: None,
                    },
                },
            ],
        }
    }

    #[test]
    fn accepts_a_pinned_replayable_plan() {
        assert_eq!(deterministic_plan().validate(), Ok(()));
    }

    #[test]
    fn rejects_unknown_operations_and_invalid_config() {
        let mut plan = deterministic_plan();
        plan.steps[0].executor = PlannedExecutor::Deterministic {
            operation_ref: "builtin:invented_by_a_model".to_string(),
            config_json: None,
        };
        assert!(plan.validate().unwrap_err().contains("unknown Operation"));

        let mut plan = deterministic_plan();
        plan.steps[0].executor = PlannedExecutor::Deterministic {
            operation_ref: "builtin:uppercase".to_string(),
            config_json: Some("not json".to_string()),
        };
        assert!(plan.validate().unwrap_err().contains("invalid JSON config"));
    }

    #[test]
    fn rejects_oversized_serialized_plans() {
        let mut plan = deterministic_plan();
        plan.steps[0].executor = PlannedExecutor::Deterministic {
            operation_ref: "builtin:uppercase".to_string(),
            config_json: Some(format!(
                "{{\"padding\":\"{}\"}}",
                "x".repeat(MAX_SERIALIZED_PLAN_BYTES)
            )),
        };

        assert!(plan.validate().unwrap_err().contains("256 KB safety limit"));
    }

    #[test]
    fn reports_replayable_interpretive_and_mixed_execution_honestly() {
        let mut plan = deterministic_plan();
        assert_eq!(plan.execution_character(), ExecutionCharacter::Replayable);
        plan.steps = vec![PlannedTransformationStep {
            name: "Rewrite".to_string(),
            rationale: "Tone requires semantic judgment".to_string(),
            scope: StepExecutionScope::WholeInput,
            failure_policy: Default::default(),
            executor: PlannedExecutor::Semantic {
                instructions: "Rewrite this warmly and concisely".to_string(),
                output_schema: None,
                model_policy: ModelPolicy::Balanced,
            },
        }];
        assert_eq!(plan.validate(), Ok(()));
        assert_eq!(plan.execution_character(), ExecutionCharacter::Interpretive);

        plan.steps.push(deterministic_plan().steps.remove(0));
        assert_eq!(plan.execution_character(), ExecutionCharacter::Mixed);
    }
}
