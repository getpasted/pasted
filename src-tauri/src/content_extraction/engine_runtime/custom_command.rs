use super::*;

pub(super) struct CustomCommandEngine;

impl ExtractorEngine for CustomCommandEngine {
    fn id(&self) -> &'static str {
        CUSTOM_COMMAND_ENGINE
    }

    fn availability(&self) -> EngineAvailability {
        EngineAvailability {
            is_available: false,
            unavailable_reason: Some("A custom executable is not configured.".into()),
        }
    }

    fn availability_with_configuration(
        &self,
        executable_path: Option<&Path>,
        _model_path: Option<&Path>,
    ) -> EngineAvailability {
        executable_availability(
            executable_path
                .filter(|path| crate::external_tools::is_executable(path))
                .map(Path::to_path_buf),
            "A custom executable is not configured or cannot be run.",
        )
    }

    fn extract(&self, _image_bytes: &[u8]) -> ExtractionOutcome {
        extraction_failure(
            "engine_unavailable",
            "A custom executable is not configured.",
        )
    }

    fn extract_with_configuration(
        &self,
        image_bytes: &[u8],
        executable_path: Option<&Path>,
        _model_path: Option<&Path>,
    ) -> ExtractionOutcome {
        let Some(executable) = executable_path else {
            return self.extract(image_bytes);
        };
        execute_custom_command(executable, CustomCommandInput::Image { bytes: image_bytes })
    }

    fn extract_files_with_configuration(
        &self,
        paths: &[String],
        executable_path: Option<&Path>,
        _model_path: Option<&Path>,
    ) -> ExtractionOutcome {
        let Some(executable) = executable_path else {
            return extraction_failure(
                "engine_unavailable",
                "A custom executable is not configured.",
            );
        };
        execute_custom_command(executable, CustomCommandInput::Files { paths })
    }
}

enum CustomCommandInput<'a> {
    Image { bytes: &'a [u8] },
    Files { paths: &'a [String] },
}

fn execute_custom_command(executable: &Path, input: CustomCommandInput<'_>) -> ExtractionOutcome {
    if !crate::external_tools::is_executable(executable) {
        return extraction_failure(
            "engine_unavailable",
            "The configured custom executable cannot be run.",
        );
    }
    let workspace = match crate::external_tools::PrivateWorkspace::create("custom-extractor") {
        Ok(workspace) => workspace,
        Err(_) => {
            return extraction_failure(
                "workspace_error",
                "A private custom extraction workspace could not be created.",
            );
        }
    };
    let request_path = workspace.join("request.json");
    let response_path = workspace.join("response.json");
    let request = match input {
        CustomCommandInput::Image { bytes } => serde_json::json!({
            "protocolVersion": 1,
            "input": {
                "kind": "image",
                "dataBase64": base64::engine::general_purpose::STANDARD.encode(bytes),
            }
        }),
        CustomCommandInput::Files { paths } => serde_json::json!({
            "protocolVersion": 1,
            "input": {
                "kind": "file_references",
                "paths": paths.iter().take(crate::resource_limits::MAX_MEDIA_PROBE_FILES).collect::<Vec<_>>(),
            }
        }),
    };
    let Ok(request) = serde_json::to_vec(&request) else {
        return extraction_failure(
            "invalid_input",
            "Custom extraction input could not be encoded.",
        );
    };
    if fs::write(&request_path, request).is_err() {
        return extraction_failure(
            "workspace_error",
            "Custom extraction input could not be staged.",
        );
    }
    let response = match fs::File::create(&response_path) {
        Ok(response) => response,
        Err(_) => {
            return extraction_failure(
                "workspace_error",
                "Custom extraction output could not be staged.",
            );
        }
    };
    let mut command = Command::new(executable);
    command
        .arg("--pasted-extract-v1")
        .arg(&request_path)
        .current_dir(workspace.join("."))
        .env_clear()
        .stdin(Stdio::null())
        .stdout(response)
        .stderr(Stdio::null());
    for name in ["PATH", "LANG", "LC_ALL", "SystemRoot", "WINDIR"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => {
            return extraction_failure(
                "engine_unavailable",
                "The custom executable could not be started.",
            );
        }
    };
    let status = match crate::external_tools::wait_bounded(&mut child, Duration::from_secs(60)) {
        Ok(status) => status,
        Err(crate::external_tools::ProcessWaitError::TimedOut) => {
            return extraction_failure(
                "engine_timeout",
                "The custom Extractor exceeded the 60-second time limit.",
            );
        }
        Err(crate::external_tools::ProcessWaitError::Failed) => {
            return extraction_failure(
                "engine_failed",
                "The custom Extractor did not complete successfully.",
            );
        }
    };
    if !status.success() {
        return extraction_failure(
            "engine_failed",
            "The custom Extractor did not complete successfully.",
        );
    }
    let Ok(metadata) = response_path.metadata() else {
        return ExtractionOutcome::NoOutput;
    };
    if metadata.len() > crate::resource_limits::MAX_OCR_TEXT_BYTES as u64 + 4_096 {
        return extraction_failure(
            "output_too_large",
            "Custom Extractor output exceeds the supported size limit.",
        );
    }
    let Ok(response) = fs::read_to_string(&response_path) else {
        return extraction_failure(
            "invalid_output",
            "The custom Extractor returned unreadable output.",
        );
    };
    let Ok(response) = serde_json::from_str::<serde_json::Value>(&response) else {
        return extraction_failure(
            "invalid_output",
            "The custom Extractor must return a JSON object.",
        );
    };
    match response.get("text") {
        Some(serde_json::Value::String(text)) => ExtractionOutcome::Produced { text: text.clone() },
        Some(serde_json::Value::Null) | None => ExtractionOutcome::NoOutput,
        _ => extraction_failure(
            "invalid_output",
            "Custom Extractor output requires a string or null text field.",
        ),
    }
}
