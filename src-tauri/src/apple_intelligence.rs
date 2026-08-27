#[cfg(target_os = "macos")]
use std::ffi::{CStr, CString};
#[cfg(target_os = "macos")]
use std::io::{Read, Write};
#[cfg(target_os = "macos")]
use std::process::{Command, Stdio};
#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "macos")]
use std::time::{Duration, Instant};

pub const HELPER_MARKER: &str = "--pasted-apple-intelligence-provider-v1";
pub const CONNECTION_ENDPOINT: &str = "pasted://apple-foundation-models";
pub const ADAPTER_ID: &str = "apple_foundation_models";

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn pasted_apple_intelligence_request(
        request: *const std::os::raw::c_char,
    ) -> *mut std::os::raw::c_char;
    fn pasted_apple_intelligence_free(response: *mut std::os::raw::c_char);
}

#[cfg(target_os = "macos")]
fn request(value: &serde_json::Value) -> Result<serde_json::Value, String> {
    let encoded = serde_json::to_string(value).map_err(|error| error.to_string())?;
    let encoded = CString::new(encoded).map_err(|error| error.to_string())?;
    let response = unsafe { pasted_apple_intelligence_request(encoded.as_ptr()) };
    if response.is_null() {
        return Err("Apple Intelligence returned no response".into());
    }
    let decoded = unsafe { CStr::from_ptr(response) }
        .to_string_lossy()
        .into_owned();
    unsafe { pasted_apple_intelligence_free(response) };
    serde_json::from_str(&decoded).map_err(|error| error.to_string())
}

pub fn run_helper(arguments: &[String]) -> Option<i32> {
    if !arguments.iter().any(|argument| argument == HELPER_MARKER) {
        return None;
    }
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("Apple Intelligence is available only on macOS.");
        Some(2)
    }
    #[cfg(target_os = "macos")]
    {
        let mut input = String::new();
        let limit = crate::resource_limits::MAX_PROVIDER_REQUEST_BYTES;
        if std::io::stdin()
            .take(limit as u64 + 1)
            .read_to_string(&mut input)
            .is_err()
            || input.len() > limit
        {
            eprintln!("Apple Intelligence request exceeds Pasted's safety limit.");
            return Some(2);
        }
        let value = serde_json::from_str(&input).unwrap_or_else(|_| serde_json::json!({}));
        match request(&value) {
            Ok(response) => {
                println!("{response}");
                Some(0)
            }
            Err(error) => {
                eprintln!("{error}");
                Some(1)
            }
        }
    }
}

#[derive(Debug)]
pub struct ProbeResult {
    pub available: bool,
    pub version: String,
}

#[cfg(target_os = "macos")]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct BridgeResponse {
    ok: bool,
    output: Option<String>,
    code: Option<String>,
    message: Option<String>,
    version: Option<String>,
}

#[cfg(target_os = "macos")]
fn run_child(
    payload: &[u8],
    timeout: Duration,
    cancellation: Option<&AtomicBool>,
) -> Result<(BridgeResponse, i64), crate::intelligence_provider::IntelligenceExecutionError> {
    use crate::intelligence_provider::IntelligenceExecutionError;

    let executable = std::env::current_exe()
        .map_err(|error| IntelligenceExecutionError::new("connection_failed", error.to_string()))?;
    let mut child = Command::new(executable)
        .arg(HELPER_MARKER)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| IntelligenceExecutionError::new("connection_failed", error.to_string()))?;
    child
        .stdin
        .take()
        .ok_or_else(|| {
            IntelligenceExecutionError::new("connection_failed", "Provider stdin was unavailable")
        })?
        .write_all(payload)
        .map_err(|error| IntelligenceExecutionError::new("connection_failed", error.to_string()))?;
    let stdout = child.stdout.take().expect("piped provider stdout exists");
    let stderr = child.stderr.take().expect("piped provider stderr exists");
    let output_limit = crate::resource_limits::MAX_PROVIDER_RESULT_BYTES + 1;
    let diagnostic_limit = crate::resource_limits::MAX_PROVIDER_WORKSPACE_BYTES + 1;
    let stdout_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.take(output_limit).read_to_end(&mut bytes);
        bytes
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stderr.take(diagnostic_limit).read_to_end(&mut bytes);
        bytes
    });
    let started = Instant::now();
    let status = loop {
        if cancellation.is_some_and(|flag| flag.load(Ordering::Acquire)) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(IntelligenceExecutionError::new(
                "execution_cancelled",
                "Apple Intelligence request was cancelled",
            ));
        }
        if let Some(status) = child.try_wait().map_err(|error| {
            IntelligenceExecutionError::new("connection_failed", error.to_string())
        })? {
            break status;
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(IntelligenceExecutionError::new(
                "connection_timeout",
                "Apple Intelligence did not finish in time",
            ));
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    let stdout = stdout_reader.join().unwrap_or_default();
    let stderr = stderr_reader.join().unwrap_or_default();
    if stdout.len() as u64 > crate::resource_limits::MAX_PROVIDER_RESULT_BYTES
        || stderr.len() as u64 > crate::resource_limits::MAX_PROVIDER_WORKSPACE_BYTES
    {
        return Err(IntelligenceExecutionError::new(
            "provider_output_too_large",
            "Apple Intelligence generated too much output",
        ));
    }
    if !status.success() {
        let message = String::from_utf8_lossy(&stderr)
            .trim()
            .chars()
            .take(600)
            .collect::<String>();
        return Err(IntelligenceExecutionError::new(
            "provider_failed",
            if message.is_empty() {
                "Apple Intelligence failed".into()
            } else {
                message
            },
        ));
    }
    let response = serde_json::from_slice::<BridgeResponse>(&stdout).map_err(|error| {
        IntelligenceExecutionError::new("invalid_provider_output", error.to_string())
    })?;
    Ok((
        response,
        started.elapsed().as_millis().min(i64::MAX as u128) as i64,
    ))
}

