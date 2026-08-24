use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn setup_test_db() -> DbState {
    let temp_dir = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db_file = temp_dir.join(format!(
        "pasted_test_{}_{:?}.db",
        nanos,
        std::thread::current().id()
    ));
    DbState::new(db_file).expect("Failed to create test DB")
}

fn search_test_clips(db: &DbState, query: &str) -> Vec<ClipItem> {
    db.search_clips(&ClipSearchRequest {
        query: query.into(),
        limit: MAX_CLIP_SEARCH_PAGE_SIZE,
        ..Default::default()
    })
    .unwrap()
    .items
}

mod analytics;
mod bins_and_transforms;
mod capture_and_lifecycle;
mod clip_transfer;
mod extractor_recipes;
mod full_backups;
mod history_and_organization_transfer;
mod migrations_and_intelligence;
mod portability_boundaries;
mod retention_and_activity;
mod revisions_and_mutations;
mod search_and_operations;
mod timestamps;
mod transforms_backup_and_protection;
