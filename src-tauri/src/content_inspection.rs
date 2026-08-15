use crate::analysis_contract::{AnalysisEnvelope, AnalysisFailure};
use crate::content_analysis::AnalysisInput;
use image::ImageReader;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const STRUCTURE_INSPECTOR_REF: &str = "inspector:structure-v1";
pub const MEDIA_INSPECTOR_REF: &str = "inspector:ffprobe-media-v1";
pub const FFPROBE_ENGINE: &str = "ffprobe-cli-v1";

const FFPROBE_TIMEOUT: Duration = Duration::from_secs(5);

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
    pub engine: Option<String>,
    pub is_available: bool,
    pub unavailable_reason: Option<String>,
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
        engine: None,
        is_available: true,
        unavailable_reason: None,
    }
}

pub fn media_inspector_definition() -> InspectorDefinition {
    let executable = find_ffprobe_executable();
    InspectorDefinition {
        stable_ref: MEDIA_INSPECTOR_REF.into(),
        name: "Media Metadata".into(),
        description: "Reads bounded audio and video metadata locally with ffprobe.".into(),
        input_contract: "file_references".into(),
        output_contract: "media_metadata".into(),
        priority: 10,
        is_builtin: true,
        engine: Some(FFPROBE_ENGINE.into()),
        is_available: executable.is_some(),
        unavailable_reason: executable
            .is_none()
            .then(|| "ffprobe is not installed. Install FFmpeg, then check again.".into()),
    }
}

pub fn inspector_definitions() -> Vec<InspectorDefinition> {
    vec![
        structure_inspector_definition(),
        media_inspector_definition(),
    ]
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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaMetadata {
    pub examined_file_count: usize,
    pub media_file_count: usize,
    pub audio_stream_count: usize,
    pub video_stream_count: usize,
    pub total_duration_ms: u64,
    pub containers: Vec<String>,
    pub codecs: Vec<String>,
}

#[derive(Deserialize)]
struct FfprobeDocument {
    #[serde(default)]
    streams: Vec<FfprobeStream>,
    format: Option<FfprobeFormat>,
}

#[derive(Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
}

#[derive(Deserialize)]
struct FfprobeFormat {
    format_name: Option<String>,
    duration: Option<String>,
}

fn find_ffprobe_executable() -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    let (name, explicit) = (
        "ffprobe.exe",
        &[
            r"C:\Program Files\ffmpeg\bin\ffprobe.exe",
            r"C:\ffmpeg\bin\ffprobe.exe",
        ][..],
    );
    #[cfg(not(windows))]
    let (name, explicit) = (
        "ffprobe",
        &[
            "/opt/homebrew/bin/ffprobe",
            "/usr/local/bin/ffprobe",
            "/usr/bin/ffprobe",
            "/home/linuxbrew/.linuxbrew/bin/ffprobe",
        ][..],
    );
    crate::external_tools::find_executable(name, explicit)
}

fn push_unique_bounded(values: &mut Vec<String>, value: &str) {
    for value in value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if values.len() >= 32 {
            break;
        }
        let value = value.chars().take(64).collect::<String>();
        if !values.contains(&value) {
            values.push(value);
        }
    }
}