#[cfg(target_os = "macos")]
pub fn probe() -> Option<ProbeResult> {
    let payload = serde_json::to_vec(&serde_json::json!({ "action": "probe" })).ok()?;
    let (response, _) = run_child(&payload, Duration::from_secs(5), None).ok()?;
    Some(ProbeResult {
        available: response.ok,
        version: response
            .version
            .unwrap_or_else(|| "SystemLanguageModel".into()),
    })
}

#[cfg(not(target_os = "macos"))]
pub fn probe() -> Option<ProbeResult> {
    None
}

#[cfg(target_os = "macos")]
pub fn execute(
    request: crate::intelligence_provider::ProviderRequest<'_>,
    cancellation: Option<&AtomicBool>,
) -> Result<
    crate::intelligence_provider::ProviderResponse,
    crate::intelligence_provider::IntelligenceExecutionError,
> {
    use crate::intelligence_provider::{IntelligenceExecutionError, ProviderResponse};

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
    let payload = serde_json::to_vec(&serde_json::json!({
        "prompt": request.prompt,
        "outputSchema": request.output_schema,
    }))
    .map_err(|error| IntelligenceExecutionError::new("invalid_plan_schema", error.to_string()))?;
    if payload.len() > crate::resource_limits::MAX_PROVIDER_REQUEST_BYTES {
        return Err(IntelligenceExecutionError::new(
            "provider_input_too_large",
            "Provider request exceeds Pasted's safety limit",
        ));
    }
    let child_result = run_child(
        &payload,
        Duration::from_secs(crate::resource_limits::PROVIDER_EXECUTION_TIMEOUT_SECS),
        cancellation,
    );
    let (response, duration_ms) = match child_result {
        Err(error) if error.code == "execution_cancelled" => {
            return Err(IntelligenceExecutionError::new(
                "execution_cancelled",
                request.cancellation_message,
            ))
        }
        result => result?,
    };
    if !response.ok {
        let code = match response.code.as_deref() {
            Some("model_not_ready") => "model_not_ready",
            Some("apple_intelligence_not_enabled") => "apple_intelligence_not_enabled",
            Some("device_not_eligible") => "device_not_eligible",
            Some("invalid_request") => "invalid_provider_request",
            Some("invalid_schema") => "invalid_plan_schema",
            _ => "provider_failed",
        };
        return Err(IntelligenceExecutionError::new(
            code,
            response
                .message
                .unwrap_or_else(|| "Apple Intelligence failed".into()),
        ));
    }
    let output = response.output.ok_or_else(|| {
        IntelligenceExecutionError::new(
            "invalid_provider_output",
            "Apple Intelligence returned no output",
        )
    })?;
    if output.len() as u64 > crate::resource_limits::MAX_PROVIDER_RESULT_BYTES {
        return Err(IntelligenceExecutionError::new(
            "provider_output_too_large",
            "Apple Intelligence returned more than 1 MB",
        ));
    }
    Ok(ProviderResponse {
        output,
        duration_ms,
    })
}
