use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use self::outcome::normalize as normalize_extraction_outcome;

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
pub const MAX_ACTIVE_EXTRACTORS_PER_INPUT: usize = 32;

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
        if paths.is_empty() {
            return ExtractionOutcome::NoOutput;
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

mod engine_runtime;
pub(crate) mod file_routing;
mod format_defaults;
mod outcome;
#[cfg(test)]
mod preset_tests;
mod runtime_status;
pub fn system_engine_registry() -> ExtractorEngineRegistry<'static> {
    engine_runtime::system_engine_registry()
}

#[cfg(test)]
use engine_runtime::*;

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
        revision: 5,
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
        revision: 4,
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
        revision: 4,
    },
];

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
        accepted_file_formats: vec!["*".into()],
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
        let mut recipe = recipe_for_legacy_definition(&definition);
        if matches!(self.stable_ref, APPLE_VISION_OCR_REF | TESSERACT_OCR_REF) {
            recipe.accepts.push(ExtractorInputKind::FileReferences);
        }
        recipe.accepted_file_formats = format_defaults::for_builtin(self.stable_ref);
        recipe
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

pub use engine_runtime::run_bundled_extractor_helper;
pub use runtime_status::{
    engine_availability, engine_availability_for, inspect_extractor_runtime, runtime_status_for,
    runtime_status_summary_for,
};
#[cfg(test)]
mod tests;
