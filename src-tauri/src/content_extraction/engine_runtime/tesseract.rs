use super::*;

pub(super) struct TesseractOcrEngine;

impl ExtractorEngine for TesseractOcrEngine {
    fn id(&self) -> &'static str {
        TESSERACT_ENGINE
    }

    fn availability(&self) -> EngineAvailability {
        executable_availability(
            find_tesseract_executable(),
            "Tesseract OCR is not installed. Install Tesseract 5, then check again.",
        )
    }

    fn extract(&self, image_bytes: &[u8]) -> ExtractionOutcome {
        let Some(executable) = find_tesseract_executable() else {
            return extraction_failure("engine_unavailable", "Tesseract OCR is not installed.");
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

pub(crate) fn perform_tesseract_ocr(
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
            return extraction_failure(
                "workspace_error",
                "A private extraction workspace could not be created.",
            );
        }
    };
    let input_path = workspace.join("input.image");
    let output_base = workspace.join("recognized");
    let output_path = workspace.join("recognized.txt");
    if fs::write(&input_path, image_bytes).is_err() {
        return extraction_failure(
            "workspace_error",
            "The image could not be staged for local extraction.",
        );
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
            return extraction_failure("engine_unavailable", "Tesseract OCR could not be started.");
        }
    };
    let status = match crate::external_tools::wait_bounded(&mut child, timeout) {
        Ok(status) => status,
        Err(crate::external_tools::ProcessWaitError::TimedOut) => {
            return extraction_failure(
                "engine_timeout",
                "Tesseract OCR exceeded the local extraction time limit.",
            );
        }
        Err(crate::external_tools::ProcessWaitError::Failed) => {
            return extraction_failure(
                "engine_failed",
                "Tesseract OCR did not complete successfully.",
            );
        }
    };
    if !status.success() {
        return extraction_failure(
            "engine_failed",
            "Tesseract OCR did not complete successfully.",
        );
    }

    let Ok(metadata) = output_path.metadata() else {
        return ExtractionOutcome::NoOutput;
    };
    if metadata.len() > crate::resource_limits::MAX_OCR_TEXT_BYTES as u64 {
        return extraction_failure(
            "output_too_large",
            "Extracted text exceeds the supported size limit.",
        );
    }
    let Ok(bytes) = fs::read(output_path) else {
        return extraction_failure("engine_failed", "Tesseract OCR output could not be read.");
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return extraction_failure("invalid_output", "Tesseract OCR returned invalid text.");
    };
    let text = text.trim().to_string();
    if text.is_empty() {
        ExtractionOutcome::NoOutput
    } else {
        ExtractionOutcome::Produced { text }
    }
}
