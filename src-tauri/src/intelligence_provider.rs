use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::db::IntelligenceConnection;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IntelligenceExecutionError {
    pub code: &'static str,
    pub message: String,
}

impl IntelligenceExecutionError {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

pub struct ProviderRequest<'a> {
    pub prompt: &'a str,
    pub output_schema: Option<&'a serde_json::Value>,
    pub cancellation_message: &'a str,
}

#[derive(Debug)]
pub struct ProviderResponse {
    pub output: String,
    pub duration_ms: i64,
}

trait IntelligenceProviderAdapter: Sync {
    fn id(&self) -> &'static str;

    fn supports(&self, connection: &IntelligenceConnection) -> bool;

    fn execute(
        &self,
        connection: &IntelligenceConnection,
        request: ProviderRequest<'_>,
        cancellation: Option<&AtomicBool>,
    ) -> Result<ProviderResponse, IntelligenceExecutionError>;
}

struct CodexCliAdapter;

impl IntelligenceProviderAdapter for CodexCliAdapter {
    fn id(&self) -> &'static str {
        "codex_cli"
    }

    fn supports(&self, connection: &IntelligenceConnection) -> bool {
        connection.provider_kind == "cli"
            && connection.endpoint.as_deref().is_some_and(|endpoint| {
                Path::new(endpoint)
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.to_ascii_lowercase().starts_with("codex"))
            })
    }

    fn execute(
        &self,
        connection: &IntelligenceConnection,
        request: ProviderRequest<'_>,
        cancellation: Option<&AtomicBool>,
    ) -> Result<ProviderResponse, IntelligenceExecutionError> {
        if request.prompt.len() > crate::resource_limits::MAX_PROVIDER_PROMPT_BYTES {
            return Err(IntelligenceExecutionError::new(
                "provider_input_too_large",
                "Provider input exceeds Pasted's 10 MB safety limit",
            ));
        }
        if cancellation.is_some_and(|flag| flag.load(Ordering::Acquire)) {
            return Err(IntelligenceExecutionError::new(
                "execution_cancelled",
                request.cancellation_message,
            ));
        }
        let executable = connection.endpoint.as_deref().ok_or_else(|| {
            IntelligenceExecutionError::new(
                "connection_unavailable",
                "Provider executable path is missing",
            )
        })?;
        let workspace = TemporaryWorkspace::create()?;
        let result_path = workspace.0.join("result.txt");
        let stdout_path = workspace.0.join("stdout.log");
        let stderr_path = workspace.0.join("stderr.log");
        let mut command = Command::new(executable);
        command
            .args([
                "exec",
                "--ephemeral",
                "--ignore-user-config",
                "--ignore-rules",
                "--skip-git-repo-check",
                "--sandbox",
                "read-only",
                "--color",
                "never",
                "-C",
            ])
            .arg(&workspace.0);
        if let Some(schema) = request.output_schema {
            let schema_path = workspace.0.join("output.schema.json");
            let schema = serde_json::to_vec(schema).map_err(|error| {
                IntelligenceExecutionError::new("invalid_plan_schema", error.to_string())
            })?;
            fs::write(&schema_path, schema).map_err(|error| {
                IntelligenceExecutionError::new("workspace_error", error.to_string())
            })?;
            command.arg("--output-schema").arg(schema_path);
        }
        command.arg("--output-last-message").arg(&result_path);
        if let Some(model) = connection
            .model
            .as_deref()
            .filter(|model| !model.trim().is_empty())
        {
            command.arg("--model").arg(model);
        }
        command
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(fs::File::create(&stdout_path).map_err(|error| {
                IntelligenceExecutionError::new("workspace_error", error.to_string())
            })?)
            .stderr(fs::File::create(&stderr_path).map_err(|error| {
                IntelligenceExecutionError::new("workspace_error", error.to_string())
            })?);

        let started = Instant::now();
        let mut child = command.spawn().map_err(|error| {
            IntelligenceExecutionError::new("connection_failed", error.to_string())
        })?;
        child
            .stdin
            .take()
            .ok_or_else(|| {
                IntelligenceExecutionError::new(
                    "connection_failed",
                    "Provider stdin was unavailable",
                )
            })?
            .write_all(request.prompt.as_bytes())
            .map_err(|error| {
                IntelligenceExecutionError::new("connection_failed", error.to_string())
            })?;
        let status = loop {
            if cancellation.is_some_and(|flag| flag.load(Ordering::Acquire)) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(IntelligenceExecutionError::new(
                    "execution_cancelled",
                    request.cancellation_message,
                ));
            }
            let result_bytes = file_size(&result_path);
            let workspace_bytes = result_bytes
                .saturating_add(file_size(&stdout_path))
                .saturating_add(file_size(&stderr_path));
            if result_bytes > crate::resource_limits::MAX_PROVIDER_RESULT_BYTES {
                let _ = child.kill();
                let _ = child.wait();
                return Err(IntelligenceExecutionError::new(
                    "provider_output_too_large",
                    "Provider returned more than 1 MB",
                ));
            }
            if workspace_bytes > crate::resource_limits::MAX_PROVIDER_WORKSPACE_BYTES {
                let _ = child.kill();
                let _ = child.wait();
                return Err(IntelligenceExecutionError::new(
                    "provider_output_too_large",
                    "Provider generated more than 8 MB of output",
                ));
            }
            if let Some(status) = child.try_wait().map_err(|error| {
                IntelligenceExecutionError::new("connection_failed", error.to_string())
            })? {
                break status;
            }
            if started.elapsed()
                >= Duration::from_secs(crate::resource_limits::PROVIDER_EXECUTION_TIMEOUT_SECS)
            {
                let _ = child.kill();
                let _ = child.wait();
                return Err(IntelligenceExecutionError::new(
                    "connection_timeout",
                    "Provider did not finish within 90 seconds",
                ));
            }
            std::thread::sleep(Duration::from_millis(25));
        };
        let result_bytes = file_size(&result_path);
        let workspace_bytes = result_bytes
            .saturating_add(file_size(&stdout_path))
            .saturating_add(file_size(&stderr_path));
        if workspace_bytes > crate::resource_limits::MAX_PROVIDER_WORKSPACE_BYTES {
            return Err(IntelligenceExecutionError::new(
                "provider_output_too_large",
                "Provider generated more than 8 MB of output",
            ));
        }
        if result_bytes > crate::resource_limits::MAX_PROVIDER_RESULT_BYTES {
            return Err(IntelligenceExecutionError::new(
                "provider_output_too_large",
                "Provider returned more than 1 MB",
            ));
        }
        if !status.success() {
            let error = fs::read_to_string(&stderr_path).unwrap_or_default();
            return Err(IntelligenceExecutionError::new(
                "provider_failed",
                diagnostic_tail(&error, 1_600),
            ));
        }
        let output = fs::read_to_string(&result_path).map_err(|error| {
            IntelligenceExecutionError::new("invalid_provider_output", error.to_string())
        })?;
        Ok(ProviderResponse {
            output,
            duration_ms: started.elapsed().as_millis().min(i64::MAX as u128) as i64,
        })
    }
}

