use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, ExitStatus};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

pub(crate) fn find_executable(name: &str, explicit_paths: &[&str]) -> Option<PathBuf> {
    explicit_paths
        .iter()
        .map(PathBuf::from)
        .find(|path| is_executable(path))
        .or_else(|| {
            let path = std::env::var_os("PATH")?;
            std::env::split_paths(&path)
                .filter(|directory| directory.is_absolute())
                .map(|directory| directory.join(name))
                .find(|path| is_executable(path))
        })
}

pub(crate) struct PrivateWorkspace(PathBuf);

impl PrivateWorkspace {
    pub(crate) fn create(label: &str) -> std::io::Result<Self> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("pasted-{label}-{}-{nonce}", std::process::id()));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&path)?;
        Ok(Self(path))
    }

    pub(crate) fn join(&self, path: impl AsRef<Path>) -> PathBuf {
        self.0.join(path)
    }
}

impl Drop for PrivateWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessWaitError {
    Failed,
    TimedOut,
}

pub(crate) fn wait_bounded(
    child: &mut Child,
    timeout: Duration,
) -> Result<ExitStatus, ProcessWaitError> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if started.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ProcessWaitError::TimedOut);
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ProcessWaitError::Failed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_workspace_is_removed_on_drop() {
        let path = {
            let workspace = PrivateWorkspace::create("external-tool-test").unwrap();
            let path = workspace.join("marker");
            fs::write(&path, b"private").unwrap();
            path
        };
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn private_workspace_rejects_access_from_other_users() {
        use std::os::unix::fs::PermissionsExt;
        let workspace = PrivateWorkspace::create("external-tool-mode-test").unwrap();
        let mode = workspace.join(".").metadata().unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0);
    }
}
