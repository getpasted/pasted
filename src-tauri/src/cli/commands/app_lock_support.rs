use super::super::*;
use super::common::{cli_input_error, read_stdin_bounded};

pub(crate) fn read_lock_passphrase(args: &[String], prompt: &str) -> Result<String> {
    let passphrase = if args.iter().any(|argument| argument == "--stdin") {
        read_stdin_bounded(4096)?
            .trim_end_matches(['\r', '\n'])
            .to_string()
    } else {
        rpassword::prompt_password(prompt)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?
    };
    if passphrase.is_empty() {
        return Err(cli_input_error("A passphrase is required.".to_string()));
    }
    Ok(passphrase)
}

pub(crate) fn read_lock_passphrase_change(args: &[String]) -> Result<(String, String)> {
    if args.iter().any(|argument| argument == "--stdin") {
        let input = read_stdin_bounded(8192)?;
        let mut lines = input.lines();
        let current = lines.next().unwrap_or_default().trim_end_matches('\r');
        let new = lines.next().unwrap_or_default().trim_end_matches('\r');
        if current.is_empty() || new.is_empty() || lines.any(|line| !line.trim().is_empty()) {
            return Err(cli_input_error(
                "Pass current and new passphrases as exactly two stdin lines.".to_string(),
            ));
        }
        return Ok((current.to_string(), new.to_string()));
    }

    let current = rpassword::prompt_password("Current app-lock passphrase: ")
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    let new = rpassword::prompt_password("New app-lock passphrase: ")
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    let confirmation = rpassword::prompt_password("Confirm new app-lock passphrase: ")
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    if new != confirmation {
        return Err(cli_input_error(
            "The new passphrases do not match.".to_string(),
        ));
    }
    if current.is_empty() || new.is_empty() {
        return Err(cli_input_error("A passphrase is required.".to_string()));
    }
    Ok((current, new))
}

pub(crate) fn require_app_lock_passphrase(db: &DbState, args: &[String]) -> Result<()> {
    if db
        .get_setting(pasted_lib::app_lock::ENABLED_SETTING)?
        .as_deref()
        != Some("true")
    {
        return Ok(());
    }
    let passphrase = read_lock_passphrase(args, "App-lock passphrase: ")?;
    if !pasted_lib::app_lock::verify(db, &passphrase).map_err(cli_input_error)? {
        return Err(cli_input_error("The passphrase is incorrect.".to_string()));
    }
    Ok(())
}

pub(crate) fn parse_app_lock_toggle(value: Option<&str>, usage: &str) -> bool {
    match value {
        Some("on" | "enable" | "enabled" | "true") => true,
        Some("off" | "disable" | "disabled" | "false") => false,
        _ => {
            eprintln!("{usage}");
            std::process::exit(2);
        }
    }
}

pub(crate) fn parse_app_lock_idle(value: Option<&str>) -> u32 {
    match value {
        Some("never" | "0") => 0,
        Some("1m" | "1") => 1,
        Some("5m" | "5") => 5,
        Some("1h" | "60") => 60,
        Some("8h" | "480") => 480,
        _ => {
            eprintln!("Usage: pasted app-lock idle <never|1m|5m|1h|8h> [--stdin] [--json]");
            std::process::exit(2);
        }
    }
}

pub(crate) fn app_lock_idle_label(minutes: u32) -> &'static str {
    match minutes {
        0 => "Never",
        1 => "1 minute",
        5 => "5 minutes",
        60 => "1 hour",
        480 => "8 hours",
        _ => "Unknown",
    }
}

pub(crate) fn print_app_lock_toggle(
    json_key: &str,
    label: &str,
    enabled: bool,
    json: bool,
) -> Result<()> {
    if json {
        let mut result = serde_json::Map::new();
        result.insert(json_key.to_string(), serde_json::Value::Bool(enabled));
        println!("{}", serde_json::to_string(&result).map_err(json_error)?);
    } else {
        println!("{label}: {}.", if enabled { "enabled" } else { "disabled" });
    }
    Ok(())
}
