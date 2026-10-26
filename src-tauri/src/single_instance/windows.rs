// Platform behavior adapted from tauri-plugin-single-instance 2.5.4.
// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::Callback;
use tauri::{
    plugin::{self, TauriPlugin},
    AppHandle, Manager, RunEvent, Runtime,
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, WPARAM},
    System::{
        DataExchange::COPYDATASTRUCT,
        LibraryLoader::GetModuleHandleW,
        Threading::{CreateMutexW, ReleaseMutex},
    },
    UI::WindowsAndMessaging::{
        self as w32wm, AllowSetForegroundWindow, CreateWindowExW, DefWindowProcW, DestroyWindow,
        FindWindowW, GetWindowThreadProcessId, RegisterClassExW, SendMessageTimeoutW,
        SetWindowLongPtrW as SetWindowLongPtrW64, CREATESTRUCTW, GWLP_USERDATA, GWL_STYLE,
        SMTO_NORMAL, WINDOW_LONG_PTR_INDEX, WM_COPYDATA, WM_CREATE, WM_DESTROY, WNDCLASSEXW,
        WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_OVERLAPPED,
        WS_POPUP, WS_VISIBLE,
    },
};

const LEGACY_PAYLOAD: usize = 1542;
const NUL_SEPARATED_PAYLOAD: usize = 1543;
const FORWARD_TIMEOUT_MS: u32 = 10_000;

struct MutexHandle(isize);
struct TargetWindowHandle(isize);

struct UserData<R: Runtime> {
    app: AppHandle<R>,
    callback: Box<Callback<R>>,
}

impl<R: Runtime> UserData<R> {
    unsafe fn from_hwnd_raw(hwnd: HWND) -> *mut Self {
        unsafe { get_window_long_ptr(hwnd, GWLP_USERDATA) as *mut Self }
    }

    unsafe fn from_hwnd<'a>(hwnd: HWND) -> &'a mut Self {
        unsafe { &mut *Self::from_hwnd_raw(hwnd) }
    }

    fn run_callback(&mut self, arguments: Vec<String>, cwd: String) {
        (self.callback)(&self.app, arguments, cwd);
    }
}

pub(super) fn init<R: Runtime>(callback: Box<Callback<R>>) -> TauriPlugin<R> {
    plugin::Builder::new("single-instance")
        .setup(move |app, _api| {
            let identifier = &app.config().identifier;
            let class_name = encode_wide(format!("{identifier}-sic"));
            let window_name = encode_wide(format!("{identifier}-siw"));
            let mutex_name = encode_wide(format!("{identifier}-sim"));

            let mutex = unsafe { CreateMutexW(std::ptr::null(), true.into(), mutex_name.as_ptr()) };

            if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
                unsafe {
                    let window = FindWindowW(class_name.as_ptr(), window_name.as_ptr());
                    if !window.is_null() {
                        allow_primary_to_focus(window);
                        forward_to_primary(window);
                        app.cleanup_before_exit();
                        std::process::exit(0);
                    }
                }
            } else {
                app.manage(MutexHandle(mutex as _));
                let user_data = Box::into_raw(Box::new(UserData {
                    app: app.clone(),
                    callback,
                }));
                let window = create_event_target_window::<R>(&class_name, &window_name, user_data);
                app.manage(TargetWindowHandle(window as _));
            }

            Ok(())
        })
        .on_event(|app, event| {
            if let RunEvent::Exit = event {
                destroy(app);
            }
        })
        .build()
}

fn destroy<R: Runtime, M: Manager<R>>(manager: &M) {
    if let Some(mutex) = manager.try_state::<MutexHandle>() {
        unsafe {
            ReleaseMutex(mutex.0 as _);
            CloseHandle(mutex.0 as _);
        }
    }
    if let Some(window) = manager.try_state::<TargetWindowHandle>() {
        unsafe {
            DestroyWindow(window.0 as _);
        }
    }
}

unsafe fn allow_primary_to_focus(window: HWND) {
    let mut process_id = 0;
    unsafe {
        GetWindowThreadProcessId(window, &mut process_id);
        if process_id != 0 {
            AllowSetForegroundWindow(process_id);
        }
    }
}

