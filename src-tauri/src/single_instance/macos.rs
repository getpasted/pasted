// Platform behavior adapted from tauri-plugin-single-instance 2.5.4.
// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{
    io::{BufWriter, Error, ErrorKind, Write},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
};

use super::Callback;
use tauri::{
    plugin::{self, TauriPlugin},
    AppHandle, Config, Manager, RunEvent, Runtime,
};
use tokio::io::AsyncReadExt;

pub(super) fn init<R: Runtime>(callback: Box<Callback<R>>) -> TauriPlugin<R> {
    plugin::Builder::new("single-instance")
        .setup(move |app, _api| {
            let socket = socket_path(app.config());

            match notify_primary(&socket) {
                Ok(()) => {
                    app.cleanup_before_exit();
                    std::process::exit(0);
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        ErrorKind::NotFound | ErrorKind::ConnectionRefused
                    ) =>
                {
                    remove_socket(&socket);
                    listen_for_other_instances(socket, app.clone(), callback);
                }
                Err(error) => {
                    eprintln!(
                        "single-instance: failed to notify the running instance, launching normally: {error}"
                    );
                }
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
    remove_socket(&socket_path(manager.config()));
}

fn socket_path(config: &Config) -> PathBuf {
    let identifier = config.identifier.replace(['.', '-'], "_");
    PathBuf::from(format!("/tmp/{identifier}_si.sock"))
}

fn remove_socket(socket: &Path) {
    let _ = std::fs::remove_file(socket);
}

fn notify_primary(socket: &Path) -> Result<(), Error> {
    let stream = UnixStream::connect(socket)?;
    let mut writer = BufWriter::new(&stream);
    let cwd = std::env::current_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    writer.write_all(cwd.as_bytes())?;
    writer.write_all(b"\0\0")?;
    let arguments = std::env::args_os()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect::<Vec<String>>()
        .join("\0");
    writer.write_all(arguments.as_bytes())?;
    writer.flush()
}

fn listen_for_other_instances<R: Runtime>(
    socket: PathBuf,
    app: AppHandle<R>,
    mut callback: Box<Callback<R>>,
) {
    tauri::async_runtime::spawn(async move {
        match tokio::net::UnixListener::bind(socket) {
            Ok(listener) => loop {
                match listener.accept().await {
                    Ok((mut stream, _address)) => {
                        let mut payload = String::new();
                        match stream.read_to_string(&mut payload).await {
                            Ok(_) => {
                                let (cwd, arguments) =
                                    payload.split_once("\0\0").unwrap_or_default();
                                let arguments = arguments.split('\0').map(String::from).collect();
                                callback(app.app_handle(), arguments, cwd.to_string());
                            }
                            Err(error) => eprintln!(
                                "single-instance: failed to read a secondary instance: {error}"
                            ),
                        }
                    }
                    Err(error) => eprintln!(
                        "single-instance: failed to accept a secondary instance: {error}"
                    ),
                }
            },
            Err(error) => eprintln!(
                "single-instance: failed to listen for secondary instances, launching normally: {error}"
            ),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::socket_path;

    #[test]
    fn socket_path_matches_the_existing_plugin_contract() {
        let config = tauri::Config {
            identifier: "com.getpasted.pasted-beta".into(),
            ..Default::default()
        };
        assert_eq!(
            socket_path(&config).to_string_lossy(),
            "/tmp/com_getpasted_pasted_beta_si.sock"
        );
    }
}
