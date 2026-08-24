use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use parking_lot::Mutex;
use rand_core::OsRng;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::db::DbState;

mod platform_auth;
pub use platform_auth::{
    platform_auth_available, platform_auth_label, platform_authenticate, SystemAuthMethod,
};

pub const ENABLED_SETTING: &str = "appLockEnabled";
pub const VERIFIER_SETTING: &str = "appLockVerifier";
pub const LEGACY_BIOMETRIC_SETTING: &str = "appLockBiometricEnabled";
pub const SYSTEM_AUTH_SETTING: &str = "appLockSystemAuthEnabled";
pub const APPLE_WATCH_SETTING: &str = "appLockAppleWatchEnabled";
pub const IDLE_MINUTES_SETTING: &str = "appLockIdleMinutes";
pub const LOCK_ON_SLEEP_SETTING: &str = "appLockOnSleep";
pub const LOCK_ON_RESTART_SETTING: &str = "appLockOnRestart";
pub const CAPTURE_WHILE_LOCKED_SETTING: &str = "appLockCaptureWhileLocked";
pub const DEFAULT_IDLE_MINUTES: u32 = 5;

pub fn is_private_setting(key: &str) -> bool {
    crate::settings_contract::is_private(key)
}

pub fn is_managed_setting(key: &str) -> bool {
    crate::settings_contract::is_managed(key)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppLockStatus {
    pub enabled: bool,
    pub locked: bool,
    pub system_auth_enabled: bool,
    pub system_auth_available: bool,
    pub system_auth_label: String,
    pub apple_watch_enabled: bool,
    pub apple_watch_available: bool,
    pub idle_minutes: u32,
    pub lock_on_sleep: bool,
    pub lock_on_restart: bool,
    pub capture_while_locked: bool,
}

pub struct AppLockState {
    locked: AtomicBool,
    failures: Mutex<FailureState>,
}

struct FailureState {
    consecutive: u32,
    retry_after: Option<Instant>,
}

impl AppLockState {
    pub fn from_db(db: &DbState) -> Self {
        let enabled = crate::features::is_enabled(db, crate::features::Feature::AppLock)
            && setting_bool(db, ENABLED_SETTING);
        Self {
            locked: AtomicBool::new(enabled && lock_on_restart(db)),
            failures: Mutex::new(FailureState {
                consecutive: 0,
                retry_after: None,
            }),
        }
    }

    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::SeqCst)
    }

    pub fn lock(&self) {
        self.locked.store(true, Ordering::SeqCst);
    }

    pub fn unlock(&self) {
        self.locked.store(false, Ordering::SeqCst);
        let mut failures = self.failures.lock();
        failures.consecutive = 0;
        failures.retry_after = None;
    }

    pub fn check_retry(&self) -> Result<(), String> {
        let mut failures = self.failures.lock();
        if let Some(retry_after) = failures.retry_after {
            let remaining = retry_after.saturating_duration_since(Instant::now());
            if !remaining.is_zero() {
                return Err(format!(
                    "Try again in {} seconds.",
                    remaining.as_secs().max(1)
                ));
            }
            failures.retry_after = None;
        }
        Ok(())
    }

    pub fn record_failure(&self) {
        let mut failures = self.failures.lock();
        failures.consecutive = failures.consecutive.saturating_add(1);
        if failures.consecutive >= 5 {
            let exponent = (failures.consecutive - 5).min(5);
            failures.retry_after = Some(Instant::now() + Duration::from_secs(2_u64.pow(exponent)));
        }
    }
}

pub fn lock_enabled(db: &DbState, state: &AppLockState) -> Result<AppLockStatus, String> {
    crate::features::require(db, crate::features::Feature::AppLock)?;
    if db
        .get_setting(ENABLED_SETTING)
        .map_err(|error| error.to_string())?
        .as_deref()
        != Some("true")
    {
        return Err("App lock is not enabled.".to_string());
    }
    state.lock();
    Ok(status(db, state))
}

