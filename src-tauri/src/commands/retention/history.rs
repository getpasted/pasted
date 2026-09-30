use tauri::AppHandle;

use crate::db::DbState;

pub(super) fn configure(
    keep_count: i64,
    keep_age_days: i64,
    app: &AppHandle,
    db: &DbState,
) -> Result<(), String> {
    db.configure_clip_retention(keep_count, keep_age_days)
        .map_err(|error| error.to_string())?;
    let applied = db
        .get_setting("keepClipCount")
        .map_err(|error| error.to_string())?;
    if let Some(value) = applied {
        super::super::settings::emit_window_appearance_change(app, "keepClipCount", &value);
    }
    super::super::settings::emit_window_appearance_change(
        app,
        "keepClipAgeDays",
        &keep_age_days.to_string(),
    );
    Ok(())
}
