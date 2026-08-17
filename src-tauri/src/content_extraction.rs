use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::analysis_contract::{RepresentationContract, RepresentationKind};
use crate::extractor_recipe::{
    ExtractorCapture, ExtractorCommandStep, ExtractorExecutable, ExtractorInputKind,
    ExtractorOutputKind, ExtractorRecipe, ExtractorResource, ExtractorResourceKind,
    ExtractorStepMode, EXTRACTOR_RECIPE_VERSION,
};

#[cfg(target_os = "macos")]
#[link(name = "Vision", kind = "framework")]
extern "C" {}

pub const APPLE_VISION_OCR_REF: &str = "extractor:apple-vision-ocr";
pub const APPLE_VISION_ENGINE: &str = "macos-vision-v1";
pub const TESSERACT_OCR_REF: &str = "extractor:tesseract-ocr";
pub const TESSERACT_ENGINE: &str = "tesseract-cli-v1";
pub const WHISPER_TRANSCRIPTION_REF: &str = "extractor:whisper-transcription";
pub const WHISPER_CPP_ENGINE: &str = "whisper-cpp-cli-v1";
pub const CUSTOM_COMMAND_ENGINE: &str = "custom-command-v1";
pub const RECIPE_ENGINE: &str = "recipe-v1";
pub const BUNDLED_EXTRACTOR_EXECUTABLE: &str = "pasted-bundled-extractor";

const TESSERACT_TIMEOUT: Duration = Duration::from_secs(15);
const WHISPER_TIMEOUT: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EngineAvailability {
    pub is_available: bool,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionFailure {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ExtractionOutcome {
    Produced { text: String },
    NoOutput,
    Failed { failure: ExtractionFailure },
}

pub trait ExtractorEngine: Sync {
    fn id(&self) -> &'static str;
    fn availability(&self) -> EngineAvailability;
    fn availability_with_model(&self, _model_path: Option<&Path>) -> EngineAvailability {
        self.availability()
    }
    fn availability_with_configuration(
        &self,
        _executable_path: Option<&Path>,
        model_path: Option<&Path>,
    ) -> EngineAvailability {
        self.availability_with_model(model_path)
    }
    fn extract(&self, image_bytes: &[u8]) -> ExtractionOutcome;
    fn extract_with_configuration(
        &self,
        image_bytes: &[u8],
        _executable_path: Option<&Path>,
        _model_path: Option<&Path>,
    ) -> ExtractionOutcome {
        self.extract(image_bytes)
    }
    fn extract_files(&self, _paths: &[String], _model_path: Option<&Path>) -> ExtractionOutcome {
        ExtractionOutcome::Failed {
            failure: ExtractionFailure {
                code: "invalid_contract".into(),
                message: "This extraction engine does not accept file references.".into(),
            },
        }
    }
    fn extract_files_with_configuration(
        &self,
        paths: &[String],
        _executable_path: Option<&Path>,
        model_path: Option<&Path>,
    ) -> ExtractionOutcome {
        self.extract_files(paths, model_path)
    }
}

pub struct ExtractorEngineRegistry<'a> {
    engines: &'a [&'a dyn ExtractorEngine],
}

