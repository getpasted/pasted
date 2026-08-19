use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager};

use crate::clipboard_monitor::ClipboardMonitorState;
use crate::db::DbState;
use crate::ocr::OcrService;
use crate::sequential_paste::SequentialQueueState;

pub const REQUEST_ARGUMENT: &str = "--pasted-live-request";
const REQUEST_PREFIX: &str = "pasted-live-";
const MAX_REQUEST_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum LiveAppAction {
    ClipboardStatus,
    ClipboardSetPaused {
        paused: bool,
    },
    QueueStatus,
    QueueStart,
    QueueStop,
    QueueAddClips {
        clip_ids: Vec<i64>,
    },
    QueueRemove {
        index: usize,
    },
    QueueReorder {
        item_ids: Vec<u64>,
    },
    QueuePaste {
        index: usize,
    },
    QueuePasteAll,
    CopyClip {
        clip_id: i64,
    },
    PasteClip {
        clip_id: i64,
    },
    OcrCancel,
    AppLockStatus,
    AppLockLock,
    AppLockUnlock {
        passphrase: String,
    },
    AppLockReset {
        confirmed: bool,
        database_path: PathBuf,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct LiveAppRequest {
    version: u32,
    request_id: String,
    command: LiveAppAction,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LiveAppResponse {
    request_id: String,
    ok: bool,
    result: Option<Value>,
    error: Option<String>,
}

fn response_path(request_path: &Path) -> PathBuf {
    request_path.with_extension("response.json")
}

fn validate_request_path(path: &Path) -> Result<(), String> {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Live-app request has an invalid filename".to_string())?;
    if !file_name.starts_with(REQUEST_PREFIX)
        || path.parent() != Some(std::env::temp_dir().as_path())
    {
        return Err("Live-app requests must use Pasted's temporary request path".to_string());
    }
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !metadata.file_type().is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_REQUEST_BYTES
    {
        return Err("Live-app request is not a bounded regular file".to_string());
    }
    Ok(())
}

fn write_private(path: &Path, contents: &[u8]) -> Result<(), String> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|error| error.to_string())?;
    file.write_all(contents).map_err(|error| error.to_string())
}

pub fn request_from_args(args: &[String]) -> Option<PathBuf> {
    args.iter()
        .position(|argument| argument == REQUEST_ARGUMENT)
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
}

pub fn handle_request_file(
    app: &tauri::AppHandle,
    path: &Path,
    allow_recovery_reset: bool,
) -> bool {
    let recovery_reset = is_recovery_reset_request(path);
    let response = match read_and_execute(app, path, allow_recovery_reset) {
        Ok((request_id, result)) => LiveAppResponse {
            request_id,
            ok: true,
            result: Some(result),
            error: None,
        },
        Err((request_id, error)) => LiveAppResponse {
            request_id,
            ok: false,
            result: None,
            error: Some(error),
        },
    };
    if let Ok(contents) = serde_json::to_vec(&response) {
        let _ = write_private(&response_path(path), &contents);
    }
    let _ = fs::remove_file(path);
    recovery_reset
}

pub fn is_recovery_reset_request(path: &Path) -> bool {
    fs::read(path)
        .ok()
        .and_then(|contents| serde_json::from_slice::<LiveAppRequest>(&contents).ok())
        .is_some_and(|request| matches!(request.command, LiveAppAction::AppLockReset { .. }))
}

fn read_and_execute(
    app: &tauri::AppHandle,
    path: &Path,
    allow_recovery_reset: bool,
) -> Result<(String, Value), (String, String)> {
    let unknown = "unknown".to_string();
    validate_request_path(path).map_err(|error| (unknown.clone(), error))?;
    let contents = fs::read(path).map_err(|error| (unknown.clone(), error.to_string()))?;
    let request: LiveAppRequest =
        serde_json::from_slice(&contents).map_err(|error| (unknown, error.to_string()))?;
    if request.version != 1 {
        return Err((
            request.request_id,
            "Unsupported live-app protocol version".to_string(),
        ));
    }
    let request_id = request.request_id.clone();
    execute(app, request.command, allow_recovery_reset)
        .map(|result| (request_id.clone(), result))
        .map_err(|error| (request_id, error))
}

fn execute(
    app: &tauri::AppHandle,
    action: LiveAppAction,
    allow_recovery_reset: bool,
) -> Result<Value, String> {
    let db = app.state::<Arc<DbState>>();
    let queue = app.state::<Arc<SequentialQueueState>>();
    let lock_state = app.state::<Arc<crate::app_lock::AppLockState>>();
    if lock_state.is_locked()
        && !matches!(
            &action,
            LiveAppAction::AppLockStatus
                | LiveAppAction::AppLockLock
                | LiveAppAction::AppLockUnlock { .. }
                | LiveAppAction::AppLockReset { .. }
                | LiveAppAction::ClipboardStatus
                | LiveAppAction::ClipboardSetPaused { .. }
        )
    {
        return Err("Pasted is locked.".to_string());
    }
    match action {
        LiveAppAction::ClipboardStatus => {
            let monitor = app.state::<Arc<ClipboardMonitorState>>();
            Ok(serde_json::json!({ "paused": monitor.is_paused() }))
        }
        LiveAppAction::ClipboardSetPaused { paused } => {
            let monitor = app.state::<Arc<ClipboardMonitorState>>();
            monitor
                .is_manually_paused
                .store(paused, std::sync::atomic::Ordering::Relaxed);
            let _ = db.log_activity(
                if paused {
                    "recording_manually_paused"
                } else {
                    "recording_manually_resumed"
                },
                if paused {
                    "Clipboard recording manually paused"
                } else {
                    "Clipboard recording manually resumed"
                },
            );
            let effective = monitor.is_paused();
            crate::app_events::emit_clipboard_pause_changed(app, effective, None);
            Ok(serde_json::json!({ "paused": effective }))
        }
        LiveAppAction::QueueStatus => {
            serde_json::to_value(queue.get_status()).map_err(|error| error.to_string())
        }
        LiveAppAction::QueueStart => {
            crate::features::require(&db, crate::features::Feature::Queue)?;
            queue.start_queue();
            let _ = db.log_activity(
                "queue_recording_started",
                "Started recording copies into the Queue",
            );
            let status = queue.get_status();
            let _ = app.emit("sequential-updated", status.clone());
            serde_json::to_value(status).map_err(|error| error.to_string())
        }
        LiveAppAction::QueueStop => {
            queue.stop_queue();
            let _ = db.log_activity(
                "queue_recording_stopped",
                "Stopped recording copies into the Queue",
            );
            let status = queue.get_status();
            let _ = app.emit("sequential-updated", status.clone());
            serde_json::to_value(status).map_err(|error| error.to_string())
        }
        LiveAppAction::QueueAddClips { clip_ids } => {
            crate::features::require(&db, crate::features::Feature::Queue)?;
            for clip_id in clip_ids {
                let text = db
                    .get_active_clip_text(clip_id)
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| format!("Clip #{clip_id} has no queueable text"))?;
                queue.push_item(text)?;
            }
            let status = queue.get_status();
            let _ = app.emit("sequential-updated", status.clone());
            serde_json::to_value(status).map_err(|error| error.to_string())
        }
        LiveAppAction::QueueRemove { index } => {
            queue
                .remove_item_by_index(index)
                .ok_or_else(|| "Queue index was not found".to_string())?;
            let status = queue.get_status();
            let _ = app.emit("sequential-updated", status.clone());
            serde_json::to_value(status).map_err(|error| error.to_string())
        }
        LiveAppAction::QueueReorder { item_ids } => {
            queue.reorder_items(&item_ids)?;
            let status = queue.get_status();
            let _ = app.emit("sequential-updated", status.clone());
            serde_json::to_value(status).map_err(|error| error.to_string())
        }
        LiveAppAction::QueuePaste { index } => {
            let pasted = crate::queue_actions::paste_item(&queue, &db, app, index, false)?;
            Ok(serde_json::json!({ "pasted": pasted.is_some(), "status": queue.get_status() }))
        }
        LiveAppAction::QueuePasteAll => {
            let pasted = crate::queue_actions::paste_all(&queue, &db, app)?;
            Ok(serde_json::json!({ "pasted": pasted.is_some(), "status": queue.get_status() }))
        }
        LiveAppAction::CopyClip { clip_id } => {
            crate::clipboard_actions::copy_clip(&db, &queue, clip_id)?;
            Ok(serde_json::json!({ "copied": true, "clipId": clip_id }))
        }
        LiveAppAction::PasteClip { clip_id } => {
            crate::clipboard_actions::paste_clip(
                &db,
                app,
                clip_id,
                crate::clipboard_actions::PasteOrigin::Hud,
            )?;
            Ok(serde_json::json!({ "pasted": true, "clipId": clip_id }))
        }
        LiveAppAction::OcrCancel => {
            app.state::<Arc<OcrService>>().cancel();
            Ok(serde_json::json!({ "cancelled": true }))
        }
        LiveAppAction::AppLockStatus => {
            let state = app.state::<Arc<crate::app_lock::AppLockState>>();
            serde_json::to_value(crate::app_lock::status(&db, &state))
                .map_err(|error| error.to_string())
        }
        LiveAppAction::AppLockLock => {
            crate::features::require(&db, crate::features::Feature::AppLock)?;
            let state = app.state::<Arc<crate::app_lock::AppLockState>>();
            if db
                .get_setting(crate::app_lock::ENABLED_SETTING)
                .map_err(|error| error.to_string())?
                .as_deref()
                != Some("true")
            {
                return Err("App lock is not enabled.".to_string());
            }
            state.lock();
            let _ = crate::app_menu::install(app, &db);
            let status = crate::app_lock::status(&db, &state);
            let _ = app.emit("app-lock-changed", &status);
            serde_json::to_value(status).map_err(|error| error.to_string())
        }
        LiveAppAction::AppLockUnlock { passphrase } => {
            crate::features::require(&db, crate::features::Feature::AppLock)?;
            let state = app.state::<Arc<crate::app_lock::AppLockState>>();
            state.check_retry()?;
            if !crate::app_lock::verify(&db, &passphrase)? {
                state.record_failure();
                return Err("The passphrase is incorrect.".to_string());
            }
            state.unlock();
            let _ = crate::app_menu::install(app, &db);
            let status = crate::app_lock::status(&db, &state);
            let _ = app.emit("app-lock-changed", &status);
            serde_json::to_value(status).map_err(|error| error.to_string())
        }
        LiveAppAction::AppLockReset {
            confirmed,
            database_path,
        } => {
            if !allow_recovery_reset {
                return Err("Quit Pasted before resetting app lock.".to_string());
            }
            if !confirmed {
                return Err("Resetting app lock requires explicit confirmation.".to_string());
            }
            let recovery_db = DbState::new(database_path).map_err(|error| error.to_string())?;
            crate::app_lock::reset(&recovery_db)?;
            let _ = recovery_db.log_activity(
                "app_lock_reset",
                "Reset app lock after local recovery confirmation",
            );
            Ok(serde_json::json!({
                "enabled": false,
                "credentialsCleared": true
            }))
        }
    }
}

