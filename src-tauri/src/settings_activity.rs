use crate::features::Feature;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingActivity {
    pub event_type: &'static str,
    pub description: String,
}

fn on_off(value: &str) -> Option<&'static str> {
    match value {
        "true" => Some("On"),
        "false" => Some("Off"),
        _ => None,
    }
}

fn friendly_value(key: &str, value: &str) -> Option<String> {
    if Feature::from_setting_key(key).is_some() {
        return on_off(value).map(str::to_string);
    }

    let value = match key {
        "language" => match value {
            "system" => "Automatic".into(),
            "en" => "English".into(),
            "de-DE" => "German".into(),
            "fr-FR" => "French".into(),
            "ja-JP" => "Japanese".into(),
            _ => return None,
        },
        "themeMode" => match value {
            "system" => "System".into(),
            "cool" => "Cool".into(),
            "dark" => "Dark".into(),
            "warm" => "Warm".into(),
            "2894" => "2894".into(),
            "sauced" => "Sauced".into(),
            "vampire" => "Vampire".into(),
            "flux" => "Flux".into(),
            "808" => "808".into(),
            _ => return None,
        },
        "rowHeight" => match value {
            "small" => "Compact".into(),
            "medium" => "Comfortable".into(),
            "large" => "Spacious".into(),
            _ => return None,
        },
        "startupView" => match value {
            "last_active" => "Last Active Page".into(),
            "clip_history" => "Clip History".into(),
            _ => return None,
        },
        "dockMenubarIcon" => match value {
            "both" => "Dock and menu bar".into(),
            "menubar_only" => "Menu bar only".into(),
            "auto_hide" => "Automatically hidden Dock icon".into(),
            _ => return None,
        },
        "menubarIconStyle" => match value {
            "clipboard" => "Clipboard".into(),
            "copycat" => "Copycat".into(),
            _ => return None,
        },
        "filePreviewMode" => match value {
            "off" => "Off".into(),
            "safe" => "Safe file types".into(),
            "all" => "All supported files".into(),
            _ => return None,
        },
        "captureFeedbackPosition" => match value {
            "top-left" => "Top left".into(),
            "top-right" => "Top right".into(),
            "bottom-left" => "Bottom left".into(),
            "bottom-right" => "Bottom right".into(),
            _ => return None,
        },
        "textSize" => format!("{value}px"),
        "maxClipSizeMb" | "filePreviewMaxMb" => format!("{value} MB"),
        "keepClipCount" | "activityLogCapacity" | "trashCapacityCount" if value == "0" => {
            "Unlimited".into()
        }
        "keepClipCount" | "revisionHistoryLimit" | "activityLogCapacity" | "trashCapacityCount" => {
            value.to_string()
        }
        "keepClipAgeDays" | "activityLogAgeDays" | "trashAgeDays" => match value {
            "0" => "Forever".into(),
            "1" => "1 day".into(),
            _ => format!("{value} days"),
        },
        "captureFeedbackDismissSeconds" => match value {
            "0" => "Never".into(),
            "3" | "5" | "7" | "10" | "15" | "30" => format!("{value} seconds"),
            _ => return None,
        },
        "appLockIdleMinutes" => match value {
            "0" => "Never".into(),
            "1" => "1 minute".into(),
            "5" => "5 minutes".into(),
            "60" => "1 hour".into(),
            "480" => "8 hours".into(),
            _ => return None,
        },
        "enableSounds"
        | "captureFeedback"
        | "captureFeedbackIgnored"
        | "captureFeedbackPreview"
        | "alwaysPastePlainText"
        | "appLockSystemAuthEnabled"
        | "appLockAppleWatchEnabled"
        | "appLockOnSleep"
        | "appLockOnRestart"
        | "appLockCaptureWhileLocked" => on_off(value)?.into(),
        _ if key.ends_with("Hotkey") => {
            if value.is_empty() {
                "Not set".into()
            } else {
                value.to_string()
            }
        }
        _ => return None,
    };
    Some(value)
}

fn setting_label(key: &str) -> Option<&'static str> {
    if let Some(feature) = Feature::from_setting_key(key) {
        return Some(feature.label());
    }
    match key {
        "language" => Some("Language"),
        "themeMode" => Some("Appearance"),
        "rowHeight" => Some("Row height"),
        "startupView" => Some("Startup View"),
        "dockMenubarIcon" => Some("Dock visibility"),
        "menubarIconStyle" => Some("Menu bar icon"),
        "filePreviewMode" => Some("File previews"),
        "textSize" => Some("Zoom"),
        "maxClipSizeMb" => Some("Clip size limit"),
        "filePreviewMaxMb" => Some("File preview limit"),
        "keepClipCount" => Some("History limit"),
        "keepClipAgeDays" => Some("History age limit"),
        "revisionHistoryLimit" => Some("Revision limit"),
        "activityLogCapacity" => Some("Activity limit"),
        "activityLogAgeDays" => Some("Activity age limit"),
        "trashCapacityCount" => Some("Trash limit"),
        "trashAgeDays" => Some("Trash age limit"),
        "enableSounds" => Some("Interaction sounds"),
        "captureFeedback" => Some("Capture feedback"),
        "captureFeedbackIgnored" => Some("Skipped capture feedback"),
        "captureFeedbackPreview" => Some("Clip previews in capture feedback"),
        "captureFeedbackPosition" => Some("Capture feedback position"),
        "captureFeedbackDismissSeconds" => Some("Capture preview dismissal"),
        "alwaysPastePlainText" => Some("Plain-text paste"),
        "appLockSystemAuthEnabled" => Some("System authentication"),
        "appLockAppleWatchEnabled" => Some("Apple Watch unlock"),
        "appLockIdleMinutes" => Some("App Lock idle timing"),
        "appLockOnSleep" => Some("Lock when device locks or sleeps"),
        "appLockOnRestart" => Some("Lock after restart"),
        "appLockCaptureWhileLocked" => Some("Capture while locked"),
        "hudHotkey" => Some("HUD shortcut"),
        "seqToggleHotkey" => Some("Copy Queue shortcut"),
        "seqPopHotkey" => Some("Paste Next shortcut"),
        "copyLastPipelineHotkey" => Some("Copy with Transform shortcut"),
        "pasteLastPipelineHotkey" => Some("Paste with Transform shortcut"),
        "openTransformationsHotkey" => Some("Transformations shortcut"),
        "openMainWindowHotkey" => Some("Main window shortcut"),
        _ if key.starts_with("pasteClip") && key.ends_with("Hotkey") => Some("Clip shortcut"),
        _ => None,
    }
}

