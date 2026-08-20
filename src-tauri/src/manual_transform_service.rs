//! Application-facing service for manually authored Transforms.
//!
//! The database still uses the historical `Pipeline` storage vocabulary. GUI,
//! CLI, and live-app adapters should enter that compatibility layer here so
//! product behavior does not grow separate persistence paths.

use rusqlite::Result;

use crate::db::DbState;

/// Application-facing names for the historical storage records. Keeping the
/// aliases here prevents database vocabulary from leaking into adapters.
pub type ManualTransform = crate::db::Pipeline;
pub type ManualTransformStepInput = crate::db::PipelineStepInput;

pub fn list(db: &DbState) -> Result<Vec<ManualTransform>> {
    db.get_pipelines()
}

pub fn create(
    db: &DbState,
    name: &str,
    steps: &[ManualTransformStepInput],
    shortcut: Option<&str>,
) -> Result<ManualTransform> {
    db.create_pipeline(name, steps, shortcut)
}

pub fn update(
    db: &DbState,
    transform_ref: &str,
    name: &str,
    steps: &[ManualTransformStepInput],
    shortcut: Option<&str>,
) -> Result<ManualTransform> {
    db.update_pipeline(transform_ref, name, steps, shortcut)
}

pub fn update_shortcut(db: &DbState, transform_ref: &str, shortcut: Option<&str>) -> Result<()> {
    db.update_pipeline_hotkey(transform_ref, shortcut)
}

pub fn delete(db: &DbState, transform_ref: &str) -> Result<()> {
    db.delete_pipeline(transform_ref)
}

pub fn hotkeys(db: &DbState) -> Result<Vec<(String, String, String)>> {
    db.get_pipeline_hotkeys()
}
