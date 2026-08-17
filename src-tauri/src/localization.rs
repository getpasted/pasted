use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::db::DbState;

pub const LANGUAGE_SETTING_KEY: &str = "language";
pub const SYSTEM_LANGUAGE: &str = "system";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocaleManifest {
    default_locale: String,
    locales: Vec<LocaleDefinition>,
}

#[derive(Debug, Deserialize)]
struct LocaleDefinition {
    code: String,
}

struct Catalogs {
    manifest: LocaleManifest,
    messages: HashMap<&'static str, HashMap<String, serde_json::Value>>,
}

fn catalogs() -> &'static Catalogs {
    static CATALOGS: OnceLock<Catalogs> = OnceLock::new();
    CATALOGS.get_or_init(|| {
        let manifest = serde_json::from_str(include_str!("../../src/locales/manifest.json"))
            .expect("the bundled locale manifest must be valid");
        let english = serde_json::from_str(include_str!("../../src/locales/en.json"))
            .expect("the bundled English catalog must be valid");
        let ar = serde_json::from_str(include_str!("../../src/locales/ar.json"))
            .expect("the bundled Arabic catalog must be valid");
        let de_de = serde_json::from_str(include_str!("../../src/locales/de-DE.json"))
            .expect("the bundled German catalog must be valid");
        let fr_fr = serde_json::from_str(include_str!("../../src/locales/fr-FR.json"))
            .expect("the bundled French catalog must be valid");
        let he = serde_json::from_str(include_str!("../../src/locales/he.json"))
            .expect("the bundled Hebrew catalog must be valid");
        let ja_jp = serde_json::from_str(include_str!("../../src/locales/ja-JP.json"))
            .expect("the bundled Japanese catalog must be valid");
        Catalogs {
            manifest,
            messages: HashMap::from([
                ("ar", ar),
                ("en", english),
                ("de-DE", de_de),
                ("fr-FR", fr_fr),
                ("he", he),
                ("ja-JP", ja_jp),
            ]),
        }
    })
}

pub fn is_supported_locale(value: &str) -> bool {
    catalogs()
        .manifest
        .locales
        .iter()
        .any(|locale| locale.code == value)
}

pub fn validate_configured_language(value: &str) -> Result<(), String> {
    if value == SYSTEM_LANGUAGE || is_supported_locale(value) {
        Ok(())
    } else {
        Err(format!(
            "Unsupported language '{value}'. Use 'system' or a supported locale code."
        ))
    }
}

pub fn configured_language(db: &DbState) -> String {
    db.get_setting(LANGUAGE_SETTING_KEY)
        .ok()
        .flatten()
        .filter(|value| validate_configured_language(value).is_ok())
        .unwrap_or_else(|| SYSTEM_LANGUAGE.to_string())
}

pub fn effective_locale(configured: &str) -> &str {
    if configured != SYSTEM_LANGUAGE && is_supported_locale(configured) {
        configured
    } else {
        // The GUI resolves the live operating-system locale. Native surfaces use
        // the default until a specific bundled locale is selected.
        &catalogs().manifest.default_locale
    }
}

pub fn text_for_locale(locale: &str, key: &str) -> String {
    let catalogs = catalogs();
    let default_locale = catalogs.manifest.default_locale.as_str();
    catalogs
        .messages
        .get(locale)
        .and_then(|catalog| catalog.get(key))
        .or_else(|| {
            catalogs
                .messages
                .get(default_locale)
                .and_then(|catalog| catalog.get(key))
        })
        .and_then(serde_json::Value::as_str)
        .unwrap_or(key)
        .to_string()
}

pub fn text(db: &DbState, key: &str) -> String {
    let configured = configured_language(db);
    text_for_locale(effective_locale(&configured), key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_system_and_manifest_locales() {
        assert!(validate_configured_language("system").is_ok());
        assert!(validate_configured_language("ar").is_ok());
        assert!(validate_configured_language("en").is_ok());
        assert!(validate_configured_language("de-DE").is_ok());
        assert!(validate_configured_language("fr-FR").is_ok());
        assert!(validate_configured_language("he").is_ok());
        assert!(validate_configured_language("ja-JP").is_ok());
        assert!(validate_configured_language("not-a-locale").is_err());
    }

    #[test]
    fn catalog_uses_keys_only_as_a_last_resort() {
        assert_eq!(text_for_locale("en", "native.file.title"), "File");
        assert_eq!(text_for_locale("ar", "native.file.title"), "ملف");
        assert_eq!(text_for_locale("de-DE", "native.file.title"), "Datei");
        assert_eq!(text_for_locale("fr-FR", "native.file.title"), "Fichier");
        assert_eq!(text_for_locale("he", "native.file.title"), "קובץ");
        assert_eq!(text_for_locale("ja-JP", "native.file.title"), "ファイル");
        assert_eq!(text_for_locale("missing", "native.file.title"), "File");
        assert_eq!(text_for_locale("en", "missing.key"), "missing.key");
    }
}
