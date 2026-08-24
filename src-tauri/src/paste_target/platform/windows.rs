use super::super::{ActiveApplicationContext, PasteAction, PasteTarget};
use std::time::Duration;

pub(in crate::paste_target) fn unavailable_reason() -> Option<String> {
    None
}

pub(in crate::paste_target) fn active_application_context(
    include_private_mode_signal: bool,
) -> Option<ActiveApplicationContext> {
    let target = frontmost_application()?;
    let accessible_title = include_private_mode_signal
        .then(|| accessible_window_title(target.native_handle))
        .flatten();
    let window_title_is_accessible = accessible_title.is_some();
    Some(ActiveApplicationContext {
        name: application_name(target.pid).unwrap_or_else(|| target.name.clone()),
        window_title: include_private_mode_signal
            .then(|| {
                accessible_title.or_else(|| (target.name != "Previous app").then_some(target.name))
            })
            .flatten(),
        window_title_is_accessible,
    })
}

pub(in crate::paste_target) fn frontmost_application() -> Option<PasteTarget> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    let handle = unsafe { GetForegroundWindow() };
    if handle == 0 {
        return None;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(handle, &mut pid) };
    if pid == 0 {
        return None;
    }
    let length = unsafe { GetWindowTextLengthW(handle) };
    let mut buffer = vec![0u16; (length.max(0) + 1) as usize];
    let copied = unsafe { GetWindowTextW(handle, buffer.as_mut_ptr(), buffer.len() as i32) };
    let name = if copied > 0 {
        OsString::from_wide(&buffer[..copied as usize])
            .to_string_lossy()
            .into_owned()
    } else {
        "Previous app".to_string()
    };
    Some(PasteTarget::available(
        pid as i32,
        format!("windows:{pid}"),
        handle as u64,
        name,
    ))
}

fn application_name(pid: i32) -> Option<String> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::Path;

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid as u32) };
    if process == 0 {
        return None;
    }
    let mut buffer = vec![0u16; 32_768];
    let mut length = buffer.len() as u32;
    let success =
        unsafe { QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length) != 0 };
    unsafe { CloseHandle(process) };
    if !success || length == 0 {
        return None;
    }
    let path = OsString::from_wide(&buffer[..length as usize]);
    Path::new(&path)
        .file_stem()
        .and_then(|name| name.to_str())
        .map(str::to_string)
}

fn accessible_window_title(handle: u64) -> Option<String> {
    use std::cell::RefCell;
    use std::ffi::c_void;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation};

    thread_local! {
        static AUTOMATION: RefCell<Option<IUIAutomation>> = const { RefCell::new(None) };
    }

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        AUTOMATION.with(|slot| {
            if slot.borrow().is_none() {
                *slot.borrow_mut() =
                    CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok();
            }
            let slot = slot.borrow();
            let element = slot
                .as_ref()?
                .ElementFromHandle(HWND(handle as *mut c_void))
                .ok()?;
            let name = element.CurrentName().ok()?;
            let title = String::from_utf16_lossy(&name);
            (!title.is_empty()).then_some(title)
        })
    }
}

pub(in crate::paste_target) fn paste_to_target(
    target: &PasteTarget,
    action: PasteAction,
) -> Result<(), String> {
    let handle = target.native_handle as isize;
    if handle == 0
        || unsafe { IsWindow(handle) } == 0
        || unsafe { SetForegroundWindow(handle) } == 0
    {
        return Err(action.target_failure(&target.name));
    }
    std::thread::sleep(Duration::from_millis(120));
    if unsafe { GetForegroundWindow() } != handle {
        return Err(action.target_failure(&target.name));
    }
    unsafe {
        keybd_event(VK_CONTROL, 0, 0, 0);
        keybd_event(b'V', 0, 0, 0);
        keybd_event(b'V', 0, KEYEVENTF_KEYUP, 0);
        keybd_event(VK_CONTROL, 0, KEYEVENTF_KEYUP, 0);
    }
    Ok(())
}

const VK_CONTROL: u8 = 0x11;
const KEYEVENTF_KEYUP: u32 = 0x0002;

#[link(name = "user32")]
extern "system" {
    fn GetForegroundWindow() -> isize;
    fn GetWindowThreadProcessId(window: isize, process_id: *mut u32) -> u32;
    fn GetWindowTextLengthW(window: isize) -> i32;
    fn GetWindowTextW(window: isize, text: *mut u16, max_count: i32) -> i32;
    fn IsWindow(window: isize) -> i32;
    fn SetForegroundWindow(window: isize) -> i32;
    fn keybd_event(virtual_key: u8, scan_code: u8, flags: u32, extra_info: usize);
}

#[link(name = "kernel32")]
extern "system" {
    fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> isize;
    fn QueryFullProcessImageNameW(
        process: isize,
        flags: u32,
        executable_name: *mut u16,
        size: *mut u32,
    ) -> i32;
    fn CloseHandle(object: isize) -> i32;
}
