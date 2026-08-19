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
    OpenTransformations,
    ToggleCopyQueue,
    PopCopyQueue,
    PasteClip(usize),
    PasteClipById(i64),
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
    pub bindings: Vec<HotkeyRegisteredBinding>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct HotkeyRegisteredBinding {
    pub id: String,
    pub description: String,
    pub trigger: String,
}

impl Default for HotkeyRegistrationStatus {
    fn default() -> Self {
        Self {
            backend: native_backend_name().to_string(),
            state: "checking".to_string(),
            configured_count: 0,
            registered_count: 0,
            issues: Vec::new(),
            bindings: Vec::new(),
        }
    }
}

pub struct HotkeyManager {
    action_map: RwLock<HashMap<Shortcut, AppHotkeyAction>>,
    registration_status: RwLock<HotkeyRegistrationStatus>,
    registration_guard: parking_lot::Mutex<()>,
    #[cfg(target_os = "linux")]
    portal_task: parking_lot::Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    #[cfg(target_os = "linux")]
    x11_task: parking_lot::Mutex<Option<X11ShortcutTask>>,
}

#[cfg(target_os = "linux")]
struct X11ShortcutTask {
    stop: std::sync::mpsc::Sender<()>,
    thread: std::thread::JoinHandle<()>,
}

impl HotkeyManager {
    pub fn new() -> Self {
        Self {
            action_map: RwLock::new(HashMap::new()),
            registration_status: RwLock::new(HotkeyRegistrationStatus::default()),
            registration_guard: parking_lot::Mutex::new(()),
            #[cfg(target_os = "linux")]
            portal_task: parking_lot::Mutex::new(None),
            #[cfg(target_os = "linux")]
            x11_task: parking_lot::Mutex::new(None),
        }
    }

    pub fn registration_status(&self) -> HotkeyRegistrationStatus {
        self.registration_status.read().clone()
    }

