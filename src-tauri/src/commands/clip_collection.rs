use std::sync::Arc;

use tauri::State;

use crate::db::{ClipCollectionPageRequest, ClipListPage, ClipSearchRequest, DbState};
use crate::features::{self, Feature};

fn required_feature(collection: &str) -> Option<Feature> {
    match collection {
        "trash" => Some(Feature::Trash),
        "bin" => Some(Feature::Bins),
        "pinned" => Some(Feature::Pinning),
        "protected" => Some(Feature::Protection),
        "concealed" => Some(Feature::Concealment),
        "named" => Some(Feature::Naming),
        "noted" => Some(Feature::Notes),
        "clipType" => Some(Feature::ClipTypes),
        "contentType" => Some(Feature::ContentTypes),
        "fileFormat" => Some(Feature::FileFormats),
        "source" => Some(Feature::Sources),
        _ => None,
    }
}

#[tauri::command]
pub async fn get_clip_collection_page(
    request: ClipCollectionPageRequest,
    db: State<'_, Arc<DbState>>,
) -> Result<ClipListPage, String> {
    if let Some(feature) = required_feature(&request.collection) {
        features::require(&db, feature)?;
    }
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        db.get_clip_collection_page(&request)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn search_clip_list(
    request: ClipSearchRequest,
    db: State<'_, Arc<DbState>>,
) -> Result<ClipListPage, String> {
    features::require(&db, Feature::Search)?;
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        db.search_clip_list(&request)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}
