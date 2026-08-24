use std::collections::HashMap;

use serde::Serialize;

use crate::application_error::ApplicationError;
use crate::{db::DbState, features::Feature};

pub const MAX_SETTING_KEY_BYTES: usize = 128;
pub const MAX_SETTING_VALUE_BYTES: usize = 1024 * 1024;

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

pub fn validate_setting(key: &str, value: &str) -> Result<(), ApplicationError> {
    if key.trim().is_empty() || key.len() > MAX_SETTING_KEY_BYTES {
        return Err(ApplicationError::invalid(format!(
            "Setting keys must contain 1–{MAX_SETTING_KEY_BYTES} bytes"
        )));
    }
    if value.len() > MAX_SETTING_VALUE_BYTES {
        return Err(ApplicationError::invalid(format!(
            "Setting values cannot exceed {MAX_SETTING_VALUE_BYTES} bytes"
        )));
    }
    crate::settings_contract::validate_direct_value(key, value).map_err(ApplicationError::invalid)
}

pub fn update_settings(
    db: &DbState,
    values: HashMap<String, String>,
) -> Result<SettingsUpdateOutcome, ApplicationError> {
    let outcome = preview_update(db, &values)?;
    db.save_settings(&values)
        .map_err(ApplicationError::persistence)?;

    let mut activities = outcome
        .changes
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

    Ok(outcome)
}

pub fn update_setting(
    db: &DbState,
    key: String,
    value: String,
) -> Result<SettingsUpdateOutcome, ApplicationError> {
    update_settings(db, HashMap::from([(key, value)]))
}

pub fn reset_page(db: &DbState, page: &str) -> Result<SettingsUpdateOutcome, ApplicationError> {
    let values =
        crate::settings_contract::reset_defaults(page).map_err(ApplicationError::invalid)?;
    update_settings(db, values)
}

pub fn preview_page_reset(
    db: &DbState,
    page: &str,
) -> Result<SettingsUpdateOutcome, ApplicationError> {
    let values =
        crate::settings_contract::reset_defaults(page).map_err(ApplicationError::invalid)?;
    preview_update(db, &values)
}

fn preview_update(
    db: &DbState,
    values: &HashMap<String, String>,
) -> Result<SettingsUpdateOutcome, ApplicationError> {
    for (key, value) in values {
        validate_setting(key, value)?;
    }
    preview_values(db, values)
}

pub fn preview_dedicated_page_reset(
    db: &DbState,
    page: &str,
) -> Result<SettingsUpdateOutcome, ApplicationError> {
    let values = crate::settings_contract::dedicated_reset_defaults(page);
    preview_values(db, &values)
}

fn preview_values(
    db: &DbState,
    values: &HashMap<String, String>,
) -> Result<SettingsUpdateOutcome, ApplicationError> {
    let mut changes = values
        .iter()
        .map(|(key, value)| {
            let previous_value = db.get_setting(key).map_err(ApplicationError::persistence)?;
            Ok((key, value, previous_value))
        })
        .collect::<Result<Vec<_>, ApplicationError>>()?
        .into_iter()
        .filter_map(|(key, value, previous_value)| {
            let previous_effective = previous_value
                .clone()
                .or_else(|| crate::settings_contract::default_value(key));
            (previous_effective.as_deref() != Some(value.as_str())).then(|| SettingChange {
                key: key.clone(),
                previous_value,
                value: value.clone(),
            })
        })
        .collect::<Vec<_>>();
    changes.sort_by(|left, right| left.key.cmp(&right.key));
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
        assert!(update_setting(
            &db,
            "excludePrivateBrowserWindows".into(),
            "sometimes".into()
        )
        .is_err());
        assert!(update_setting(
            &db,
            "privateBrowserUnavailablePolicy".into(),
            "guess".into()
        )
        .is_err());
        drop(db);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn private_browser_policy_accepts_only_the_shared_gui_and_cli_contract() {
        let (db, path) = test_db();
        update_setting(&db, "excludePrivateBrowserWindows".into(), "true".into()).unwrap();
        update_setting(
            &db,
            "privateBrowserUnavailablePolicy".into(),
            "capture".into(),
        )
        .unwrap();
        assert_eq!(
            db.get_setting("privateBrowserUnavailablePolicy")
                .unwrap()
                .as_deref(),
            Some("capture")
        );
        update_setting(
            &db,
            "privateBrowserUnavailablePolicy".into(),
            "exclude_browser".into(),
        )
        .unwrap();
        assert_eq!(
            db.get_setting("privateBrowserUnavailablePolicy")
                .unwrap()
                .as_deref(),
            Some("exclude_browser")
        );
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

    #[test]
    fn page_resets_share_scoped_defaults_without_touching_other_pages() {
        let (db, path) = test_db();
        assert!(preview_page_reset(&db, "general")
            .unwrap()
            .changes
            .is_empty());
        update_settings(
            &db,
            HashMap::from([
                ("themeMode".into(), "warm".into()),
                ("captureFeedback".into(), "false".into()),
                ("hudHotkey".into(), "Ctrl+Shift+9".into()),
            ]),
        )
        .unwrap();
        let preview = preview_page_reset(&db, "general").unwrap();
        assert!(preview
            .changes
            .iter()
            .any(|change| change.key == "themeMode"));
        assert_eq!(
            db.get_setting("themeMode").unwrap().as_deref(),
            Some("warm")
        );
        let reset = reset_page(&db, "general").unwrap();
        assert_eq!(reset, preview);
        assert!(reset.changes.iter().any(|change| change.key == "themeMode"));
        assert_eq!(
            db.get_setting("themeMode").unwrap().as_deref(),
            Some("system")
        );
        assert_eq!(
            db.get_setting("captureFeedback").unwrap().as_deref(),
            Some("false")
        );
        assert_eq!(
            db.get_setting("hudHotkey").unwrap().as_deref(),
            Some("Ctrl+Shift+9")
        );
        assert!(reset_page(&db, "unknown").is_err());
        drop(db);
        let _ = std::fs::remove_file(path);
    }
}
