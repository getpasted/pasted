use std::collections::HashMap;

use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

#[cfg(target_os = "linux")]
use super::wayland_backend::is_wayland_session;
use super::{HotkeyManager, HotkeyRegistrationIssue, HotkeyRegistrationStatus, HotkeySpec};

impl HotkeyManager {
    #[cfg_attr(target_os = "linux", allow(dead_code))]
    pub(super) fn register_native(
        &self,
        app: &AppHandle,
        specs: Vec<HotkeySpec>,
    ) -> Result<(), String> {
        let configured_count = specs.len();
        let mut registered_count = 0;
        let mut issues = Vec::new();
        let mut map = self.action_map.write();
        let mut parsed_specs = Vec::new();
        let mut hotkey_counts = HashMap::<Shortcut, usize>::new();

        for spec in specs {
            let Some(shortcuts) = crate::keyboard_shortcuts::parse_for_current_layout(&spec.hotkey)
            else {
                issues.push(HotkeyRegistrationIssue {
                    hotkey: spec.hotkey,
                    description: spec.description,
                    message: "Pasted could not understand this hotkey.".into(),
                });
                continue;
            };
            for shortcut in &shortcuts {
                *hotkey_counts.entry(*shortcut).or_default() += 1;
            }
            parsed_specs.push((spec, shortcuts));
        }

        for (spec, shortcuts) in parsed_specs {
            if shortcuts
                .iter()
                .any(|shortcut| hotkey_counts.get(shortcut).copied().unwrap_or_default() > 1)
            {
                issues.push(HotkeyRegistrationIssue {
                    hotkey: spec.hotkey,
                    description: spec.description,
                    message: "This hotkey is assigned to more than one action.".into(),
                });
                continue;
            }

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
                let message = last_error.unwrap_or_else(|| "The hotkey is unavailable.".into());
                eprintln!(
                    "[Pasted Hotkeys] Could not register '{}' for {}: {message}",
                    spec.hotkey, spec.description
                );
                issues.push(HotkeyRegistrationIssue {
                    hotkey: spec.hotkey,
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
                "{} hotkey{} could not be registered",
                issues.len(),
                if issues.len() == 1 { "" } else { "s" }
            ))
        }
    }
}

pub(super) fn native_backend_name() -> &'static str {
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
