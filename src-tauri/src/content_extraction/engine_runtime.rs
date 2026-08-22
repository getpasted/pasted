use super::*;

struct AppleVisionOcrEngine;
struct TesseractOcrEngine;
struct WhisperCppEngine;
struct CustomCommandEngine;

impl ExtractorEngine for AppleVisionOcrEngine {
    fn id(&self) -> &'static str {
        APPLE_VISION_ENGINE
    }

    fn availability(&self) -> EngineAvailability {
        if cfg!(target_os = "macos") {
            EngineAvailability {
                is_available: true,
                unavailable_reason: None,
            }
        } else {
            EngineAvailability {
                is_available: false,
                unavailable_reason: Some("Apple Vision is available only on macOS.".into()),
            }
        }
    }

    fn extract(&self, image_bytes: &[u8]) -> ExtractionOutcome {
        perform_apple_vision_ocr(image_bytes)
            .filter(|text| !text.trim().is_empty())
            .map_or(ExtractionOutcome::NoOutput, |text| {
                ExtractionOutcome::Produced { text }
            })
    }
}

impl ExtractorEngine for TesseractOcrEngine {
    fn id(&self) -> &'static str {
        TESSERACT_ENGINE
    }

    fn availability(&self) -> EngineAvailability {
        if find_tesseract_executable().is_some() {
            EngineAvailability {
                is_available: true,
                unavailable_reason: None,
            }
        } else {
            EngineAvailability {
                is_available: false,
                unavailable_reason: Some(
                    "Tesseract OCR is not installed. Install Tesseract 5, then check again.".into(),
                ),
            }
        }
    }

    fn extract(&self, image_bytes: &[u8]) -> ExtractionOutcome {
        let Some(executable) = find_tesseract_executable() else {
            return ExtractionOutcome::Failed {
                failure: ExtractionFailure {
                    code: "engine_unavailable".into(),
                    message: "Tesseract OCR is not installed.".into(),
                },
            };
        };
        perform_tesseract_ocr(&executable, image_bytes, TESSERACT_TIMEOUT)
    }

    fn availability_with_configuration(
        &self,
        executable_path: Option<&Path>,
        _model_path: Option<&Path>,
    ) -> EngineAvailability {
        executable_availability(
            configured_or_discovered_executable(executable_path, find_tesseract_executable),
            "Tesseract OCR is not installed. Install Tesseract 5, then check again.",
        )
    }

    fn extract_with_configuration(
        &self,
        image_bytes: &[u8],
        executable_path: Option<&Path>,
        _model_path: Option<&Path>,
    ) -> ExtractionOutcome {
        let Some(executable) =
            configured_or_discovered_executable(executable_path, find_tesseract_executable)
        else {
            return extraction_failure("engine_unavailable", "Tesseract OCR is not installed.");
        };
        perform_tesseract_ocr(&executable, image_bytes, TESSERACT_TIMEOUT)
    }
}