pub fn status(db: &DbState, state: &AppLockState) -> AppLockStatus {
    let feature_enabled = crate::features::is_enabled(db, crate::features::Feature::AppLock);
    AppLockStatus {
        enabled: feature_enabled && setting_bool(db, ENABLED_SETTING),
        locked: feature_enabled && state.is_locked(),
        system_auth_enabled: setting_bool_with_legacy(db, SYSTEM_AUTH_SETTING),
        system_auth_available: feature_enabled
            && platform_auth_available(SystemAuthMethod::Primary),
        system_auth_label: platform_auth_label().to_string(),
        apple_watch_enabled: setting_bool_with_legacy(db, APPLE_WATCH_SETTING),
        apple_watch_available: feature_enabled
            && platform_auth_available(SystemAuthMethod::AppleWatch),
        idle_minutes: db
            .get_setting(IDLE_MINUTES_SETTING)
            .ok()
            .flatten()
            .and_then(|value| value.parse().ok())
            .filter(|value| matches!(value, 0 | 1 | 5 | 60 | 480))
            .or_else(|| {
                crate::settings_contract::default_u64(IDLE_MINUTES_SETTING)
                    .map(|value| value as u32)
            })
            .unwrap_or(DEFAULT_IDLE_MINUTES),
        lock_on_sleep: setting_bool(db, LOCK_ON_SLEEP_SETTING),
        lock_on_restart: lock_on_restart(db),
        capture_while_locked: capture_while_locked(db),
    }
}

pub fn lock_on_restart(db: &DbState) -> bool {
    setting_bool(db, LOCK_ON_RESTART_SETTING)
}

pub fn capture_while_locked(db: &DbState) -> bool {
    setting_bool(db, CAPTURE_WHILE_LOCKED_SETTING)
}

pub fn capture_allowed(db: &DbState, state: &AppLockState) -> bool {
    !state.is_locked() || capture_while_locked(db)
}

pub fn configure(db: &DbState, passphrase: &str) -> Result<(), String> {
    validate_passphrase(passphrase)?;
    let salt = SaltString::generate(&mut OsRng);
    let verifier = Argon2::default()
        .hash_password(passphrase.as_bytes(), &salt)
        .map_err(|_| "Could not secure the app passphrase.".to_string())?
        .to_string();
    db.save_settings(&std::collections::HashMap::from([
        (ENABLED_SETTING.to_string(), "true".to_string()),
        (VERIFIER_SETTING.to_string(), verifier),
    ]))
    .map_err(|error| error.to_string())
}

pub fn change_passphrase(
    db: &DbState,
    current_passphrase: &str,
    new_passphrase: &str,
) -> Result<(), String> {
    if !verify(db, current_passphrase)? {
        return Err("The current passphrase is incorrect.".to_string());
    }
    configure(db, new_passphrase)
}

pub fn disable(db: &DbState, passphrase: &str) -> Result<(), String> {
    if !verify(db, passphrase)? {
        return Err("The passphrase is incorrect.".to_string());
    }
    clear_credentials(db)
}

pub fn reset(db: &DbState) -> Result<(), String> {
    clear_credentials(db)
}

fn clear_credentials(db: &DbState) -> Result<(), String> {
    db.save_and_delete_settings(
        &std::collections::HashMap::from([
            (ENABLED_SETTING.to_string(), "false".to_string()),
            (LEGACY_BIOMETRIC_SETTING.to_string(), "false".to_string()),
            (SYSTEM_AUTH_SETTING.to_string(), "false".to_string()),
            (APPLE_WATCH_SETTING.to_string(), "false".to_string()),
        ]),
        &[VERIFIER_SETTING],
    )
    .map_err(|error| error.to_string())
}

pub fn set_bool_policy(db: &DbState, setting: &str, enabled: bool) -> Result<(), String> {
    if !matches!(
        setting,
        SYSTEM_AUTH_SETTING
            | APPLE_WATCH_SETTING
            | LOCK_ON_SLEEP_SETTING
            | LOCK_ON_RESTART_SETTING
            | CAPTURE_WHILE_LOCKED_SETTING
    ) {
        return Err("Unknown app-lock policy.".to_string());
    }
    let next = if enabled { "true" } else { "false" };
    let previous = db.get_setting(setting).map_err(|error| error.to_string())?;
    db.save_setting(setting, next)
        .map_err(|error| error.to_string())?;
    if let Some(activity) =
        crate::settings_activity::describe_setting_change(setting, previous.as_deref(), next)
    {
        let _ = db.log_activity(activity.event_type, &activity.description);
    }
    Ok(())
}

