#[cfg(feature = "gui")]
use crate::db::DbState;
#[cfg(feature = "gui")]
use crate::paste_target::ActiveApplicationContext;

pub const ENABLED_SETTING: &str = "excludePrivateBrowserWindows";
pub const UNAVAILABLE_POLICY_SETTING: &str = "privateBrowserUnavailablePolicy";
pub const SUPPORTED_BROWSER_MODES: &[(&str, &[&str])] = &[
    ("Safari", &["Private Browsing"]),
    ("Chrome", &["Incognito"]),
    ("Edge", &["InPrivate"]),
    ("Firefox", &["Private Browsing"]),
    ("DuckDuckGo", &["Fire Window"]),
    ("Brave", &["Private Window", "Private Window with Tor"]),
];

#[cfg(feature = "gui")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BrowserWindowState {
    NotBrowser,
    Normal,
    Private,
    Unavailable,
}

#[cfg(feature = "gui")]
#[derive(Clone, Copy)]
struct BrowserAdapter {
    app_names: &'static [&'static str],
    private_title_markers: &'static [&'static str],
    normal_title_markers: &'static [&'static str],
    native_title_distinguishes_mode: bool,
}

#[cfg(feature = "gui")]
const BROWSERS: &[BrowserAdapter] = &[
    BrowserAdapter {
        app_names: &["Safari"],
        private_title_markers: &["Private Browsing", "Private Window", "— Private"],
        normal_title_markers: &[],
        native_title_distinguishes_mode: false,
    },
    BrowserAdapter {
        app_names: &["Google Chrome", "chrome", "Chromium", "chromium"],
        private_title_markers: &["(Incognito)"],
        normal_title_markers: &[" - Google Chrome", " - Chromium"],
        native_title_distinguishes_mode: false,
    },
    BrowserAdapter {
        app_names: &["Microsoft Edge", "msedge"],
        private_title_markers: &["InPrivate", "InPrivate - Microsoft Edge"],
        normal_title_markers: &[" - Microsoft Edge"],
        native_title_distinguishes_mode: false,
    },
    BrowserAdapter {
        app_names: &["Firefox", "firefox"],
        private_title_markers: &[
            "(Private Browsing)",
            "— Private Browsing",
            "Firefox Private Browsing",
        ],
        normal_title_markers: &["Mozilla Firefox"],
        native_title_distinguishes_mode: true,
    },
    BrowserAdapter {
        app_names: &["DuckDuckGo", "DuckDuckGo Browser"],
        private_title_markers: &["Fire Window"],
        normal_title_markers: &[],
        native_title_distinguishes_mode: false,
    },
    BrowserAdapter {
        app_names: &["Brave Browser", "brave", "brave-browser"],
        private_title_markers: &["Private Window", "Private with Tor", "(Private)", "(Tor)"],
        normal_title_markers: &[" - Brave"],
        native_title_distinguishes_mode: false,
    },
];

#[cfg(feature = "gui")]
fn normalized(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(feature = "gui")]
fn adapter_for_app(app_name: &str) -> Option<&'static BrowserAdapter> {
    let app_name = normalized(app_name);
    BROWSERS.iter().find(|adapter| {
        adapter
            .app_names
            .iter()
            .any(|candidate| normalized(candidate) == app_name)
    })
}

#[cfg(feature = "gui")]
pub(crate) fn detect(context: &ActiveApplicationContext) -> BrowserWindowState {
    let Some(adapter) = adapter_for_app(&context.name) else {
        return BrowserWindowState::NotBrowser;
    };
    let Some(title) = context.window_title.as_deref() else {
        return BrowserWindowState::Unavailable;
    };
    if adapter
        .private_title_markers
        .iter()
        .any(|marker| title == *marker || title.ends_with(marker))
    {
        BrowserWindowState::Private
    } else if adapter
        .normal_title_markers
        .iter()
        .any(|marker| title.ends_with(marker))
        && (context.window_title_is_accessible || adapter.native_title_distinguishes_mode)
    {
        BrowserWindowState::Normal
    } else {
        BrowserWindowState::Unavailable
    }
}