pub fn describe_setting_change(
    key: &str,
    previous: Option<&str>,
    next: &str,
) -> Option<SettingActivity> {
    if previous == Some(next) {
        return None;
    }

    if key == "openAtLogin" {
        let enabled = next == "true";
        return Some(SettingActivity {
            event_type: if enabled {
                "autostart_enabled"
            } else {
                "autostart_disabled"
            },
            description: if enabled {
                "Enabled opening Pasted at login".into()
            } else {
                "Disabled opening Pasted at login".into()
            },
        });
    }

    if key == "blacklistApps" {
        return Some(SettingActivity {
            event_type: "setting_changed",
            description: "Updated excluded apps".into(),
        });
    }

    let label = setting_label(key)?;
    let next = friendly_value(key, next)?;
    let description = match previous.and_then(|value| friendly_value(key, value)) {
        Some(previous) => format!("Changed {label}: {previous} → {next}"),
        None => format!("Changed {label} to {next}"),
    };
    Some(SettingActivity {
        event_type: "setting_changed",
        description,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_safe_human_readable_changes() {
        assert_eq!(
            describe_setting_change("themeMode", Some("dark"), "warm"),
            Some(SettingActivity {
                event_type: "setting_changed",
                description: "Changed Appearance: Dark → Warm".into(),
            })
        );
        assert_eq!(
            describe_setting_change("themeMode", Some("warm"), "2894")
                .unwrap()
                .description,
            "Changed Appearance: Warm → 2894"
        );
        assert_eq!(
            describe_setting_change("themeMode", Some("2894"), "sauced")
                .unwrap()
                .description,
            "Changed Appearance: 2894 → Sauced"
        );
        assert_eq!(
            describe_setting_change("enableBins", Some("true"), "false")
                .unwrap()
                .description,
            "Changed Bins: On → Off"
        );
        assert_eq!(
            describe_setting_change("startupView", Some("last_active"), "clip_history")
                .unwrap()
                .description,
            "Changed Startup View: Last Active Page → Clip History"
        );
        assert_eq!(
            describe_setting_change("menubarIconStyle", Some("clipboard"), "copycat")
                .unwrap()
                .description,
            "Changed Menu bar icon: Clipboard → Copycat"
        );
        assert_eq!(
            describe_setting_change("language", Some("system"), "en")
                .unwrap()
                .description,
            "Changed Language: Automatic → English"
        );
        assert_eq!(
            describe_setting_change("language", Some("en"), "de-DE")
                .unwrap()
                .description,
            "Changed Language: English → German"
        );
        assert_eq!(
            describe_setting_change("language", Some("de-DE"), "fr-FR")
                .unwrap()
                .description,
            "Changed Language: German → French"
        );
        assert_eq!(
            describe_setting_change("language", Some("fr-FR"), "ja-JP")
                .unwrap()
                .description,
            "Changed Language: French → Japanese"
        );
    }

    #[test]
    fn omits_sensitive_and_internal_values() {
        assert!(describe_setting_change("apiKey", Some("old"), "secret").is_none());
        assert!(describe_setting_change("windowPosition", Some("1,2"), "3,4").is_none());
        assert_eq!(
            describe_setting_change("blacklistApps", Some("secret-json"), "new-secret-json")
                .unwrap()
                .description,
            "Updated excluded apps"
        );
    }

    #[test]
    fn distinguishes_autostart_lifecycle_events() {
        assert_eq!(
            describe_setting_change("openAtLogin", Some("false"), "true")
                .unwrap()
                .event_type,
            "autostart_enabled"
        );
        assert!(describe_setting_change("openAtLogin", Some("true"), "true").is_none());
    }

    #[test]
    fn reports_shortcut_changes_without_exposing_unrelated_settings() {
        assert_eq!(
            describe_setting_change("hudHotkey", Some("Alt+Shift+V"), "Command+Space")
                .unwrap()
                .description,
            "Changed HUD shortcut: Alt+Shift+V → Command+Space"
        );
    }

    #[test]
    fn formats_app_lock_setting_changes_without_sensitive_values() {
        assert_eq!(
            describe_setting_change("appLockIdleMinutes", Some("5"), "60")
                .unwrap()
                .description,
            "Changed App Lock idle timing: 5 minutes → 1 hour"
        );
        assert_eq!(
            describe_setting_change("appLockSystemAuthEnabled", Some("false"), "true")
                .unwrap()
                .description,
            "Changed System authentication: Off → On"
        );
        assert_eq!(
            describe_setting_change("appLockOnRestart", Some("true"), "false")
                .unwrap()
                .description,
            "Changed Lock after restart: On → Off"
        );
        assert_eq!(
            describe_setting_change("appLockCaptureWhileLocked", Some("true"), "false")
                .unwrap()
                .description,
            "Changed Capture while locked: On → Off"
        );
        assert!(describe_setting_change("appLockVerifier", Some("old"), "new").is_none());
    }
}
