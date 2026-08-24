use std::time::Duration;

#[cfg(any(target_os = "macos", test))]
const AUTH_FAILED_ERROR: &str = "app_lock_auth_failed";
#[cfg(any(target_os = "macos", test))]
const AUTH_WATCH_FAILED_ERROR: &str = "app_lock_auth_watch_failed";
#[cfg(any(target_os = "macos", test))]
const AUTH_WATCH_UNAVAILABLE_ERROR: &str = "app_lock_auth_watch_unavailable";
#[cfg(target_os = "macos")]
const AUTH_TIMEOUT_ERROR: &str = "app_lock_auth_timeout";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemAuthMethod {
    Primary,
    AppleWatch,
}

#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeAuthError {
    domain: String,
    code: i64,
}

#[cfg(any(target_os = "macos", test))]
fn classify_native_auth_result(
    method: SystemAuthMethod,
    success: bool,
    error: Option<NativeAuthError>,
) -> Result<bool, String> {
    if success {
        return Ok(true);
    }
    let Some(error) = error else {
        return Err(AUTH_FAILED_ERROR.to_string());
    };
    let is_local_auth = error.domain == "com.apple.LocalAuthentication";
    if is_local_auth && matches!(error.code, -2 | -3 | -4 | -9) {
        return Ok(false);
    }
    if method == SystemAuthMethod::AppleWatch && is_local_auth && matches!(error.code, -11 | -1000)
    {
        return Err(AUTH_WATCH_UNAVAILABLE_ERROR.to_string());
    }
    if method == SystemAuthMethod::AppleWatch && is_local_auth && error.code == -1 {
        return Err(AUTH_WATCH_FAILED_ERROR.to_string());
    }
    Err(AUTH_FAILED_ERROR.to_string())
}

#[cfg(target_os = "macos")]
pub fn platform_auth_label() -> &'static str {
    "Touch ID"
}

#[cfg(target_os = "windows")]
pub fn platform_auth_label() -> &'static str {
    "Windows Hello"
}

#[cfg(target_os = "linux")]
pub fn platform_auth_label() -> &'static str {
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
        .and_then(|operation| operation.join())
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
    use windows::Security::Credentials::UI::{UserConsentVerificationResult, UserConsentVerifier};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::WinRT::IUserConsentVerifierInterop;
    use windows_future::IAsyncOperation;
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
        .join()
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
    use block2::RcBlock;
    use objc::runtime::{Object, BOOL, YES};
    use objc::{class, msg_send, sel, sel_impl};
    use objc2::runtime::Bool;
    use std::sync::mpsc;

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
        if !evaluate {
            let _: () = msg_send![context, release];
            return Ok(available == YES);
        }
        if available != YES {
            let result = classify_native_auth_result(method, false, macos_error(error.cast()));
            let _: () = msg_send![context, release];
            return result;
        }

        let reason: *mut Object =
            msg_send![class!(NSString), stringWithUTF8String: c"Unlock Pasted".as_ptr()];
        let (sender, receiver) = mpsc::sync_channel(1);
        let reply: RcBlock<dyn Fn(Bool, *mut std::ffi::c_void)> =
            RcBlock::new(move |success: Bool, error: *mut std::ffi::c_void| {
                let _ = sender.send((success.as_bool(), macos_error(error)));
            });
        let _: () =
            msg_send![context, evaluatePolicy: policy localizedReason: reason reply: &*reply];
        let received = receiver.recv_timeout(Duration::from_secs(120));
        if received.is_err() {
            let _: () = msg_send![context, invalidate];
        }
        let _: () = msg_send![context, release];
        let (success, error) = received.map_err(|_| AUTH_TIMEOUT_ERROR.to_string())?;
        classify_native_auth_result(method, success, error)
    }
}

#[cfg(target_os = "macos")]
unsafe fn macos_error(error: *mut std::ffi::c_void) -> Option<NativeAuthError> {
    use objc::runtime::Object;
    use objc::{msg_send, sel, sel_impl};
    use std::ffi::CStr;

    let error = error.cast::<Object>();
    if error.is_null() {
        return None;
    }
    let code: i64 = msg_send![error, code];
    let domain: *mut Object = msg_send![error, domain];
    let domain_utf8: *const std::ffi::c_char = msg_send![domain, UTF8String];
    let domain = if domain_utf8.is_null() {
        String::new()
    } else {
        CStr::from_ptr(domain_utf8).to_string_lossy().into_owned()
    };
    Some(NativeAuthError { domain, code })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_auth_outcomes_distinguish_cancellation_and_watch_failures() {
        let local_auth = |code| NativeAuthError {
            domain: "com.apple.LocalAuthentication".to_string(),
            code,
        };
        assert!(classify_native_auth_result(SystemAuthMethod::AppleWatch, true, None).unwrap());
        assert!(!classify_native_auth_result(
            SystemAuthMethod::AppleWatch,
            false,
            Some(local_auth(-2)),
        )
        .unwrap());
        assert_eq!(
            classify_native_auth_result(
                SystemAuthMethod::AppleWatch,
                false,
                Some(local_auth(-11)),
            )
            .unwrap_err(),
            AUTH_WATCH_UNAVAILABLE_ERROR
        );
        assert_eq!(
            classify_native_auth_result(SystemAuthMethod::AppleWatch, false, Some(local_auth(-1)))
                .unwrap_err(),
            AUTH_WATCH_FAILED_ERROR
        );
    }
}