impl<'a> ExtractorEngineRegistry<'a> {
    pub const fn new(engines: &'a [&'a dyn ExtractorEngine]) -> Self {
        Self { engines }
    }

    pub fn availability(&self, engine: &str) -> EngineAvailability {
        self.engines
            .iter()
            .find(|candidate| candidate.id() == engine)
            .map(|candidate| candidate.availability())
            .unwrap_or_else(|| EngineAvailability {
                is_available: false,
                unavailable_reason: Some("This extraction engine is not installed.".into()),
            })
    }

    pub fn availability_for(
        &self,
        engine: &str,
        executable_path: Option<&Path>,
        model_path: Option<&Path>,
    ) -> EngineAvailability {
        self.engines
            .iter()
            .find(|candidate| candidate.id() == engine)
            .map(|candidate| candidate.availability_with_configuration(executable_path, model_path))
            .unwrap_or_else(|| EngineAvailability {
                is_available: false,
                unavailable_reason: Some("This extraction engine is not installed.".into()),
            })
    }

    pub fn execute(&self, extractor: &Extractor, image_bytes: &[u8]) -> ExtractionOutcome {
        if !extractor.supports_contract(
            RepresentationKind::ImageBytes,
            RepresentationKind::SearchableText,
        ) {
            return ExtractionOutcome::Failed {
                failure: ExtractionFailure {
                    code: "invalid_contract".into(),
                    message: "This extraction contract is not supported.".into(),
                },
            };
        }
        if extractor.engine == RECIPE_ENGINE {
            return normalize_extraction_outcome(crate::extractor_recipe::execute_image(
                &extractor.recipe,
                image_bytes,
            ));
        }
        let Some(engine) = self
            .engines
            .iter()
            .find(|candidate| candidate.id() == extractor.engine)
        else {
            return ExtractionOutcome::Failed {
                failure: ExtractionFailure {
                    code: "engine_not_installed".into(),
                    message: "This extraction engine is not installed.".into(),
                },
            };
        };
        let executable_path = extractor.executable_path.as_deref().map(Path::new);
        let model_path = extractor.model_path.as_deref().map(Path::new);
        let availability = engine.availability_with_configuration(executable_path, model_path);
        if !availability.is_available {
            return ExtractionOutcome::Failed {
                failure: ExtractionFailure {
                    code: "engine_unavailable".into(),
                    message: availability
                        .unavailable_reason
                        .unwrap_or_else(|| "This extraction engine is unavailable.".into()),
                },
            };
        }
        normalize_extraction_outcome(engine.extract_with_configuration(
            image_bytes,
            executable_path,
            model_path,
        ))
    }

    pub fn execute_files(&self, extractor: &Extractor, paths: &[String]) -> ExtractionOutcome {
        if !extractor.supports_contract(
            RepresentationKind::FileReferences,
            RepresentationKind::SearchableText,
        ) {
            return ExtractionOutcome::Failed {
                failure: ExtractionFailure {
                    code: "invalid_contract".into(),
                    message: "This extraction contract is not supported.".into(),
                },
            };
        }
        if extractor.engine == RECIPE_ENGINE {
            return normalize_extraction_outcome(crate::extractor_recipe::execute_files(
                &extractor.recipe,
                paths,
            ));
        }
        let Some(engine) = self
            .engines
            .iter()
            .find(|candidate| candidate.id() == extractor.engine)
        else {
            return ExtractionOutcome::Failed {
                failure: ExtractionFailure {
                    code: "engine_not_installed".into(),
                    message: "This extraction engine is not installed.".into(),
                },
            };
        };
        let executable_path = extractor.executable_path.as_deref().map(Path::new);
        let model_path = extractor.model_path.as_deref().map(Path::new);
        let availability = engine.availability_with_configuration(executable_path, model_path);
        if !availability.is_available {
            return ExtractionOutcome::Failed {
                failure: ExtractionFailure {
                    code: "engine_unavailable".into(),
                    message: availability
                        .unavailable_reason
                        .unwrap_or_else(|| "This extraction engine is unavailable.".into()),
                },
            };
        }
        normalize_extraction_outcome(engine.extract_files_with_configuration(
            paths,
            executable_path,
            model_path,
        ))
    }
}

fn normalize_extraction_outcome(outcome: ExtractionOutcome) -> ExtractionOutcome {
    match outcome {
        ExtractionOutcome::Produced { text } if text.trim().is_empty() => {
            ExtractionOutcome::NoOutput
        }
        ExtractionOutcome::Produced { text }
            if text.len() > crate::resource_limits::MAX_OCR_TEXT_BYTES =>
        {
            ExtractionOutcome::Failed {
                failure: ExtractionFailure {
                    code: "output_too_large".into(),
                    message: "Extracted text exceeds the supported size limit.".into(),
                },
            }
        }
        outcome => outcome,
    }
}

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

pub fn system_engine_registry() -> ExtractorEngineRegistry<'static> {
    ExtractorEngineRegistry::new(&SYSTEM_ENGINES)
}

