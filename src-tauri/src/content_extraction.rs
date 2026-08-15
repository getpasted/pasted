use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::analysis_contract::{RepresentationContract, RepresentationKind};

#[cfg(target_os = "macos")]
#[link(name = "Vision", kind = "framework")]
extern "C" {}

pub const APPLE_VISION_OCR_REF: &str = "extractor:apple-vision-ocr";
pub const APPLE_VISION_ENGINE: &str = "macos-vision-v1";
pub const TESSERACT_OCR_REF: &str = "extractor:tesseract-ocr";
pub const TESSERACT_ENGINE: &str = "tesseract-cli-v1";

const TESSERACT_TIMEOUT: Duration = Duration::from_secs(15);

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
    fn extract(&self, image_bytes: &[u8]) -> ExtractionOutcome;
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
        let availability = engine.availability();
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
        match engine.extract(image_bytes) {
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
}

struct AppleVisionOcrEngine;
struct TesseractOcrEngine;

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
}

static APPLE_VISION_OCR_ENGINE: AppleVisionOcrEngine = AppleVisionOcrEngine;
static TESSERACT_OCR_ENGINE: TesseractOcrEngine = TesseractOcrEngine;
static SYSTEM_ENGINES: [&dyn ExtractorEngine; 2] =
    [&APPLE_VISION_OCR_ENGINE, &TESSERACT_OCR_ENGINE];

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
    pub input_contract: String,
    pub output_contract: String,
    pub enabled: bool,
    pub priority: i64,
    pub is_builtin: bool,
    pub is_available: bool,
    pub unavailable_reason: Option<String>,
    pub defaults: Option<ExtractorInput>,
}

impl Extractor {
    pub fn representation_contract(&self) -> Result<RepresentationContract, String> {
        RepresentationContract::parse(&self.input_contract, &self.output_contract)
    }

    pub fn supports_contract(&self, input: RepresentationKind, output: RepresentationKind) -> bool {
        self.representation_contract()
            .is_ok_and(|contract| contract.input == input && contract.output == output)
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
    pub input_contract: &'static str,
    pub output_contract: &'static str,
    pub priority: i64,
}

pub const EXTRACTOR_PRESETS: &[ExtractorPreset] = &[
    ExtractorPreset {
        stable_ref: APPLE_VISION_OCR_REF,
        name: "Apple Vision OCR",
        description: "Extracts searchable text from images locally with Apple Vision.",
        engine: APPLE_VISION_ENGINE,
        input_contract: RepresentationKind::ImageBytes.stable_name(),
        output_contract: RepresentationKind::SearchableText.stable_name(),
        priority: 10,
    },
    ExtractorPreset {
        stable_ref: TESSERACT_OCR_REF,
        name: "Tesseract OCR",
        description: "Extracts searchable text from images locally with Tesseract.",
        engine: TESSERACT_ENGINE,
        input_contract: RepresentationKind::ImageBytes.stable_name(),
        output_contract: RepresentationKind::SearchableText.stable_name(),
        priority: 20,
    },
];

pub fn engine_availability(engine: &str) -> EngineAvailability {
    system_engine_registry().availability(engine)
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
    let contract = RepresentationContract::parse(&input.input_contract, &input.output_contract)
        .map_err(|_| "This version supports only image → searchable_text Extractors".to_string())?;
    if contract.input != RepresentationKind::ImageBytes
        || contract.output != RepresentationKind::SearchableText
    {
        return Err("This version supports only image → searchable_text Extractors".to_string());
    }
    Ok(())
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
            input_contract: "image".into(),
            output_contract: "searchable_text".into(),
            enabled: true,
            priority: 10,
            is_builtin: false,
            is_available: true,
            unavailable_reason: None,
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
        invalid.output_contract = "mystery".into();

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
}
