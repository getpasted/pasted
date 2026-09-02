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