impl ExtractorEngine for WhisperCppEngine {
    fn id(&self) -> &'static str {
        WHISPER_CPP_ENGINE
    }

    fn availability(&self) -> EngineAvailability {
        self.availability_with_model(None)
    }

    fn availability_with_model(&self, model_path: Option<&Path>) -> EngineAvailability {
        if find_whisper_cpp_executable().is_none() {
            return EngineAvailability {
                is_available: false,
                unavailable_reason: Some(
                    "Whisper.cpp is not installed. Install whisper-cpp, then check again.".into(),
                ),
            };
        }
        let Some(model_path) = model_path else {
            return EngineAvailability {
                is_available: false,
                unavailable_reason: Some("A local Whisper GGML model is not configured.".into()),
            };
        };
        if !model_path.is_file() {
            return EngineAvailability {
                is_available: false,
                unavailable_reason: Some("The configured Whisper model is unavailable.".into()),
            };
        }
        EngineAvailability {
            is_available: true,
            unavailable_reason: None,
        }
    }

    fn extract(&self, _image_bytes: &[u8]) -> ExtractionOutcome {
        ExtractionOutcome::Failed {
            failure: ExtractionFailure {
                code: "invalid_contract".into(),
                message: "Whisper transcription requires audio file references.".into(),
            },
        }
    }

    fn extract_files(&self, paths: &[String], model_path: Option<&Path>) -> ExtractionOutcome {
        let Some(executable) = find_whisper_cpp_executable() else {
            return ExtractionOutcome::Failed {
                failure: ExtractionFailure {
                    code: "engine_unavailable".into(),
                    message: "Whisper.cpp is not installed.".into(),
                },
            };
        };
        let Some(model_path) = model_path else {
            return ExtractionOutcome::Failed {
                failure: ExtractionFailure {
                    code: "engine_unavailable".into(),
                    message: "A local Whisper GGML model is not configured.".into(),
                },
            };
        };
        perform_whisper_cpp_transcription(&executable, model_path, paths, WHISPER_TIMEOUT)
    }

    fn availability_with_configuration(
        &self,
        executable_path: Option<&Path>,
        model_path: Option<&Path>,
    ) -> EngineAvailability {
        if configured_or_discovered_executable(executable_path, find_whisper_cpp_executable)
            .is_none()
        {
            return EngineAvailability {
                is_available: false,
                unavailable_reason: Some(
                    "Whisper.cpp is not installed. Install whisper-cpp, then check again.".into(),
                ),
            };
        }
        whisper_model_availability(model_path)
    }

    fn extract_files_with_configuration(
        &self,
        paths: &[String],
        executable_path: Option<&Path>,
        model_path: Option<&Path>,
    ) -> ExtractionOutcome {
        let Some(executable) =
            configured_or_discovered_executable(executable_path, find_whisper_cpp_executable)
        else {
            return extraction_failure("engine_unavailable", "Whisper.cpp is not installed.");
        };
        let Some(model_path) = model_path else {
            return extraction_failure(
                "engine_unavailable",
                "A local Whisper GGML model is not configured.",
            );
        };
        perform_whisper_cpp_transcription(&executable, model_path, paths, WHISPER_TIMEOUT)
    }
}

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

static APPLE_VISION_OCR_ENGINE: AppleVisionOcrEngine = AppleVisionOcrEngine;
static TESSERACT_OCR_ENGINE: TesseractOcrEngine = TesseractOcrEngine;
static WHISPER_CPP_ENGINE_IMPLEMENTATION: WhisperCppEngine = WhisperCppEngine;
static CUSTOM_COMMAND_ENGINE_IMPLEMENTATION: CustomCommandEngine = CustomCommandEngine;
static SYSTEM_ENGINES: [&dyn ExtractorEngine; 4] = [
    &APPLE_VISION_OCR_ENGINE,
    &TESSERACT_OCR_ENGINE,
    &WHISPER_CPP_ENGINE_IMPLEMENTATION,
    &CUSTOM_COMMAND_ENGINE_IMPLEMENTATION,
];

pub(super) fn system_engine_registry() -> ExtractorEngineRegistry<'static> {
    ExtractorEngineRegistry::new(&SYSTEM_ENGINES)
}

fn configured_or_discovered_executable(
    configured: Option<&Path>,
    discover: impl FnOnce() -> Option<std::path::PathBuf>,
) -> Option<std::path::PathBuf> {
    match configured {
        Some(path) if crate::external_tools::is_executable(path) => Some(path.to_path_buf()),
        Some(_) => None,
        None => discover(),
    }
}

fn executable_availability(
    executable: Option<std::path::PathBuf>,
    unavailable_reason: &str,
) -> EngineAvailability {
    EngineAvailability {
        is_available: executable.is_some(),
        unavailable_reason: executable.is_none().then(|| unavailable_reason.into()),
    }
}

