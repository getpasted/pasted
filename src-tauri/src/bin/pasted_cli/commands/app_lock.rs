use super::super::{
    app_lock_idle_label, cli_input_error, json_error, parse_app_lock_idle, parse_app_lock_toggle,
    print_app_lock_toggle, print_live_result, read_lock_passphrase, read_lock_passphrase_change,
    require_app_lock_passphrase, send_live_or_exit,
};
use pasted_lib::db::DbState;
use pasted_lib::features::Feature;
use rusqlite::{Connection, Result};
use std::path::PathBuf;

pub(crate) fn run(args: &[String], db_path: PathBuf, conn: Connection) -> Result<()> {
    let subcommand = args.get(2).map(String::as_str).unwrap_or("status");
    if subcommand != "reset"
        && !pasted_lib::features::is_enabled(&DbState::new(db_path.clone())?, Feature::AppLock)
    {
        eprintln!("App Lock is disabled in Settings → Functionality.");
        std::process::exit(1);
    }
    let json = args.iter().any(|argument| argument == "--json");
    match subcommand {
        "status" => {
            drop(conn);
            let db = DbState::new(db_path)?;
            let enabled = db
                .get_setting(pasted_lib::app_lock::ENABLED_SETTING)?
                .as_deref()
                == Some("true");
            let lock_state = pasted_lib::app_lock::AppLockState::from_db(&db);
            let lock_status = pasted_lib::app_lock::status(&db, &lock_state);
            let idle_minutes = db
                .get_setting(pasted_lib::app_lock::IDLE_MINUTES_SETTING)?
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(5);
            let lock_on_sleep = db
                .get_setting(pasted_lib::app_lock::LOCK_ON_SLEEP_SETTING)?
                .as_deref()
                != Some("false");
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::json!({ "enabled": enabled, "systemAuthEnabled": lock_status.system_auth_enabled, "systemAuthAvailable": lock_status.system_auth_available, "systemAuthLabel": lock_status.system_auth_label, "appleWatchEnabled": lock_status.apple_watch_enabled, "appleWatchAvailable": lock_status.apple_watch_available, "idleMinutes": idle_minutes, "lockOnSleep": lock_on_sleep, "lockOnRestart": lock_status.lock_on_restart, "captureWhileLocked": lock_status.capture_while_locked })
                );
            } else {
                println!(
                "App lock: {}\n{}: {} ({})\nApple Watch: {} ({})\nLock after restart: {}\nLock after sleep: {}\nAuto-lock: {}\nCapture while locked: {}",
                if enabled { "enabled" } else { "disabled" },
                lock_status.system_auth_label,
                if lock_status.system_auth_enabled {
                    "enabled"
                } else {
                    "disabled"
                },
                if lock_status.system_auth_available { "available" } else { "unavailable" },
                if lock_status.apple_watch_enabled { "enabled" } else { "disabled" },
                if lock_status.apple_watch_available { "available" } else { "unavailable" },
                if lock_status.lock_on_restart {
                    "enabled"
                } else {
                    "disabled"
                },
                if lock_on_sleep { "enabled" } else { "disabled" },
                if idle_minutes == 0 {
                    "Never".to_string()
                } else if idle_minutes == 60 {
                    "1 hour".to_string()
                } else if idle_minutes == 480 {
                    "8 hours".to_string()
                } else {
                    format!("{idle_minutes} minutes")
                },
                if lock_status.capture_while_locked { "enabled" } else { "disabled" }
            );
            }
        }
        "enable" => {
            drop(conn);
            let db = DbState::new(db_path)?;
            if db
                .get_setting(pasted_lib::app_lock::ENABLED_SETTING)?
                .as_deref()
                == Some("true")
            {
                eprintln!(
                    "App lock is already enabled. Change its passphrase in Settings → Security."
                );
                std::process::exit(2);
            }
            let passphrase = read_lock_passphrase(args, "New app-lock passphrase: ")?;
            pasted_lib::app_lock::configure(&db, &passphrase).map_err(cli_input_error)?;
            let _ = db.log_activity("app_lock_enabled", "Enabled app lock");
            if json {
                println!("{}", serde_json::json!({ "enabled": true }));
            } else {
                println!("Enabled app lock.");
            }
        }
        "change-passphrase" => {
            drop(conn);
            let db = DbState::new(db_path)?;
            if db
                .get_setting(pasted_lib::app_lock::ENABLED_SETTING)?
                .as_deref()
                != Some("true")
            {
                eprintln!("App lock is not enabled.");
                std::process::exit(2);
            }
            let (current, new) = read_lock_passphrase_change(args)?;
            pasted_lib::app_lock::change_passphrase(&db, &current, &new)
                .map_err(cli_input_error)?;
            let _ = db.log_activity("app_lock_passphrase_changed", "Changed app lock passphrase");
            if json {
                println!("{}", serde_json::json!({ "changed": true }));
            } else {
                println!("Changed app-lock passphrase.");
            }
        }
        "disable" => {
            drop(conn);
            let db = DbState::new(db_path)?;
            let passphrase = read_lock_passphrase(args, "App-lock passphrase: ")?;
            pasted_lib::app_lock::disable(&db, &passphrase).map_err(cli_input_error)?;
            let _ = db.log_activity("app_lock_disabled", "Disabled app lock");
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "enabled": false, "credentialsCleared": true })
                );
            } else {
                println!("Disabled app lock and cleared its unlock credentials.");
            }
        }
        "lock" => {
            let result = send_live_or_exit(pasted_lib::live_app::LiveAppAction::AppLockLock);
            print_live_result(&result, json)?;
        }
        "unlock" => {
            let passphrase = read_lock_passphrase(args, "App-lock passphrase: ")?;
            let result = send_live_or_exit(pasted_lib::live_app::LiveAppAction::AppLockUnlock {
                passphrase,
            });
            print_live_result(&result, json)?;
        }
        "idle" => {
            let minutes = parse_app_lock_idle(args.get(3).map(String::as_str));
            drop(conn);
            let db = DbState::new(db_path)?;
            require_app_lock_passphrase(&db, args)?;
            pasted_lib::app_lock::set_idle_minutes(&db, minutes).map_err(cli_input_error)?;
            if json {
                println!("{}", serde_json::json!({ "idleMinutes": minutes }));
            } else {
                println!("Auto-lock: {}.", app_lock_idle_label(minutes));
            }
        }
        "lock-on-sleep" => {
            let enabled = parse_app_lock_toggle(
                args.get(3).map(String::as_str),
                "Usage: pasted app-lock lock-on-sleep <on|off> [--stdin] [--json]",
            );
            drop(conn);
            let db = DbState::new(db_path)?;
            require_app_lock_passphrase(&db, args)?;
            pasted_lib::app_lock::set_bool_policy(
                &db,
                pasted_lib::app_lock::LOCK_ON_SLEEP_SETTING,
                enabled,
            )
            .map_err(cli_input_error)?;
            print_app_lock_toggle("lockOnSleep", "Lock after sleep", enabled, json)?;
        }
        "lock-on-restart" => {
            let enabled = parse_app_lock_toggle(
                args.get(3).map(String::as_str),
                "Usage: pasted app-lock lock-on-restart <on|off> [--stdin] [--json]",
            );
            drop(conn);
            let db = DbState::new(db_path)?;
            require_app_lock_passphrase(&db, args)?;
            pasted_lib::app_lock::set_bool_policy(
                &db,
                pasted_lib::app_lock::LOCK_ON_RESTART_SETTING,
                enabled,
            )
            .map_err(cli_input_error)?;
            print_app_lock_toggle("lockOnRestart", "Lock after restart", enabled, json)?;
        }
        "capture-while-locked" => {
            let enabled = parse_app_lock_toggle(
                args.get(3).map(String::as_str),
                "Usage: pasted app-lock capture-while-locked <on|off> [--stdin] [--json]",
            );
            drop(conn);
            let db = DbState::new(db_path)?;
            require_app_lock_passphrase(&db, args)?;
            pasted_lib::app_lock::set_bool_policy(
                &db,
                pasted_lib::app_lock::CAPTURE_WHILE_LOCKED_SETTING,
                enabled,
            )
            .map_err(cli_input_error)?;
            print_app_lock_toggle("captureWhileLocked", "Capture while locked", enabled, json)?;
        }
        "system-auth" | "apple-watch" => {
            let enabled = parse_app_lock_toggle(
                args.get(3).map(String::as_str),
                if subcommand == "system-auth" {
                    "Usage: pasted app-lock system-auth <on|off> [--stdin] [--json]"
                } else {
                    "Usage: pasted app-lock apple-watch <on|off> [--stdin] [--json]"
                },
            );
            drop(conn);
            let db = DbState::new(db_path)?;
            require_app_lock_passphrase(&db, args)?;
            let (method, setting, json_key, label) = if subcommand == "system-auth" {
                (
                    pasted_lib::app_lock::SystemAuthMethod::Primary,
                    pasted_lib::app_lock::SYSTEM_AUTH_SETTING,
                    "systemAuthEnabled",
                    pasted_lib::app_lock::platform_auth_label(),
                )
            } else {
                (
                    pasted_lib::app_lock::SystemAuthMethod::AppleWatch,
                    pasted_lib::app_lock::APPLE_WATCH_SETTING,
                    "appleWatchEnabled",
                    "Apple Watch",
                )
            };
            if enabled && !pasted_lib::app_lock::platform_auth_available(method) {
                eprintln!("{label} is not available on this device or desktop session.");
                std::process::exit(1);
            }
            pasted_lib::app_lock::set_bool_policy(&db, setting, enabled)
                .map_err(cli_input_error)?;
            print_app_lock_toggle(json_key, label, enabled, json)?;
        }
        "reset" => {
            if !args.iter().any(|argument| argument == "--yes") {
                eprintln!("Resetting app lock removes its passphrase and system-authentication preferences. Re-run with --yes to continue.");
                std::process::exit(2);
            }
            drop(conn);
            let result = send_live_or_exit(pasted_lib::live_app::LiveAppAction::AppLockReset {
                confirmed: true,
                database_path: db_path,
            });
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).map_err(json_error)?
                );
            } else {
                println!("Reset app lock and cleared its unlock credentials.");
            }
        }
        _ => {
            eprintln!("Usage: pasted app-lock status|enable|change-passphrase|disable|lock|unlock|idle|lock-on-sleep|lock-on-restart|capture-while-locked|system-auth|apple-watch|reset [--stdin] [--json]");
            std::process::exit(2);
        }
    }
    Ok(())
}
