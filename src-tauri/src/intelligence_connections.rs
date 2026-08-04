use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DetectedIntelligenceConnection {
    pub adapter_id: &'static str,
    pub name: &'static str,
    pub provider_kind: &'static str,
    pub executable_path: Option<String>,
    pub default_endpoint: Option<&'static str>,
    pub version: Option<String>,
    pub capabilities: Vec<&'static str>,
}

struct AdapterDefinition {
    adapter_id: &'static str,
    name: &'static str,
    executable: &'static str,
    explicit_paths: &'static [&'static str],
    provider_kind: &'static str,
    default_endpoint: Option<&'static str>,
    capabilities: &'static [&'static str],
}

const ADAPTERS: &[AdapterDefinition] = &[
    AdapterDefinition {
        adapter_id: "codex_cli",
        name: "Codex CLI",
        executable: "codex",
        explicit_paths: &[],
        provider_kind: "cli",
        default_endpoint: None,
        capabilities: &["structured_output", "json_events", "local_models"],
    },
    AdapterDefinition {
        adapter_id: "claude_cli",
        name: "Claude CLI",
        executable: "claude",
        explicit_paths: &[],
        provider_kind: "cli",
        default_endpoint: None,
        capabilities: &["non_interactive", "structured_output"],
    },
    AdapterDefinition {
        adapter_id: "gemini_cli",
        name: "Gemini CLI",
        executable: "gemini",
        explicit_paths: &[],
        provider_kind: "cli",
        default_endpoint: None,
        capabilities: &["non_interactive"],
    },
    AdapterDefinition {
        adapter_id: "ollama",
        name: "Ollama",
        executable: "ollama",
        explicit_paths: &[],
        provider_kind: "ollama",
        default_endpoint: Some("http://127.0.0.1:11434"),
        capabilities: &["local", "openai_compatible"],
    },
    AdapterDefinition {
        adapter_id: "antigravity_ide",
        name: "Antigravity IDE",
        executable: "antigravity-ide",
        explicit_paths: &[
            "/Applications/Antigravity IDE.app/Contents/Resources/app/bin/antigravity-ide",
        ],
        provider_kind: "cli",
        default_endpoint: None,
        capabilities: &["interactive_chat", "mcp_client"],
    },
];

fn candidate_directories() -> Vec<PathBuf> {
    let mut directories = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();
    #[cfg(target_os = "macos")]
    directories.extend([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
    ]);
    #[cfg(target_os = "linux")]
    directories.extend([
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/snap/bin"),
    ]);
    if let Some(home) = dirs::home_dir() {
        directories.extend([
            home.join(".local/bin"),
            home.join(".npm-global/bin"),
            home.join(".bun/bin"),
            home.join(".volta/bin"),
        ]);
    }
    let mut seen = HashSet::new();
    directories
        .into_iter()
        .filter(|directory| seen.insert(directory.clone()))
        .collect()
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

fn locate_executable(
    name: &str,
    explicit_paths: &[&str],
    directories: &[PathBuf],
) -> Option<PathBuf> {
    if let Some(path) = explicit_paths
        .iter()
        .map(PathBuf::from)
        .find(|path| is_executable(path))
    {
        return Some(path);
    }
    #[cfg(windows)]
    let names = {
        let extensions =
            std::env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".to_string());
        let mut names = vec![name.to_string()];
        names.extend(
            extensions
                .split(';')
                .filter(|extension| !extension.is_empty())
                .map(|extension| format!("{name}{}", extension.to_ascii_lowercase())),
        );
        names
    };
    #[cfg(not(windows))]
    let names = [name.to_string()];

    directories.iter().find_map(|directory| {
        names
            .iter()
            .map(|candidate| directory.join(candidate))
            .find(|path| is_executable(path))
    })
}

fn detect_version(path: &Path) -> Option<String> {
    let mut child = Command::new(path)
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let started = Instant::now();
    loop {
        if child.try_wait().ok()?.is_some() {
            break;
        }
        if started.elapsed() >= Duration::from_millis(750) {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().ok()?;
    let value = if output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).into_owned()
    } else {
        String::from_utf8_lossy(&output.stdout).into_owned()
    };
    let first_line = value.lines().next()?.trim();
    if first_line.is_empty() {
        None
    } else {
        Some(first_line.chars().take(160).collect())
    }
}

pub fn detect_intelligence_connections() -> Vec<DetectedIntelligenceConnection> {
    let directories = candidate_directories();
    ADAPTERS
        .iter()
        .filter_map(|adapter| {
            let path = locate_executable(adapter.executable, adapter.explicit_paths, &directories)?;
            Some(DetectedIntelligenceConnection {
                adapter_id: adapter.adapter_id,
                name: adapter.name,
                provider_kind: adapter.provider_kind,
                executable_path: Some(path.to_string_lossy().into_owned()),
                default_endpoint: adapter.default_endpoint,
                version: detect_version(&path),
                capabilities: adapter.capabilities.to_vec(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_ids_and_executables_are_unique() {
        let mut ids = HashSet::new();
        let mut executables = HashSet::new();
        for adapter in ADAPTERS {
            assert!(ids.insert(adapter.adapter_id));
            assert!(executables.insert(adapter.executable));
            assert!(!adapter.capabilities.is_empty());
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn candidate_directories_include_standard_mac_locations() {
        let directories = candidate_directories();
        assert!(directories.contains(&PathBuf::from("/opt/homebrew/bin")));
        assert!(directories.contains(&PathBuf::from("/usr/local/bin")));
    }
}
