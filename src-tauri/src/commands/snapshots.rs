use crate::db::DbState;
use crate::library_storage::{snapshots, LibrarySession};
use parking_lot::Mutex;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

#[derive(Default)]
pub struct SnapshotRuntimeStatus(Mutex<bool>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotStatus {
    automatic_creation_failed: bool,
    #[serde(flatten)]
    schedule: snapshots::SnapshotSchedule,
}

#[tauri::command]
pub async fn get_snapshot_status(
    status: State<'_, Arc<SnapshotRuntimeStatus>>,
    db: State<'_, Arc<DbState>>,
    session: State<'_, Arc<LibrarySession>>,
) -> Result<SnapshotStatus, String> {
    let automatic_creation_failed = *status.0.lock();
    let db = db.inner().clone();
    let app_data = session.app_data.clone();
    let schedule = tauri::async_runtime::spawn_blocking(move || {
        snapshots::schedule(&db, &app_data, chrono::Utc::now())
    })
    .await
    .map_err(|error| error.to_string())??;
    Ok(SnapshotStatus {
        automatic_creation_failed,
        schedule,
    })
}

#[tauri::command]
pub async fn create_snapshot(
    app: AppHandle,
    status: State<'_, Arc<SnapshotRuntimeStatus>>,
    db: State<'_, Arc<DbState>>,
    session: State<'_, Arc<LibrarySession>>,
) -> Result<snapshots::Snapshot, String> {
    let window_state = super::backups::capture_window_state(&app);
    let db = db.inner().clone();
    let session = session.inner().clone();
    let snapshot = tauri::async_runtime::spawn_blocking(move || {
        session.stable(|| {
            snapshots::create(
                &db,
                &session.app_data,
                chrono::Utc::now(),
                window_state.as_deref(),
            )
        })
    })
    .await
    .map_err(|error| error.to_string())??;
    *status.0.lock() = false;
    let _ = app.emit(crate::app_event_names::SNAPSHOTS_CHANGED, ());
    Ok(snapshot)
}

#[tauri::command]
pub async fn export_snapshot(
    id: String,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
    session: State<'_, Arc<LibrarySession>>,
) -> Result<Option<crate::db::FullBackupReport>, String> {
    crate::features::require(&db, crate::features::Feature::Snapshots)?;
    crate::features::require(&db, crate::features::Feature::Backups)?;
    let suggested_name = format!(
        "Pasted_Snapshot_{}.pastedbackup",
        chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
    );
    let Some(selected_file) = app
        .dialog()
        .file()
        .set_title(crate::localization::text(
            &db,
            "snapshots.exportPickerTitle",
        ))
        .set_file_name(suggested_name)
        .add_filter(
            crate::localization::text(&db, "component.welcomeBackupRestore.pastedFullBackup"),
            &["pastedbackup"],
        )
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let destination = selected_file
        .into_path()
        .map_err(|error| format!("The selected backup location is not writable: {error}"))?;
    let db = db.inner().clone();
    let session = session.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        session.stable(|| snapshots::export(&db, &session.app_data, &id, &destination))
    })
    .await
    .map_err(|error| error.to_string())?
    .map(Some)
}

#[tauri::command]
pub async fn enforce_snapshot_retention(
    keep_count: usize,
    db: State<'_, Arc<DbState>>,
    session: State<'_, Arc<LibrarySession>>,
) -> Result<(), String> {
    let db = db.inner().clone();
    let session = session.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        session.stable(|| snapshots::enforce_retention(&db, &session.app_data, keep_count))
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn list_snapshots(
    db: State<'_, Arc<DbState>>,
    session: State<'_, Arc<LibrarySession>>,
) -> Result<Vec<snapshots::Snapshot>, String> {
    crate::features::require(&db, crate::features::Feature::Snapshots)?;
    let app_data = session.app_data.clone();
    tauri::async_runtime::spawn_blocking(move || snapshots::list(&app_data))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn delete_snapshot(
    id: String,
    db: State<'_, Arc<DbState>>,
    session: State<'_, Arc<LibrarySession>>,
) -> Result<(), String> {
    let db = db.inner().clone();
    let session = session.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        session.stable(|| snapshots::delete(&db, &session.app_data, &id))
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn restore_snapshot(
    id: String,
    current_client_state_json: Option<String>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
    session: State<'_, Arc<LibrarySession>>,
) -> Result<Option<crate::db::FullRestoreReport>, String> {
    crate::features::require(&db, crate::features::Feature::Snapshots)?;
    let app_data = session.app_data.clone();
    let (_lease, path) =
        tauri::async_runtime::spawn_blocking(move || snapshots::restore_source(&app_data, &id))
            .await
            .map_err(|error| error.to_string())??;
    super::backups::activation::activate(
        session,
        current_client_state_json,
        path,
        app,
        db,
        crate::features::Feature::Snapshots,
    )
    .await
}

pub(crate) fn start_worker(app: AppHandle, db: Arc<DbState>) {
    let session = app.state::<Arc<LibrarySession>>().inner().clone();
    let status = Arc::new(SnapshotRuntimeStatus::default());
    app.manage(status.clone());
    let mut watcher = crate::library_storage::settings_watch::StorageSettingsWatcher::default();
    watcher.poll(&db);
    std::thread::spawn(move || {
        let mut next_snapshot = std::time::Instant::now();
        loop {
            if crate::app_runtime::exit_requested() {
                break;
            }
            for (key, value) in watcher.poll(&db) {
                super::settings::emit_window_appearance_change(&app, key, &value);
                if key == snapshots::RETENTION_KEY
                    && crate::features::is_enabled(&db, crate::features::Feature::Snapshots)
                {
                    if let Err(error) = session.stable(|| {
                        let keep = value.parse::<usize>().unwrap_or(24);
                        snapshots::enforce_retention(&db, &session.app_data, keep)
                    }) {
                        eprintln!("Snapshot retention could not be applied: {error}");
                    }
                }
            }
            if std::time::Instant::now() < next_snapshot {
                std::thread::sleep(std::time::Duration::from_secs(1));
                continue;
            }
            next_snapshot = std::time::Instant::now() + std::time::Duration::from_secs(30);
            let window_state = app.path().app_config_dir().ok().and_then(|directory| {
                std::fs::read_to_string(directory.join(".window-state.json")).ok()
            });
            let result = session.stable(|| {
                snapshots::check(
                    &db,
                    &session.app_data,
                    chrono::Utc::now(),
                    window_state.as_deref(),
                )
            });
            match result {
                Ok(created) => {
                    *status.0.lock() = false;
                    if created.is_some() {
                        let _ = app.emit(crate::app_event_names::SNAPSHOTS_CHANGED, ());
                    }
                }
                Err(error) => {
                    *status.0.lock() = true;
                    eprintln!("Automatic snapshot could not be saved: {error}");
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });
}