fn file_size(path: &Path) -> u64 {
    fs::metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0)
}

static CODEX_CLI: CodexCliAdapter = CodexCliAdapter;
static ADAPTERS: [&'static dyn IntelligenceProviderAdapter; 1] = [&CODEX_CLI];

pub fn supports_adapter_id(adapter_id: &str) -> bool {
    ADAPTERS.iter().any(|adapter| adapter.id() == adapter_id)
}

fn adapter_for(
    connection: &IntelligenceConnection,
) -> Option<&'static dyn IntelligenceProviderAdapter> {
    ADAPTERS
        .iter()
        .copied()
        .find(|adapter| adapter.supports(connection))
}

pub fn supports_connection(connection: &IntelligenceConnection) -> bool {
    adapter_id(connection).is_some()
}

pub fn adapter_id(connection: &IntelligenceConnection) -> Option<&'static str> {
    adapter_for(connection).map(IntelligenceProviderAdapter::id)
}

pub fn execute(
    connection: &IntelligenceConnection,
    request: ProviderRequest<'_>,
    cancellation: Option<&AtomicBool>,
) -> Result<ProviderResponse, IntelligenceExecutionError> {
    let adapter = adapter_for(connection).ok_or_else(|| {
        IntelligenceExecutionError::new(
            "connection_unavailable",
            "No provider adapter supports this connection",
        )
    })?;
    adapter.execute(connection, request, cancellation)
}

