#[cfg(debug_assertions)]
pub(crate) fn database_path() -> Result<Option<std::path::PathBuf>, String> {
    let Some(path) = std::env::var_os("PASTED_PREVIEW_DATABASE_PATH") else {
        return Ok(None);
    };
    validate_database_path(std::path::PathBuf::from(path)).map(Some)
}

#[cfg(not(debug_assertions))]
pub(crate) fn database_path() -> Result<Option<std::path::PathBuf>, String> {
    Ok(None)
}

pub(crate) fn prepare(app: &tauri::AppHandle) -> Result<Option<std::path::PathBuf>, String> {
    let database_path = database_path()?;
    configure_full_rate(app, database_path.is_some());
    Ok(database_path)
}

#[cfg(all(debug_assertions, target_os = "macos"))]
pub(crate) fn configure_full_rate(app: &tauri::AppHandle, has_valid_preview_database: bool) {
    if !has_valid_preview_database
        || std::env::var_os("PASTED_LOCAL_WEBKIT_FULL_RATE").as_deref()
            != Some(std::ffi::OsStr::new("1"))
    {
        return;
    }

    use objc::runtime::Object;
    use objc::{msg_send, sel, sel_impl};
    use tauri::Manager;

    for window in app.webview_windows().values() {
        let label = window.label().to_string();
        let closure_label = label.clone();
        let result = window.with_webview(move |webview| unsafe {
            let wk_webview = webview.inner() as *mut Object;
            let configuration: *mut Object = msg_send![wk_webview, configuration];
            let preferences: *mut Object = msg_send![configuration, preferences];
            let features: *mut Object = msg_send![objc::class!(WKPreferences), _features];
            let count: usize = msg_send![features, count];

            for index in 0..count {
                let feature: *mut Object = msg_send![features, objectAtIndex: index];
                let key: *mut Object = msg_send![feature, key];
                let utf8: *const std::ffi::c_char = msg_send![key, UTF8String];
                if utf8.is_null()
                    || std::ffi::CStr::from_ptr(utf8).to_bytes()
                        != b"PreferPageRenderingUpdatesNear60FPSEnabled"
                {
                    continue;
                }

                let _: () = msg_send![preferences, _setEnabled: 0i8 forFeature: feature];
                eprintln!("Enabled full-rate WebKit rendering for preview window {closure_label}");
                return;
            }

            eprintln!("Could not find WebKit's full-rate rendering preference for {closure_label}");
        });
        if let Err(error) = result {
            eprintln!("Could not configure full-rate WebKit rendering for {label}: {error}");
        }
    }
}

#[cfg(not(all(debug_assertions, target_os = "macos")))]
pub(crate) fn configure_full_rate(_app: &tauri::AppHandle, _has_valid_preview_database: bool) {}

#[cfg(debug_assertions)]
fn validate_database_path(path: std::path::PathBuf) -> Result<std::path::PathBuf, String> {
    if !path.is_absolute() || path.file_name().and_then(|name| name.to_str()) != Some("pasted.db") {
        return Err("The preview database must be an absolute path ending in pasted.db".into());
    }

    let temporary_root = std::env::temp_dir()
        .canonicalize()
        .map_err(|error| format!("Could not resolve the temporary directory: {error}"))?;
    let parent = path
        .parent()
        .ok_or_else(|| "The preview database has no parent directory".to_string())?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|error| format!("Could not resolve the preview directory: {error}"))?;
    let directory_name = canonical_parent
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if canonical_parent.parent() != Some(temporary_root.as_path())
        || !directory_name.starts_with("pasted-local-webkit.")
    {
        return Err(
            "The preview database must be inside a script-managed temporary directory".into(),
        );
    }

    let metadata = path
        .symlink_metadata()
        .map_err(|error| format!("Could not inspect the preview database: {error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("The preview database must be a regular file".into());
    }
    let canonical_path = path
        .canonicalize()
        .map_err(|error| format!("Could not resolve the preview database: {error}"))?;
    if canonical_path.parent() != Some(canonical_parent.as_path()) {
        return Err("The preview database must not resolve outside its temporary directory".into());
    }
    Ok(canonical_path)
}
