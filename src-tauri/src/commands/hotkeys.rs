use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

use crate::db::DbState;
use crate::features::{self, Feature};

pub fn register_all_app_shortcuts(app: &AppHandle) -> Result<(), String> {
    if let Some(mgr) = app.try_state::<Arc<crate::hotkey_manager::HotkeyManager>>() {
        mgr.register_all(app)
    } else {
        Err("HotkeyManager state not initialized".to_string())
    }
}

pub(crate) fn register_changed_hotkeys(
    app: &AppHandle,
    changed_hotkeys: &[String],
) -> Result<(), String> {
    let Err(error) = register_all_app_shortcuts(app) else {
        return Ok(());
    };
    let Some(manager) = app.try_state::<Arc<crate::hotkey_manager::HotkeyManager>>() else {
        return Err(error);
    };
    let status = manager.registration_status();
    if status.state != "conflict" {
        return Err(error);
    }
    if changed_hotkeys_have_registration_issue(changed_hotkeys, &status.issues) {
        Err(error)
    } else {
        Ok(())
    }
}

fn changed_hotkeys_have_registration_issue(
    changed_hotkeys: &[String],
    issues: &[crate::hotkey_manager::HotkeyRegistrationIssue],
) -> bool {
    changed_hotkeys.iter().any(|changed| {
        let changed = changed.trim();
        !changed.is_empty() && issues.iter().any(|issue| issue.hotkey.trim() == changed)
    })
}

pub type AccessibilityStatus = crate::platform_capabilities::AccessibilityStatus;

pub fn check_accessibility_permission() -> AccessibilityStatus {
    crate::platform_capabilities::accessibility_status()
}

#[derive(serde::Serialize)]
pub struct HotkeyCapabilityStatus {
    pub platform: String,
    pub backend: String,
    pub state: String,
    pub is_trusted: bool,
    pub is_dev_mode: bool,
    pub configured_count: usize,
    pub registered_count: usize,
    pub issues: Vec<crate::hotkey_manager::HotkeyRegistrationIssue>,
    pub bindings: Vec<crate::hotkey_manager::HotkeyRegisteredBinding>,
}

#[tauri::command]
pub fn get_hotkey_capability_status(app: AppHandle) -> HotkeyCapabilityStatus {
    let accessibility = check_accessibility_permission();
    let registration = app
        .try_state::<Arc<crate::hotkey_manager::HotkeyManager>>()
        .map(|manager| manager.registration_status())
        .unwrap_or_default();
    let platform = if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unsupported"
    };

    HotkeyCapabilityStatus {
        platform: platform.into(),
        backend: registration.backend,
        state: registration.state,
        is_trusted: accessibility.is_trusted,
        is_dev_mode: accessibility.is_dev_mode,
        configured_count: registration.configured_count,
        registered_count: registration.registered_count,
        issues: registration.issues,
        bindings: registration.bindings,
    }
}

#[tauri::command]
pub fn request_accessibility_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let _ = Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();
        let _ = Command::new("open")
            .arg("x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_Accessibility")
            .spawn();

        let status = check_accessibility_permission();
        status.is_trusted
    }
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let _ = Command::new("cmd")
            .arg("/c")
            .arg("start ms-settings:privacy-accessibility")
            .spawn();
        true
    }
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        let _ = Command::new("gnome-control-center").spawn();
        true
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    true
}

#[tauri::command]
pub fn register_app_setting_hotkey(
    key: String,
    value: String,
    app: AppHandle,
) -> Result<(), String> {
    if !is_app_setting_hotkey_key(&key) {
        return Err("Unknown app hotkey setting.".to_string());
    }
    persist_hotkey_settings_and_register(std::iter::once((key, value)).collect(), &app)
}

fn is_app_setting_hotkey_key(key: &str) -> bool {
    matches!(
        key,
        "hudHotkey"
            | "seqToggleHotkey"
            | "seqPopHotkey"
            | "copyLastPipelineHotkey"
            | "pasteLastPipelineHotkey"
            | "openTransformationsHotkey"
            | "openMainWindowHotkey"
            | "lockAppHotkey"
    ) || key
        .strip_prefix("pasteClip")
        .and_then(|suffix| suffix.strip_suffix("Hotkey"))
        .and_then(|position| position.parse::<usize>().ok())
        .is_some_and(|position| (1..=9).contains(&position))
}