struct TemporaryWorkspace(PathBuf);

impl TemporaryWorkspace {
    fn create() -> Result<Self, IntelligenceExecutionError> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "pasted-intelligence-{}-{nonce}",
            std::process::id()
        ));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&path).map_err(|error| {
            IntelligenceExecutionError::new("workspace_error", error.to_string())
        })?;
        Ok(Self(path))
    }
}

impl Drop for TemporaryWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn diagnostic_tail(value: &str, max_chars: usize) -> String {
    let length = value.chars().count();
    value
        .chars()
        .skip(length.saturating_sub(max_chars))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn fake_codex_executable(body: &str) -> (PathBuf, PathBuf) {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("pasted-provider-test-{nonce}"));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&directory)
            .unwrap();
        let path = directory.join("codex-provider-test");
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&path, permissions).unwrap();
        (path, directory)
    }

    #[cfg(unix)]
    #[test]
    fn temporary_workspaces_are_private_and_removed_on_drop() {
        use std::os::unix::fs::PermissionsExt;

        let workspace = TemporaryWorkspace::create().unwrap();
        let path = workspace.0.clone();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700);

        drop(workspace);
        assert!(!path.exists());
    }

    #[test]
    fn adapter_registry_routes_only_supported_executables() {
        let connection = |endpoint: &str| IntelligenceConnection {
            id: endpoint.to_string(),
            name: endpoint.to_string(),
            provider_kind: "cli".to_string(),
            endpoint: Some(endpoint.to_string()),
            model: None,
            credential_ref: None,
            enabled: true,
            priority: 0,
            created_at: String::new(),
            updated_at: String::new(),
        };

        assert_eq!(
            adapter_for(&connection("/usr/local/bin/codex"))
                .unwrap()
                .id(),
            "codex_cli"
        );
        assert!(adapter_for(&connection("/usr/local/bin/claude")).is_none());
        assert!(adapter_for(&connection("/usr/local/bin/ollama")).is_none());
    }

    #[test]
    fn oversized_provider_prompts_fail_before_launching_a_process() {
        let connection = IntelligenceConnection {
            id: "oversized".to_string(),
            name: "Oversized".to_string(),
            provider_kind: "cli".to_string(),
            endpoint: Some("/definitely/not/a/real/codex".to_string()),
            model: None,
            credential_ref: None,
            enabled: true,
            priority: 0,
            created_at: String::new(),
            updated_at: String::new(),
        };
        let prompt = "x".repeat(crate::resource_limits::MAX_PROVIDER_PROMPT_BYTES + 1);
        let error = execute(
            &connection,
            ProviderRequest {
                prompt: &prompt,
                output_schema: None,
                cancellation_message: "cancelled",
            },
            None,
        )
        .unwrap_err();

        assert_eq!(error.code, "provider_input_too_large");
    }

    #[test]
    fn adapter_registry_reports_discovery_support_by_stable_id() {
        assert!(supports_adapter_id("codex_cli"));
        assert!(!supports_adapter_id("claude_cli"));
        assert!(!supports_adapter_id("ollama"));
    }

    #[test]
    fn cancelled_requests_never_launch_the_provider() {
        let connection = IntelligenceConnection {
            id: "cancelled".to_string(),
            name: "Codex CLI".to_string(),
            provider_kind: "cli".to_string(),
            endpoint: Some("/definitely/missing/codex".to_string()),
            model: None,
            credential_ref: None,
            enabled: true,
            priority: 0,
            created_at: String::new(),
            updated_at: String::new(),
        };
        let cancellation = AtomicBool::new(true);
        let error = execute(
            &connection,
            ProviderRequest {
                prompt: "Never run this",
                output_schema: None,
                cancellation_message: "Cancelled before launch",
            },
            Some(&cancellation),
        )
        .unwrap_err();

        assert_eq!(error.code, "execution_cancelled");
        assert_eq!(error.message, "Cancelled before launch");
    }

    #[test]
    fn unsupported_connections_fail_before_execution() {
        let connection = IntelligenceConnection {
            id: "unsupported".to_string(),
            name: "Claude CLI".to_string(),
            provider_kind: "cli".to_string(),
            endpoint: Some("/usr/local/bin/claude".to_string()),
            model: None,
            credential_ref: None,
            enabled: true,
            priority: 0,
            created_at: String::new(),
            updated_at: String::new(),
        };
        let error = execute(
            &connection,
            ProviderRequest {
                prompt: "Do not run this",
                output_schema: None,
                cancellation_message: "Cancelled",
            },
            None,
        )
        .unwrap_err();

        assert_eq!(error.code, "connection_unavailable");
    }

    #[cfg(unix)]
    #[test]
    fn running_provider_processes_can_be_cancelled() {
        let (executable, directory) =
            fake_codex_executable("cat >/dev/null\ntouch \"$0.started\"\nsleep 5");
        let marker = PathBuf::from(format!("{}.started", executable.display()));
        let connection = IntelligenceConnection {
            id: "running".to_string(),
            name: "Codex CLI".to_string(),
            provider_kind: "cli".to_string(),
            endpoint: Some(executable.to_string_lossy().into_owned()),
            model: None,
            credential_ref: None,
            enabled: true,
            priority: 0,
            created_at: String::new(),
            updated_at: String::new(),
        };
        let cancellation = std::sync::Arc::new(AtomicBool::new(false));
        let execution_cancellation = std::sync::Arc::clone(&cancellation);
        let started = Instant::now();
        let execution = std::thread::spawn(move || {
            execute(
                &connection,
                ProviderRequest {
                    prompt: "Begin",
                    output_schema: None,
                    cancellation_message: "Cancelled while running",
                },
                Some(execution_cancellation.as_ref()),
            )
        });
        let marker_deadline = Instant::now() + Duration::from_secs(2);
        while !marker.exists() {
            assert!(
                Instant::now() < marker_deadline,
                "provider did not start in time"
            );
            std::thread::yield_now();
        }
        cancellation.store(true, Ordering::Release);
        let error = execution.join().unwrap().unwrap_err();

        assert_eq!(error.code, "execution_cancelled");
        assert!(started.elapsed() < Duration::from_secs(2));
        let _ = fs::remove_dir_all(directory);
    }

    #[cfg(unix)]
    #[test]
    fn provider_log_floods_are_rejected_even_when_the_process_exits_quickly() {
        let (executable, directory) =
            fake_codex_executable("cat >/dev/null\nhead -c 9000000 /dev/zero >&2\nexit 1");
        let connection = IntelligenceConnection {
            id: "log-flood".to_string(),
            name: "Codex CLI".to_string(),
            provider_kind: "cli".to_string(),
            endpoint: Some(executable.to_string_lossy().into_owned()),
            model: None,
            credential_ref: None,
            enabled: true,
            priority: 0,
            created_at: String::new(),
            updated_at: String::new(),
        };
        let error = execute(
            &connection,
            ProviderRequest {
                prompt: "Begin",
                output_schema: None,
                cancellation_message: "Cancelled",
            },
            None,
        )
        .unwrap_err();

        assert_eq!(error.code, "provider_output_too_large");
        let _ = fs::remove_dir_all(directory);
    }
}
