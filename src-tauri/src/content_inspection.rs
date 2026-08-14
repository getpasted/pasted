use crate::analysis_contract::{
    AnalysisEnvelope, AnalysisFailure, AnalysisPolicy, AnalysisTargetKind,
};
use crate::content_analysis::{AnalysisInput, AnalysisRequest};
use image::ImageReader;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::Path;

pub const STRUCTURE_INSPECTOR_REF: &str = "inspector:structure-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginKind {
    ClipboardContent,
    FileReference,
    Screenshot,
    CommandLine,
}

impl OriginKind {
    pub const fn stable_name(self) -> &'static str {
        match self {
            Self::ClipboardContent => "clipboard_content",
            Self::FileReference => "file_reference",
            Self::Screenshot => "screenshot",
            Self::CommandLine => "command_line",
        }
    }
}

pub fn origin_kind(clip_kind: &str, source: Option<&str>) -> OriginKind {
    let source = source.unwrap_or_default().trim().to_ascii_lowercase();
    if matches!(clip_kind.to_ascii_lowercase().as_str(), "image" | "file")
        && (source.contains("screenshot")
            || source.contains("screencapture")
            || source.contains("cleanshot"))
    {
        return OriginKind::Screenshot;
    }
    if clip_kind.eq_ignore_ascii_case("file") {
        return OriginKind::FileReference;
    }
    if matches!(source.as_str(), "cli terminal" | "pasted cli") {
        return OriginKind::CommandLine;
    }
    OriginKind::ClipboardContent
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectorDefinition {
    pub stable_ref: String,
    pub name: String,
    pub description: String,
    pub input_contract: String,
    pub output_contract: String,
    pub priority: i64,
    pub is_builtin: bool,
}

pub fn structure_inspector_definition() -> InspectorDefinition {
    InspectorDefinition {
        stable_ref: STRUCTURE_INSPECTOR_REF.into(),
        name: "Structure".into(),
        description: "Measures stable clip structure without retaining clipboard contents.".into(),
        input_contract: "clip".into(),
        output_contract: "structural_metadata".into(),
        priority: 0,
        is_builtin: true,
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextStructure {
    pub character_count: usize,
    pub word_count: usize,
    pub line_count: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageStructure {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileStructure {
    pub item_count: usize,
    pub extensions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuralMetadata {
    pub origin: OriginKind,
    pub byte_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextStructure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<ImageStructure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<FileStructure>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileObservations {
    pub available_count: usize,
    pub file_count: usize,
    pub directory_count: usize,
    pub total_size_bytes: u64,
}

pub type InspectionResult = AnalysisEnvelope<StructuralMetadata>;

pub fn parse_file_paths(value: &str) -> Vec<String> {
    let parsed = serde_json::from_str::<serde_json::Value>(value).ok();
    let paths = match parsed {
        Some(serde_json::Value::Array(values)) => values
            .into_iter()
            .filter_map(|value| value.as_str().map(str::to_owned))
            .collect(),
        Some(serde_json::Value::String(path)) => vec![path],
        _ => value.lines().map(str::to_owned).collect(),
    };
    paths
        .into_iter()
        .filter_map(|path| {
            let path = path.trim();
            if path.is_empty() {
                return None;
            }
            if path.starts_with("file://") {
                return url::Url::parse(path)
                    .ok()
                    .and_then(|url| url.to_file_path().ok())
                    .map(|path| path.to_string_lossy().into_owned());
            }
            Some(path.to_string())
        })
        .collect()
}

pub fn observe_files(paths: &[String]) -> FileObservations {
    let mut observations = FileObservations::default();
    for path in paths {
        if let Ok(metadata) = std::fs::symlink_metadata(path) {
            observations.available_count += 1;
            if metadata.is_dir() {
                observations.directory_count += 1;
            } else {
                observations.file_count += 1;
                observations.total_size_bytes =
                    observations.total_size_bytes.saturating_add(metadata.len());
            }
        }
    }
    observations
}

pub(crate) fn inspect_input(input: &AnalysisInput) -> Result<StructuralMetadata, AnalysisFailure> {
    match input {
        AnalysisInput::Text { text, source } => Ok(StructuralMetadata {
            origin: origin_kind("text", source.as_deref()),
            byte_count: text.len(),
            text: Some(TextStructure {
                character_count: text.chars().count(),
                word_count: text.split_whitespace().count(),
                line_count: if text.is_empty() {
                    0
                } else {
                    text.lines().count()
                },
            }),
            image: None,
            files: None,
        }),
        AnalysisInput::Image {
            image_bytes,
            source,
            ..
        } => {
            let dimensions = ImageReader::new(Cursor::new(image_bytes))
                .with_guessed_format()
                .ok()
                .and_then(|reader| reader.into_dimensions().ok())
                .ok_or_else(|| AnalysisFailure {
                    code: "invalid_image".into(),
                    message: "Image dimensions could not be inspected.".into(),
                })?;
            if !crate::resource_limits::image_dimensions_within_limit(dimensions.0, dimensions.1) {
                return Err(AnalysisFailure {
                    code: "image_too_large".into(),
                    message: "Image dimensions exceed the supported safety limit.".into(),
                });
            }
            Ok(StructuralMetadata {
                origin: origin_kind("image", source.as_deref()),
                byte_count: image_bytes.len(),
                text: None,
                image: Some(ImageStructure {
                    width: dimensions.0,
                    height: dimensions.1,
                }),
                files: None,
            })
        }
        AnalysisInput::Files { paths, source } => {
            let mut extensions = Vec::new();
            for extension in paths
                .iter()
                .filter_map(|path| Path::new(path).extension())
                .filter_map(|extension| extension.to_str())
                .map(|extension| extension.to_ascii_uppercase())
                .filter(|extension| !extension.is_empty())
            {
                if !extensions.contains(&extension) {
                    extensions.push(extension);
                }
            }
            Ok(StructuralMetadata {
                origin: origin_kind("file", source.as_deref()),
                byte_count: paths
                    .iter()
                    .fold(0usize, |total, path| total.saturating_add(path.len())),
                text: None,
                image: None,
                files: Some(FileStructure {
                    item_count: paths.len(),
                    extensions,
                }),
            })
        }
    }
}

fn inspect(
    input: AnalysisInput,
    policy: AnalysisPolicy,
) -> Result<InspectionResult, AnalysisFailure> {
    let within_limit = match &input {
        AnalysisInput::Text { text, .. } => {
            text.len() <= crate::resource_limits::MAX_CLIP_TEXT_BYTES
        }
        AnalysisInput::Image { image_bytes, .. } => {
            image_bytes.len() <= crate::resource_limits::MAX_ENCODED_IMAGE_BYTES
        }
        AnalysisInput::Files { paths, .. } => crate::resource_limits::file_list_within_limit(paths),
    };
    if !within_limit {
        return Err(AnalysisFailure {
            code: "input_too_large".into(),
            message: "Inspection input exceeds the supported safety limit.".into(),
        });
    }
    let report = crate::content_analysis::analyze(AnalysisRequest {
        input,
        policy,
        inspector: true,
        extractor: None,
        detectors: None,
    });
    let resolution =
        report.resolve_participant(STRUCTURE_INSPECTOR_REF, AnalysisTargetKind::Inspector);
    if let Some(failure) = resolution.failure {
        return Err(failure);
    }
    let metadata = report
        .context
        .structural_metadata
        .ok_or_else(|| AnalysisFailure {
            code: "missing_output".into(),
            message: "Inspection completed without structural metadata.".into(),
        })?;
    Ok(AnalysisEnvelope::new(policy, metadata, report.runs))
}

pub fn inspect_text(text: &str, source: Option<&str>) -> Result<InspectionResult, AnalysisFailure> {
    inspect_text_with_policy(text, source, AnalysisPolicy::Interactive)
}

pub(crate) fn inspect_text_with_policy(
    text: &str,
    source: Option<&str>,
    policy: AnalysisPolicy,
) -> Result<InspectionResult, AnalysisFailure> {
    inspect(
        AnalysisInput::Text {
            text: text.into(),
            source: source.map(str::to_owned),
        },
        policy,
    )
}

pub fn inspect_image(
    image_bytes: Vec<u8>,
    source: Option<&str>,
) -> Result<InspectionResult, AnalysisFailure> {
    inspect_image_with_policy(image_bytes, source, AnalysisPolicy::Interactive)
}

pub(crate) fn inspect_image_with_policy(
    image_bytes: Vec<u8>,
    source: Option<&str>,
    policy: AnalysisPolicy,
) -> Result<InspectionResult, AnalysisFailure> {
    inspect(
        AnalysisInput::Image {
            image_bytes,
            searchable_text: None,
            source: source.map(str::to_owned),
        },
        policy,
    )
}

pub fn inspect_files(
    paths: Vec<String>,
    source: Option<&str>,
) -> Result<InspectionResult, AnalysisFailure> {
    inspect_files_with_policy(paths, source, AnalysisPolicy::Interactive)
}

pub(crate) fn inspect_files_with_policy(
    paths: Vec<String>,
    source: Option<&str>,
    policy: AnalysisPolicy,
) -> Result<InspectionResult, AnalysisFailure> {
    inspect(
        AnalysisInput::Files {
            paths,
            source: source.map(str::to_owned),
        },
        policy,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_inspection_has_a_versioned_content_free_contract() {
        let result = inspect_text("one two\nthree", Some("CLI Terminal")).unwrap();
        assert_eq!(result.result.origin, OriginKind::CommandLine);
        assert_eq!(result.result.byte_count, 13);
        assert_eq!(
            result.result.text.as_ref().unwrap(),
            &TextStructure {
                character_count: 13,
                word_count: 3,
                line_count: 2,
            }
        );
        let json = serde_json::to_string(&result).unwrap();
        assert!(!json.contains("one two"));
        assert!(json.contains(STRUCTURE_INSPECTOR_REF));
    }

    #[test]
    fn text_counts_use_utf8_bytes_unicode_scalars_and_rust_line_semantics() {
        let result = inspect_text("é 😀\n", None).unwrap();
        assert_eq!(result.result.byte_count, 8);
        assert_eq!(
            result.result.text.unwrap(),
            TextStructure {
                character_count: 4,
                word_count: 2,
                line_count: 1,
            }
        );
    }

    #[test]
    fn file_structure_is_stable_and_file_observations_are_separate() {
        let paths = vec!["/missing/B.txt".into(), "/missing/a.TXT".into()];
        let result = inspect_files(paths.clone(), Some("Finder")).unwrap();
        assert_eq!(result.result.origin, OriginKind::FileReference);
        assert_eq!(result.result.files.unwrap().extensions, vec!["TXT"]);
        assert_eq!(observe_files(&paths), FileObservations::default());
    }

    #[test]
    fn screenshot_sources_are_consistent_across_image_and_file_clips() {
        assert_eq!(
            origin_kind("image", Some("CleanShot X")),
            OriginKind::Screenshot
        );
        assert_eq!(
            origin_kind("file", Some("screencapture")),
            OriginKind::Screenshot
        );
    }
}
