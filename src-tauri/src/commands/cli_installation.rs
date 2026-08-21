#[tauri::command]
pub fn install_cli_to_path() -> Result<String, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let binary_directory = executable
        .parent()
        .ok_or("Cannot locate binary directory")?;
    let cli_executable = binary_directory.join("pasted");

    if !cli_executable.exists() {
        return Err(format!(
            "pasted binary not found at '{:?}'. Run 'cargo build --bin pasted' first.",
            cli_executable
        ));
    }

    #[cfg(unix)]
    {
        let target_directory = dirs::home_dir()
            .map(|home| home.join(".local/bin"))
            .ok_or("Cannot locate your home directory")?;
        let symlink_path = install_cli_symlink(&cli_executable, &target_directory)?;
        Ok(format!(
            "Successfully installed the pasted command at '{}'. Make sure that directory is in your PATH.",
            symlink_path.display()
        ))
    }

    #[cfg(not(unix))]
    {
        Err("Automatic CLI installation is not supported on this platform yet".to_string())
    }
}

#[cfg(unix)]
fn install_cli_symlink(
    cli_executable: &std::path::Path,
    target_directory: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    use std::fs;
    use std::os::unix::fs::symlink;

    fs::create_dir_all(target_directory).map_err(|error| {
        format!(
            "Failed to create CLI directory '{}': {error}",
            target_directory.display()
        )
    })?;
    let symlink_path = target_directory.join("pasted");
    match fs::symlink_metadata(&symlink_path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            let existing_target = fs::read_link(&symlink_path).map_err(|error| {
                format!(
                    "Failed to inspect existing CLI link '{}': {error}",
                    symlink_path.display()
                )
            })?;
            if existing_target == cli_executable {
                return Ok(symlink_path);
            }
            return Err(format!(
                "Refusing to replace existing CLI link '{}' (currently points to '{}')",
                symlink_path.display(),
                existing_target.display()
            ));
        }
        Ok(_) => {
            return Err(format!(
                "Refusing to replace existing file '{}'",
                symlink_path.display()
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "Failed to inspect CLI destination '{}': {error}",
                symlink_path.display()
            ));
        }
    }

    symlink(cli_executable, &symlink_path).map_err(|error| {
        format!(
            "Failed to create CLI link '{}': {error}",
            symlink_path.display()
        )
    })?;
    Ok(symlink_path)
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use super::install_cli_symlink;

    #[cfg(unix)]
    fn unique_test_directory(label: &str) -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("pasted-{label}-{}-{nonce}", std::process::id()))
    }

    #[test]
    #[cfg(unix)]
    fn cli_install_never_overwrites_an_existing_file() {
        let root = unique_test_directory("cli-preserve");
        let bin_directory = root.join("bin");
        std::fs::create_dir_all(&bin_directory).unwrap();
        let destination = bin_directory.join("pasted");
        std::fs::write(&destination, "user-owned").unwrap();

        let error = install_cli_symlink(&root.join("source"), &bin_directory).unwrap_err();
        assert!(error.contains("Refusing to replace existing file"));
        assert_eq!(std::fs::read_to_string(&destination).unwrap(), "user-owned");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn cli_install_is_idempotent_for_its_existing_link() {
        let root = unique_test_directory("cli-idempotent");
        let source = root.join("pasted-source");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(&source, "binary").unwrap();
        let bin_directory = root.join("bin");

        let first = install_cli_symlink(&source, &bin_directory).unwrap();
        let second = install_cli_symlink(&source, &bin_directory).unwrap();
        assert_eq!(first, second);
        assert_eq!(std::fs::read_link(second).unwrap(), source);

        std::fs::remove_dir_all(root).unwrap();
    }
}
