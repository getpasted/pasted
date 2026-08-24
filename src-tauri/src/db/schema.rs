use super::*;

mod canonical;
mod helpers;
mod library_items;
mod migrations {
    pub(super) mod analysis;
    pub(super) mod core;
    pub(super) mod settings;
    pub(super) mod transforms;
}
mod registry;
mod transformation_tables;

pub(super) use helpers::{add_column_if_missing, column_exists, insert_default_bins, table_exists};
pub(super) use migrations::analysis::{
    migrate_analysis_terminology_schema, migrate_legacy_semantic_clip_types,
    migrate_multi_type_classifications, retire_structural_content_type_entries,
};
pub(super) use migrations::core::{migrate_clip_source_schema, migrate_legacy_container_schema};
pub(super) use migrations::settings::migrate_app_exclusion_hotkey_setting;
pub(super) use migrations::transforms::migrate_pipelines_to_saved_transforms;
pub(super) use migrations::transforms::{
    backfill_current_transformation, migrate_transform_activity_terminology,
};
pub(super) use registry::run_registered_migrations;
