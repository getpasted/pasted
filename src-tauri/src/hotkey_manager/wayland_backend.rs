use std::collections::HashMap;
#[cfg(target_os = "linux")]
use std::sync::Arc;

#[cfg(target_os = "linux")]
use futures_util::StreamExt;
#[cfg(target_os = "linux")]
use tauri::{AppHandle, Emitter};

#[cfg(target_os = "linux")]
use super::{AppHotkeyAction, HotkeyManager, HotkeyRegisteredBinding, HotkeyRegistrationStatus};
use super::{HotkeyRegistrationIssue, HotkeySpec};

#[cfg(target_os = "linux")]
impl HotkeyManager {
    #[cfg(target_os = "linux")]
    pub(super) fn register_wayland_portal(
        self: &Arc<Self>,
        app: AppHandle,
        specs: Vec<HotkeySpec>,
    ) -> Result<(), String> {
        let configured_count = specs.len();
        let (prepared, issues) = prepare_xdg_hotkeys(specs);
        *self.registration_status.write() = HotkeyRegistrationStatus {
            backend: "wayland-portal".into(),
            state: "checking".into(),
            configured_count,
            registered_count: 0,
            issues: issues.clone(),
            bindings: Vec::new(),
        };

        if !issues.is_empty() {
            self.registration_status.write().state = "conflict".into();
            let _ = app.emit("hotkey-registration-changed", ());
            return Err(format!(
                "{} hotkey{} could not be prepared for the desktop portal",
                issues.len(),
                if issues.len() == 1 { "" } else { "s" }
            ));
        }

        if prepared.is_empty() {
            self.registration_status.write().state = "ready".into();
            let _ = app.emit("hotkey-registration-changed", ());
            return Ok(());
        }

        let manager = Arc::clone(self);
        let failure_app = app.clone();
        let task = tauri::async_runtime::spawn(async move {
            if let Err(error) = manager.run_wayland_portal(app, prepared).await {
                eprintln!("[Pasted Hotkeys] Wayland portal unavailable: {error}");
                let mut status = manager.registration_status.write();
                status.state = "unavailable".into();
                status.registered_count = 0;
                status.bindings.clear();
                status.issues = vec![HotkeyRegistrationIssue {
                    hotkey: String::new(),
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
        specs: Vec<(HotkeySpec, String)>,
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
            .map_err(|error| format!("Could not listen for portal hotkeys: {error}"))?;
        let mut shortcuts_changed = portal
            .receive_shortcuts_changed()
            .await
            .map_err(|error| format!("Could not listen for portal hotkey changes: {error}"))?;
        let session = portal
            .create_session(CreateSessionOptions::default())
            .await
            .map_err(|error| format!("Could not create a portal hotkey session: {error}"))?;

        let portal_shortcuts: Vec<NewShortcut> = specs
            .iter()
            .map(|(spec, trigger)| {
                NewShortcut::new(spec.id.clone(), spec.description.clone())
                    .preferred_trigger(Some(trigger.as_str()))
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
            .map_err(|error| format!("Could not ask the desktop to bind hotkeys: {error}"))?;
        let response = request
            .response()
            .map_err(|error| format!("The desktop declined the hotkey request: {error}"))?;
        let bound_ids: std::collections::HashSet<&str> = response
            .shortcuts()
            .iter()
            .map(|shortcut| shortcut.id())
            .collect();
        let actions: HashMap<String, AppHotkeyAction> = specs
            .iter()
            .filter(|(spec, _)| bound_ids.contains(spec.id.as_str()))
            .map(|(spec, _)| (spec.id.clone(), spec.action.clone()))
            .collect();
        let issues: Vec<HotkeyRegistrationIssue> = specs
            .iter()
            .filter(|(spec, _)| !bound_ids.contains(spec.id.as_str()))
            .map(|(spec, _)| HotkeyRegistrationIssue {
                hotkey: spec.hotkey.clone(),
                description: spec.description.clone(),
                message: "The desktop did not enable this hotkey.".into(),
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
}

#[cfg(target_os = "linux")]
pub(super) fn is_wayland_session() -> bool {
    std::env::var("XDG_SESSION_TYPE")
        .map(|value| value.eq_ignore_ascii_case("wayland"))
        .unwrap_or(false)
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

#[cfg_attr(not(any(target_os = "linux", test)), allow(dead_code))]
pub(super) fn shortcut_to_xdg_trigger(shortcut: &str) -> Option<String> {
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

#[cfg_attr(not(any(target_os = "linux", test)), allow(dead_code))]
pub(super) fn prepare_xdg_hotkeys(
    specs: Vec<HotkeySpec>,
) -> (Vec<(HotkeySpec, String)>, Vec<HotkeyRegistrationIssue>) {
    let mut parsed = Vec::new();
    let mut issues = Vec::new();
    let mut trigger_counts = HashMap::<String, usize>::new();

    for spec in specs {
        let Some(trigger) = shortcut_to_xdg_trigger(&spec.hotkey) else {
            issues.push(HotkeyRegistrationIssue {
                hotkey: spec.hotkey,
                description: spec.description,
                message: "Pasted could not understand this hotkey.".into(),
            });
            continue;
        };
        *trigger_counts.entry(trigger.clone()).or_default() += 1;
        parsed.push((spec, trigger));
    }

    let mut prepared = Vec::new();
    for (spec, trigger) in parsed {
        if trigger_counts.get(&trigger).copied().unwrap_or_default() > 1 {
            issues.push(HotkeyRegistrationIssue {
                hotkey: spec.hotkey,
                description: spec.description,
                message: "This hotkey is assigned to more than one action.".into(),
            });
        } else {
            prepared.push((spec, trigger));
        }
    }
    (prepared, issues)
}