unsafe fn forward_to_primary(window: HWND) {
    let mut payload = std::env::current_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    for argument in std::env::args_os() {
        payload.push('\0');
        payload.push_str(&argument.to_string_lossy());
    }

    let bytes = payload.as_bytes();
    let copy_data = COPYDATASTRUCT {
        dwData: NUL_SEPARATED_PAYLOAD,
        cbData: bytes.len() as _,
        lpData: bytes.as_ptr() as _,
    };
    let sent = unsafe {
        SendMessageTimeoutW(
            window,
            WM_COPYDATA,
            0,
            &copy_data as *const _ as _,
            SMTO_NORMAL,
            FORWARD_TIMEOUT_MS,
            std::ptr::null_mut(),
        )
    };
    if sent == 0 {
        eprintln!(
            "single-instance: failed to forward arguments within {FORWARD_TIMEOUT_MS}ms (error {})",
            unsafe { GetLastError() }
        );
    }
}

unsafe extern "system" fn event_target_window_proc<R: Runtime>(
    window: HWND,
    message: u32,
    word_parameter: WPARAM,
    long_parameter: LPARAM,
) -> LRESULT {
    match message {
        WM_CREATE => {
            let create = unsafe { &*(long_parameter as *const CREATESTRUCTW) };
            let user_data = create.lpCreateParams as *const UserData<R>;
            unsafe {
                set_window_long_ptr(window, GWLP_USERDATA, user_data as _);
            }
            0
        }
        WM_COPYDATA => {
            let copy_data = unsafe { &*(long_parameter as *const COPYDATASTRUCT) };
            let separator = match copy_data.dwData {
                NUL_SEPARATED_PAYLOAD => '\0',
                LEGACY_PAYLOAD => '|',
                _ => return 1,
            };
            let bytes = if copy_data.lpData.is_null() {
                &[][..]
            } else {
                unsafe {
                    std::slice::from_raw_parts(copy_data.lpData as *const u8, copy_data.cbData as _)
                }
            };
            let bytes = if separator == '|' {
                bytes.split(|byte| *byte == 0).next().unwrap_or_default()
            } else {
                bytes
            };
            let payload = String::from_utf8_lossy(bytes);
            let mut fields = payload.split(separator);
            let cwd = fields.next().unwrap_or_default().to_string();
            let arguments = fields.map(String::from).collect();
            unsafe { UserData::<R>::from_hwnd(window) }.run_callback(arguments, cwd);
            1
        }
        WM_DESTROY => {
            let user_data = unsafe { UserData::<R>::from_hwnd_raw(window) };
            drop(unsafe { Box::from_raw(user_data) });
            0
        }
        _ => unsafe { DefWindowProcW(window, message, word_parameter, long_parameter) },
    }
}

fn create_event_target_window<R: Runtime>(
    class_name: &[u16],
    window_name: &[u16],
    user_data: *const UserData<R>,
) -> HWND {
    unsafe {
        let class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(event_target_window_proc::<R>),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: GetModuleHandleW(std::ptr::null()),
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&class);

        let window = CreateWindowExW(
            WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            window_name.as_ptr(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            GetModuleHandleW(std::ptr::null()),
            user_data as _,
        );
        set_window_long_ptr(window, GWL_STYLE, (WS_VISIBLE | WS_POPUP) as isize);
        window
    }
}

fn encode_wide(string: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    std::os::windows::prelude::OsStrExt::encode_wide(string.as_ref())
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(target_pointer_width = "32")]
unsafe fn set_window_long_ptr(window: HWND, index: WINDOW_LONG_PTR_INDEX, value: isize) -> isize {
    unsafe { w32wm::SetWindowLongW(window, index, value as _) as _ }
}

#[cfg(target_pointer_width = "64")]
unsafe fn set_window_long_ptr(window: HWND, index: WINDOW_LONG_PTR_INDEX, value: isize) -> isize {
    unsafe { SetWindowLongPtrW64(window, index, value) }
}

#[cfg(target_pointer_width = "32")]
unsafe fn get_window_long_ptr(window: HWND, index: WINDOW_LONG_PTR_INDEX) -> isize {
    unsafe { w32wm::GetWindowLongW(window, index) as _ }
}

#[cfg(target_pointer_width = "64")]
unsafe fn get_window_long_ptr(window: HWND, index: WINDOW_LONG_PTR_INDEX) -> isize {
    unsafe { w32wm::GetWindowLongPtrW(window, index) }
}