fn whisper_model_availability(model_path: Option<&Path>) -> EngineAvailability {
    let Some(model_path) = model_path else {
        return EngineAvailability {
            is_available: false,
            unavailable_reason: Some("A local Whisper GGML model is not configured.".into()),
        };
    };
    if !model_path.is_file() {
        return EngineAvailability {
            is_available: false,
            unavailable_reason: Some("The configured Whisper model is unavailable.".into()),
        };
    }
    EngineAvailability {
        is_available: true,
        unavailable_reason: None,
    }
}

fn runtime_dependency(
    name: &str,
    path: Option<std::path::PathBuf>,
    version_arguments: &[&str],
    unavailable_reason: &str,
) -> ExtractorRuntimeDependency {
    let is_available = path.is_some();
    let version = path
        .as_deref()
        .and_then(|path| crate::external_tools::probe_version(path, version_arguments));
    ExtractorRuntimeDependency {
        name: name.into(),
        location: path
            .as_deref()
            .map(|path| path.to_string_lossy().into_owned()),
        version,
        is_available,
        unavailable_reason: (!is_available).then(|| unavailable_reason.into()),
    }
}

pub fn runtime_status_for(engine: &str, executable_path: Option<&str>) -> ExtractorRuntimeStatus {
    let configured = executable_path.map(Path::new);
    match engine {
        APPLE_VISION_ENGINE => ExtractorRuntimeStatus {
            method: "system".into(),
            location: Some("macOS Vision framework".into()),
            version: apple_vision_runtime_version(),
            uses_automatic_discovery: false,
            dependencies: Vec::new(),
        },
        TESSERACT_ENGINE => {
            let path = configured_or_discovered_executable(configured, find_tesseract_executable);
            let version = path
                .as_deref()
                .and_then(|path| crate::external_tools::probe_version(path, &["--version"]));
            ExtractorRuntimeStatus {
                method: "command".into(),
                location: path
                    .as_deref()
                    .map(|path| path.to_string_lossy().into_owned()),
                version,
                uses_automatic_discovery: configured.is_none(),
                dependencies: Vec::new(),
            }
        }
        WHISPER_CPP_ENGINE => {
            let path = configured_or_discovered_executable(configured, find_whisper_cpp_executable);
            let version = path
                .as_deref()
                .and_then(|path| crate::external_tools::probe_version(path, &["--version"]));
            ExtractorRuntimeStatus {
                method: "command".into(),
                location: path
                    .as_deref()
                    .map(|path| path.to_string_lossy().into_owned()),
                version,
                uses_automatic_discovery: configured.is_none(),
                dependencies: vec![runtime_dependency(
                    "FFmpeg",
                    find_ffmpeg_executable(),
                    &["-version"],
                    "FFmpeg is not installed. M4A and AAC audio cannot be prepared.",
                )],
            }
        }
        CUSTOM_COMMAND_ENGINE => {
            let path = configured.filter(|path| crate::external_tools::is_executable(path));
            ExtractorRuntimeStatus {
                method: "command".into(),
                location: path.map(|path| path.to_string_lossy().into_owned()),
                version: path
                    .and_then(|path| crate::external_tools::probe_version(path, &["--version"])),
                uses_automatic_discovery: false,
                dependencies: Vec::new(),
            }
        }
        _ => ExtractorRuntimeStatus {
            method: "unregistered".into(),
            location: executable_path.map(str::to_string),
            version: None,
            uses_automatic_discovery: false,
            dependencies: Vec::new(),
        },
    }
}

