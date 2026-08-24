use super::*;

pub(super) struct WhisperCppEngine;

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
        whisper_model_availability(model_path)
    }

    fn extract(&self, _image_bytes: &[u8]) -> ExtractionOutcome {
        extraction_failure(
            "invalid_contract",
            "Whisper transcription requires audio file references.",
        )
    }

    fn extract_files(&self, paths: &[String], model_path: Option<&Path>) -> ExtractionOutcome {
        let Some(executable) = find_whisper_cpp_executable() else {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WhisperAudioPreparation {
    Native,
    FfmpegWav,
}

pub(crate) fn whisper_audio_preparation(path: &Path) -> Option<WhisperAudioPreparation> {
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

pub(crate) fn prepare_whisper_audio<'a>(
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

pub(crate) fn perform_whisper_cpp_transcription(
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
            return extraction_failure(
                "workspace_error",
                "A private transcription workspace could not be created.",
            );
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
            return extraction_failure(
                "output_too_large",
                "Transcribed text exceeds the supported size limit.",
            );
        }
        if let Ok(text) = fs::read_to_string(&output_path) {
            let text = text.trim();
            if !text.is_empty() {
                transcript_bytes = transcript_bytes
                    .saturating_add(text.len())
                    .saturating_add(2);
                if transcript_bytes > crate::resource_limits::MAX_OCR_TEXT_BYTES {
                    return extraction_failure(
                        "output_too_large",
                        "Transcribed text exceeds the supported size limit.",
                    );
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
