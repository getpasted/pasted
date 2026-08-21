use crate::db::DbState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Feature {
    Insights,
    Bins,
    ClipTypes,
    FileFormats,
    ContentClassification,
    Concealment,
    Naming,
    Notes,
    Notifications,
    AppLock,
    Ocr,
    Transcriptions,
    Pinning,
    Protection,
    Queue,
    Revisions,
    Hud,
    Hotkeys,
    Trash,
    Transformations,
    ActivityLog,
    ContentTypes,
    Sources,
    Search,
    Cli,
    Help,
}

impl Feature {
    pub const ALL: [Feature; 26] = [
        Feature::Insights,
        Feature::Bins,
        Feature::ClipTypes,
        Feature::FileFormats,
        Feature::ContentClassification,
        Feature::Concealment,
        Feature::Naming,
        Feature::Notes,
        Feature::Notifications,
        Feature::AppLock,
        Feature::Ocr,
        Feature::Transcriptions,
        Feature::Pinning,
        Feature::Protection,
        Feature::Queue,
        Feature::Revisions,
        Feature::Hud,
        Feature::Hotkeys,
        Feature::Trash,
        Feature::Transformations,
        Feature::ActivityLog,
        Feature::ContentTypes,
        Feature::Sources,
        Feature::Search,
        Feature::Cli,
        Feature::Help,
    ];

    pub const fn setting_key(self) -> &'static str {
        match self {
            Feature::Insights => "enableAnalytics",
            Feature::Bins => "enableBins",
            Feature::ClipTypes => "enableClipTypes",
            Feature::FileFormats => "enableFileFormats",
            Feature::ContentClassification => "enableContentClassification",
            Feature::Concealment => "enableConcealment",
            Feature::Naming => "enableNaming",
            Feature::Notes => "enableNotes",
            Feature::Notifications => "enableNotifications",
            Feature::AppLock => "enableAppLock",
            Feature::Ocr => "enableOcr",
            Feature::Transcriptions => "enableTranscriptions",
            Feature::Pinning => "enablePinning",
            Feature::Protection => "enableProtection",
            Feature::Queue => "enableQueue",
            Feature::Revisions => "enableRevisions",
            Feature::Hud => "enableHud",
            Feature::Hotkeys => "enableHotkeys",
            Feature::Trash => "enableTrash",
            Feature::Transformations => "enableTransformations",
            Feature::ActivityLog => "enableActivityLog",
            Feature::ContentTypes => "enableTypes",
            Feature::Sources => "enableSources",
            Feature::Search => "enableSearch",
            Feature::Cli => "enableCli",
            Feature::Help => "enableHelp",
        }
    }

    pub fn from_setting_key(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|feature| feature.setting_key() == key)
    }

    pub const fn label(self) -> &'static str {
        match self {
            Feature::Insights => "Insights",
            Feature::Bins => "Bins",
            Feature::ClipTypes => "Clip Types",
            Feature::FileFormats => "File Formats",
            Feature::ContentClassification => "Content Classification",
            Feature::Concealment => "Concealment",
            Feature::Naming => "Naming",
            Feature::Notes => "Notes",
            Feature::Notifications => "Notifications",
            Feature::AppLock => "App Lock",
            Feature::Ocr => "OCR",
            Feature::Transcriptions => "Transcriptions",
            Feature::Pinning => "Pinning",
            Feature::Protection => "Protection",
            Feature::Queue => "Queue",
            Feature::Revisions => "Revision History",
            Feature::Hud => "HUD",
            Feature::Hotkeys => "Hotkeys",
            Feature::Trash => "Trash",
            Feature::Transformations => "Transformations",
            Feature::ActivityLog => "Activity",
            Feature::ContentTypes => "Content Types",
            Feature::Sources => "Sources",
            Feature::Search => "Clip Search",
            Feature::Cli => "CLI",
            Feature::Help => "Help",
        }
    }
}

pub fn setting_value_is_enabled(value: Option<&str>) -> bool {
    !matches!(value.map(str::trim), Some("false") | Some("0"))
}

pub fn is_enabled(db: &DbState, feature: Feature) -> bool {
    let value = db.get_setting(feature.setting_key()).ok().flatten();
    setting_value_is_enabled(value.as_deref())
}

pub fn require(db: &DbState, feature: Feature) -> Result<(), String> {
    if is_enabled(db, feature) {
        Ok(())
    } else {
        Err(format!(
            "{} is disabled in Settings → Functionality",
            feature.label()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn missing_and_legacy_values_remain_enabled() {
        assert!(setting_value_is_enabled(None));
        assert!(setting_value_is_enabled(Some("true")));
        assert!(setting_value_is_enabled(Some("unexpected")));
    }

    #[test]
    fn explicit_false_values_disable_a_feature() {
        assert!(!setting_value_is_enabled(Some("false")));
        assert!(!setting_value_is_enabled(Some(" false ")));
        assert!(!setting_value_is_enabled(Some("0")));
    }

    #[test]
    fn frontend_and_native_setting_keys_are_stable() {
        assert_eq!(Feature::ALL.len(), 26);
        for feature in Feature::ALL {
            assert_eq!(
                Feature::from_setting_key(feature.setting_key()),
                Some(feature)
            );
        }
        assert_eq!(Feature::from_setting_key("unrelatedSetting"), None);
    }

    #[test]
    fn persisted_policy_defaults_on_and_applies_atomic_presets() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("pasted_feature_policy_{nonce}.db"));
        let db = DbState::new(path.clone()).unwrap();

        assert!(is_enabled(&db, Feature::Bins));
        assert!(is_enabled(&db, Feature::Hotkeys));
        let values = HashMap::from([
            (Feature::Bins.setting_key().to_string(), "false".to_string()),
            (
                Feature::Notes.setting_key().to_string(),
                "false".to_string(),
            ),
            (
                Feature::Hotkeys.setting_key().to_string(),
                "false".to_string(),
            ),
        ]);
        db.save_settings(&values).unwrap();
        assert!(!is_enabled(&db, Feature::Bins));
        assert!(!is_enabled(&db, Feature::Notes));
        assert!(!is_enabled(&db, Feature::Hotkeys));
        assert!(is_enabled(&db, Feature::Pinning));

        drop(db);
        let _ = std::fs::remove_file(path);
    }
}