#[tauri::command]
pub fn register_app_setting_hotkeys(
    values: std::collections::HashMap<String, String>,
    app: AppHandle,
) -> Result<(), String> {
    if values.keys().any(|key| !is_app_setting_hotkey_key(key)) {
        return Err("Unknown app hotkey setting.".to_string());
    }
    persist_hotkey_settings_and_register(values, &app)
}

fn persist_hotkey_settings_and_register(
    values: std::collections::HashMap<String, String>,
    app: &AppHandle,
) -> Result<(), String> {
    let db = app.state::<Arc<DbState>>();
    features::require(&db, Feature::Hotkeys)?;
    let previous: std::collections::HashMap<String, Option<String>> = values
        .keys()
        .map(|key| {
            db.get_setting(key)
                .map(|value| (key.clone(), value))
                .map_err(|error| error.to_string())
        })
        .collect::<Result<_, _>>()?;
    if values.iter().all(|(key, value)| {
        previous
            .get(key)
            .and_then(|previous_value| previous_value.as_deref())
            == Some(value.as_str())
    }) {
        return Ok(());
    }
    let changed_hotkeys: Vec<String> = values
        .iter()
        .filter(|(key, value)| {
            previous
                .get(*key)
                .and_then(|previous_value| previous_value.as_deref())
                != Some(value.as_str())
        })
        .map(|(_, value)| value.clone())
        .collect();
    db.save_settings(&values)
        .map_err(|error| error.to_string())?;
    if let Err(registration_error) = register_changed_hotkeys(app, &changed_hotkeys) {
        let restored: std::collections::HashMap<String, String> = previous
            .iter()
            .filter_map(|(key, value)| value.clone().map(|value| (key.clone(), value)))
            .collect();
        let deleted: Vec<&str> = previous
            .iter()
            .filter_map(|(key, value)| value.is_none().then_some(key.as_str()))
            .collect();
        db.save_and_delete_settings(&restored, &deleted)
            .map_err(|error| {
                format!(
                    "{registration_error}; restoring the previous shortcut settings failed: {error}"
                )
            })?;
        if let Err(rollback_error) = register_all_app_shortcuts(app) {
            return Err(format!(
                "{registration_error}; restoring the previous native shortcuts failed: {rollback_error}"
            ));
        }
        return Err(registration_error);
    }
    Ok(())
}

#[tauri::command]
pub fn resolve_logical_shortcut_key(code: String, fallback: String) -> String {
    use std::str::FromStr;

    tauri_plugin_global_shortcut::Code::from_str(&code)
        .ok()
        .and_then(crate::keyboard_layout::logical_key_for_code)
        .unwrap_or(fallback)
}

#[tauri::command]
pub fn register_hud_hotkey(hotkey: String, app: AppHandle) -> Result<(), String> {
    persist_hotkey_settings_and_register(
        std::iter::once(("hudHotkey".to_string(), hotkey)).collect(),
        &app,
    )
}

