use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

mod action_dispatch;
mod native_backend;
mod registration;
mod wayland_backend;
#[cfg(target_os = "linux")]
mod x11_backend;
use native_backend::native_backend_name;
#[cfg(test)]
use wayland_backend::{prepare_xdg_hotkeys, shortcut_to_xdg_trigger};

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
    PasteWithManualTransform(String),
    CopyWithLastManualTransform,
    PasteWithLastManualTransform,
    SmartPaste,
    OpenBin(i64),
}

#[derive(Debug, Clone)]
struct HotkeySpec {
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    id: String,
    description: String,
    hotkey: String,
    action: AppHotkeyAction,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct HotkeyRegistrationIssue {
    pub hotkey: String,
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
    clipboard_action_guard: Arc<parking_lot::Mutex<()>>,
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
            clipboard_action_guard: Arc::new(parking_lot::Mutex::new(())),
            #[cfg(target_os = "linux")]
            portal_task: parking_lot::Mutex::new(None),
            #[cfg(target_os = "linux")]
            x11_task: parking_lot::Mutex::new(None),
        }
    }

    pub fn registration_status(&self) -> HotkeyRegistrationStatus {
        self.registration_status.read().clone()
    }

    fn clear_registrations(&self, app: &AppHandle) {
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
    }
}

#[cfg(test)]
mod tests;