#[cfg(feature = "gui")]
pub(crate) fn is_enabled(db: &DbState) -> bool {
    db.get_setting(ENABLED_SETTING)
        .ok()
        .flatten()
        .map(|value| value == "true")
        .unwrap_or_else(|| crate::settings_contract::default_bool(ENABLED_SETTING).unwrap_or(false))
}

#[cfg(feature = "gui")]
pub(crate) fn should_exclude(db: &DbState, context: &ActiveApplicationContext) -> bool {
    if !is_enabled(db) {
        return false;
    }
    match detect(context) {
        BrowserWindowState::Private => true,
        BrowserWindowState::Unavailable => {
            let policy = db
                .get_setting(UNAVAILABLE_POLICY_SETTING)
                .ok()
                .flatten()
                .or_else(|| crate::settings_contract::default_value(UNAVAILABLE_POLICY_SETTING));
            policy.as_deref() == Some("exclude_browser")
        }
        BrowserWindowState::NotBrowser | BrowserWindowState::Normal => false,
    }
}

#[cfg(all(test, feature = "gui"))]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn context(name: &str, title: Option<&str>) -> ActiveApplicationContext {
        ActiveApplicationContext {
            name: name.into(),
            window_title: title.map(str::to_string),
            window_title_is_accessible: true,
        }
    }

    fn database() -> DbState {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        DbState::new(std::env::temp_dir().join(format!("pasted_private_browser_{nanos}.db")))
            .unwrap()
    }

    #[test]
    fn recognizes_each_shipped_private_mode_without_page_content() {
        for (app, title) in [
            ("Safari", "Private Browsing"),
            ("Google Chrome", "New Tab (Incognito)"),
            ("msedge", "New tab - InPrivate - Microsoft Edge"),
            ("Firefox", "Mozilla Firefox (Private Browsing)"),
            ("Firefox", "Mozilla Firefox — Private Browsing"),
            ("DuckDuckGo", "Fire Window"),
            ("Brave Browser", "New Tab (Private)"),
            ("brave-browser", "New Tab - Private with Tor"),
        ] {
            assert_eq!(
                detect(&context(app, Some(title))),
                BrowserWindowState::Private
            );
        }
    }

    #[test]
    fn distinguishes_normal_non_browser_and_unavailable_windows() {
        assert_eq!(
            detect(&context("Google Chrome", Some("New Tab - Google Chrome"))),
            BrowserWindowState::Normal
        );
        assert_eq!(
            detect(&context("Firefox", None)),
            BrowserWindowState::Unavailable
        );
        assert_eq!(
            detect(&context("Terminal", Some("Private Browsing"))),
            BrowserWindowState::NotBrowser
        );
        assert_eq!(
            detect(&context(
                "Google Chrome",
                Some("Incognito mode explained - Google Chrome")
            )),
            BrowserWindowState::Normal
        );
        let mut native_chrome = context("Google Chrome", Some("New Tab - Google Chrome"));
        native_chrome.window_title_is_accessible = false;
        assert_eq!(detect(&native_chrome), BrowserWindowState::Unavailable);
        let mut native_firefox = context("Firefox", Some("Mozilla Firefox"));
        native_firefox.window_title_is_accessible = false;
        assert_eq!(detect(&native_firefox), BrowserWindowState::Normal);
    }

    #[test]
    fn exclusion_is_opt_in_and_unavailable_defaults_to_capture() {
        let db = database();
        let private = context("Google Chrome", Some("New Tab (Incognito)"));
        let unavailable = context("Google Chrome", None);
        assert!(!should_exclude(&db, &private));

        db.save_setting(ENABLED_SETTING, "true").unwrap();
        assert!(should_exclude(&db, &private));
        assert!(!should_exclude(&db, &unavailable));

        db.save_setting(UNAVAILABLE_POLICY_SETTING, "exclude_browser")
            .unwrap();
        assert!(should_exclude(&db, &unavailable));
    }
}