pub fn execute(extractor: &Extractor, image_bytes: &[u8]) -> ExtractionOutcome {
    system_engine_registry().execute(extractor, image_bytes)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Extractor {
    pub id: i64,
    pub stable_ref: String,
    pub name: String,
    pub description: String,
    pub engine: String,
    pub executable_path: Option<String>,
    pub model_path: Option<String>,
    pub input_contract: String,
    pub output_contract: String,
    pub enabled: bool,
    pub priority: i64,
    pub revision: i64,
    pub is_builtin: bool,
    pub is_available: bool,
    pub unavailable_reason: Option<String>,
    pub runtime: ExtractorRuntimeStatus,
    pub recipe: ExtractorRecipe,
    pub recipe_hash: String,
    pub default_recipe: Option<ExtractorRecipe>,
    pub defaults: Option<ExtractorDefinitionInput>,
}

impl Extractor {
    pub fn representation_contract(&self) -> Result<RepresentationContract, String> {
        RepresentationContract::parse(&self.input_contract, &self.output_contract)
    }

    pub fn supports_contract(&self, input: RepresentationKind, output: RepresentationKind) -> bool {
        if output != RepresentationKind::SearchableText {
            return false;
        }
        match input {
            RepresentationKind::ImageBytes => self.recipe.accepts(ExtractorInputKind::Image),
            RepresentationKind::FileReferences => {
                self.recipe.accepts(ExtractorInputKind::FileReferences)
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractorInput {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub priority: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractorDefinitionInput {
    pub name: String,
    pub description: String,
    pub engine: String,
    pub executable_path: Option<String>,
    pub model_path: Option<String>,
    pub input_contract: String,
    pub output_contract: String,
    pub enabled: bool,
    pub priority: i64,
}

pub struct ExtractorPreset {
    pub stable_ref: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub engine: &'static str,
    pub executable_path: Option<&'static str>,
    pub model_path: Option<&'static str>,
    pub input_contract: &'static str,
    pub output_contract: &'static str,
    pub priority: i64,
    pub revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractorRuntimeDependency {
    pub name: String,
    pub location: Option<String>,
    pub version: Option<String>,
    pub is_available: bool,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractorRuntimeStatus {
    pub method: String,
    pub location: Option<String>,
    pub version: Option<String>,
    pub uses_automatic_discovery: bool,
    pub dependencies: Vec<ExtractorRuntimeDependency>,
}

pub const EXTRACTOR_PRESETS: &[ExtractorPreset] = &[
    ExtractorPreset {
        stable_ref: APPLE_VISION_OCR_REF,
        name: "Apple Vision OCR",
        description: "Extracts searchable text from images locally with Apple Vision.",
        engine: RECIPE_ENGINE,
        executable_path: None,
        model_path: None,
        input_contract: RepresentationKind::ImageBytes.stable_name(),
        output_contract: RepresentationKind::SearchableText.stable_name(),
        priority: 10,
        revision: 3,
    },
    ExtractorPreset {
        stable_ref: TESSERACT_OCR_REF,
        name: "Tesseract OCR",
        description: "Extracts searchable text from images locally with Tesseract.",
        engine: RECIPE_ENGINE,
        executable_path: None,
        model_path: None,
        input_contract: RepresentationKind::ImageBytes.stable_name(),
        output_contract: RepresentationKind::SearchableText.stable_name(),
        priority: 20,
        revision: 2,
    },
    ExtractorPreset {
        stable_ref: WHISPER_TRANSCRIPTION_REF,
        name: "Whisper Transcription",
        description: "Extracts searchable text from local audio files with whisper.cpp.",
        engine: RECIPE_ENGINE,
        executable_path: None,
        model_path: None,
        input_contract: RepresentationKind::FileReferences.stable_name(),
        output_contract: RepresentationKind::SearchableText.stable_name(),
        priority: 30,
        revision: 3,
    },
];

pub fn engine_availability(engine: &str) -> EngineAvailability {
    system_engine_registry().availability(engine)
}

pub fn engine_availability_for(
    engine: &str,
    executable_path: Option<&str>,
    model_path: Option<&str>,
) -> EngineAvailability {
    system_engine_registry().availability_for(
        engine,
        executable_path.map(Path::new),
        model_path.map(Path::new),
    )
}

pub fn validate_extractor_input(input: &ExtractorInput) -> Result<(), String> {
    if input.name.trim().is_empty() || input.name.trim().len() > 80 {
        return Err("Extractor names require 1–80 characters".to_string());
    }
    if input.description.trim().len() > 240 {
        return Err("Extractor descriptions cannot exceed 240 characters".to_string());
    }
    if !(0..=10_000).contains(&input.priority) {
        return Err("Extractor priority must be between 0 and 10,000".to_string());
    }
    Ok(())
}

pub fn validate_extractor_definition(input: &ExtractorDefinitionInput) -> Result<(), String> {
    validate_extractor_input(&ExtractorInput {
        name: input.name.clone(),
        description: input.description.clone(),
        enabled: input.enabled,
        priority: input.priority,
    })?;
    if input.engine.trim().is_empty() || input.engine.trim().len() > 80 {
        return Err("Extractor engines require 1–80 characters".to_string());
    }
    if !matches!(
        input.engine.as_str(),
        APPLE_VISION_ENGINE
            | TESSERACT_ENGINE
            | WHISPER_CPP_ENGINE
            | CUSTOM_COMMAND_ENGINE
            | RECIPE_ENGINE
    ) {
        return Err("Extractors require a registered execution method".to_string());
    }
    if input
        .executable_path
        .as_deref()
        .is_some_and(|path| path.is_empty() || path.len() > 4_096 || path.contains('\0'))
    {
        return Err("Extractor executable paths require 1–4,096 characters".to_string());
    }
    if input
        .executable_path
        .as_deref()
        .is_some_and(|path| !Path::new(path).is_absolute())
    {
        return Err("Extractor executable paths must be absolute".to_string());
    }
    if input.engine == CUSTOM_COMMAND_ENGINE && input.executable_path.is_none() {
        return Err("Custom command Extractors require an executable path".to_string());
    }
    if input
        .model_path
        .as_deref()
        .is_some_and(|path| path.is_empty() || path.len() > 4_096 || path.contains('\0'))
    {
        return Err("Extractor model paths require 1–4,096 characters".to_string());
    }
    let contract = RepresentationContract::parse(&input.input_contract, &input.output_contract)
        .map_err(|_| "Extractors require a supported searchable-text contract".to_string())?;
    if !matches!(
        (contract.input, contract.output),
        (
            RepresentationKind::ImageBytes | RepresentationKind::FileReferences,
            RepresentationKind::SearchableText
        )
    ) {
        return Err("Extractors require image or file references → searchable_text".to_string());
    }
    Ok(())
}

pub fn recipe_for_legacy_definition(input: &ExtractorDefinitionInput) -> ExtractorRecipe {
    let accepts = ExtractorInputKind::from_legacy(&input.input_contract)
        .into_iter()
        .collect::<Vec<_>>();
    let executable = |discover: &[&str], version_arguments: &[&str]| ExtractorExecutable {
        path: input.executable_path.clone(),
        discover: discover.iter().map(|value| (*value).to_string()).collect(),
        version_arguments: version_arguments
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
    };
    let (steps, resources) = match input.engine.as_str() {
        APPLE_VISION_ENGINE => (
            vec![ExtractorCommandStep {
                id: "extract".into(),
                executable: executable(&[BUNDLED_EXTRACTOR_EXECUTABLE], &[]),
                arguments: vec![
                    "--pasted-extractor-helper-v1".into(),
                    "apple-vision-ocr".into(),
                    "{request.path}".into(),
                ],
                mode: ExtractorStepMode::Once,
                capture: ExtractorCapture::PastedJsonV1,
                output_extension: None,
                timeout_seconds: 15,
            }],
            Vec::new(),
        ),
        TESSERACT_ENGINE => (
            vec![ExtractorCommandStep {
                id: "extract".into(),
                executable: executable(&["tesseract"], &["--version"]),
                arguments: vec!["{input.stagedPath}".into(), "stdout".into()],
                mode: ExtractorStepMode::Once,
                capture: ExtractorCapture::StdoutText,
                output_extension: None,
                timeout_seconds: TESSERACT_TIMEOUT.as_secs(),
            }],
            Vec::new(),
        ),
        WHISPER_CPP_ENGINE => (
            vec![
                ExtractorCommandStep {
                    id: "prepare_audio".into(),
                    executable: ExtractorExecutable {
                        path: None,
                        discover: vec!["ffmpeg".into()],
                        version_arguments: vec!["-version".into()],
                    },
                    arguments: vec![
                        "-nostdin".into(),
                        "-y".into(),
                        "-i".into(),
                        "{input.path}".into(),
                        "-ar".into(),
                        "16000".into(),
                        "-ac".into(),
                        "1".into(),
                        "-c:a".into(),
                        "pcm_s16le".into(),
                        "-f".into(),
                        "wav".into(),
                        "{output.path}".into(),
                    ],
                    mode: ExtractorStepMode::EachInput,
                    capture: ExtractorCapture::Ignore,
                    output_extension: Some("wav".into()),
                    timeout_seconds: 60,
                },
                ExtractorCommandStep {
                    id: "transcribe".into(),
                    executable: executable(&["whisper-cli"], &["--version"]),
                    arguments: vec![
                        "-m".into(),
                        "{resource.model.path}".into(),
                        "-f".into(),
                        "{step.prepare_audio.output}".into(),
                        "-otxt".into(),
                        "-of".into(),
                        "{output.base}".into(),
                        "-np".into(),
                        "-nt".into(),
                        "-l".into(),
                        "auto".into(),
                    ],
                    mode: ExtractorStepMode::EachInput,
                    capture: ExtractorCapture::FileText,
                    output_extension: Some("txt".into()),
                    timeout_seconds: WHISPER_TIMEOUT.as_secs(),
                },
            ],
            vec![ExtractorResource {
                id: "model".into(),
                label: "Whisper GGML model".into(),
                kind: ExtractorResourceKind::File,
                required: true,
                path: input.model_path.clone(),
            }],
        ),
        _ => (
            vec![ExtractorCommandStep {
                id: "extract".into(),
                executable: executable(&[], &["--version"]),
                arguments: vec!["--pasted-extract-v1".into(), "{request.path}".into()],
                mode: ExtractorStepMode::Once,
                capture: ExtractorCapture::PastedJsonV1,
                output_extension: None,
                timeout_seconds: 60,
            }],
            input
                .model_path
                .as_ref()
                .map(|path| ExtractorResource {
                    id: "model".into(),
                    label: "Model".into(),
                    kind: ExtractorResourceKind::File,
                    required: true,
                    path: Some(path.clone()),
                })
                .into_iter()
                .collect(),
        ),
    };
    ExtractorRecipe {
        definition_version: EXTRACTOR_RECIPE_VERSION,
        accepts,
        output: ExtractorOutputKind::SearchableText,
        steps,
        resources,
    }
}

#[cfg(test)]
pub fn test_recipe(input_contract: &str) -> ExtractorRecipe {
    recipe_for_legacy_definition(&ExtractorDefinitionInput {
        name: "Test Extractor".into(),
        description: String::new(),
        engine: CUSTOM_COMMAND_ENGINE.into(),
        executable_path: Some("/test/extractor".into()),
        model_path: None,
        input_contract: input_contract.into(),
        output_contract: "searchable_text".into(),
        enabled: true,
        priority: 10,
    })
}

impl ExtractorPreset {
    pub fn definition(&self) -> ExtractorDefinitionInput {
        ExtractorDefinitionInput {
            name: self.name.into(),
            description: self.description.into(),
            engine: self.engine.into(),
            executable_path: self.executable_path.map(str::to_string),
            model_path: self.model_path.map(str::to_string),
            input_contract: self.input_contract.into(),
            output_contract: self.output_contract.into(),
            enabled: true,
            priority: self.priority,
        }
    }

    pub fn recipe(&self) -> ExtractorRecipe {
        let mut definition = self.definition();
        definition.engine = match self.stable_ref {
            APPLE_VISION_OCR_REF => APPLE_VISION_ENGINE,
            TESSERACT_OCR_REF => TESSERACT_ENGINE,
            WHISPER_TRANSCRIPTION_REF => WHISPER_CPP_ENGINE,
            _ => CUSTOM_COMMAND_ENGINE,
        }
        .into();
        recipe_for_legacy_definition(&definition)
    }
}

pub fn migrate_builtin_recipe_compatibility(
    stable_ref: &str,
    current: &ExtractorRecipe,
    legacy_model_path: Option<&str>,
) -> ExtractorRecipe {
    let mut migrated = current.clone();

    if stable_ref == APPLE_VISION_OCR_REF {
        for step in &mut migrated.steps {
            let uses_bundled_helper = step
                .arguments
                .windows(2)
                .any(|arguments| arguments == ["--pasted-extractor-helper-v1", "apple-vision-ocr"]);
            if uses_bundled_helper && step.executable.discover == ["pasted"] {
                step.executable.discover = vec![BUNDLED_EXTRACTOR_EXECUTABLE.into()];
                step.executable.version_arguments.clear();
            }
        }
    }

    if stable_ref == WHISPER_TRANSCRIPTION_REF {
        let model_path = migrated
            .resources
            .iter()
            .find(|resource| resource.id == "model")
            .and_then(|resource| resource.path.clone())
            .or_else(|| legacy_model_path.map(str::to_string));
        let interim_recipe = migrated.steps.len() == 1
            && migrated.steps[0].executable.discover == ["whisper-cli"]
            && migrated.steps[0].arguments
                == [
                    "--model",
                    "{resource.model.path}",
                    "--file",
                    "{input.path}",
                    "--no-timestamps",
                ];
        if interim_recipe {
            let whisper_path = migrated.steps[0].executable.path.clone();
            migrated = EXTRACTOR_PRESETS
                .iter()
                .find(|preset| preset.stable_ref == WHISPER_TRANSCRIPTION_REF)
                .expect("shipped Whisper Extractor")
                .recipe();
            if let Some(step) = migrated
                .steps
                .iter_mut()
                .find(|step| step.id == "transcribe")
            {
                step.executable.path = whisper_path;
            }
        }
        if let Some(resource) = migrated
            .resources
            .iter_mut()
            .find(|resource| resource.id == "model" && resource.path.is_none())
        {
            resource.path = model_path;
        }
    }

    migrated
}

pub fn merge_shipped_definition(
    current: &ExtractorDefinitionInput,
    previous: &ExtractorDefinitionInput,
    next: &ExtractorDefinitionInput,
) -> ExtractorDefinitionInput {
    ExtractorDefinitionInput {
        name: if current.name != previous.name {
            current.name.clone()
        } else {
            next.name.clone()
        },
        description: if current.description != previous.description {
            current.description.clone()
        } else {
            next.description.clone()
        },
        engine: if current.engine != previous.engine {
            current.engine.clone()
        } else {
            next.engine.clone()
        },
        executable_path: if current.executable_path != previous.executable_path {
            current.executable_path.clone()
        } else {
            next.executable_path.clone()
        },
        model_path: if current.model_path != previous.model_path {
            current.model_path.clone()
        } else {
            next.model_path.clone()
        },
        input_contract: if current.input_contract != previous.input_contract {
            current.input_contract.clone()
        } else {
            next.input_contract.clone()
        },
        output_contract: if current.output_contract != previous.output_contract {
            current.output_contract.clone()
        } else {
            next.output_contract.clone()
        },
        enabled: if current.enabled != previous.enabled {
            current.enabled
        } else {
            next.enabled
        },
        priority: if current.priority != previous.priority {
            current.priority
        } else {
            next.priority
        },
    }
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

fn find_tesseract_executable() -> Option<std::path::PathBuf> {
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

fn find_ffmpeg_executable() -> Option<std::path::PathBuf> {
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
enum WhisperAudioPreparation {
    Native,
    FfmpegWav,
}

fn whisper_audio_preparation(path: &Path) -> Option<WhisperAudioPreparation> {
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

fn prepare_whisper_audio<'a>(
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

fn perform_whisper_cpp_transcription(
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

fn perform_tesseract_ocr(
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
fn perform_apple_vision_ocr(image_bytes: &[u8]) -> Option<String> {
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
fn perform_apple_vision_ocr(_image_bytes: &[u8]) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ocr_acceptance_image() -> Vec<u8> {
        const SCALE: u32 = 12;
        const GLYPH_WIDTH: u32 = 5;
        const GLYPH_HEIGHT: u32 = 7;
        const TEXT: &str = "PASTED OCR";
        fn glyph(character: char) -> [&'static str; GLYPH_HEIGHT as usize] {
            match character {
                'P' => [
                    "11110", "10001", "10001", "11110", "10000", "10000", "10000",
                ],
                'A' => [
                    "01110", "10001", "10001", "11111", "10001", "10001", "10001",
                ],
                'S' => [
                    "01111", "10000", "10000", "01110", "00001", "00001", "11110",
                ],
                'T' => [
                    "11111", "00100", "00100", "00100", "00100", "00100", "00100",
                ],
                'E' => [
                    "11111", "10000", "10000", "11110", "10000", "10000", "11111",
                ],
                'D' => [
                    "11110", "10001", "10001", "10001", "10001", "10001", "11110",
                ],
                'O' => [
                    "01110", "10001", "10001", "10001", "10001", "10001", "01110",
                ],
                'C' => [
                    "01111", "10000", "10000", "10000", "10000", "10000", "01111",
                ],
                'R' => [
                    "11110", "10001", "10001", "11110", "10100", "10010", "10001",
                ],
                _ => ["00000"; GLYPH_HEIGHT as usize],
            }
        }

        let margin = 24;
        let advance = (GLYPH_WIDTH + 2) * SCALE;
        let width = margin * 2 + advance * TEXT.chars().count() as u32;
        let height = margin * 2 + GLYPH_HEIGHT * SCALE;
        let mut image = image::GrayImage::from_pixel(width, height, image::Luma([255]));
        for (index, character) in TEXT.chars().enumerate() {
            for (row, pixels) in glyph(character).iter().enumerate() {
                for (column, pixel) in pixels.bytes().enumerate() {
                    if pixel != b'1' {
                        continue;
                    }
                    for y in 0..SCALE {
                        for x in 0..SCALE {
                            image.put_pixel(
                                margin + index as u32 * advance + column as u32 * SCALE + x,
                                margin + row as u32 * SCALE + y,
                                image::Luma([0]),
                            );
                        }
                    }
                }
            }
        }
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageLuma8(image)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn advertised_apple_vision_engine_is_linked() {
        assert!(objc::runtime::Class::get("VNRecognizeTextRequest").is_some());
    }

    struct FixedEngine {
        outcome: ExtractionOutcome,
    }

    impl ExtractorEngine for FixedEngine {
        fn id(&self) -> &'static str {
            "test-v1"
        }

        fn availability(&self) -> EngineAvailability {
            EngineAvailability {
                is_available: true,
                unavailable_reason: None,
            }
        }

        fn extract(&self, _image_bytes: &[u8]) -> ExtractionOutcome {
            self.outcome.clone()
        }
    }

    fn extractor(engine: &str) -> Extractor {
        Extractor {
            id: 1,
            stable_ref: "extractor:test".into(),
            name: "Test Extractor".into(),
            description: String::new(),
            engine: engine.into(),
            executable_path: None,
            model_path: None,
            input_contract: "image".into(),
            output_contract: "searchable_text".into(),
            enabled: true,
            priority: 10,
            revision: 1,
            is_builtin: false,
            is_available: true,
            unavailable_reason: None,
            runtime: runtime_status_for(engine, None),
            recipe: test_recipe("image"),
            recipe_hash: "test".into(),
            default_recipe: None,
            defaults: None,
        }
    }

    #[test]
    fn registry_dispatches_typed_engine_outcomes() {
        let engine = FixedEngine {
            outcome: ExtractionOutcome::Produced {
                text: "recognized".into(),
            },
        };
        let engines: [&dyn ExtractorEngine; 1] = [&engine];
        let registry = ExtractorEngineRegistry::new(&engines);

        assert_eq!(
            registry.execute(&extractor("test-v1"), b"image"),
            ExtractionOutcome::Produced {
                text: "recognized".into()
            }
        );
    }

    #[test]
    fn registry_rejects_unknown_contracts_before_engine_dispatch() {
        let engine = FixedEngine {
            outcome: ExtractionOutcome::Produced {
                text: "should not run".into(),
            },
        };
        let engines: [&dyn ExtractorEngine; 1] = [&engine];
        let registry = ExtractorEngineRegistry::new(&engines);
        let mut invalid = extractor("test-v1");
        invalid.recipe.accepts = vec![ExtractorInputKind::FileReferences];

        assert_eq!(
            registry.execute(&invalid, b"image"),
            ExtractionOutcome::Failed {
                failure: ExtractionFailure {
                    code: "invalid_contract".into(),
                    message: "This extraction contract is not supported.".into(),
                }
            }
        );
    }

    #[test]
    fn registry_normalizes_blank_and_oversized_engine_output() {
        let blank_engine = FixedEngine {
            outcome: ExtractionOutcome::Produced { text: "  ".into() },
        };
        let blank_engines: [&dyn ExtractorEngine; 1] = [&blank_engine];
        let blank_registry = ExtractorEngineRegistry::new(&blank_engines);
        assert_eq!(
            blank_registry.execute(&extractor("test-v1"), b"image"),
            ExtractionOutcome::NoOutput
        );

        let oversized_engine = FixedEngine {
            outcome: ExtractionOutcome::Produced {
                text: "x".repeat(crate::resource_limits::MAX_OCR_TEXT_BYTES + 1),
            },
        };
        let oversized_engines: [&dyn ExtractorEngine; 1] = [&oversized_engine];
        let oversized_registry = ExtractorEngineRegistry::new(&oversized_engines);
        assert!(matches!(
            oversized_registry.execute(&extractor("test-v1"), b"image"),
            ExtractionOutcome::Failed {
                failure: ExtractionFailure { ref code, .. }
            } if code == "output_too_large"
        ));
    }

    #[cfg(unix)]
    #[test]
    fn custom_command_executes_the_bounded_v1_protocol() {
        use std::os::unix::fs::PermissionsExt;

        let workspace =
            crate::external_tools::PrivateWorkspace::create("custom-engine-test").unwrap();
        let executable = workspace.join("extractor");
        fs::write(
            &executable,
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'Example Extractor 1.2.3'; exit 0; fi\nif [ \"$1\" = \"--pasted-extract-v1\" ] && [ -f \"$2\" ]; then printf '{\"text\":\"custom searchable text\"}'; exit 0; fi\nexit 2\n",
        )
        .unwrap();
        let mut permissions = executable.metadata().unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&executable, permissions).unwrap();

        let mut custom = extractor(CUSTOM_COMMAND_ENGINE);
        custom.executable_path = Some(executable.to_string_lossy().into_owned());
        custom.runtime =
            runtime_status_for(CUSTOM_COMMAND_ENGINE, custom.executable_path.as_deref());
        assert_eq!(
            custom.runtime.version.as_deref(),
            Some("Example Extractor 1.2.3")
        );
        assert!(
            system_engine_registry()
                .availability_for(CUSTOM_COMMAND_ENGINE, Some(&executable), None,)
                .is_available
        );
        assert_eq!(
            system_engine_registry().execute(&custom, b"image"),
            ExtractionOutcome::Produced {
                text: "custom searchable text".into(),
            }
        );
    }

    #[test]
    fn shipped_definition_upgrades_preserve_only_user_overrides() {
        let previous = ExtractorDefinitionInput {
            name: "Shipped".into(),
            description: "Old description".into(),
            engine: TESSERACT_ENGINE.into(),
            executable_path: None,
            model_path: None,
            input_contract: "image".into(),
            output_contract: "searchable_text".into(),
            enabled: true,
            priority: 20,
        };
        let current = ExtractorDefinitionInput {
            name: "My OCR".into(),
            executable_path: Some("/custom/tesseract".into()),
            ..previous.clone()
        };
        let next = ExtractorDefinitionInput {
            description: "New shipped description".into(),
            priority: 15,
            ..previous.clone()
        };
        let merged = merge_shipped_definition(&current, &previous, &next);
        assert_eq!(merged.name, "My OCR");
        assert_eq!(merged.executable_path.as_deref(), Some("/custom/tesseract"));
        assert_eq!(merged.description, "New shipped description");
        assert_eq!(merged.priority, 15);
    }

    #[test]
    fn bundled_recipe_migration_repairs_the_interim_apple_locator() {
        let mut recipe = EXTRACTOR_PRESETS
            .iter()
            .find(|preset| preset.stable_ref == APPLE_VISION_OCR_REF)
            .unwrap()
            .recipe();
        recipe.steps[0].executable.discover = vec!["pasted".into()];
        recipe.steps[0].executable.version_arguments = vec!["--version".into()];

        let migrated = migrate_builtin_recipe_compatibility(APPLE_VISION_OCR_REF, &recipe, None);

        assert_eq!(
            migrated.steps[0].executable.discover,
            [BUNDLED_EXTRACTOR_EXECUTABLE]
        );
        assert!(migrated.steps[0].executable.version_arguments.is_empty());
    }

    #[test]
    fn bundled_recipe_migration_preserves_the_configured_whisper_model() {
        let mut recipe = EXTRACTOR_PRESETS
            .iter()
            .find(|preset| preset.stable_ref == WHISPER_TRANSCRIPTION_REF)
            .unwrap()
            .recipe();
        recipe.steps = vec![ExtractorCommandStep {
            id: "extract".into(),
            executable: ExtractorExecutable {
                path: Some("/custom/whisper-cli".into()),
                discover: vec!["whisper-cli".into()],
                version_arguments: vec!["--version".into()],
            },
            arguments: vec![
                "--model".into(),
                "{resource.model.path}".into(),
                "--file".into(),
                "{input.path}".into(),
                "--no-timestamps".into(),
            ],
            mode: ExtractorStepMode::EachInput,
            capture: ExtractorCapture::StdoutText,
            output_extension: None,
            timeout_seconds: 300,
        }];

        let migrated = migrate_builtin_recipe_compatibility(
            WHISPER_TRANSCRIPTION_REF,
            &recipe,
            Some("/models/ggml-base.bin"),
        );

        assert_eq!(migrated.steps.len(), 2);
        assert_eq!(
            migrated.steps[1].executable.path.as_deref(),
            Some("/custom/whisper-cli")
        );
        assert_eq!(
            migrated.resources[0].path.as_deref(),
            Some("/models/ggml-base.bin")
        );
    }

    #[test]
    fn unknown_and_unavailable_engines_fail_with_stable_codes() {
        let registry = ExtractorEngineRegistry::new(&[]);
        assert_eq!(
            registry.execute(&extractor("missing-v1"), b"image"),
            ExtractionOutcome::Failed {
                failure: ExtractionFailure {
                    code: "engine_not_installed".into(),
                    message: "This extraction engine is not installed.".into(),
                }
            }
        );

        let apple = system_engine_registry().availability(APPLE_VISION_ENGINE);
        assert_eq!(apple.is_available, cfg!(target_os = "macos"));
        assert_eq!(
            apple.unavailable_reason.is_none(),
            cfg!(target_os = "macos")
        );

        let tesseract = system_engine_registry().availability(TESSERACT_ENGINE);
        assert_eq!(
            tesseract.is_available,
            find_tesseract_executable().is_some()
        );
        assert_eq!(
            tesseract.unavailable_reason.is_none(),
            tesseract.is_available
        );
    }

    #[test]
    fn apple_vision_adapter_rejects_empty_and_invalid_input_without_output() {
        assert_eq!(perform_apple_vision_ocr(&[]), None);
        assert_eq!(perform_apple_vision_ocr(&[0, 1, 2, 3, 4]), None);
    }

    #[test]
    fn whisper_classifies_native_container_and_unsupported_audio() {
        assert_eq!(
            whisper_audio_preparation(Path::new("recording.WAV")),
            Some(WhisperAudioPreparation::Native)
        );
        assert_eq!(
            whisper_audio_preparation(Path::new("recording.m4a")),
            Some(WhisperAudioPreparation::FfmpegWav)
        );
        assert_eq!(
            whisper_audio_preparation(Path::new("recording.AAC")),
            Some(WhisperAudioPreparation::FfmpegWav)
        );
        assert_eq!(whisper_audio_preparation(Path::new("notes.txt")), None);
    }

    #[test]
    fn whisper_reports_unsupported_files_instead_of_no_speech() {
        let workspace =
            crate::external_tools::PrivateWorkspace::create("unsupported-audio-test").unwrap();
        let input = workspace.join("notes.txt");
        fs::write(&input, b"not audio").unwrap();
        let outcome = perform_whisper_cpp_transcription(
            Path::new("unused-whisper"),
            Path::new("unused-model"),
            &[input.to_string_lossy().into_owned()],
            Duration::from_secs(1),
        );
        assert!(matches!(
            outcome,
            ExtractionOutcome::Failed {
                failure: ExtractionFailure { ref code, .. }
            } if code == "unsupported_input"
        ));
    }

    #[test]
    fn ffmpeg_prepares_m4a_for_whisper_when_installed() {
        let Some(ffmpeg) = find_ffmpeg_executable() else {
            return;
        };
        let workspace = crate::external_tools::PrivateWorkspace::create("m4a-test").unwrap();
        let input = workspace.join("tone.m4a");
        let status = Command::new(ffmpeg)
            .args(["-nostdin", "-v", "error", "-y", "-f", "lavfi", "-i"])
            .arg("sine=frequency=440:duration=0.2")
            .args(["-c:a", "aac"])
            .arg(&input)
            .status()
            .unwrap();
        assert!(status.success());

        let prepared = prepare_whisper_audio(
            &input,
            WhisperAudioPreparation::FfmpegWav,
            &workspace,
            0,
            Duration::from_secs(10),
        )
        .unwrap();
        assert_eq!(
            prepared.extension().and_then(|value| value.to_str()),
            Some("wav")
        );
        assert!(prepared.metadata().unwrap().len() > 44);
    }

    #[test]
    fn tesseract_adapter_recognizes_text_when_installed() {
        let Some(executable) = find_tesseract_executable() else {
            return;
        };
        let outcome = perform_tesseract_ocr(
            &executable,
            &ocr_acceptance_image(),
            Duration::from_secs(15),
        );
        assert!(
            matches!(outcome, ExtractionOutcome::Produced { ref text }
                if text.to_ascii_uppercase().contains("PASTE")),
            "unexpected Tesseract result: {outcome:?}"
        );
    }

    #[test]
    fn shipped_tesseract_recipe_uses_the_universal_runner() {
        if find_tesseract_executable().is_none() {
            return;
        }
        let recipe = EXTRACTOR_PRESETS
            .iter()
            .find(|preset| preset.stable_ref == TESSERACT_OCR_REF)
            .unwrap()
            .recipe();
        let outcome = crate::extractor_recipe::execute_image(&recipe, &ocr_acceptance_image());
        assert!(
            matches!(outcome, ExtractionOutcome::Produced { ref text }
                if text.to_ascii_uppercase().contains("PASTE")),
            "unexpected recipe result: {outcome:?}"
        );
    }
}