fn apple_vision_runtime_version() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        crate::external_tools::probe_version(Path::new("/usr/bin/sw_vers"), &["-productVersion"])
            .map(|version| format!("macOS {version}"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
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

pub(super) fn find_tesseract_executable() -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    let (name, explicit) = (
        "tesseract.exe",
        &[
            r"C:\Program Files\Tesseract-OCR\tesseract.exe",
            r"C:\Program Files (x86)\Tesseract-OCR\tesseract.exe",
        ][..],
    );
    #[cfg(not(windows))]
    let (name, explicit) = (
        "tesseract",
        &[
            "/opt/homebrew/bin/tesseract",
            "/usr/local/bin/tesseract",
            "/usr/bin/tesseract",
            "/home/linuxbrew/.linuxbrew/bin/tesseract",
        ][..],
    );

    crate::external_tools::find_executable(name, explicit)
}

fn find_whisper_cpp_executable() -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    let (name, explicit) = (
        "whisper-cli.exe",
        &[
            r"C:\Program Files\whisper.cpp\whisper-cli.exe",
            r"C:\whisper.cpp\whisper-cli.exe",
        ][..],
    );
    #[cfg(not(windows))]
    let (name, explicit) = (
        "whisper-cli",
        &[
            "/opt/homebrew/bin/whisper-cli",
            "/usr/local/bin/whisper-cli",
            "/usr/bin/whisper-cli",
            "/home/linuxbrew/.linuxbrew/bin/whisper-cli",
        ][..],
    );
    crate::external_tools::find_executable(name, explicit)
}

pub(super) fn find_ffmpeg_executable() -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    let (name, explicit) = (
        "ffmpeg.exe",
        &[
            r"C:\Program Files\ffmpeg\bin\ffmpeg.exe",
            r"C:\ffmpeg\bin\ffmpeg.exe",
        ][..],
    );
    #[cfg(not(windows))]
    let (name, explicit) = (
        "ffmpeg",
        &[
            "/opt/homebrew/bin/ffmpeg",
            "/usr/local/bin/ffmpeg",
            "/usr/bin/ffmpeg",
            "/home/linuxbrew/.linuxbrew/bin/ffmpeg",
        ][..],
    );
    crate::external_tools::find_executable(name, explicit)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WhisperAudioPreparation {
    Native,
    FfmpegWav,
}

pub(super) fn whisper_audio_preparation(path: &Path) -> Option<WhisperAudioPreparation> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())?
        .to_ascii_lowercase()
        .as_str()
    {
        "flac" | "mp3" | "ogg" | "wav" => Some(WhisperAudioPreparation::Native),
        "aac" | "m4a" => Some(WhisperAudioPreparation::FfmpegWav),
        _ => None,
    }
}

fn extraction_failure(code: &str, message: &str) -> ExtractionOutcome {
    ExtractionOutcome::Failed {
        failure: ExtractionFailure {
            code: code.into(),
            message: message.into(),
        },
    }
}

pub(super) fn prepare_whisper_audio<'a>(
    audio_path: &'a Path,
    preparation: WhisperAudioPreparation,
    workspace: &crate::external_tools::PrivateWorkspace,
    index: usize,
    remaining: Duration,
) -> Result<std::borrow::Cow<'a, Path>, ExtractionOutcome> {
    if preparation == WhisperAudioPreparation::Native {
        return Ok(std::borrow::Cow::Borrowed(audio_path));
    }
    if audio_path.metadata().is_ok_and(|metadata| {
        metadata.len() > crate::resource_limits::MAX_TRANSCRIPTION_AUDIO_BYTES
    }) {
        return Err(extraction_failure(
            "input_too_large",
            "The audio file exceeds the transcription size limit.",
        ));
    }
    let Some(ffmpeg) = find_ffmpeg_executable() else {
        return Err(extraction_failure(
            "preparation_unavailable",
            "FFmpeg is required to transcribe M4A or AAC audio.",
        ));
    };
    let prepared_path = workspace.join(format!("prepared-{index}.wav"));
    let mut child = Command::new(ffmpeg)
        .arg("-nostdin")
        .arg("-v")
        .arg("error")
        .arg("-y")
        .arg("-i")
        .arg(audio_path)
        .arg("-map")
        .arg("0:a:0")
        .arg("-vn")
        .arg("-ac")
        .arg("1")
        .arg("-ar")
        .arg("16000")
        .arg("-c:a")
        .arg("pcm_s16le")
        .arg("-fs")
        .arg(crate::resource_limits::MAX_TRANSCRIPTION_AUDIO_BYTES.to_string())
        .arg(&prepared_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| {
            extraction_failure(
                "preparation_unavailable",
                "FFmpeg could not be started to prepare the audio.",
            )
        })?;
    if remaining.is_zero() {
        let _ = child.kill();
        let _ = child.wait();
        return Err(extraction_failure(
            "engine_timeout",
            "Audio preparation exceeded the transcription time limit.",
        ));
    }
    let status =
        crate::external_tools::wait_bounded(&mut child, remaining).map_err(
            |error| match error {
                crate::external_tools::ProcessWaitError::TimedOut => extraction_failure(
                    "engine_timeout",
                    "Audio preparation exceeded the transcription time limit.",
                ),
                crate::external_tools::ProcessWaitError::Failed => extraction_failure(
                    "preparation_failed",
                    "The audio file could not be prepared for transcription.",
                ),
            },
        )?;
    if !status.success() {
        return Err(extraction_failure(
            "preparation_failed",
            "The audio file could not be prepared for transcription.",
        ));
    }
    let Ok(metadata) = prepared_path.metadata() else {
        return Err(extraction_failure(
            "preparation_failed",
            "The audio file could not be prepared for transcription.",
        ));
    };
    if metadata.len() >= crate::resource_limits::MAX_TRANSCRIPTION_AUDIO_BYTES {
        return Err(extraction_failure(
            "input_too_large",
            "The prepared audio exceeds the transcription size limit.",
        ));
    }
    Ok(std::borrow::Cow::Owned(prepared_path))
}

