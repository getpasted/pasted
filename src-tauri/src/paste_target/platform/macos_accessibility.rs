use std::ffi::{c_void, CStr, CString};
use std::ptr;

type CfTypeRef = *const c_void;
type CfStringRef = *const c_void;
type CfTypeId = usize;
type AxUiElementRef = *const c_void;
const UTF8: u32 = 0x0800_0100;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> AxUiElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AxUiElementRef,
        attribute: CfStringRef,
        value: *mut CfTypeRef,
    ) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFStringCreateWithCString(
        allocator: *const c_void,
        text: *const i8,
        encoding: u32,
    ) -> CfStringRef;
    fn CFStringGetCString(
        string: CfStringRef,
        buffer: *mut i8,
        buffer_size: isize,
        encoding: u32,
    ) -> bool;
    fn CFStringGetTypeID() -> CfTypeId;
    fn CFGetTypeID(value: CfTypeRef) -> CfTypeId;
    fn CFRelease(value: CfTypeRef);
}

unsafe fn attribute(name: &str) -> CfStringRef {
    let name = CString::new(name).ok();
    name.map_or(ptr::null(), |name| {
        CFStringCreateWithCString(ptr::null(), name.as_ptr(), UTF8)
    })
}

unsafe fn value(element: AxUiElementRef, name: &str) -> Option<CfTypeRef> {
    let key = attribute(name);
    if key.is_null() {
        return None;
    }
    let mut value: CfTypeRef = ptr::null();
    let status = AXUIElementCopyAttributeValue(element, key, &mut value);
    CFRelease(key);
    (status == 0 && !value.is_null()).then_some(value)
}

unsafe fn string_value(element: AxUiElementRef, name: &str) -> Option<String> {
    let value = value(element, name)?;
    if CFGetTypeID(value) != CFStringGetTypeID() {
        CFRelease(value);
        return None;
    }
    let mut buffer = vec![0i8; 2049];
    let copied = CFStringGetCString(value, buffer.as_mut_ptr(), buffer.len() as isize, UTF8);
    CFRelease(value);
    copied.then(|| {
        CStr::from_ptr(buffer.as_ptr())
            .to_string_lossy()
            .into_owned()
    })
}

unsafe fn focused_element(pid: i32, attribute_name: &str) -> Option<AxUiElementRef> {
    let application = AXUIElementCreateApplication(pid);
    if application.is_null() {
        return None;
    }
    let focused = value(application, attribute_name);
    CFRelease(application);
    focused
}

pub(super) fn focused_window_title(pid: i32) -> Option<String> {
    unsafe {
        let window = focused_element(pid, "AXFocusedWindow")?;
        let title = string_value(window, "AXTitle");
        CFRelease(window);
        title
    }
}

pub(super) fn focused_element_attributes(pid: i32) -> Option<[Option<String>; 5]> {
    unsafe {
        let focused = focused_element(pid, "AXFocusedUIElement")?;
        let values = [
            string_value(focused, "AXRoleDescription"),
            string_value(focused, "AXTitle"),
            string_value(focused, "AXDescription"),
            string_value(focused, "AXHelp"),
            string_value(focused, "AXPlaceholderValue"),
        ];
        CFRelease(focused);
        Some(values)
    }
}
