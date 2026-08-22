use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const FAILED_VERSION_PROBE_TTL: Duration = Duration::from_secs(30);

struct VersionCacheEntry {
    value: Option<String>,
    checked_at: Instant,
}

fn cached_version(
    cache: &Mutex<std::collections::HashMap<String, VersionCacheEntry>>,
    key: &str,
) -> Option<Option<String>> {
    let cache = cache.lock().ok()?;
    let entry = cache.get(key)?;
    (entry.value.is_some() || entry.checked_at.elapsed() < FAILED_VERSION_PROBE_TTL)
        .then(|| entry.value.clone())
}

pub(crate) fn is_executable(path: &Path) -> bool {
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

pub(crate) fn probe_version(path: &Path, arguments: &[&str]) -> Option<String> {
    if !is_executable(path) {
        return None;
    }
    let metadata = path.metadata().ok()?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    let cache_key = format!(
        "{}\0{}\0{}\0{}",
        path.to_string_lossy(),
        arguments.join("\0"),
        metadata.len(),
        modified,
    );
    static VERSION_CACHE: OnceLock<Mutex<std::collections::HashMap<String, VersionCacheEntry>>> =
        OnceLock::new();
    let cache = VERSION_CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    if let Some(version) = cached_version(cache, &cache_key) {
        return version;
    }
    let version = (|| {
        let workspace = PrivateWorkspace::create("version-probe").ok()?;
        let stdout_path = workspace.join("stdout");
        let stderr_path = workspace.join("stderr");
        let stdout = fs::File::create(&stdout_path).ok()?;
        let stderr = fs::File::create(&stderr_path).ok()?;
        let mut child = Command::new(path)
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .ok()?;
        let status = wait_bounded(&mut child, Duration::from_secs(2)).ok()?;
        if !status.success() {
            return None;
        }
        for output_path in [&stdout_path, &stderr_path] {
            if output_path.metadata().ok()?.len() > 16 * 1024 {
                continue;
            }
            let output = fs::read_to_string(output_path).ok()?;
            if let Some(line) = output.lines().map(str::trim).find(|line| !line.is_empty()) {
                return Some(line.chars().take(160).collect());
            }
        }
        None
    })();
    if let Ok(mut cache) = cache.lock() {
        cache.insert(
            cache_key,
            VersionCacheEntry {
                value: version.clone(),
                checked_at: Instant::now(),
            },
        );
    }
    version
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

    #[test]
    fn failed_version_probes_are_cached_temporarily() {
        let cache = Mutex::new(std::collections::HashMap::from([(
            "failed".into(),
            VersionCacheEntry {
                value: None,
                checked_at: Instant::now(),
            },
        )]));

        assert_eq!(cached_version(&cache, "failed"), Some(None));

        cache.lock().unwrap().get_mut("failed").unwrap().checked_at =
            Instant::now() - FAILED_VERSION_PROBE_TTL - Duration::from_secs(1);
        assert_eq!(cached_version(&cache, "failed"), None);
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