#[tauri::command]
pub fn get_clip_hotkey_assignments(
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<ClipHotkeyAssignment>, String> {
    features::require(&db, Feature::Hotkeys)?;
    db.get_clip_hotkeys()
        .map(|assignments| {
            assignments
                .into_iter()
                .map(|(clip_id, hotkey)| ClipHotkeyAssignment { clip_id, hotkey })
                .collect()
        })
        .map_err(|error| error.to_string())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipHotkeyAssignment {
    clip_id: i64,
    hotkey: String,
}

#[tauri::command]
pub fn update_clip_hotkey(
    clip_id: i64,
    hotkey: Option<String>,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<crate::db::ClipItem, String> {
    features::require(&db, Feature::Protection)?;
    features::require(&db, Feature::Hotkeys)?;
    let previous = db
        .get_clip_by_id(clip_id)
        .map_err(|error| error.to_string())?;
    let previous_shortcut = previous.shortcut.clone();
    let previous_explicit = previous
        .is_explicitly_protected
        .unwrap_or(previous.is_protected);
    db.update_clip_hotkey(clip_id, hotkey.as_deref())
        .map_err(|error| error.to_string())?;
    let changed_hotkeys: Vec<String> = hotkey.clone().into_iter().collect();
    if let Err(error) = register_changed_hotkeys(&app, &changed_hotkeys) {
        db.restore_clip_hotkey_state(clip_id, previous_shortcut.as_deref(), previous_explicit)
            .map_err(|rollback| {
                format!("{error}; restoring the previous clip hotkey failed: {rollback}")
            })?;
        let _ = register_all_app_shortcuts(&app);
        return Err(error);
    }
    let assigned = hotkey
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    let activity_description = if assigned {
        format!("Assigned a hotkey to clip #{clip_id}")
    } else {
        format!("Removed the hotkey from clip #{clip_id}")
    };
    let _ = db.log_activity("clip_hotkey_changed", &activity_description);
    let clip = db
        .get_clip_by_id(clip_id)
        .map_err(|error| error.to_string())?;
    crate::app_events::emit_clip_library_changed(&app, vec![clip_id]);
    Ok(clip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shortcut_str_variations() {
        assert!(crate::keyboard_shortcuts::parse("CmdOrCtrl+Shift+V").is_some());
        assert!(crate::keyboard_shortcuts::parse("Control+Alt+C").is_some());
        assert!(crate::keyboard_shortcuts::parse("Ctrl+Alt+KeyC").is_some());
        assert!(crate::keyboard_shortcuts::parse("Alt+Super+KeyV").is_some());
        assert!(crate::keyboard_shortcuts::parse("Option+Cmd+C").is_some());
        assert!(crate::keyboard_shortcuts::parse("Command+Shift+V").is_some());
        assert!(crate::keyboard_shortcuts::parse("Control+Option+C").is_some());
        assert!(crate::keyboard_shortcuts::parse("Control+Option+V").is_some());
        assert!(crate::keyboard_shortcuts::parse("Super+Alt+KeyC").is_some());
        assert!(crate::keyboard_shortcuts::parse("").is_none());
        assert!(crate::keyboard_shortcuts::parse("   ").is_none());

        // Equivalence checks for key representations
        let sc1 = crate::keyboard_shortcuts::parse("Option+Command+C").unwrap();
        let sc2 = crate::keyboard_shortcuts::parse("Alt+Super+KeyC").unwrap();
        assert_eq!(
            sc1, sc2,
            "Option+Command+C should resolve to identical Shortcut struct as Alt+Super+KeyC"
        );
    }

    #[test]
    fn app_setting_hotkey_keys_are_narrowly_scoped() {
        assert!(is_app_setting_hotkey_key("hudHotkey"));
        assert!(is_app_setting_hotkey_key("lockAppHotkey"));
        assert!(is_app_setting_hotkey_key("pasteClip1Hotkey"));
        assert!(is_app_setting_hotkey_key("pasteClip9Hotkey"));
        assert!(!is_app_setting_hotkey_key("unlockAppHotkey"));
        assert!(!is_app_setting_hotkey_key("pasteClip0Hotkey"));
        assert!(!is_app_setting_hotkey_key("pasteClip10Hotkey"));
        assert!(!is_app_setting_hotkey_key("enableAppLock"));
    }

    #[test]
    fn unrelated_hotkey_conflicts_do_not_reject_a_change() {
        let issues = vec![crate::hotkey_manager::HotkeyRegistrationIssue {
            hotkey: "Alt+Shift+V".into(),
            description: "HUD".into(),
            message: "Unavailable".into(),
        }];
        assert!(!changed_hotkeys_have_registration_issue(
            &["Alt+Shift+L".into()],
            &issues
        ));
        assert!(changed_hotkeys_have_registration_issue(
            &[" Alt+Shift+V ".into()],
            &issues
        ));
        assert!(!changed_hotkeys_have_registration_issue(
            &[String::new()],
            &issues
        ));
    }
}
