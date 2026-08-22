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
mod tests {
    use super::*;

    #[test]
    fn test_hotkey_manager_maps_actions() {
        let mgr = HotkeyManager::new();
        let sc = crate::keyboard_shortcuts::parse("CmdOrCtrl+Shift+V").unwrap();

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
    fn converts_pasted_hotkeys_to_xdg_triggers() {
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

    #[test]
    fn xdg_preflight_rejects_invalid_and_duplicate_hotkeys() {
        let specs = vec![
            HotkeySpec {
                id: "first".into(),
                description: "First".into(),
                hotkey: "Alt+Shift+V".into(),
                action: AppHotkeyAction::ToggleHud,
            },
            HotkeySpec {
                id: "duplicate".into(),
                description: "Duplicate".into(),
                hotkey: "Option+Shift+V".into(),
                action: AppHotkeyAction::ToggleMainWindow,
            },
            HotkeySpec {
                id: "invalid".into(),
                description: "Invalid".into(),
                hotkey: "Alt+NoSuchKey".into(),
                action: AppHotkeyAction::OpenTransformations,
            },
            HotkeySpec {
                id: "valid".into(),
                description: "Valid".into(),
                hotkey: "Control+F8".into(),
                action: AppHotkeyAction::ToggleCopyQueue,
            },
        ];

        let (prepared, issues) = prepare_xdg_hotkeys(specs);
        assert_eq!(prepared.len(), 1);
        assert_eq!(prepared[0].0.id, "valid");
        assert_eq!(prepared[0].1, "CTRL+F8");
        assert_eq!(issues.len(), 3);
        assert!(issues
            .iter()
            .any(|issue| issue.message.contains("more than one action")));
        assert!(issues
            .iter()
            .any(|issue| issue.message.contains("could not understand")));
    }

    #[test]
    fn clipboard_hotkey_actions_do_not_overlap() {
        let manager = HotkeyManager::new();
        let first = manager.clipboard_action_guard.try_lock();
        assert!(first.is_some());
        assert!(manager.clipboard_action_guard.try_lock().is_none());
        drop(first);
        assert!(manager.clipboard_action_guard.try_lock().is_some());
    }
}
