use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crate::db::{
    DbState, IntelligenceConnection, TransformClipApplication, TransformationExecutionStart,
};
use crate::operation_registry::BUILTIN_OPERATIONS;
use crate::transformation_intent::{
    IntentPlanningMode, PlannedExecutor, StepExecutionScope, TransformationPlan,
};

pub use crate::intelligence_provider::IntelligenceExecutionError;

mod connections;
mod execution;
mod extractor_authoring;
mod extractor_repair;
mod planning;
mod saved_transforms;

pub use execution::{execute_plan, ExecutePlanOutcome, ExecutePlanRequest};
pub(crate) use execution::{execute_plan_with_cancellation, execute_semantic_operation};
pub use extractor_authoring::{
    propose_extractor_recipe, ExtractorRecipeProposal, ProposeExtractorRecipeRequest,
};
pub use extractor_repair::{
    repair_extractor_recipe, ExtractorRepairOutcome, ExtractorRepairStatus,
    RepairExtractorRecipeRequest,
};
#[cfg(feature = "gui")]
pub(crate) use planning::plan_intent_with_cancellation;
pub use planning::{plan_intent, PlanIntentOutcome, PlanIntentRequest};
pub use saved_transforms::{
    apply_smart_bin_transforms_for_clip, execute_saved_transform, SavedTransformExecutionContext,
};

use connections::{finish_scheduler_permit, is_retryable_provider_error, select_connections};
use execution::ensure_not_cancelled;

#[cfg(test)]
use connections::select_connection;
#[cfg(test)]
use execution::semantic_prompt;
#[cfg(test)]
use extractor_authoring::extractor_recipe_schema;
#[cfg(test)]
use planning::{parse_plan, planning_prompt};

#[cfg(test)]
mod tests;