pub fn send(action: LiveAppAction) -> Result<Value, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let request_id = format!("{}-{stamp}", std::process::id());
    let request_path = std::env::temp_dir().join(format!("{REQUEST_PREFIX}{request_id}.json"));
    let response_path = response_path(&request_path);
    let request = LiveAppRequest {
        version: 1,
        request_id: request_id.clone(),
        command: action,
    };
    write_private(
        &request_path,
        &serde_json::to_vec(&request).map_err(|error| error.to_string())?,
    )?;

    let current = std::env::current_exe().map_err(|error| error.to_string())?;
    let app_name = if cfg!(windows) {
        "pasted-app.exe"
    } else {
        "pasted-app"
    };
    let app_executable = current
        .parent()
        .map(|parent| parent.join(app_name))
        .filter(|path| path.is_file())
        .ok_or_else(|| "The Pasted app executable is not beside the CLI".to_string())?;
    std::process::Command::new(app_executable)
        .arg(REQUEST_ARGUMENT)
        .arg(&request_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|error| format!("Could not contact the Pasted app: {error}"))?;

    let deadline = std::time::Instant::now() + Duration::from_secs(8);
    while std::time::Instant::now() < deadline {
        if response_path.is_file() {
            let contents = fs::read(&response_path).map_err(|error| error.to_string())?;
            let _ = fs::remove_file(&response_path);
            let _ = fs::remove_file(&request_path);
            let response: LiveAppResponse =
                serde_json::from_slice(&contents).map_err(|error| error.to_string())?;
            if response.request_id != request_id {
                return Err("Live-app response did not match the request".to_string());
            }
            return if response.ok {
                Ok(response.result.unwrap_or(Value::Null))
            } else {
                Err(response
                    .error
                    .unwrap_or_else(|| "Live-app command failed".to_string()))
            };
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let _ = fs::remove_file(&request_path);
    Err("Pasted did not respond to the live-app command. Start the app and try again.".to_string())
}