fn spawn_whisper_cpp(
    executable: &Path,
    model_path: &Path,
    audio_path: &Path,
    output_base: &Path,
    disable_gpu: bool,
) -> std::io::Result<std::process::Child> {
    let mut command = Command::new(executable);
    if disable_gpu {
        command.arg("-ng");
    }
    command
        .arg("-m")
        .arg(model_path)
        .arg("-f")
        .arg(audio_path)
        .arg("-otxt")
        .arg("-of")
        .arg(output_base)
        .arg("-np")
        .arg("-nt")
        .arg("-l")
        .arg("auto")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}

fn wait_for_whisper(
    child: &mut std::process::Child,
    remaining: Duration,
) -> Result<std::process::ExitStatus, ExtractionOutcome> {
    if remaining.is_zero() {
        let _ = child.kill();
        let _ = child.wait();
        return Err(extraction_failure(
            "engine_timeout",
            "Whisper.cpp exceeded the transcription time limit.",
        ));
    }
    crate::external_tools::wait_bounded(child, remaining).map_err(|error| match error {
        crate::external_tools::ProcessWaitError::TimedOut => extraction_failure(
            "engine_timeout",
            "Whisper.cpp exceeded the transcription time limit.",
        ),
        crate::external_tools::ProcessWaitError::Failed => extraction_failure(
            "engine_failed",
            "Whisper.cpp did not complete successfully.",
        ),
    })
}

