use crate::db::DbState;
use serde::Deserialize;

const MAX_APP_EXCLUSIONS: usize = 256;
const MAX_APP_NAME_BYTES: usize = 256;
const DEFAULT_APP_EXCLUSIONS: &[&str] = &[
    "1Password",
    "Passwords",
    "Keychain Access",
    "Bitwarden",
    "Dashlane",
    "Enpass",
    "KeePassXC",
];

const fn enabled() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppExclusionRule {
    pub name: String,
    #[serde(default = "enabled")]
    pub ignore_text: bool,
    #[serde(default = "enabled")]
    pub ignore_images: bool,
    #[serde(default = "enabled")]
    pub ignore_files: bool,
    #[serde(default)]
    pub ignore_shortcuts: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExcludedCaptureKind {
    Text,
    Image,
    Files,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StoredAppExclusion {
    Name(String),
    Rule(AppExclusionRule),
}

fn default_rules() -> Vec<AppExclusionRule> {
    DEFAULT_APP_EXCLUSIONS
        .iter()
        .map(|name| AppExclusionRule {
            name: (*name).into(),
            ignore_text: true,
            ignore_images: true,
            ignore_files: true,
            ignore_shortcuts: false,
        })
        .collect()
}

fn valid_name(name: &str) -> bool {
    !name.trim().is_empty() && name.len() <= MAX_APP_NAME_BYTES
}

pub(crate) fn parse_rules(value: Option<&str>) -> Vec<AppExclusionRule> {
    let Some(value) = value else {
        return default_rules();
    };
    let Ok(stored) = serde_json::from_str::<Vec<StoredAppExclusion>>(value) else {
        return default_rules();
    };
    stored
        .into_iter()
        .take(MAX_APP_EXCLUSIONS)
        .filter_map(|entry| {
            let rule = match entry {
                StoredAppExclusion::Name(name) => AppExclusionRule {
                    name,
                    ignore_text: true,
                    ignore_images: true,
                    ignore_files: true,
                    ignore_shortcuts: false,
                },
                StoredAppExclusion::Rule(rule) => rule,
            };
            valid_name(&rule.name).then_some(rule)
        })
        .collect()
}

pub(crate) fn load_rules(db: &DbState) -> Vec<AppExclusionRule> {
    let setting = db.get_setting("blacklistApps").ok().flatten();
    parse_rules(setting.as_deref())
}

fn name_matches(active_app: &str, excluded_app: &str) -> bool {
    let normalize = |value: &str| {
        value
            .to_lowercase()
            .chars()
            .map(|character| {
                if character.is_alphanumeric() {
                    character
                } else {
                    ' '
                }
            })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let active = normalize(active_app);
    let excluded = normalize(excluded_app);
    if active == excluded {
        return true;
    }
    active
        .strip_prefix(&excluded)
        .is_some_and(|suffix| suffix.starts_with(' '))
}

pub(crate) fn matching_rule<'a>(
    rules: &'a [AppExclusionRule],
    active_app: &str,
) -> Option<&'a AppExclusionRule> {
    rules
        .iter()
        .find(|rule| name_matches(active_app, &rule.name))
}

pub(crate) fn ignores_capture(rule: &AppExclusionRule, kind: ExcludedCaptureKind) -> bool {
    match kind {
        ExcludedCaptureKind::Text => rule.ignore_text,
        ExcludedCaptureKind::Image => rule.ignore_images,
        ExcludedCaptureKind::Files => rule.ignore_files,
    }
}

pub(crate) fn ignores_all_capture(rule: &AppExclusionRule) -> bool {
    rule.ignore_text && rule.ignore_images && rule.ignore_files
}

pub(crate) fn should_ignore_shortcuts(db: &DbState, active_app: Option<&str>) -> bool {
    let Some(active_app) = active_app else {
        return false;
    };
    matching_rule(&load_rules(db), active_app).is_some_and(|rule| rule.ignore_shortcuts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn missing_invalid_and_legacy_settings_migrate_safely() {
        assert!(parse_rules(None)
            .iter()
            .any(|rule| rule.name == "1Password"));
        assert!(parse_rules(Some("not-json"))
            .iter()
            .any(|rule| rule.name == "1Password"));

        let legacy = parse_rules(Some(r#"["Terminal"]"#));
        assert_eq!(legacy.len(), 1);
        assert!(legacy[0].ignore_text);
        assert!(legacy[0].ignore_images);
        assert!(legacy[0].ignore_files);
        assert!(!legacy[0].ignore_shortcuts);
    }

    #[test]
    fn explicit_empty_lists_remain_empty() {
        assert!(parse_rules(Some("[]")).is_empty());
    }

    #[test]
    fn older_object_rules_default_files_to_excluded() {
        let rules = parse_rules(Some(
            r#"[{"name":"Terminal","ignoreText":false,"ignoreImages":true,"ignoreShortcuts":true}]"#,
        ));
        assert_eq!(rules.len(), 1);
        assert!(!rules[0].ignore_text);
        assert!(rules[0].ignore_images);
        assert!(rules[0].ignore_files);
        assert!(rules[0].ignore_shortcuts);
    }

    #[test]
    fn content_rules_are_independent_and_full_pause_requires_all_three() {
        let rules = parse_rules(Some(
            r#"[{"name":"Example App","ignoreText":true,"ignoreImages":false,"ignoreFiles":true,"ignoreShortcuts":true}]"#,
        ));
        let rule = matching_rule(&rules, "Example App").unwrap();
        assert!(ignores_capture(rule, ExcludedCaptureKind::Text));
        assert!(!ignores_capture(rule, ExcludedCaptureKind::Image));
        assert!(ignores_capture(rule, ExcludedCaptureKind::Files));
        assert!(!ignores_all_capture(rule));
        assert!(rule.ignore_shortcuts);
    }

    #[test]
    fn app_matching_accepts_named_helpers_without_substring_false_positives() {
        let rules = parse_rules(Some(
            r#"[{"name":"Arc","ignoreText":true,"ignoreImages":true,"ignoreFiles":true}]"#,
        ));
        assert!(matching_rule(&rules, "Arc Helper").is_some());
        assert!(matching_rule(&rules, "Arc-Helper").is_some());
        assert!(matching_rule(&rules, "Search").is_none());
    }

    #[test]
    fn persisted_shortcut_rules_gate_only_the_matching_focused_app() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db =
            DbState::new(std::env::temp_dir().join(format!("pasted_app_exclusions_{nanos}.db")))
                .unwrap();
        db.save_setting(
            "blacklistApps",
            r#"[{"name":"Terminal","ignoreText":false,"ignoreImages":false,"ignoreFiles":false,"ignoreShortcuts":true}]"#,
        )
        .unwrap();

        assert!(should_ignore_shortcuts(&db, Some("Terminal")));
        assert!(!should_ignore_shortcuts(&db, Some("Finder")));
        assert!(!should_ignore_shortcuts(&db, None));
    }
}
