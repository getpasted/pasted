use super::json_error;
use pasted_lib::db::DbState;
use rusqlite::{Connection, Result};
use serde::Serialize;
use std::path::PathBuf;

use pasted_lib::private_browsing::{
    ENABLED_SETTING, SUPPORTED_BROWSER_MODES, UNAVAILABLE_POLICY_SETTING,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Status<'a> {
    enabled: bool,
    unavailable_policy: &'a str,
    supported_browsers: Vec<SupportedBrowser<'a>>,
}

#[derive(Serialize)]
struct SupportedBrowser<'a> {
    browser: &'a str,
    modes: &'a [&'a str],
}

fn status(db: &DbState) -> Status<'static> {
    let enabled = db.get_setting(ENABLED_SETTING).ok().flatten().as_deref() == Some("true");
    let unavailable_policy = match db
        .get_setting(UNAVAILABLE_POLICY_SETTING)
        .ok()
        .flatten()
        .as_deref()
    {
        Some("exclude_browser") => "exclude_browser",
        _ => "capture",
    };
    Status {
        enabled,
        unavailable_policy,
        supported_browsers: SUPPORTED_BROWSER_MODES
            .iter()
            .map(|(browser, modes)| SupportedBrowser { browser, modes })
            .collect(),
    }
}

pub(crate) fn run(args: &[String], db_path: PathBuf, conn: Connection) -> Result<()> {
    drop(conn);
    let db = DbState::new(db_path)?;
    let subcommand = args.get(2).map(String::as_str).unwrap_or("status");
    let json = args.iter().any(|argument| argument == "--json");
    match subcommand {
        "status" => {}
        "enable" | "disable" => {
            pasted_lib::settings_service::update_setting(
                &db,
                ENABLED_SETTING.into(),
                (subcommand == "enable").to_string(),
            )
            .map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))?;
        }
        "fallback" => {
            let policy = match args.get(3).map(String::as_str) {
                Some("capture") => "capture",
                Some("exclude-browser" | "exclude_browser") => "exclude_browser",
                _ => {
                    eprintln!("Usage: pasted private-browsing fallback <capture|exclude-browser> [--json]");
                    std::process::exit(2);
                }
            };
            pasted_lib::settings_service::update_setting(
                &db,
                UNAVAILABLE_POLICY_SETTING.into(),
                policy.into(),
            )
            .map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))?;
        }
        _ => {
            eprintln!("Usage: pasted private-browsing status|enable|disable|fallback [--json]");
            std::process::exit(2);
        }
    }
    let status = status(&db);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&status).map_err(json_error)?
        );
    } else {
        println!(
            "Private browser exclusion: {}",
            if status.enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
        println!(
            "When detection is unavailable: {}",
            status.unavailable_policy
        );
        for browser in status.supported_browsers {
            println!("{}\t{}", browser.browser, browser.modes.join(", "));
        }
    }
    Ok(())
}
