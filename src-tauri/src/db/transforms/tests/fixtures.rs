use crate::transformation_intent::{
    IntentPlanningMode, PlannedExecutor, PlannedTransformationStep, StepExecutionScope,
    TransformationPlan, TRANSFORMATION_PLAN_SCHEMA_VERSION,
};

pub(super) fn deterministic_plan() -> TransformationPlan {
    TransformationPlan {
        schema_version: TRANSFORMATION_PLAN_SCHEMA_VERSION,
        intent: "Uppercase".to_string(),
        summary: "Uppercase text".to_string(),
        planning_mode: IntentPlanningMode::Pinned,
        steps: vec![PlannedTransformationStep {
            name: "Uppercase".to_string(),
            rationale: "Replayable".to_string(),
            scope: StepExecutionScope::WholeInput,
            failure_policy: Default::default(),
            executor: PlannedExecutor::Deterministic {
                operation_ref: "builtin:uppercase".to_string(),
                config_json: None,
            },
        }],
    }
}
