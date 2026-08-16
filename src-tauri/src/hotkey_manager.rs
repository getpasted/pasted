use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

#[cfg(target_os = "linux")]
use futures_util::StreamExt;

use crate::commands;
use crate::db::DbState;
use crate::features::{self, Feature};
use crate::sequential_paste::SequentialQueueState;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppHotkeyAction {
    ToggleHud,
    ToggleMainWindow,
    LockApp,
    UnlockApp,
    OpenTransformations,
    ToggleCopyQueue,
    PopCopyQueue,
    PasteClip(usize),
    PasteWithPipeline(String),
    CopyWithLastPipeline,
    PasteWithLastPipeline,
    OpenBin(i64),
}

#[derive(Debug, Clone)]
struct HotkeySpec {
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    id: String,
    description: String,
    shortcut: String,
    action: AppHotkeyAction,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct HotkeyRegistrationIssue {
    pub shortcut: String,
    pub description: String,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct HotkeyRegistrationStatus {
    pub backend: String,
    pub state: String,
    pub configured_count: usize,
    pub registered_count: usize,
    pub issues: Vec<HotkeyRegistrationIssue>,
}

impl Default for HotkeyRegistrationStatus {
    fn default() -> Self {
        Self {
            backend: native_backend_name().to_string(),
            state: "checking".to_string(),
            configured_count: 0,
            registered_count: 0,
            issues: Vec::new(),
        }
    }
}

pub struct HotkeyManager {
    action_map: RwLock<HashMap<Shortcut, AppHotkeyAction>>,
    registration_status: RwLock<HotkeyRegistrationStatus>,
    #[cfg(target_os = "linux")]
    portal_task: parking_lot::Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
}

impl HotkeyManager {
    pub fn new() -> Self {
        Self {
            action_map: RwLock::new(HashMap::new()),
            registration_status: RwLock::new(HotkeyRegistrationStatus::default()),
            #[cfg(target_os = "linux")]
            portal_task: parking_lot::Mutex::new(None),
        }
    }

    pub fn registration_status(&self) -> HotkeyRegistrationStatus {
        self.registration_status.read().clone()
    }

    pub fn register_all(self: &Arc<Self>, app: &AppHandle) -> Result<(), String> {
        let _ = app.global_shortcut().unregister_all();
        self.action_map.write().clear();

        #[cfg(target_os = "linux")]
        if let Some(task) = self.portal_task.lock().take() {
            task.abort();
        }

        let db_opt = app.try_state::<Arc<DbState>>();
        let Some(db) = db_opt else {
            return Err("Database state not initialized".to_string());
        };

        let get_setting = |key: &str, default_val: &str| -> Option<String> {
            match db.get_setting(key) {
                Ok(Some(s)) => {
                    let trimmed = s.trim().to_string();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed)
                    }
                }
                _ => {
                    if default_val.trim().is_empty() {
                        None
                    } else {
                        Some(default_val.to_string())
                    }
                }
            }
        };

        let mut specs = Vec::new();
        let mut add_shortcut = |id: String,
                                description: String,
                                setting_str_opt: Option<String>,
                                action: AppHotkeyAction| {
            let Some(setting_str) = setting_str_opt else {
                return;
            };
            specs.push(HotkeySpec {
                id,
                description,
                shortcut: setting_str,
                action,
            });
        };

        if features::is_enabled(&db, Feature::Hud) {
            // HUD shortcut (default Option+Shift+V)
            let hud_sc = get_setting("hudHotkey", "Alt+Shift+V");
            add_shortcut(
                "hud".into(),
                "Show or hide the HUD".into(),
                hud_sc,
                AppHotkeyAction::ToggleHud,
            );
        }

        // Main window shortcut
        let main_sc = get_setting("openMainWindowHotkey", "");
        add_shortcut(
            "main-window".into(),
            "Show or hide Pasted".into(),
            main_sc,
            AppHotkeyAction::ToggleMainWindow,
        );

        if features::is_enabled(&db, Feature::AppLock) {
            add_shortcut(
                "app-lock".into(),
                "Lock Pasted".into(),
                get_setting("lockAppHotkey", "Alt+Shift+L"),
                AppHotkeyAction::LockApp,
            );
            add_shortcut(
                "app-unlock".into(),
                "Unlock Pasted".into(),
                get_setting("unlockAppHotkey", "Alt+Shift+U"),
                AppHotkeyAction::UnlockApp,
            );
        }

        if features::is_enabled(&db, Feature::Transformations) {
            let transformations_sc = get_setting("openTransformationsHotkey", "");
            add_shortcut(
                "transformations".into(),
                "Open Transformations".into(),
                transformations_sc,
                AppHotkeyAction::OpenTransformations,
            );
        }

        if features::is_enabled(&db, Feature::Queue) {
            // Sequential Stack toggle (default Option+Shift+C)
            let seq_toggle_sc = get_setting("seqToggleHotkey", "Alt+Shift+C");
            add_shortcut(
                "queue-toggle".into(),
                "Enable or disable the Queue".into(),
                seq_toggle_sc,
                AppHotkeyAction::ToggleCopyQueue,
            );

            // Sequential Stack pop (default Option+Shift+X)
            let seq_pop_sc = get_setting("seqPopHotkey", "Alt+Shift+X");
            add_shortcut(
                "queue-paste-next".into(),
                "Paste the next Queue item".into(),
                seq_pop_sc,
                AppHotkeyAction::PopCopyQueue,
            );
        }

        // Recent clip shortcuts
        for i in 1..=9 {
            let key = format!("pasteClip{}Hotkey", i);
            let sc = get_setting(&key, "");
            add_shortcut(
                format!("paste-clip-{i}"),
                format!("Paste clip {i}"),
                sc,
                AppHotkeyAction::PasteClip(i),
            );
        }

        if features::is_enabled(&db, Feature::Transformations) {
            // Last-Pipeline shortcuts
            let copy_last_pipeline_sc = get_setting("copyLastPipelineHotkey", "");
            add_shortcut(
                "copy-last-transform".into(),
                "Copy with the last Advanced Transform".into(),
                copy_last_pipeline_sc,
                AppHotkeyAction::CopyWithLastPipeline,
            );
            let paste_last_pipeline_sc = get_setting("pasteLastPipelineHotkey", "");
            add_shortcut(
                "paste-last-transform".into(),
                "Paste with the last Advanced Transform".into(),
                paste_last_pipeline_sc,
                AppHotkeyAction::PasteWithLastPipeline,
            );

            // Per-Pipeline shortcuts
            if let Ok(pipelines) = db.get_pipelines() {
                for pipeline in pipelines {
                    if let Some(sc) = pipeline.shortcut {
                        if !sc.trim().is_empty() {
                            add_shortcut(
                                format!("transform-{}", pipeline.id),
                                format!("Run {}", pipeline.name),
                                Some(sc),
                                AppHotkeyAction::PasteWithPipeline(pipeline.stable_ref),
                            );
                        }
                    }
                }
            }
        }

        if features::is_enabled(&db, Feature::Bins) {
            // Bin shortcuts
            if let Ok(bins) = db.get_bins() {
                for b in bins {
                    if let Some(sc) = b.shortcut {
                        if !sc.trim().is_empty() {
                            add_shortcut(
                                format!("bin-{}", b.id),
                                format!("Open {}", b.name),
                                Some(sc),
                                AppHotkeyAction::OpenBin(b.id),
                            );
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        if is_wayland_session() {
            return self.register_wayland_portal(app.clone(), specs);
        }

        self.register_native(app, specs)
    }

    fn register_native(&self, app: &AppHandle, specs: Vec<HotkeySpec>) -> Result<(), String> {
        let configured_count = specs.len();
        let mut registered_count = 0;
        let mut issues = Vec::new();
        let mut map = self.action_map.write();

        for spec in specs {
            let Some(shortcuts) = commands::parse_shortcut_str_all_layouts(&spec.shortcut) else {
                issues.push(HotkeyRegistrationIssue {
                    shortcut: spec.shortcut,
                    description: spec.description,
                    message: "Pasted could not understand this shortcut.".into(),
                });
                continue;
            };

            let mut registered_any = false;
            let mut last_error = None;
            for shortcut in shortcuts {
                match app.global_shortcut().register(shortcut) {
                    Ok(()) => {
                        registered_any = true;
                        map.insert(shortcut, spec.action.clone());
                    }
                    Err(error) => last_error = Some(error.to_string()),
                }
            }

            if registered_any {
                registered_count += 1;
            } else {
                let message = last_error.unwrap_or_else(|| "The shortcut is unavailable.".into());
                eprintln!(
                    "[Pasted Hotkeys] Could not register '{}' for {}: {message}",
                    spec.shortcut, spec.description
                );
                issues.push(HotkeyRegistrationIssue {
                    shortcut: spec.shortcut,
                    description: spec.description,
                    message,
                });
            }
        }

        let state = if issues.is_empty() {
            "ready"
        } else {
            "conflict"
        };
        *self.registration_status.write() = HotkeyRegistrationStatus {
            backend: native_backend_name().to_string(),
            state: state.to_string(),
            configured_count,
            registered_count,
            issues: issues.clone(),
        };

        if issues.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "{} shortcut{} could not be registered",
                issues.len(),
                if issues.len() == 1 { "" } else { "s" }
            ))
        }
    }

    #[cfg(target_os = "linux")]
    fn register_wayland_portal(
        self: &Arc<Self>,
        app: AppHandle,
        specs: Vec<HotkeySpec>,
    ) -> Result<(), String> {
        let configured_count = specs.len();
        *self.registration_status.write() = HotkeyRegistrationStatus {
            backend: "wayland-portal".into(),
            state: "checking".into(),
            configured_count,
            registered_count: 0,
            issues: Vec::new(),
        };

        if specs.is_empty() {
            self.registration_status.write().state = "ready".into();
            return Ok(());
        }

        let manager = Arc::clone(self);
        let task = tauri::async_runtime::spawn(async move {
            if let Err(error) = manager.run_wayland_portal(app, specs).await {
                eprintln!("[Pasted Hotkeys] Wayland portal unavailable: {error}");
                let mut status = manager.registration_status.write();
                status.state = "unavailable".into();
                status.registered_count = 0;
                status.issues = vec![HotkeyRegistrationIssue {
                    shortcut: String::new(),
                    description: "Wayland global hotkeys".into(),
                    message: error,
                }];
            }
        });
        *self.portal_task.lock() = Some(task);
        Ok(())
    }

    #[cfg(target_os = "linux")]
    async fn run_wayland_portal(
        &self,
        app: AppHandle,
        specs: Vec<HotkeySpec>,
    ) -> Result<(), String> {
        use ashpd::desktop::{
            global_shortcuts::{BindShortcutsOptions, GlobalShortcuts, NewShortcut},
            CreateSessionOptions,
        };

        let portal = GlobalShortcuts::new().await.map_err(|error| {
            format!("The desktop does not provide the Global Shortcuts portal: {error}")
        })?;
        let mut activated = portal
            .receive_activated()
            .await
            .map_err(|error| format!("Could not listen for portal shortcuts: {error}"))?;
        let session = portal
            .create_session(CreateSessionOptions::default())
            .await
            .map_err(|error| format!("Could not create a portal shortcut session: {error}"))?;

        let portal_shortcuts: Vec<NewShortcut> = specs
            .iter()
            .map(|spec| {
                let trigger = shortcut_to_xdg_trigger(&spec.shortcut);
                NewShortcut::new(spec.id.clone(), spec.description.clone())
                    .preferred_trigger(trigger.as_deref())
            })
            .collect();
        let request = portal
            .bind_shortcuts(
                &session,
                &portal_shortcuts,
                None,
                BindShortcutsOptions::default(),
            )
            .await
            .map_err(|error| format!("Could not ask the desktop to bind shortcuts: {error}"))?;
        let response = request
            .response()
            .map_err(|error| format!("The desktop declined the shortcut request: {error}"))?;
        let bound_ids: std::collections::HashSet<&str> = response
            .shortcuts()
            .iter()
            .map(|shortcut| shortcut.id())
            .collect();
        let actions: HashMap<String, AppHotkeyAction> = specs
            .iter()
            .filter(|spec| bound_ids.contains(spec.id.as_str()))
            .map(|spec| (spec.id.clone(), spec.action.clone()))
            .collect();
        let issues: Vec<HotkeyRegistrationIssue> = specs
            .iter()
            .filter(|spec| !bound_ids.contains(spec.id.as_str()))
            .map(|spec| HotkeyRegistrationIssue {
                shortcut: spec.shortcut.clone(),
                description: spec.description.clone(),
                message: "The desktop did not enable this shortcut.".into(),
            })
            .collect();

        *self.registration_status.write() = HotkeyRegistrationStatus {
            backend: "wayland-portal".into(),
            state: if issues.is_empty() {
                "ready"
            } else {
                "conflict"
            }
            .into(),
            configured_count: specs.len(),
            registered_count: actions.len(),
            issues,
        };

        while let Some(event) = activated.next().await {
            if let Some(action) = actions.get(event.shortcut_id()).cloned() {
                self.dispatch_action(&app, action);
            }
        }

        Err("The desktop closed the Global Shortcuts session.".into())
    }

    pub fn dispatch(&self, app: &AppHandle, shortcut: &Shortcut) {
        let action_opt = {
            let map = self.action_map.read();
            map.get(shortcut).cloned()
        };

        let Some(action) = action_opt else {
            eprintln!(
                "[Pasted Hotkeys] Ignoring unmapped shortcut: key={:?}, modifiers={:?}",
                shortcut.key, shortcut.mods
            );
            return;
        };

        self.dispatch_action(app, action);
    }

    fn dispatch_action(&self, app: &AppHandle, action: AppHotkeyAction) {
        let lock_state = app.try_state::<Arc<crate::app_lock::AppLockState>>();
        let locked = lock_state.as_ref().is_some_and(|state| state.is_locked());
        if locked && !matches!(&action, AppHotkeyAction::UnlockApp) {
            return;
        }
        if matches!(
            &action,
            AppHotkeyAction::LockApp | AppHotkeyAction::UnlockApp
        ) {
            let app_handle = app.clone();
            if let Err(error) = app.run_on_main_thread(move || match action {
                AppHotkeyAction::LockApp => {
                    let db = app_handle.state::<Arc<DbState>>();
                    let state = app_handle.state::<Arc<crate::app_lock::AppLockState>>();
                    if features::is_enabled(&db, Feature::AppLock)
                        && db
                            .get_setting(crate::app_lock::ENABLED_SETTING)
                            .ok()
                            .flatten()
                            .as_deref()
                            == Some("true")
                    {
                        state.lock();
                        let _ = crate::app_menu::install(&app_handle, &db);
                        let _ = app_handle
                            .emit("app-lock-changed", crate::app_lock::status(&db, &state));
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
                AppHotkeyAction::UnlockApp => {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                    let _ = app_handle.emit("app-lock-unlock-requested", ());
                }
                _ => {}
            }) {
                eprintln!("[Pasted Hotkeys] Could not dispatch app-lock shortcut: {error}");
            }
            return;
        }
        if let Some(db) = app.try_state::<Arc<DbState>>() {
            let active_app = crate::paste_target::active_application_name();
            if crate::app_exclusions::should_ignore_hotkeys(&db, active_app.as_deref()) {
                return;
            }
        }
        let app_handle = app.clone();
        if let Err(error) = app.run_on_main_thread(move || match action {
            AppHotkeyAction::ToggleHud => {
                let _ = commands::toggle_hud_window(app_handle.clone());
            }
            AppHotkeyAction::ToggleMainWindow => {
                if let Some(w) = app_handle.get_webview_window("main") {
                    if w.is_visible().unwrap_or(false) {
                        let _ = w.hide();
                    } else {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
            }
            AppHotkeyAction::LockApp | AppHotkeyAction::UnlockApp => {}
            AppHotkeyAction::OpenTransformations => {
                if let Some(w) = app_handle.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                    let _ = app_handle.emit("navigate-tab", "transformations");
                }
            }
            AppHotkeyAction::ToggleCopyQueue => {
                let seq = app_handle.state::<Arc<SequentialQueueState>>();
                let db = app_handle.state::<Arc<DbState>>();
                let status = seq.get_status();
                if status.is_active {
                    seq.stop_queue();
                    let _ = db.log_activity(
                        "queue_recording_stopped",
                        "Stopped recording copies into the Queue",
                    );
                } else {
                    seq.start_queue();
                    let _ = db.log_activity(
                        "queue_recording_started",
                        "Started recording copies into the Queue",
                    );
                }
                let updated = seq.get_status();
                let _ = app_handle.emit("sequential-updated", updated);
            }
            AppHotkeyAction::PopCopyQueue => {
                let queue_app = app_handle.clone();
                std::thread::spawn(move || {
                    let seq = queue_app.state::<Arc<SequentialQueueState>>();
                    let db = queue_app.state::<Arc<DbState>>();
                    let _ = commands::paste_next_queue_item(&seq, &db, &queue_app);
                });
            }
            AppHotkeyAction::PasteClip(index) => {
                let paste_app = app_handle.clone();
                std::thread::spawn(move || {
                    let Some(db) = paste_app.try_state::<Arc<DbState>>() else {
                        return;
                    };
                    let Ok(clips) = db.get_clips_page(
                        None,
                        None,
                        false,
                        Some(1),
                        Some(index.saturating_sub(1) as i64),
                    ) else {
                        return;
                    };
                    let Some(clip) = clips.first() else {
                        return;
                    };
                    if let Err(error) = commands::paste_clip_from_hud(&db, &paste_app, clip.id) {
                        eprintln!("[Pasted HUD] {error}");
                    }
                });
            }
            AppHotkeyAction::PasteWithPipeline(pipeline_ref) => {
                let db_opt = app_handle.try_state::<Arc<DbState>>();
                if let Some(db) = db_opt {
                    if let Err(error) =
                        commands::execute_clipboard_pipeline(&db, Some(&pipeline_ref), true)
                    {
                        eprintln!("[Pasted Pipeline Shortcut] {error}");
                    }
                }
            }
            AppHotkeyAction::CopyWithLastPipeline => {
                let db_opt = app_handle.try_state::<Arc<DbState>>();
                if let Some(db) = db_opt {
                    if let Err(error) = commands::execute_clipboard_pipeline(&db, None, false) {
                        eprintln!("[Pasted Last Pipeline Copy] {error}");
                    }
                }
            }
            AppHotkeyAction::PasteWithLastPipeline => {
                let db_opt = app_handle.try_state::<Arc<DbState>>();
                if let Some(db) = db_opt {
                    if let Err(error) = commands::execute_clipboard_pipeline(&db, None, true) {
                        eprintln!("[Pasted Last Pipeline Paste] {error}");
                    }
                }
            }
            AppHotkeyAction::OpenBin(bin_id) => {
                if let Some(w) = app_handle.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                    let _ = app_handle.emit("navigate-bin", bin_id);
                }
            }
        }) {
            eprintln!("[Pasted Hotkeys] Could not dispatch shortcut action: {error}");
        }
    }
}

fn native_backend_name() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
    #[cfg(target_os = "linux")]
    {
        if is_wayland_session() {
            "wayland-portal"
        } else {
            "x11"
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        "unsupported"
    }
}

#[cfg(target_os = "linux")]
fn is_wayland_session() -> bool {
    std::env::var("XDG_SESSION_TYPE")
        .map(|value| value.eq_ignore_ascii_case("wayland"))
        .unwrap_or(false)
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

#[cfg_attr(not(any(target_os = "linux", test)), allow(dead_code))]
fn shortcut_to_xdg_trigger(shortcut: &str) -> Option<String> {
    let mut parts: Vec<&str> = shortcut
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    let key = parts.pop()?;
    let mut modifiers = Vec::new();
    for modifier in parts {
        let normalized = match modifier.to_ascii_lowercase().as_str() {
            "cmdorctrl" | "commandorcontrol" | "ctrl" | "control" => "CTRL",
            "alt" | "option" => "ALT",
            "shift" => "SHIFT",
            "cmd" | "command" | "meta" | "super" | "logo" => "LOGO",
            _ => return None,
        };
        if !modifiers.contains(&normalized) {
            modifiers.push(normalized);
        }
    }

    let key = match key.to_ascii_lowercase().as_str() {
        "space" | "spacebar" => "space".to_string(),
        "enter" | "return" => "Return".to_string(),
        "esc" | "escape" => "Escape".to_string(),
        "arrowup" | "up" => "Up".to_string(),
        "arrowdown" | "down" => "Down".to_string(),
        "arrowleft" | "left" => "Left".to_string(),
        "arrowright" | "right" => "Right".to_string(),
        "backspace" => "BackSpace".to_string(),
        "delete" => "Delete".to_string(),
        "tab" => "Tab".to_string(),
        "home" => "Home".to_string(),
        "end" => "End".to_string(),
        "pageup" => "Page_Up".to_string(),
        "pagedown" => "Page_Down".to_string(),
        "minus" | "-" => "minus".to_string(),
        "equal" | "=" => "equal".to_string(),
        value if value.len() == 1 => value.to_string(),
        value if value.starts_with('f') && value[1..].chars().all(|c| c.is_ascii_digit()) => {
            value.to_ascii_uppercase()
        }
        _ => return None,
    };

    modifiers.push(&key);
    Some(modifiers.join("+"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotkey_manager_maps_actions() {
        let mgr = HotkeyManager::new();
        let sc = commands::parse_shortcut_str("CmdOrCtrl+Shift+V").unwrap();

        {
            let mut map = mgr.action_map.write();
            map.insert(sc, AppHotkeyAction::ToggleHud);
        }

        {
            let map = mgr.action_map.read();
            assert_eq!(map.get(&sc), Some(&AppHotkeyAction::ToggleHud));
        }

        {
            let mut map = mgr.action_map.write();
            map.clear();
            assert_eq!(map.get(&sc), None);
        }
    }

    #[test]
    fn converts_pasted_shortcuts_to_xdg_triggers() {
        assert_eq!(
            shortcut_to_xdg_trigger("CmdOrCtrl+Shift+V"),
            Some("CTRL+SHIFT+v".into())
        );
        assert_eq!(
            shortcut_to_xdg_trigger("Alt+Shift+Space"),
            Some("ALT+SHIFT+space".into())
        );
        assert_eq!(shortcut_to_xdg_trigger("Super+F8"), Some("LOGO+F8".into()));
        assert_eq!(shortcut_to_xdg_trigger("Ctrl+NoSuchKey"), None);
    }
}
