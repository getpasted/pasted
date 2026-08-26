use super::*;

mod analysis_history;
mod clips;
mod content_compatibility;
mod content_registry;
mod extractor_identity_migrations;
mod extractors;
mod organization;
mod search_history;
mod visual_labels;

use clips::initialize_clip_schema;
use content_compatibility::finalize_content_registry;
use content_registry::initialize_content_registry;
use extractors::initialize_extractor_registry;
use organization::initialize_organization_schema;
use search_history::initialize_search_history_schema;

impl DbState {
    pub(in crate::db) fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock();

        initialize_clip_schema(&conn)?;
        initialize_organization_schema(&conn)?;
        initialize_search_history_schema(&conn)?;
        self.init_transformation_tables(&conn)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;
        migrate_pipelines_to_saved_transforms(&conn)?;
        migrate_analysis_terminology_schema(&conn)?;

        initialize_content_registry(&conn)?;
        extractor_identity_migrations::migrate_apple_vision_labels_identity(&conn)?;
        initialize_extractor_registry(&conn)?;
        finalize_content_registry(&conn)?;
        Self::init_library_items(&conn)?;
        migrate_canonical_timestamps(&conn)?;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM bins", [], |row| row.get(0))
            .unwrap_or(0);
        if count == 0 {
            insert_default_bins(&conn)?;
        }

        Ok(())
    }
}
