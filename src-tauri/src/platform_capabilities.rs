#[derive(serde::Serialize, Clone)]
pub struct AccessibilityStatus {
    pub is_trusted: bool,
    pub is_dev_mode: bool,
}

pub fn accessibility_status() -> AccessibilityStatus {
    let is_trusted = {
        #[cfg(target_os = "macos")]
        {
            use std::ptr;
            #[link(name = "ApplicationServices", kind = "framework")]
            extern "C" {
                fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
            }
            unsafe { AXIsProcessTrustedWithOptions(ptr::null()) }
        }
        #[cfg(not(target_os = "macos"))]
        true
    };

    AccessibilityStatus {
        is_trusted,
        is_dev_mode: cfg!(debug_assertions),
    }
}
