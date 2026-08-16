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

pub const ENABLED_SETTING: &str = "appLockEnabled";
pub const VERIFIER_SETTING: &str = "appLockVerifier";
pub const LEGACY_BIOMETRIC_SETTING: &str = "appLockBiometricEnabled";
pub const SYSTEM_AUTH_SETTING: &str = "appLockSystemAuthEnabled";
pub const APPLE_WATCH_SETTING: &str = "appLockAppleWatchEnabled";
pub const IDLE_MINUTES_SETTING: &str = "appLockIdleMinutes";
pub const LOCK_ON_SLEEP_SETTING: &str = "appLockOnSleep";
pub const CAPTURE_WHILE_LOCKED_SETTING: &str = "appLockCaptureWhileLocked";

pub fn is_private_setting(key: &str) -> bool {
    key == VERIFIER_SETTING
}

pub fn is_managed_setting(key: &str) -> bool {
    matches!(
        key,
        ENABLED_SETTING
            | VERIFIER_SETTING
            | LEGACY_BIOMETRIC_SETTING
            | SYSTEM_AUTH_SETTING
            | APPLE_WATCH_SETTING
            | IDLE_MINUTES_SETTING
            | LOCK_ON_SLEEP_SETTING
            | CAPTURE_WHILE_LOCKED_SETTING
    )
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
            locked: AtomicBool::new(enabled),
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
            .unwrap_or(5),
        lock_on_sleep: db
            .get_setting(LOCK_ON_SLEEP_SETTING)
            .ok()
            .flatten()
            .as_deref()
            != Some("false"),
        capture_while_locked: capture_while_locked(db),
    }
}

pub fn capture_while_locked(db: &DbState) -> bool {
    db.get_setting(CAPTURE_WHILE_LOCKED_SETTING)
        .ok()
        .flatten()
        .as_deref()
        != Some("false")
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
    db.get_setting(key).ok().flatten().as_deref() == Some("true")
}

fn setting_bool_with_legacy(db: &DbState, key: &str) -> bool {
    match db.get_setting(key).ok().flatten() {
        Some(value) => value == "true",
        None => setting_bool(db, LEGACY_BIOMETRIC_SETTING),
    }
}

#[derive(Clone, Copy)]
pub enum SystemAuthMethod {
    Primary,
    AppleWatch,
}

#[cfg(target_os = "macos")]
fn platform_auth_label() -> &'static str {
    "Touch ID"
}

#[cfg(target_os = "windows")]
fn platform_auth_label() -> &'static str {
    "Windows Hello"
}

#[cfg(target_os = "linux")]
fn platform_auth_label() -> &'static str {
    "System authentication"
}

#[cfg(target_os = "macos")]
pub fn platform_auth_available(method: SystemAuthMethod) -> bool {
    macos_auth(method, false).unwrap_or(false)
}

#[cfg(target_os = "windows")]
pub fn platform_auth_available(method: SystemAuthMethod) -> bool {
    if !matches!(method, SystemAuthMethod::Primary) {
        return false;
    }
    use windows::Security::Credentials::UI::{
        UserConsentVerifier, UserConsentVerifierAvailability,
    };
    UserConsentVerifier::CheckAvailabilityAsync()
        .and_then(|operation| operation.get())
        .is_ok_and(|availability| availability == UserConsentVerifierAvailability::Available)
}

#[cfg(target_os = "linux")]
pub fn platform_auth_available(_method: SystemAuthMethod) -> bool {
    false
}

#[cfg(target_os = "macos")]
pub fn platform_authenticate(
    method: SystemAuthMethod,
    _window_handle: Option<isize>,
) -> Result<bool, String> {
    macos_auth(method, true)
}

#[cfg(target_os = "windows")]
pub fn platform_authenticate(
    method: SystemAuthMethod,
    window_handle: Option<isize>,
) -> Result<bool, String> {
    if !matches!(method, SystemAuthMethod::Primary) {
        return Err("That authentication method is not available on Windows.".to_string());
    }
    use windows::core::{factory, HSTRING};
    use windows::Foundation::IAsyncOperation;
    use windows::Security::Credentials::UI::{UserConsentVerificationResult, UserConsentVerifier};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::WinRT::IUserConsentVerifierInterop;
    let window_handle =
        window_handle.ok_or_else(|| "The Pasted window is unavailable.".to_string())?;
    let interop = factory::<UserConsentVerifier, IUserConsentVerifierInterop>()
        .map_err(|error| format!("Windows Hello is unavailable: {error}"))?;
    let operation: IAsyncOperation<UserConsentVerificationResult> = unsafe {
        interop.RequestVerificationForWindowAsync(
            HWND(window_handle as *mut std::ffi::c_void),
            &HSTRING::from("Unlock Pasted"),
        )
    }
    .map_err(|error| format!("Windows Hello could not start: {error}"))?;
    let result = operation
        .get()
        .map_err(|error| format!("Windows Hello could not finish: {error}"))?;
    Ok(result == UserConsentVerificationResult::Verified)
}

#[cfg(target_os = "linux")]
pub fn platform_authenticate(
    _method: SystemAuthMethod,
    _window_handle: Option<isize>,
) -> Result<bool, String> {
    Err("Desktop system authentication is not available in this Linux session.".to_string())
}

#[cfg(target_os = "macos")]
fn macos_auth(method: SystemAuthMethod, evaluate: bool) -> Result<bool, String> {
    use block::ConcreteBlock;
    use objc::runtime::{Object, BOOL, YES};
    use objc::{class, msg_send, sel, sel_impl};
    use std::sync::mpsc;

    // macOS owns enrollment and returns only success or failure; Pasted receives
    // no fingerprint or Watch data. Policies 1 and 3 are Touch ID and Watch.
    let policy: i64 = match method {
        SystemAuthMethod::Primary => 1,
        SystemAuthMethod::AppleWatch => 3,
    };
    #[link(name = "LocalAuthentication", kind = "framework")]
    extern "C" {}
    unsafe {
        let context: *mut Object = msg_send![class!(LAContext), new];
        if context.is_null() {
            return Ok(false);
        }
        let mut error: *mut Object = std::ptr::null_mut();
        let available: BOOL = msg_send![context, canEvaluatePolicy: policy error: &mut error];
        if available != YES || !evaluate {
            let _: () = msg_send![context, release];
            return Ok(available == YES);
        }

        let reason: *mut Object =
            msg_send![class!(NSString), stringWithUTF8String: c"Unlock Pasted".as_ptr()];
        let (sender, receiver) = mpsc::sync_channel(1);
        let reply = ConcreteBlock::new(move |success: BOOL, _error: *mut Object| {
            let _ = sender.send(success == YES);
        });
        let reply = reply.copy();
        let _: () =
            msg_send![context, evaluatePolicy: policy localizedReason: reason reply: &*reply];
        let result = receiver
            .recv_timeout(Duration::from_secs(120))
            .map_err(|_| "System authentication timed out.".to_string());
        let _: () = msg_send![context, release];
        result
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
