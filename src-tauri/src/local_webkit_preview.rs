#[cfg(debug_assertions)]
pub(crate) fn database_path() -> Option<std::path::PathBuf> {
    std::env::var_os("PASTED_PREVIEW_DATABASE_PATH").map(Into::into)
}

#[cfg(not(debug_assertions))]
pub(crate) fn database_path() -> Option<std::path::PathBuf> {
    None
}
