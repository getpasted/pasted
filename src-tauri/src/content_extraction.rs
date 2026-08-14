use serde::{Deserialize, Serialize};

pub const APPLE_VISION_OCR_REF: &str = "extractor:apple-vision-ocr";
pub const APPLE_VISION_ENGINE: &str = "macos-vision-v1";

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

pub const EXTRACTOR_PRESETS: &[ExtractorPreset] = &[ExtractorPreset {
    stable_ref: APPLE_VISION_OCR_REF,
    name: "Apple Vision OCR",
    description: "Extracts searchable text from images locally with Apple Vision.",
    engine: APPLE_VISION_ENGINE,
    input_contract: "image",
    output_contract: "searchable_text",
    priority: 10,
}];

pub fn availability(engine: &str) -> (bool, Option<String>) {
    match engine {
        APPLE_VISION_ENGINE if cfg!(target_os = "macos") => (true, None),
        APPLE_VISION_ENGINE => (
            false,
            Some("Apple Vision is available only on macOS.".to_string()),
        ),
        _ => (
            false,
            Some("This extraction engine is not installed.".to_string()),
        ),
    }
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
    if input.input_contract != "image" || input.output_contract != "searchable_text" {
        return Err("This version supports only image → searchable_text Extractors".to_string());
    }
    Ok(())
}