pub fn set_idle_minutes(db: &DbState, minutes: u32) -> Result<(), String> {
    if !matches!(minutes, 0 | 1 | 5 | 60 | 480) {
        return Err("Choose Never, 1 minute, 5 minutes, 1 hour, or 8 hours.".to_string());
    }
    let next = minutes.to_string();
    let previous = db
        .get_setting(IDLE_MINUTES_SETTING)
        .map_err(|error| error.to_string())?;
    db.save_setting(IDLE_MINUTES_SETTING, &next)
        .map_err(|error| error.to_string())?;
    if let Some(activity) = crate::settings_activity::describe_setting_change(
        IDLE_MINUTES_SETTING,
        previous.as_deref(),
        &next,
    ) {
        let _ = db.log_activity(activity.event_type, &activity.description);
    }
    Ok(())
}

pub fn reset_policy(db: &DbState) -> Result<(), String> {
    db.save_settings(&crate::settings_contract::dedicated_reset_defaults(
        "security",
    ))
    .map_err(|error| error.to_string())
}

pub fn verify(db: &DbState, passphrase: &str) -> Result<bool, String> {
    let Some(encoded) = db
        .get_setting(VERIFIER_SETTING)
        .map_err(|error| error.to_string())?
    else {
        return Ok(false);
    };
    let parsed = PasswordHash::new(&encoded)
        .map_err(|_| "The saved app-lock credential is invalid.".to_string())?;
    Ok(Argon2::default()
        .verify_password(passphrase.as_bytes(), &parsed)
        .is_ok())
}

pub fn validate_passphrase(passphrase: &str) -> Result<(), String> {
    let length = passphrase.chars().count();
    if length < 1 {
        return Err("Enter a passphrase.".to_string());
    }
    if length > 1024 {
        return Err("The passphrase is too long.".to_string());
    }
    Ok(())
}

fn setting_bool(db: &DbState, key: &str) -> bool {
    db.get_setting(key)
        .ok()
        .flatten()
        .map(|value| value == "true")
        .or_else(|| crate::settings_contract::default_bool(key))
        .unwrap_or(false)
}

