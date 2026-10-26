// Platform behavior adapted from tauri-plugin-single-instance 2.5.4.
// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::Callback;
use tauri::{
    plugin::{self, TauriPlugin},
    AppHandle, Manager, RunEvent, Runtime,
};
use zbus::{interface, names::WellKnownName, Connection};

struct ConnectionHandle(Connection);

struct SingleInstanceService<R: Runtime> {
    callback: Box<Callback<R>>,
    app_handle: AppHandle<R>,
}

#[interface(name = "org.SingleInstance.DBus")]
impl<R: Runtime> SingleInstanceService<R> {
    fn execute_callback(&mut self, argv: Vec<String>, cwd: String) {
        (self.callback)(&self.app_handle, argv, cwd);
    }
}

struct ServiceName(String);

pub(super) fn init<R: Runtime>(callback: Box<Callback<R>>) -> TauriPlugin<R> {
    plugin::Builder::new("single-instance")
        .setup(move |app, _api| {
            let (service_name, object_path) = service_identity(&app.config().identifier);
            let service = SingleInstanceService {
                callback,
                app_handle: app.clone(),
            };

            let result = tauri::async_runtime::block_on(
                zbus::connection::Builder::session()
                    .name(service_name.as_str())
                    .serve_at(object_path.as_str(), service)
                    .build(),
            );

            match result {
                Ok(connection) => {
                    app.manage(ConnectionHandle(connection));
                }
                Err(zbus::Error::NameTaken) => {
                    forward_to_primary(&service_name, &object_path)?;
                    app.cleanup_before_exit();
                    std::process::exit(0);
                }
                Err(error) => {
                    eprintln!(
                        "single-instance: D-Bus session bus unavailable, launching normally: {error}"
                    );
                    return Ok(());
                }
            }

            app.manage(ServiceName(service_name));
            Ok(())
        })
        .on_event(move |app, event| {
            if let RunEvent::Exit = event {
                destroy(app);
            }
        })
        .build()
}

fn service_identity(identifier: &str) -> (String, String) {
    let service_name = format!("{identifier}.SingleInstance");
    let mut object_path = service_name.replace('.', "/").replace('-', "_");
    if !object_path.starts_with('/') {
        object_path.insert(0, '/');
    }
    (service_name, object_path)
}

fn forward_to_primary(service_name: &str, object_path: &str) -> Result<(), String> {
    tauri::async_runtime::block_on(async {
        let connection = Connection::session()
            .await
            .map_err(|error| format!("failed to connect to the running instance: {error}"))?;
        connection
            .call_method(
                Some(service_name),
                object_path,
                Some("org.SingleInstance.DBus"),
                "ExecuteCallback",
                &(
                    std::env::args_os()
                        .map(|argument| argument.to_string_lossy().into_owned())
                        .collect::<Vec<String>>(),
                    std::env::current_dir()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .as_ref(),
                ),
            )
            .await
            .map_err(|error| {
                format!("failed to forward arguments to the running instance: {error}")
            })?;
        Ok(())
    })
}

fn destroy<R: Runtime, M: Manager<R>>(manager: &M) {
    let Some(connection) = manager.try_state::<ConnectionHandle>() else {
        return;
    };
    let Some(service_name) = manager
        .try_state::<ServiceName>()
        .and_then(|name| WellKnownName::try_from(name.0.clone()).ok())
    else {
        return;
    };

    let _ = tauri::async_runtime::block_on(connection.0.release_name(service_name));
}

#[cfg(test)]
mod tests {
    use super::service_identity;

    #[test]
    fn service_identity_matches_the_existing_plugin_contract() {
        assert_eq!(
            service_identity("com.getpasted.pasted"),
            (
                "com.getpasted.pasted.SingleInstance".into(),
                "/com/getpasted/pasted/SingleInstance".into(),
            )
        );
    }

    #[test]
    fn service_identity_normalizes_hyphens_for_the_object_path_only() {
        assert_eq!(
            service_identity("com.example.pasted-beta"),
            (
                "com.example.pasted-beta.SingleInstance".into(),
                "/com/example/pasted_beta/SingleInstance".into(),
            )
        );
    }
}