pub(super) fn perform_whisper_cpp_transcription(
    executable: &Path,
    model_path: &Path,
    paths: &[String],
    timeout: Duration,
) -> ExtractionOutcome {
    let audio_paths = paths
        .iter()
        .map(Path::new)
        .filter(|path| path.is_file())
        .filter_map(|path| whisper_audio_preparation(path).map(|preparation| (path, preparation)))
        .take(crate::resource_limits::MAX_MEDIA_PROBE_FILES)
        .collect::<Vec<_>>();
    if audio_paths.is_empty() {
        return if paths.iter().map(Path::new).any(Path::is_file) {
            extraction_failure(
                "unsupported_input",
                "Whisper Transcription supports FLAC, MP3, OGG, WAV, M4A, or AAC audio files.",
            )
        } else {
            ExtractionOutcome::NoOutput
        };
    }
    let workspace = match crate::external_tools::PrivateWorkspace::create("transcription") {
        Ok(workspace) => workspace,
        Err(_) => {
            return ExtractionOutcome::Failed {
                failure: ExtractionFailure {
                    code: "workspace_error".into(),
                    message: "A private transcription workspace could not be created.".into(),
                },
            };
        }
    };
    let started = Instant::now();
    let mut transcripts = Vec::new();
    let mut transcript_bytes = 0usize;
    for (index, (audio_path, preparation)) in audio_paths.into_iter().enumerate() {
        let remaining = timeout.saturating_sub(started.elapsed());
        let prepared_audio =
            match prepare_whisper_audio(audio_path, preparation, &workspace, index, remaining) {
                Ok(path) => path,
                Err(outcome) => return outcome,
            };
        let output_base = workspace.join(format!("transcript-{index}"));
        let output_path = workspace.join(format!("transcript-{index}.txt"));
        let mut child = match spawn_whisper_cpp(
            executable,
            model_path,
            prepared_audio.as_ref(),
            &output_base,
            false,
        ) {
            Ok(child) => child,
            Err(_) => {
                return extraction_failure(
                    "engine_unavailable",
                    "Whisper.cpp could not be started.",
                );
            }
        };
        let remaining = timeout.saturating_sub(started.elapsed());
        let status = match wait_for_whisper(&mut child, remaining) {
            Ok(status) => status,
            Err(outcome) => return outcome,
        };
        if !status.success() {
            let _ = fs::remove_file(&output_path);
            let mut fallback = match spawn_whisper_cpp(
                executable,
                model_path,
                prepared_audio.as_ref(),
                &output_base,
                true,
            ) {
                Ok(child) => child,
                Err(_) => {
                    return extraction_failure(
                        "engine_unavailable",
                        "Whisper.cpp could not be started.",
                    );
                }
            };
            let remaining = timeout.saturating_sub(started.elapsed());
            let status = match wait_for_whisper(&mut fallback, remaining) {
                Ok(status) => status,
                Err(outcome) => return outcome,
            };
            if !status.success() {
                return extraction_failure(
                    "engine_failed",
                    "Whisper.cpp did not complete successfully.",
                );
            }
        }
        let Ok(metadata) = output_path.metadata() else {
            continue;
        };
        if metadata.len() > crate::resource_limits::MAX_OCR_TEXT_BYTES as u64 {
            return ExtractionOutcome::Failed {
                failure: ExtractionFailure {
                    code: "output_too_large".into(),
                    message: "Transcribed text exceeds the supported size limit.".into(),
                },
            };
        }
        if let Ok(text) = fs::read_to_string(&output_path) {
            let text = text.trim();
            if !text.is_empty() {
                transcript_bytes = transcript_bytes
                    .saturating_add(text.len())
                    .saturating_add(2);
                if transcript_bytes > crate::resource_limits::MAX_OCR_TEXT_BYTES {
                    return ExtractionOutcome::Failed {
                        failure: ExtractionFailure {
                            code: "output_too_large".into(),
                            message: "Transcribed text exceeds the supported size limit.".into(),
                        },
                    };
                }
                transcripts.push(text.to_string());
            }
        }
    }
    if transcripts.is_empty() {
        ExtractionOutcome::NoOutput
    } else {
        ExtractionOutcome::Produced {
            text: transcripts.join("\n\n"),
        }
    }
}