fn setting_bool_with_legacy(db: &DbState, key: &str) -> bool {
    match db.get_setting(key).ok().flatten() {
        Some(value) => value == "true",
        None => setting_bool(db, LEGACY_BIOMETRIC_SETTING),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbState;

    fn db() -> DbState {
        DbState::new(std::path::PathBuf::from(":memory:")).expect("in-memory database")
    }

    #[test]
    fn passphrase_is_salted_and_verified_without_storage_in_plaintext() {
        let db = db();
        configure(&db, "correct horse battery staple").unwrap();
        let encoded = db.get_setting(VERIFIER_SETTING).unwrap().unwrap();
        assert!(!encoded.contains("correct horse battery staple"));
        assert!(verify(&db, "correct horse battery staple").unwrap());
        assert!(!verify(&db, "wrong passphrase").unwrap());
    }

    #[test]
    fn disabling_and_resetting_remove_every_unlock_credential() {
        let db = db();
        configure(&db, "remembered").unwrap();
        db.save_setting(SYSTEM_AUTH_SETTING, "true").unwrap();
        db.save_setting(APPLE_WATCH_SETTING, "true").unwrap();
        disable(&db, "remembered").unwrap();
        assert_eq!(
            db.get_setting(ENABLED_SETTING).unwrap().as_deref(),
            Some("false")
        );
        assert_eq!(db.get_setting(VERIFIER_SETTING).unwrap(), None);
        assert!(!status(&db, &AppLockState::from_db(&db)).system_auth_enabled);
        assert!(!status(&db, &AppLockState::from_db(&db)).apple_watch_enabled);

        configure(&db, "forgotten").unwrap();
        reset(&db).unwrap();
        assert_eq!(db.get_setting(VERIFIER_SETTING).unwrap(), None);
        assert!(!status(&db, &AppLockState::from_db(&db)).enabled);
    }

    #[test]
    fn policy_mutations_are_bounded_and_logged() {
        let db = db();
        set_idle_minutes(&db, 60).unwrap();
        assert_eq!(
            db.get_setting(IDLE_MINUTES_SETTING).unwrap().as_deref(),
            Some("60")
        );
        assert!(set_idle_minutes(&db, 2).is_err());

        set_bool_policy(&db, LOCK_ON_SLEEP_SETTING, false).unwrap();
        assert!(!status(&db, &AppLockState::from_db(&db)).lock_on_sleep);
        assert!(set_bool_policy(&db, ENABLED_SETTING, false).is_err());
    }

    #[test]
    fn policy_reset_restores_defaults_without_removing_credentials() {
        let db = db();
        configure(&db, "remembered").unwrap();
        db.save_settings(&std::collections::HashMap::from([
            (SYSTEM_AUTH_SETTING.to_string(), "true".to_string()),
            (APPLE_WATCH_SETTING.to_string(), "true".to_string()),
            (IDLE_MINUTES_SETTING.to_string(), "60".to_string()),
            (LOCK_ON_SLEEP_SETTING.to_string(), "false".to_string()),
            (LOCK_ON_RESTART_SETTING.to_string(), "false".to_string()),
            (
                CAPTURE_WHILE_LOCKED_SETTING.to_string(),
                "false".to_string(),
            ),
        ]))
        .unwrap();

        reset_policy(&db).unwrap();
        let status = status(&db, &AppLockState::from_db(&db));
        assert!(status.enabled);
        assert!(!status.system_auth_enabled);
        assert!(!status.apple_watch_enabled);
        assert_eq!(status.idle_minutes, DEFAULT_IDLE_MINUTES);
        assert!(status.lock_on_sleep);
        assert!(status.lock_on_restart);
        assert!(status.capture_while_locked);
        assert!(verify(&db, "remembered").unwrap());
    }

    #[test]
    fn private_setting_is_narrowly_scoped() {
        assert!(is_private_setting(VERIFIER_SETTING));
        assert!(!is_private_setting(ENABLED_SETTING));
    }

    #[test]
    fn one_character_passphrases_are_allowed() {
        let db = db();
        configure(&db, "x").unwrap();
        assert!(verify(&db, "x").unwrap());
    }

    #[test]
    fn capture_while_locked_defaults_on_and_can_be_disabled() {
        let db = db();
        assert!(capture_while_locked(&db));
        db.save_setting(CAPTURE_WHILE_LOCKED_SETTING, "false")
            .unwrap();
        assert!(!capture_while_locked(&db));

        let state = AppLockState::from_db(&db);
        assert!(capture_allowed(&db, &state));
        state.lock();
        assert!(!capture_allowed(&db, &state));
        state.unlock();
        assert!(capture_allowed(&db, &state));
    }

    #[test]
    fn lock_on_restart_defaults_on_and_can_be_disabled() {
        let db = db();
        configure(&db, "remembered").unwrap();

        let default_state = AppLockState::from_db(&db);
        assert!(status(&db, &default_state).lock_on_restart);
        assert!(default_state.is_locked());

        db.save_setting(LOCK_ON_RESTART_SETTING, "false").unwrap();
        let unlocked_state = AppLockState::from_db(&db);
        assert!(!status(&db, &unlocked_state).lock_on_restart);
        assert!(!unlocked_state.is_locked());

        db.save_setting(LOCK_ON_RESTART_SETTING, "true").unwrap();
        assert!(AppLockState::from_db(&db).is_locked());
    }

    #[test]
    fn functionality_gate_disables_lock_without_deleting_configuration() {
        let db = db();
        configure(&db, "remembered").unwrap();
        let state = AppLockState::from_db(&db);
        assert!(status(&db, &state).locked);

        db.save_setting(crate::features::Feature::AppLock.setting_key(), "false")
            .unwrap();
        let disabled_state = AppLockState::from_db(&db);
        assert!(!status(&db, &disabled_state).enabled);
        assert!(!status(&db, &disabled_state).locked);
        assert!(verify(&db, "remembered").unwrap());
    }
}