pub(crate) fn inspect_media_paths(
    paths: &[String],
) -> Result<Option<MediaMetadata>, AnalysisFailure> {
    let executable = find_ffprobe_executable().ok_or_else(|| AnalysisFailure {
        code: "engine_unavailable".into(),
        message: "ffprobe is not installed. Install FFmpeg, then check again.".into(),
    })?;
    let workspace =
        crate::external_tools::PrivateWorkspace::create("media-inspector").map_err(|_| {
            AnalysisFailure {
                code: "workspace_error".into(),
                message: "A private media inspection workspace could not be created.".into(),
            }
        })?;
    let mut result = MediaMetadata::default();
    let started = Instant::now();
    for (index, path) in paths
        .iter()
        .filter(|path| Path::new(path).is_file())
        .take(crate::resource_limits::MAX_MEDIA_PROBE_FILES)
        .enumerate()
    {
        result.examined_file_count += 1;
        let output_path = workspace.join(format!("probe-{index}.json"));
        let output = fs::File::create(&output_path).map_err(|_| AnalysisFailure {
            code: "workspace_error".into(),
            message: "Media inspection output could not be staged.".into(),
        })?;
        let mut child = match Command::new(&executable)
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=format_name,duration:stream=codec_type,codec_name",
                "-of",
                "json",
            ])
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::from(output))
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(_) => {
                return Err(AnalysisFailure {
                    code: "engine_unavailable".into(),
                    message: "ffprobe could not be started.".into(),
                });
            }
        };
        let remaining = FFPROBE_TIMEOUT.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AnalysisFailure {
                code: "engine_timeout".into(),
                message: "ffprobe exceeded the media inspection time limit.".into(),
            });
        }
        let status = crate::external_tools::wait_bounded(&mut child, remaining).map_err(
            |error| match error {
                crate::external_tools::ProcessWaitError::TimedOut => AnalysisFailure {
                    code: "engine_timeout".into(),
                    message: "ffprobe exceeded the media inspection time limit.".into(),
                },
                crate::external_tools::ProcessWaitError::Failed => AnalysisFailure {
                    code: "engine_failed".into(),
                    message: "ffprobe did not complete successfully.".into(),
                },
            },
        )?;
        if !status.success() {
            continue;
        }
        let metadata = match output_path.metadata() {
            Ok(metadata)
                if metadata.len() <= crate::resource_limits::MAX_MEDIA_PROBE_OUTPUT_BYTES =>
            {
                metadata
            }
            Ok(_) => {
                return Err(AnalysisFailure {
                    code: "output_too_large".into(),
                    message: "ffprobe output exceeds the supported size limit.".into(),
                });
            }
            Err(_) => continue,
        };
        if metadata.len() == 0 {
            continue;
        }
        let document = match fs::read(&output_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<FfprobeDocument>(&bytes).ok())
        {
            Some(document) => document,
            None => continue,
        };
        if document.streams.is_empty() && document.format.is_none() {
            continue;
        }
        result.media_file_count += 1;
        for stream in document.streams {
            match stream.codec_type.as_deref() {
                Some("audio") => result.audio_stream_count += 1,
                Some("video") => result.video_stream_count += 1,
                _ => {}
            }
            if let Some(codec) = stream.codec_name.as_deref() {
                push_unique_bounded(&mut result.codecs, codec);
            }
        }
        if let Some(format) = document.format {
            if let Some(container) = format.format_name.as_deref() {
                push_unique_bounded(&mut result.containers, container);
            }
            if let Some(duration_ms) = format
                .duration
                .as_deref()
                .and_then(|duration| duration.parse::<f64>().ok())
                .filter(|duration| duration.is_finite() && *duration >= 0.0)
                .map(|duration| (duration * 1_000.0).round() as u64)
            {
                result.total_duration_ms = result.total_duration_ms.saturating_add(duration_ms);
            }
        }
    }
    Ok((result.media_file_count > 0).then_some(result))
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

pub fn inspect_text(text: &str, source: Option<&str>) -> Result<InspectionResult, AnalysisFailure> {
    crate::inspection_execution::inspect_text(text, source)
}

pub fn inspect_image(
    image_bytes: Vec<u8>,
    source: Option<&str>,
) -> Result<InspectionResult, AnalysisFailure> {
    crate::inspection_execution::inspect_image(image_bytes, source)
}

pub fn inspect_files(
    paths: Vec<String>,
    source: Option<&str>,
) -> Result<InspectionResult, AnalysisFailure> {
    crate::inspection_execution::inspect_files(paths, source)
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

    #[test]
    fn ffprobe_definition_reports_the_discovered_runtime() {
        let definition = media_inspector_definition();
        assert_eq!(definition.engine.as_deref(), Some(FFPROBE_ENGINE));
        assert_eq!(definition.is_available, find_ffprobe_executable().is_some());
        assert_eq!(
            definition.unavailable_reason.is_none(),
            definition.is_available
        );
    }

    #[test]
    fn ffprobe_inspects_bounded_audio_metadata_without_exposing_its_path() {
        if find_ffprobe_executable().is_none() {
            return;
        }
        let workspace = crate::external_tools::PrivateWorkspace::create("ffprobe-test").unwrap();
        let path = workspace.join("private-recording.wav");
        let sample_count = 8_000u32;
        let mut wav = Vec::with_capacity(44 + sample_count as usize);
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + sample_count).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&8_000u32.to_le_bytes());
        wav.extend_from_slice(&8_000u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&8u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&sample_count.to_le_bytes());
        wav.resize(44 + sample_count as usize, 128);
        fs::write(&path, wav).unwrap();

        let metadata = inspect_media_paths(&[path.to_string_lossy().into_owned()])
            .unwrap()
            .expect("WAV metadata");
        assert_eq!(metadata.examined_file_count, 1);
        assert_eq!(metadata.media_file_count, 1);
        assert_eq!(metadata.audio_stream_count, 1);
        assert_eq!(metadata.video_stream_count, 0);
        assert!((990..=1_010).contains(&metadata.total_duration_ms));
        assert!(metadata.codecs.iter().any(|codec| codec == "pcm_u8"));
        assert!(!serde_json::to_string(&metadata)
            .unwrap()
            .contains("private-recording"));
    }
}