pub(super) fn perform_tesseract_ocr(
    executable: &Path,
    image_bytes: &[u8],
    timeout: Duration,
) -> ExtractionOutcome {
    if image_bytes.is_empty() || image_bytes.len() > crate::resource_limits::MAX_ENCODED_IMAGE_BYTES
    {
        return ExtractionOutcome::NoOutput;
    }

    let workspace = match crate::external_tools::PrivateWorkspace::create("extractor") {
        Ok(workspace) => workspace,
        Err(_) => {
            return ExtractionOutcome::Failed {
                failure: ExtractionFailure {
                    code: "workspace_error".into(),
                    message: "A private extraction workspace could not be created.".into(),
                },
            };
        }
    };
    let input_path = workspace.join("input.image");
    let output_base = workspace.join("recognized");
    let output_path = workspace.join("recognized.txt");
    if fs::write(&input_path, image_bytes).is_err() {
        return ExtractionOutcome::Failed {
            failure: ExtractionFailure {
                code: "workspace_error".into(),
                message: "The image could not be staged for local extraction.".into(),
            },
        };
    }
    #[cfg(unix)]
    if let Ok(metadata) = input_path.metadata() {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o600);
        let _ = fs::set_permissions(&input_path, permissions);
    }

    let mut child = match Command::new(executable)
        .arg(&input_path)
        .arg(&output_base)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => {
            return ExtractionOutcome::Failed {
                failure: ExtractionFailure {
                    code: "engine_unavailable".into(),
                    message: "Tesseract OCR could not be started.".into(),
                },
            };
        }
    };
    let status = match crate::external_tools::wait_bounded(&mut child, timeout) {
        Ok(status) => status,
        Err(crate::external_tools::ProcessWaitError::TimedOut) => {
            return ExtractionOutcome::Failed {
                failure: ExtractionFailure {
                    code: "engine_timeout".into(),
                    message: "Tesseract OCR exceeded the local extraction time limit.".into(),
                },
            };
        }
        Err(crate::external_tools::ProcessWaitError::Failed) => {
            return ExtractionOutcome::Failed {
                failure: ExtractionFailure {
                    code: "engine_failed".into(),
                    message: "Tesseract OCR did not complete successfully.".into(),
                },
            };
        }
    };
    if !status.success() {
        return ExtractionOutcome::Failed {
            failure: ExtractionFailure {
                code: "engine_failed".into(),
                message: "Tesseract OCR did not complete successfully.".into(),
            },
        };
    }

    let Ok(metadata) = output_path.metadata() else {
        return ExtractionOutcome::NoOutput;
    };
    if metadata.len() > crate::resource_limits::MAX_OCR_TEXT_BYTES as u64 {
        return ExtractionOutcome::Failed {
            failure: ExtractionFailure {
                code: "output_too_large".into(),
                message: "Extracted text exceeds the supported size limit.".into(),
            },
        };
    }
    let Ok(bytes) = fs::read(output_path) else {
        return ExtractionOutcome::Failed {
            failure: ExtractionFailure {
                code: "engine_failed".into(),
                message: "Tesseract OCR output could not be read.".into(),
            },
        };
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return ExtractionOutcome::Failed {
            failure: ExtractionFailure {
                code: "invalid_output".into(),
                message: "Tesseract OCR returned invalid text.".into(),
            },
        };
    };
    let text = text.trim().to_string();
    if text.is_empty() {
        ExtractionOutcome::NoOutput
    } else {
        ExtractionOutcome::Produced { text }
    }
}

