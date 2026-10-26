// Platform behavior adapted from tauri-plugin-single-instance 2.5.4.
// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0 OR MIT

use tauri::{plugin::TauriPlugin, AppHandle};

#[cfg(target_os = "linux")]
#[path = "single_instance/linux.rs"]
mod platform;
#[cfg(target_os = "macos")]
#[path = "single_instance/macos.rs"]
mod platform;
#[cfg(target_os = "windows")]
#[path = "single_instance/windows.rs"]
mod platform;

type Callback<R> = dyn FnMut(&AppHandle<R>, Vec<String>, String) + Send + Sync + 'static;

pub(crate) fn init() -> TauriPlugin<tauri::Wry> {
    platform::init(Box::new(|app, arguments, _cwd| {
        crate::app_runtime::handle_single_instance(app, &arguments);
    }))
}
