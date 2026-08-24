mod cancellation;
mod compatibility;
mod contracts;
mod operations;
mod orchestration;

pub use cancellation::{cancel_execution, CancellationRegistration};
pub use compatibility::{
    execute_last_manual_transform, execute_legacy_preview, execute_shortcut_manual_transform,
    get_last_manual_transform_ref,
};
pub use contracts::{
    ExecutionDestination, ExecutionError, ExecutionOutcome, ExecutionRequest, ExecutionTarget,
    ExecutionTrigger,
};
pub(crate) use operations::execute_operation_inline;
pub use orchestration::{execute, execute_with_cancellation, preview_manual_transform_steps};

#[cfg(test)]
mod tests;
