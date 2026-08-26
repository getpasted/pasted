use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Manager, State};

use crate::db::DbState;
use crate::installation_diagnostics::InstallationDiagnostics;
use crate::third_party_licenses::ThirdPartyLicenseDocument;

#[tauri::command]
pub async fn get_installation_diagnostics(
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<InstallationDiagnostics, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let app_path = executable
        .ancestors()
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
        .map(PathBuf::from)
        .unwrap_or(executable);
    let data_path = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let database_path = db.database_path();
    tauri::async_runtime::spawn_blocking(move || {
        InstallationDiagnostics::collect_with_database(app_path, data_path, database_path)
    })
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_third_party_licenses() -> Result<ThirdPartyLicenseDocument, String> {
    tauri::async_runtime::spawn_blocking(|| crate::third_party_licenses::document().clone())
        .await
        .map_err(|error| error.to_string())
}

const BACKING_URL: &str = "https://back.getpasted.app";

#[tauri::command]
pub fn open_backing_page() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = std::process::Command::new("open");
        command.arg(BACKING_URL);
        command
    };

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = std::process::Command::new("cmd");
        command.args(["/c", "start", "", BACKING_URL]);
        command
    };

    #[cfg(target_os = "linux")]
    let mut command = {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(BACKING_URL);
        command
    };

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    return command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open the backing page: {error}"));

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    Err("Opening the backing page is unavailable on this platform".to_string())
}
