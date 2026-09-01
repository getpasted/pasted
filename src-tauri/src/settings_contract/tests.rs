use std::collections::HashSet;

use super::*;

#[test]
fn contract_is_unique_complete_and_self_validating() {
    assert_eq!(version(), 1);
    assert_eq!(CONTRACT.factory_reset, "delete_all");
    let pages = CONTRACT
        .pages
        .iter()
        .map(|page| page.id.as_str())
        .collect::<HashSet<_>>();
    assert_eq!(pages.len(), CONTRACT.pages.len());
    let keys = CONTRACT
        .settings
        .iter()
        .map(|setting| setting.key.as_str())
        .collect::<HashSet<_>>();
    assert_eq!(keys.len(), CONTRACT.settings.len());
    for setting in &CONTRACT.settings {
        assert!(
            pages.contains(setting.owner.as_str()),
            "unknown owner for {}",
            setting.key
        );
        if let Some(default) = &setting.default {
            validate(setting, &persisted_value(default)).unwrap();
        }
        if setting.reset == ResetBehavior::Default {
            assert!(
                setting.default.is_some(),
                "missing reset default for {}",
                setting.key
            );
        }
    }
    assert_eq!(
        default_u64("maxClipSizeMb").unwrap() as usize * 1024 * 1024,
        crate::resource_limits::DEFAULT_CLIP_CAPTURE_BYTES
    );
    assert_eq!(
        default_u64("appLockIdleMinutes"),
        Some(u64::from(crate::app_lock::DEFAULT_IDLE_MINUTES))
    );
    assert_eq!(default_u64("windowTransparency"), Some(40));
    assert_eq!(default_u64("windowBlur"), Some(4));
    assert!(validate_direct_value("windowTransparency", "0").is_ok());
    assert!(validate_direct_value("windowTransparency", "100").is_ok());
    assert!(validate_direct_value("windowTransparency", "-1").is_err());
    assert!(validate_direct_value("windowTransparency", "101").is_err());
    assert!(validate_direct_value("windowBlur", "0").is_ok());
    assert!(validate_direct_value("windowBlur", "30").is_ok());
    assert!(validate_direct_value("windowBlur", "31").is_err());
}

#[test]
fn page_resets_are_repeatable_and_scoped_by_ownership() {
    let first = reset_defaults("general").unwrap();
    let second = reset_defaults("general").unwrap();
    assert_eq!(first, second);
    assert_eq!(first.get("themeMode").map(String::as_str), Some("system"));
    assert!(!first.contains_key("captureFeedback"));
    assert!(reset_defaults("security").is_err());
    assert!(reset_defaults("unknown").is_err());
}

#[test]
fn factory_reset_removes_every_registered_setting() {
    let path = std::env::temp_dir().join(format!(
        "pasted_settings_contract_factory_{}.db",
        std::process::id()
    ));
    let db = crate::db::DbState::new(path.clone()).unwrap();
    let values = CONTRACT
        .settings
        .iter()
        .map(|setting| (setting.key.clone(), "mutated".to_string()))
        .collect();
    db.save_settings(&values).unwrap();
    db.factory_reset().unwrap();
    assert!(db.get_all_settings().unwrap().is_empty());
    drop(db);
    let _ = std::fs::remove_file(path);
}