#[cfg(target_os = "macos")]
pub(super) fn perform_apple_vision_ocr(image_bytes: &[u8]) -> Option<String> {
    use objc::runtime::{Class, Object};
    use objc::{msg_send, sel, sel_impl};
    use std::ptr::null_mut;

    type Id = *mut Object;

    if image_bytes.is_empty() || image_bytes.len() > crate::resource_limits::MAX_ENCODED_IMAGE_BYTES
    {
        return None;
    }

    unsafe {
        let ns_data_class = Class::get("NSData")?;
        let ns_data: Id =
            msg_send![ns_data_class, dataWithBytes:image_bytes.as_ptr() length:image_bytes.len()];
        if ns_data.is_null() {
            return None;
        }

        let ns_image_class = Class::get("NSImage")?;
        let ns_image: Id = msg_send![ns_image_class, alloc];
        let ns_image: Id = msg_send![ns_image, initWithData: ns_data];
        if ns_image.is_null() {
            return None;
        }

        let cg_image: Id = msg_send![
            ns_image,
            CGImageForProposedRect: null_mut::<Object>()
            context: null_mut::<Object>()
            hints: null_mut::<Object>()
        ];
        if cg_image.is_null() {
            return None;
        }

        let handler_class = Class::get("VNImageRequestHandler")?;
        let handler: Id = msg_send![handler_class, alloc];
        let handler: Id = msg_send![handler, initWithCGImage:cg_image options:null_mut::<Object>()];
        if handler.is_null() {
            return None;
        }

        let request_class = Class::get("VNRecognizeTextRequest")?;
        let request: Id = msg_send![request_class, alloc];
        let request: Id = msg_send![request, init];
        if request.is_null() {
            return None;
        }

        let _: () = msg_send![request, setRecognitionLevel: 1i64];

        let array_class = Class::get("NSArray")?;
        let requests: Id = msg_send![array_class, arrayWithObject: request];

        let mut error: Id = null_mut();
        let success: bool = msg_send![handler, performRequests: requests error: &mut error];
        if !success {
            return None;
        }

        let results: Id = msg_send![request, results];
        if results.is_null() {
            return None;
        }

        let count: usize = msg_send![results, count];
        if count == 0 {
            return None;
        }

        let mut lines = Vec::new();
        let mut recognized_bytes = 0usize;
        for i in 0..count {
            let observation: Id = msg_send![results, objectAtIndex: i];
            if observation.is_null() {
                continue;
            }

            let top_candidates: Id = msg_send![observation, topCandidates: 1usize];
            if !top_candidates.is_null() {
                let candidate_count: usize = msg_send![top_candidates, count];
                if candidate_count > 0 {
                    let candidate: Id = msg_send![top_candidates, objectAtIndex: 0usize];
                    if !candidate.is_null() {
                        let string_value: Id = msg_send![candidate, string];
                        if !string_value.is_null() {
                            let utf8: *const std::os::raw::c_char =
                                msg_send![string_value, UTF8String];
                            if !utf8.is_null() {
                                if let Ok(value) = std::ffi::CStr::from_ptr(utf8).to_str() {
                                    let trimmed = value.trim();
                                    if !trimmed.is_empty() {
                                        recognized_bytes = recognized_bytes
                                            .saturating_add(trimmed.len())
                                            .saturating_add(1);
                                        if recognized_bytes
                                            > crate::resource_limits::MAX_OCR_TEXT_BYTES
                                        {
                                            return None;
                                        }
                                        lines.push(trimmed.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        (!lines.is_empty()).then(|| lines.join("\n"))
    }
}

#[cfg(not(target_os = "macos"))]
pub(super) fn perform_apple_vision_ocr(_image_bytes: &[u8]) -> Option<String> {
    None
}

pub fn run_bundled_extractor_helper(arguments: &[String]) -> Option<i32> {
    let marker = arguments
        .iter()
        .position(|argument| argument == "--pasted-extractor-helper-v1")?;
    let method = arguments.get(marker + 1).map(String::as_str);
    let request_path = arguments.get(marker + 2).map(Path::new);
    let result = match (method, request_path) {
        (Some("apple-vision-ocr"), Some(request_path)) => {
            let request = fs::metadata(request_path)
                .ok()
                .filter(|metadata| metadata.is_file() && metadata.len() <= 1024 * 1024)
                .and_then(|_| fs::read(request_path).ok())
                .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
            let image_path = request
                .as_ref()
                .and_then(|request| request.pointer("/input/path"))
                .and_then(serde_json::Value::as_str)
                .map(Path::new);
            let image = image_path
                .and_then(|path| fs::metadata(path).ok().map(|metadata| (path, metadata)))
                .filter(|(_, metadata)| {
                    metadata.is_file()
                        && metadata.len() <= crate::resource_limits::MAX_ENCODED_IMAGE_BYTES as u64
                })
                .and_then(|(path, _)| fs::read(path).ok());
            image.map_or_else(
                || Err("invalid_input"),
                |image| Ok(perform_apple_vision_ocr(&image)),
            )
        }
        _ => Err("unsupported_helper"),
    };
    match result {
        Ok(text) => match serde_json::to_string(&serde_json::json!({ "text": text })) {
            Ok(output) => {
                println!("{output}");
                Some(0)
            }
            Err(_) => Some(1),
        },
        Err(code) => {
            eprintln!("{code}");
            Some(2)
        }
    }
}
