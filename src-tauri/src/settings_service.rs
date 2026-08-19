use std::collections::HashMap;

use serde::Serialize;

use crate::{db::DbState, features::Feature};

pub const MAX_SETTING_KEY_BYTES: usize = 128;
pub const MAX_SETTING_VALUE_BYTES: usize = 1024 * 1024;

const BOOLEAN_SETTINGS: &[&str] = &[
    "enableSounds",
    "captureFeedback",
    "captureFeedbackIgnored",
    "captureFeedbackPreview",
    "openAtLogin",
    "alwaysPastePlainText",
];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingChange {
    pub key: String,
    pub previous_value: Option<String>,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsUpdateOutcome {
    pub changes: Vec<SettingChange>,
    pub changed_feature_keys: Vec<String>,
}

impl SettingsUpdateOutcome {
    pub fn changed_features(&self) -> Vec<Feature> {
        self.changed_feature_keys
            .iter()
            .filter_map(|key| Feature::from_setting_key(key))
            .collect()
    }
}

fn validate_boolean(key: &str, value: &str) -> Result<(), String> {
    if matches!(value, "true" | "false") {
        Ok(())
    } else {
        Err(format!("{key} must be true or false"))
    }
}

fn validate_integer(key: &str, value: &str, minimum: i64, maximum: i64) -> Result<(), String> {
    let parsed = value
        .parse::<i64>()
        .map_err(|_| format!("{key} must be a whole number"))?;
    if (minimum..=maximum).contains(&parsed) {
        Ok(())
    } else {
        Err(format!("{key} must be between {minimum} and {maximum}"))
    }
}

fn validate_choice(key: &str, value: &str, choices: &[&str]) -> Result<(), String> {
    if choices.contains(&value) {
        Ok(())
    } else {
        Err(format!("{key} has an unsupported value"))
    }
}

pub fn validate_setting(key: &str, value: &str) -> Result<(), String> {
    if key.trim().is_empty() || key.len() > MAX_SETTING_KEY_BYTES {
        return Err(format!(
            "Setting keys must contain 1–{MAX_SETTING_KEY_BYTES} bytes"
        ));
    }
    if value.len() > MAX_SETTING_VALUE_BYTES {
        return Err(format!(
            "Setting values cannot exceed {MAX_SETTING_VALUE_BYTES} bytes"
        ));
    }
    if key == "pendingFullBackupClientState" || crate::app_lock::is_managed_setting(key) {
        return Err("That setting must be changed through its dedicated controls".into());
    }
    if key == crate::localization::LANGUAGE_SETTING_KEY {
        return crate::localization::validate_configured_language(value);
    }
    if Feature::from_setting_key(key).is_some() || BOOLEAN_SETTINGS.contains(&key) {
        return validate_boolean(key, value);
    }
    match key {
        "captureFeedbackPosition" => validate_choice(
            key,
            value,
            &["top-left", "top-right", "bottom-left", "bottom-right"],
        ),
        "dockMenubarIcon" => validate_choice(key, value, &["auto_hide", "both", "menubar_only"]),
        "filePreviewMode" => validate_choice(key, value, &["off", "safe", "all"]),
        "menubarIconStyle" => validate_choice(key, value, &["clipboard", "copycat"]),
        "rowHeight" => validate_choice(key, value, &["small", "medium", "large"]),
        "startupView" => validate_choice(key, value, &["last_active", "clip_history"]),
        "themeMode" => validate_choice(
            key,
            value,
            &[
                "system", "cool", "dark", "warm", "2894", "sauced", "vampire", "flux", "808",
            ],
        ),
        "captureFeedbackDismissSeconds" => {
            validate_choice(key, value, &["0", "3", "5", "7", "10", "15", "30"])
        }
        "textSize" => validate_integer(key, value, 12, 24),
        "maxClipSizeMb" => validate_integer(key, value, 1, 256),
        "filePreviewMaxMb" => validate_integer(key, value, 1, 64),
        "keepClipCount" | "trashCapacityCount" | "activityLogCapacity" => {
            validate_integer(key, value, 0, 100_000)
        }
        "keepClipAgeDays" | "trashAgeDays" | "activityLogAgeDays" => {
            validate_integer(key, value, 0, 36_500)
        }
        "revisionHistoryLimit" => validate_integer(key, value, 0, 10_000),
        _ => Ok(()),
    }
}

pub fn update_settings(
    db: &DbState,
    values: HashMap<String, String>,
) -> Result<SettingsUpdateOutcome, String> {
    if values.is_empty() {
        return Ok(SettingsUpdateOutcome {
            changes: Vec::new(),
            changed_feature_keys: Vec::new(),
        });
    }
    for (key, value) in &values {
        validate_setting(key, value)?;
    }

    let mut changes = Vec::new();
    for (key, value) in &values {
        let previous_value = db.get_setting(key).map_err(|error| error.to_string())?;
        if previous_value.as_deref() != Some(value.as_str()) {
            changes.push(SettingChange {
                key: key.clone(),
                previous_value,
                value: value.clone(),
            });
        }
    }
    changes.sort_by(|left, right| left.key.cmp(&right.key));
    db.save_settings(&values)
        .map_err(|error| error.to_string())?;

    let mut activities = changes
        .iter()
        .filter_map(|change| {
            crate::settings_activity::describe_setting_change(
                &change.key,
                change.previous_value.as_deref(),
                &change.value,
            )
        })
        .collect::<Vec<_>>();
    activities.sort_by(|left, right| left.description.cmp(&right.description));
    if activities.len() == 1 {
        let activity = &activities[0];
        let _ = db.log_activity(activity.event_type, &activity.description);
    } else if !activities.is_empty() {
        let description = activities
            .iter()
            .map(|activity| activity.description.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        let _ = db.log_activity("settings_changed", &description);
    }

    let changed_feature_keys = changes
        .iter()
        .filter(|change| Feature::from_setting_key(&change.key).is_some())
        .map(|change| change.key.clone())
        .collect();
    Ok(SettingsUpdateOutcome {
        changes,
        changed_feature_keys,
    })
}

pub fn update_setting(
    db: &DbState,
    key: String,
    value: String,
) -> Result<SettingsUpdateOutcome, String> {
    update_settings(db, HashMap::from([(key, value)]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_db() -> (DbState, std::path::PathBuf) {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("pasted_settings_service_{nonce}.db"));
        (DbState::new(path.clone()).unwrap(), path)
    }

    #[test]
    fn invalid_known_values_never_reach_persistence() {
        let (db, path) = test_db();
        assert!(update_setting(&db, "enableBins".into(), "sometimes".into()).is_err());
        assert_eq!(db.get_setting("enableBins").unwrap(), None);
        assert!(update_setting(&db, "filePreviewMaxMb".into(), "65".into()).is_err());
        assert_eq!(db.get_setting("filePreviewMaxMb").unwrap(), None);
        drop(db);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn updates_are_atomic_sorted_and_report_feature_changes() {
        let (db, path) = test_db();
        let outcome = update_settings(
            &db,
            HashMap::from([
                ("themeMode".into(), "warm".into()),
                ("enableBins".into(), "false".into()),
            ]),
        )
        .unwrap();
        assert_eq!(
            outcome
                .changes
                .iter()
                .map(|change| change.key.as_str())
                .collect::<Vec<_>>(),
            vec!["enableBins", "themeMode"]
        );
        assert_eq!(outcome.changed_feature_keys, vec!["enableBins"]);
        assert_eq!(
            db.get_setting("themeMode").unwrap().as_deref(),
            Some("warm")
        );
        drop(db);
        let _ = std::fs::remove_file(path);
    }
}
