use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::db::{ClipItem, DbState};
use crate::features::{self, Feature};

#[tauri::command]
pub fn update_clip_name(
    clip_id: i64,
    name: Option<String>,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<ClipItem, String> {
    features::require(&db, Feature::Naming)?;
    let updated = db
        .update_clip_name(clip_id, name.as_deref())
        .map_err(|error| error.to_string())?;
    crate::app_events::emit_clip_library_changed(&app, vec![clip_id]);
    Ok(updated)
}
