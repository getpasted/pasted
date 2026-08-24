use super::*;

mod apple_vision;
mod custom_command;
mod discovery;
mod tesseract;
mod whisper;

use apple_vision::AppleVisionOcrEngine;
use custom_command::CustomCommandEngine;
pub(super) use discovery::{
    configured_or_discovered_executable, find_ffmpeg_executable, find_tesseract_executable,
    find_whisper_cpp_executable,
};
#[cfg(test)]
pub(super) use tesseract::perform_tesseract_ocr;
use tesseract::TesseractOcrEngine;
use whisper::WhisperCppEngine;
#[cfg(test)]
pub(super) use whisper::{
    perform_whisper_cpp_transcription, prepare_whisper_audio, whisper_audio_preparation,
    WhisperAudioPreparation,
};

#[cfg(test)]
pub(super) use apple_vision::perform_apple_vision_ocr;
pub use apple_vision::run_bundled_extractor_helper;

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

fn executable_availability(
    executable: Option<std::path::PathBuf>,
    unavailable_reason: &str,
) -> EngineAvailability {
    discovery::executable_availability(executable, unavailable_reason)
}

fn extraction_failure(code: &str, message: &str) -> ExtractionOutcome {
    ExtractionOutcome::Failed {
        failure: ExtractionFailure {
            code: code.into(),
            message: message.into(),
        },
    }
}