    pub fn register_all(self: &Arc<Self>, app: &AppHandle) -> Result<(), String> {
        let _registration = self.registration_guard.lock();
        let _ = app.global_shortcut().unregister_all();
        self.action_map.write().clear();

        #[cfg(target_os = "linux")]
        if let Some(task) = self.portal_task.lock().take() {
            task.abort();
        }
        #[cfg(target_os = "linux")]
        if let Some(task) = self.x11_task.lock().take() {
            let _ = task.stop.send(());
            let _ = task.thread.join();
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

        if let Ok(clips) = db.get_clip_hotkeys() {
            for (clip_id, shortcut) in clips {
                add_shortcut(
                    format!("clip-{clip_id}"),
                    format!("Paste assigned clip #{clip_id}"),
                    Some(shortcut),
                    AppHotkeyAction::PasteClipById(clip_id),
                );
            }
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

        #[cfg(target_os = "linux")]
        return self.register_x11(app.clone(), specs);

        #[cfg(not(target_os = "linux"))]
        self.register_native(app, specs)
    }

    #[cfg_attr(target_os = "linux", allow(dead_code))]
    fn register_native(&self, app: &AppHandle, specs: Vec<HotkeySpec>) -> Result<(), String> {
        let configured_count = specs.len();
        let mut registered_count = 0;
        let mut issues = Vec::new();
        let mut map = self.action_map.write();

        for spec in specs {
            let Some(shortcuts) = commands::parse_shortcut_str_for_current_layout(&spec.shortcut)
            else {
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
                    Err(error) => {
                        last_error = Some(error.to_string());
                    }
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
            bindings: Vec::new(),
        };
        let _ = app.emit("hotkey-registration-changed", ());

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
    fn register_x11(
        self: &Arc<Self>,
        app: AppHandle,
        specs: Vec<HotkeySpec>,
    ) -> Result<(), String> {
        let (stop_sender, stop_receiver) = std::sync::mpsc::channel();
        let (ready_sender, ready_receiver) = std::sync::mpsc::sync_channel(1);
        let manager = Arc::clone(self);
        let thread = std::thread::spawn(move || {
            let result = run_x11_shortcuts(&manager, &app, &specs, &stop_receiver, &ready_sender);
            if let Err(error) = result {
                let _ = ready_sender.try_send(Err(error.clone()));
                eprintln!("[Pasted Hotkeys] X11 shortcut backend stopped: {error}");
            }
        });
        let result = match ready_receiver.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(result) => result,
            Err(_) => {
                let _ = stop_sender.send(());
                let _ = thread.join();
                return Err("Timed out while registering X11 shortcuts.".to_string());
            }
        };
        if result.is_ok() {
            *self.x11_task.lock() = Some(X11ShortcutTask {
                stop: stop_sender,
                thread,
            });
        } else {
            let _ = stop_sender.send(());
            let _ = thread.join();
        }
        result
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
            bindings: Vec::new(),
        };

        if specs.is_empty() {
            self.registration_status.write().state = "ready".into();
            return Ok(());
        }

        let manager = Arc::clone(self);
        let failure_app = app.clone();
        let task = tauri::async_runtime::spawn(async move {
            if let Err(error) = manager.run_wayland_portal(app, specs).await {
                eprintln!("[Pasted Hotkeys] Wayland portal unavailable: {error}");
                let mut status = manager.registration_status.write();
                status.state = "unavailable".into();
                status.registered_count = 0;
                status.bindings.clear();
                status.issues = vec![HotkeyRegistrationIssue {
                    shortcut: String::new(),
                    description: "Wayland global hotkeys".into(),
                    message: error,
                }];
                drop(status);
                let _ = failure_app.emit("hotkey-registration-changed", ());
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
        let mut shortcuts_changed = portal
            .receive_shortcuts_changed()
            .await
            .map_err(|error| format!("Could not listen for portal shortcut changes: {error}"))?;
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
        let bindings: Vec<HotkeyRegisteredBinding> = response
            .shortcuts()
            .iter()
            .map(|shortcut| HotkeyRegisteredBinding {
                id: shortcut.id().to_string(),
                description: shortcut.description().to_string(),
                trigger: shortcut.trigger_description().to_string(),
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
            bindings,
        };
        let _ = app.emit("hotkey-registration-changed", ());

        loop {
            use futures_util::FutureExt as _;
            futures_util::select! {
                event = activated.next().fuse() => {
                    let Some(event) = event else {
                        return Err("The desktop closed the Global Shortcuts activation stream.".into());
                    };
                    if let Some(action) = actions.get(event.shortcut_id()).cloned() {
                        self.dispatch_action(&app, action);
                    }
                },
                changed = shortcuts_changed.next().fuse() => {
                    let Some(changed) = changed else {
                        return Err("The desktop closed the Global Shortcuts update stream.".into());
                    };
                    let bindings: Vec<HotkeyRegisteredBinding> = changed
                        .shortcuts()
                        .iter()
                        .filter(|shortcut| actions.contains_key(shortcut.id()))
                        .map(|shortcut| HotkeyRegisteredBinding {
                            id: shortcut.id().to_string(),
                            description: shortcut.description().to_string(),
                            trigger: shortcut.trigger_description().to_string(),
                        })
                        .collect();
                    let mut status = self.registration_status.write();
                    status.registered_count = bindings.len();
                    status.bindings = bindings;
                    drop(status);
                    let _ = app.emit("hotkey-registration-changed", ());
                },
            }
        }
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
        if locked {
            return;
        }
        if matches!(&action, AppHotkeyAction::LockApp) {
            let db = app.state::<Arc<DbState>>();
            let state = app.state::<Arc<crate::app_lock::AppLockState>>();
            let status = match commands::lock_app_state(&db, &state) {
                Ok(status) => status,
                Err(error) => {
                    eprintln!("[Pasted Hotkeys] Could not lock Pasted: {error}");
                    return;
                }
            };
            let _ = app.emit("app-lock-changed", &status);
            let app_handle = app.clone();
            if let Err(error) = app.run_on_main_thread(move || {
                let db = app_handle.state::<Arc<DbState>>();
                if let Err(error) = crate::app_menu::install(&app_handle, &db) {
                    eprintln!("[Pasted Hotkeys] Could not refresh the locked menu: {error}");
                }
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
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
            AppHotkeyAction::LockApp => {}
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
            AppHotkeyAction::PasteClipById(clip_id) => {
                let paste_app = app_handle.clone();
                std::thread::spawn(move || {
                    let Some(db) = paste_app.try_state::<Arc<DbState>>() else {
                        return;
                    };
                    if let Err(error) = commands::paste_clip_from_hud(&db, &paste_app, clip_id) {
                        eprintln!("[Pasted Clip Shortcut] {error}");
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

#[cfg(target_os = "linux")]
#[derive(Clone)]
struct X11RegisteredShortcut {
    keycode: u8,
    modifiers: x11rb::protocol::xproto::ModMask,
    action: AppHotkeyAction,
    pressed: bool,
}

#[cfg(target_os = "linux")]
fn run_x11_shortcuts(
    manager: &Arc<HotkeyManager>,
    app: &AppHandle,
    specs: &[HotkeySpec],
    stop: &std::sync::mpsc::Receiver<()>,
    ready: &std::sync::mpsc::SyncSender<Result<(), String>>,
) -> Result<(), String> {
    use x11rb::connection::Connection as _;
    use x11rb::protocol::xkb::ConnectionExt as _;
    use x11rb::protocol::{xkb, Event};

    let (connection, screen) = x11rb::connect(None).map_err(|error| error.to_string())?;
    connection
        .xkb_use_extension(1, 0)
        .map_err(|error| error.to_string())?
        .reply()
        .map_err(|error| error.to_string())?;
    connection
        .xkb_select_events(
            xkb::ID::USE_CORE_KBD.into(),
            xkb::EventType::default(),
            xkb::EventType::MAP_NOTIFY | xkb::EventType::STATE_NOTIFY,
            xkb::MapPart::KEY_SYMS,
            xkb::MapPart::KEY_SYMS,
            &xkb::SelectEventsAux::new(),
        )
        .map_err(|error| error.to_string())?
        .check()
        .map_err(|error| error.to_string())?;
    let root = connection.setup().roots[screen].root;
    let mut registered = Vec::new();
    let initial = rebuild_x11_shortcuts(manager, app, &connection, root, specs, &mut registered);
    let _ = ready.send(initial.clone());
    initial?;

    let full_mask = x11rb::protocol::xproto::KeyButMask::CONTROL
        | x11rb::protocol::xproto::KeyButMask::SHIFT
        | x11rb::protocol::xproto::KeyButMask::MOD4
        | x11rb::protocol::xproto::KeyButMask::MOD1;
    loop {
        if stop.try_recv().is_ok() {
            unregister_x11_shortcuts(&connection, root, &registered);
            return Ok(());
        }
        while let Some(event) = connection
            .poll_for_event()
            .map_err(|error| error.to_string())?
        {
            match event {
                Event::KeyPress(event) => {
                    let modifiers =
                        x11rb::protocol::xproto::ModMask::from((event.state & full_mask).bits());
                    for shortcut in registered.iter_mut().filter(|shortcut| {
                        shortcut.keycode == event.detail && shortcut.modifiers == modifiers
                    }) {
                        if !shortcut.pressed {
                            shortcut.pressed = true;
                            manager.dispatch_action(app, shortcut.action.clone());
                        }
                    }
                }
                Event::KeyRelease(event) => {
                    for shortcut in registered
                        .iter_mut()
                        .filter(|shortcut| shortcut.keycode == event.detail)
                    {
                        shortcut.pressed = false;
                    }
                }
                Event::XkbMapNotify(_) | Event::XkbNewKeyboardNotify(_) => {
                    if let Err(error) = rebuild_x11_shortcuts(
                        manager,
                        app,
                        &connection,
                        root,
                        specs,
                        &mut registered,
                    ) {
                        eprintln!(
                            "[Pasted Hotkeys] Could not refresh shortcuts after an X11 map change: {error}"
                        );
                    }
                }
                Event::XkbStateNotify(event)
                    if event.changed.contains(
                        xkb::StatePart::GROUP_STATE
                            | xkb::StatePart::GROUP_BASE
                            | xkb::StatePart::GROUP_LATCH
                            | xkb::StatePart::GROUP_LOCK,
                    ) =>
                {
                    if let Err(error) = rebuild_x11_shortcuts(
                        manager,
                        app,
                        &connection,
                        root,
                        specs,
                        &mut registered,
                    ) {
                        eprintln!(
                            "[Pasted Hotkeys] Could not refresh shortcuts after an X11 group change: {error}"
                        );
                    }
                }
                _ => {}
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[cfg(target_os = "linux")]
fn rebuild_x11_shortcuts<C: x11rb::connection::Connection>(
    manager: &HotkeyManager,
    app: &AppHandle,
    connection: &C,
    root: u32,
    specs: &[HotkeySpec],
    registered: &mut Vec<X11RegisteredShortcut>,
) -> Result<(), String> {
    use x11rb::protocol::xproto::ConnectionExt as _;

    unregister_x11_shortcuts(connection, root, registered);
    registered.clear();
    let mut issues = Vec::new();
    for spec in specs {
        let parsed = parse_x11_shortcut(&spec.shortcut).and_then(|(modifiers, keysym)| {
            resolve_x11_keycode(connection, keysym).map(|keycode| (modifiers, keycode))
        });
        let (modifiers, keycode) = match parsed {
            Ok(parsed) => parsed,
            Err(message) => {
                issues.push(HotkeyRegistrationIssue {
                    shortcut: spec.shortcut.clone(),
                    description: spec.description.clone(),
                    message,
                });
                continue;
            }
        };
        let mut failure = None;
        for ignored in x11_ignored_modifiers() {
            match connection.grab_key(
                false,
                root,
                modifiers | ignored,
                keycode,
                x11rb::protocol::xproto::GrabMode::ASYNC,
                x11rb::protocol::xproto::GrabMode::ASYNC,
            ) {
                Ok(cookie) => {
                    if let Err(error) = cookie.check() {
                        failure = Some(error.to_string());
                        break;
                    }
                }
                Err(error) => {
                    failure = Some(error.to_string());
                    break;
                }
            }
        }
        if let Some(message) = failure {
            for ignored in x11_ignored_modifiers() {
                let _ = connection.ungrab_key(keycode, root, modifiers | ignored);
            }
            issues.push(HotkeyRegistrationIssue {
                shortcut: spec.shortcut.clone(),
                description: spec.description.clone(),
                message,
            });
        } else {
            registered.push(X11RegisteredShortcut {
                keycode,
                modifiers,
                action: spec.action.clone(),
                pressed: false,
            });
        }
    }
    connection.flush().map_err(|error| error.to_string())?;
    *manager.registration_status.write() = HotkeyRegistrationStatus {
        backend: "x11".into(),
        state: if issues.is_empty() {
            "ready"
        } else {
            "conflict"
        }
        .into(),
        configured_count: specs.len(),
        registered_count: registered.len(),
        issues: issues.clone(),
        bindings: Vec::new(),
    };
    let _ = app.emit("hotkey-registration-changed", ());
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
fn unregister_x11_shortcuts<C: x11rb::connection::Connection>(
    connection: &C,
    root: u32,
    registered: &[X11RegisteredShortcut],
) {
    use x11rb::protocol::xproto::ConnectionExt as _;
    for shortcut in registered {
        for ignored in x11_ignored_modifiers() {
            let _ = connection.ungrab_key(shortcut.keycode, root, shortcut.modifiers | ignored);
        }
    }
    let _ = connection.flush();
}

#[cfg(target_os = "linux")]
fn x11_ignored_modifiers() -> [x11rb::protocol::xproto::ModMask; 4] {
    use x11rb::protocol::xproto::ModMask;
    [
        ModMask::default(),
        ModMask::LOCK,
        ModMask::M2,
        ModMask::LOCK | ModMask::M2,
    ]
}

#[cfg(target_os = "linux")]
fn parse_x11_shortcut(shortcut: &str) -> Result<(x11rb::protocol::xproto::ModMask, u32), String> {
    use x11rb::protocol::xproto::ModMask;

    let mut parts: Vec<&str> = shortcut.split('+').map(str::trim).collect();
    let key = parts
        .pop()
        .filter(|key| !key.is_empty())
        .ok_or_else(|| "The shortcut has no key.".to_string())?;
    let mut modifiers = ModMask::default();
    for modifier in parts {
        match modifier.to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "cmdorctrl" | "commandorcontrol" => {
                modifiers |= ModMask::CONTROL;
            }
            "alt" | "option" => modifiers |= ModMask::M1,
            "shift" => modifiers |= ModMask::SHIFT,
            "cmd" | "command" | "meta" | "super" | "logo" => modifiers |= ModMask::M4,
            _ => return Err(format!("Unknown shortcut modifier: {modifier}")),
        }
    }
    let normalized = key.to_ascii_lowercase();
    let keysym = match normalized.as_str() {
        value if value.chars().count() == 1 => value.chars().next().unwrap() as u32,
        "space" | "spacebar" => 0x20,
        "tab" => 0xff09,
        "enter" | "return" => 0xff0d,
        "escape" | "esc" => 0xff1b,
        "backspace" => 0xff08,
        "delete" => 0xffff,
        "home" => 0xff50,
        "arrowleft" | "left" => 0xff51,
        "arrowup" | "up" => 0xff52,
        "arrowright" | "right" => 0xff53,
        "arrowdown" | "down" => 0xff54,
        "pageup" => 0xff55,
        "pagedown" => 0xff56,
        "end" => 0xff57,
        value if value.starts_with('f') => {
            let number = value[1..]
                .parse::<u32>()
                .map_err(|_| format!("Unknown shortcut key: {key}"))?;
            if !(1..=35).contains(&number) {
                return Err(format!("Unknown shortcut key: {key}"));
            }
            0xffbd + number
        }
        _ => return Err(format!("Unknown shortcut key: {key}")),
    };
    Ok((modifiers, keysym))
}

#[cfg(target_os = "linux")]
fn resolve_x11_keycode<C: x11rb::connection::Connection>(
    connection: &C,
    target_keysym: u32,
) -> Result<u8, String> {
    use x11rb::protocol::xkb;
    use x11rb::protocol::xkb::ConnectionExt as _;

    let device: xkb::DeviceSpec = xkb::ID::USE_CORE_KBD.into();
    let state = connection
        .xkb_get_state(device)
        .map_err(|error| error.to_string())?
        .reply()
        .map_err(|error| error.to_string())?;
    let map = connection
        .xkb_get_map(
            device,
            xkb::MapPart::KEY_SYMS,
            xkb::MapPart::default(),
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            xkb::VMod::default(),
            0,
            0,
            0,
            0,
            0,
            0,
        )
        .map_err(|error| error.to_string())?
        .reply()
        .map_err(|error| error.to_string())?;
    let group = u8::from(state.group) as usize;
    let keymaps = map
        .map
        .syms_rtrn
        .ok_or_else(|| "X11 did not return a keyboard symbol map.".to_string())?;
    for (offset, keymap) in keymaps.iter().enumerate() {
        let width = keymap.width as usize;
        if width == 0 {
            continue;
        }
        let group_count = keymap.syms.len() / width;
        if group_count == 0 {
            continue;
        }
        let active_group = group.min(group_count - 1);
        if keymap.syms.get(active_group * width).copied() == Some(target_keysym) {
            return map
                .first_key_sym
                .checked_add(offset as u8)
                .ok_or_else(|| "The X11 keycode is out of range.".to_string());
        }
    }
    Err(format!(
        "The active X11 layout does not provide keysym 0x{target_keysym:x}."
    ))
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
    fn clip_hotkeys_keep_their_stable_clip_id() {
        let action = AppHotkeyAction::PasteClipById(42);
        assert_eq!(action, AppHotkeyAction::PasteClipById(42));
        assert_ne!(action, AppHotkeyAction::PasteClip(1));
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
