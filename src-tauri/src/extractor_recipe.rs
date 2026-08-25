use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::content_extraction::{ExtractionFailure, ExtractionOutcome};

mod diagnostics;
mod local_configuration;
mod runtime_status;
pub use diagnostics::{
    diagnose, ExtractorDiagnosticCode, ExtractorDiagnosticIssue, ExtractorDiagnosticReport,
};
pub use local_configuration::{reset_preserving_local_paths, without_local_paths};
pub use runtime_status::{runtime_status, runtime_status_summary};

pub const EXTRACTOR_RECIPE_VERSION: u32 = 1;
pub const EXTRACTOR_AUTHORING_VERSION: u32 = 1;
pub const DEFAULT_MINIMUM_VISUAL_LABEL_CONFIDENCE: u8 = 80;
const MAX_ARGUMENTS: usize = 128;
const MAX_ARGUMENT_BYTES: usize = 4_096;
const MAX_STEPS: usize = 16;
const MAX_RESOURCES: usize = 32;
const MAX_ACCEPTED_FILE_FORMATS: usize = 64;
const MAX_TRANSCRIPT_MESSAGES: usize = 256;
const MAX_TRANSCRIPT_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ExtractorInputKind {
    Image,
    FileReferences,
}

impl ExtractorInputKind {
    pub fn from_legacy(value: &str) -> Option<Self> {
        match value {
            "image" => Some(Self::Image),
            "file_references" => Some(Self::FileReferences),
            _ => None,
        }
    }

    pub const fn stable_name(&self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::FileReferences => "file_references",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractorOutputKind {
    SearchableText,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractorCapture {
    Ignore,
    StdoutText,
    FileText,
    PastedJsonV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractorStepMode {
    Once,
    EachInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractorExecutable {
    pub path: Option<String>,
    #[serde(default)]
    pub discover: Vec<String>,
    #[serde(default)]
    pub version_arguments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractorCommandStep {
    pub id: String,
    pub executable: ExtractorExecutable,
    #[serde(default)]
    pub arguments: Vec<String>,
    pub mode: ExtractorStepMode,
    pub capture: ExtractorCapture,
    #[serde(default)]
    pub output_extension: Option<String>,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractorResourceKind {
    File,
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractorResource {
    pub id: String,
    pub label: String,
    pub kind: ExtractorResourceKind,
    pub required: bool,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractorRecipe {
    pub definition_version: u32,
    pub accepts: Vec<ExtractorInputKind>,
    #[serde(default = "default_accepted_file_formats")]
    pub accepted_file_formats: Vec<String>,
    #[serde(default = "default_minimum_visual_label_confidence")]
    pub minimum_visual_label_confidence: u8,
    pub output: ExtractorOutputKind,
    #[serde(default)]
    pub steps: Vec<ExtractorCommandStep>,
    #[serde(default)]
    pub resources: Vec<ExtractorResource>,
}

fn default_accepted_file_formats() -> Vec<String> {
    vec!["*".into()]
}

const fn default_minimum_visual_label_confidence() -> u8 {
    DEFAULT_MINIMUM_VISUAL_LABEL_CONFIDENCE
}

impl ExtractorRecipe {
    pub fn hash(&self) -> Result<String, String> {
        let bytes = serde_json::to_vec(self).map_err(|error| error.to_string())?;
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Ok(crate::hashing::finalize_sha256_hex(hasher))
    }

    pub fn accepts(&self, kind: ExtractorInputKind) -> bool {
        self.accepts.contains(&kind)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractorAuthoringSource {
    Ai,
    Manual,
    Shipped,
    Migrated,
}

impl ExtractorAuthoringSource {
    pub const fn stable_name(&self) -> &'static str {
        match self {
            Self::Ai => "ai",
            Self::Manual => "manual",
            Self::Shipped => "shipped",
            Self::Migrated => "migrated",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ai" => Some(Self::Ai),
            "manual" => Some(Self::Manual),
            "shipped" => Some(Self::Shipped),
            "migrated" => Some(Self::Migrated),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractorAuthoringRole {
    User,
    Assistant,
    Tool,
    System,
}

impl ExtractorAuthoringRole {
    pub const fn stable_name(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
            Self::System => "system",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "user" => Some(Self::User),
            "assistant" => Some(Self::Assistant),
            "tool" => Some(Self::Tool),
            "system" => Some(Self::System),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractorAuthoringMessage {
    pub role: ExtractorAuthoringRole,
    pub content: String,
    pub created_at: String,
    pub structured_content: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractorAuthoringManifest {
    pub manifest_version: u32,
    pub source: ExtractorAuthoringSource,
    pub original_prompt: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    #[serde(default)]
    pub messages: Vec<ExtractorAuthoringMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractorRecipeDefinitionInput {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub priority: i64,
    pub recipe: ExtractorRecipe,
    pub authoring: Option<ExtractorAuthoringManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractorAuthoringSession {
    pub id: i64,
    pub extractor_id: i64,
    pub source: ExtractorAuthoringSource,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub original_prompt: Option<String>,
    pub created_at: String,
    pub messages: Vec<ExtractorAuthoringMessage>,
}

pub fn validate_recipe(recipe: &ExtractorRecipe) -> Result<(), String> {
    if recipe.definition_version != EXTRACTOR_RECIPE_VERSION {
        return Err("Unsupported Extractor recipe version".into());
    }
    if recipe.accepts.is_empty() {
        return Err("Extractor recipes require at least one input".into());
    }
    let unique_inputs = recipe.accepts.iter().collect::<HashSet<_>>();
    if unique_inputs.len() != recipe.accepts.len() {
        return Err("Extractor recipe inputs must be unique".into());
    }
    if recipe.accepted_file_formats.is_empty()
        || recipe.accepted_file_formats.len() > MAX_ACCEPTED_FILE_FORMATS
    {
        return Err(format!(
            "Extractor recipes require 1–{MAX_ACCEPTED_FILE_FORMATS} accepted file formats"
        ));
    }
    let mut formats = HashSet::new();
    for format in &recipe.accepted_file_formats {
        if format != "*"
            && (format.is_empty()
                || format.len() > 16
                || !format
                    .chars()
                    .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit()))
        {
            return Err(
                "Accepted file formats require lowercase letters or numbers without a dot".into(),
            );
        }
        if !formats.insert(format.as_str()) {
            return Err("Accepted file formats must be unique".into());
        }
    }
    if recipe.accepted_file_formats.len() > 1
        && recipe
            .accepted_file_formats
            .iter()
            .any(|format| format == "*")
    {
        return Err("The any-format selector cannot be combined with specific formats".into());
    }
    if recipe.minimum_visual_label_confidence > 100 {
        return Err("Minimum Visual Label confidence must be between 0 and 100".into());
    }
    if recipe.steps.is_empty() || recipe.steps.len() > MAX_STEPS {
        return Err(format!(
            "Extractor recipes require 1–{MAX_STEPS} command steps"
        ));
    }
    if recipe.resources.len() > MAX_RESOURCES {
        return Err(format!(
            "Extractor recipes support up to {MAX_RESOURCES} resources"
        ));
    }

    let mut ids = HashSet::new();
    for resource in &recipe.resources {
        validate_identifier(&resource.id, "resource")?;
        if !ids.insert(resource.id.as_str()) {
            return Err("Extractor resource identifiers must be unique".into());
        }
        if resource.label.trim().is_empty() || resource.label.len() > 80 {
            return Err("Extractor resource labels require 1–80 characters".into());
        }
        validate_optional_path(resource.path.as_deref(), "resource")?;
    }

    ids.clear();
    for step in &recipe.steps {
        validate_identifier(&step.id, "step")?;
        if !ids.insert(step.id.as_str()) {
            return Err("Extractor step identifiers must be unique".into());
        }
        validate_optional_path(step.executable.path.as_deref(), "executable")?;
        if step.executable.path.is_none() && step.executable.discover.is_empty() {
            return Err(
                "Extractor command steps require an executable path or discovery name".into(),
            );
        }
        for candidate in &step.executable.discover {
            if candidate.trim().is_empty()
                || candidate.len() > 256
                || candidate.contains('\0')
                || candidate.contains('/')
                || candidate.contains('\\')
            {
                return Err(
                    "Extractor executable discovery names must be plain command names".into(),
                );
            }
        }
        validate_arguments(&step.arguments)?;
        validate_arguments(&step.executable.version_arguments)?;
        if step.output_extension.as_deref().is_some_and(|extension| {
            extension.is_empty()
                || extension.len() > 16
                || !extension
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
        }) {
            return Err("Extractor output extensions require 1–16 letters or numbers".into());
        }
        if !(1..=600).contains(&step.timeout_seconds) {
            return Err("Extractor command time limits must be between 1 and 600 seconds".into());
        }
    }
    Ok(())
}

pub fn validate_authoring_manifest(manifest: &ExtractorAuthoringManifest) -> Result<(), String> {
    if manifest.manifest_version != EXTRACTOR_AUTHORING_VERSION {
        return Err("Unsupported Extractor authoring manifest version".into());
    }
    if manifest.messages.len() > MAX_TRANSCRIPT_MESSAGES {
        return Err(format!(
            "Extractor authoring history supports up to {MAX_TRANSCRIPT_MESSAGES} messages"
        ));
    }
    let mut bytes = manifest
        .original_prompt
        .as_deref()
        .map(str::len)
        .unwrap_or(0);
    if manifest
        .provider
        .as_deref()
        .is_some_and(|value| value.is_empty() || value.len() > 160)
        || manifest
            .model
            .as_deref()
            .is_some_and(|value| value.is_empty() || value.len() > 240)
    {
        return Err("Extractor authoring provider metadata exceeds its limit".into());
    }
    for message in &manifest.messages {
        bytes = bytes.saturating_add(message.content.len());
        if let Some(structured) = &message.structured_content {
            bytes = bytes.saturating_add(
                serde_json::to_vec(structured)
                    .map_err(|_| "Extractor authoring data is invalid".to_string())?
                    .len(),
            );
        }
        chrono::DateTime::parse_from_rfc3339(&message.created_at)
            .map_err(|_| "Extractor authoring timestamps must use RFC 3339".to_string())?;
    }
    if bytes > MAX_TRANSCRIPT_BYTES {
        return Err("Extractor authoring history exceeds the 1 MB limit".into());
    }
    Ok(())
}

pub fn canonicalize_authoring_manifest(
    manifest: &ExtractorAuthoringManifest,
) -> Result<ExtractorAuthoringManifest, String> {
    validate_authoring_manifest(manifest)?;
    let mut canonical = manifest.clone();
    for message in &mut canonical.messages {
        let timestamp = chrono::DateTime::parse_from_rfc3339(&message.created_at)
            .map_err(|_| "Extractor authoring timestamps must use RFC 3339".to_string())?;
        message.created_at = timestamp
            .with_timezone(&chrono::Utc)
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    }
    Ok(canonical)
}

pub fn validate_definition(input: &ExtractorRecipeDefinitionInput) -> Result<(), String> {
    crate::content_extraction::validate_extractor_input(
        &crate::content_extraction::ExtractorInput {
            name: input.name.clone(),
            description: input.description.clone(),
            enabled: input.enabled,
            priority: input.priority,
        },
    )?;
    validate_recipe(&input.recipe)?;
    if let Some(authoring) = &input.authoring {
        validate_authoring_manifest(authoring)?;
    }
    Ok(())
}

pub fn availability(recipe: &ExtractorRecipe) -> crate::content_extraction::EngineAvailability {
    if let Err(reason) = validate_recipe(recipe) {
        return unavailable(reason);
    }
    for resource in &recipe.resources {
        if resource.required
            && resource
                .path
                .as_deref()
                .is_none_or(|path| !resource_path_is_available(resource, Path::new(path)))
        {
            return unavailable(format!(
                "{} is not configured or is unavailable.",
                resource.label
            ));
        }
    }
    for step in &recipe.steps {
        if resolve_executable(&step.executable).is_none() {
            return unavailable(format!("{} is not installed.", executable_label(step)));
        }
    }
    crate::content_extraction::EngineAvailability {
        is_available: true,
        unavailable_reason: None,
    }
}

fn resource_path_is_available(resource: &ExtractorResource, path: &Path) -> bool {
    path.metadata().is_ok_and(|metadata| match resource.kind {
        ExtractorResourceKind::File => metadata.is_file(),
        ExtractorResourceKind::Directory => metadata.is_dir(),
    })
}

pub fn execute_image(recipe: &ExtractorRecipe, image_bytes: &[u8]) -> ExtractionOutcome {
    execute_recipe(recipe, RecipeInput::Image(image_bytes))
}

pub fn execute_files(recipe: &ExtractorRecipe, paths: &[String]) -> ExtractionOutcome {
    execute_recipe(recipe, RecipeInput::Files(paths))
}

enum RecipeInput<'a> {
    Image(&'a [u8]),
    Files(&'a [String]),
}

fn execute_recipe(recipe: &ExtractorRecipe, input: RecipeInput<'_>) -> ExtractionOutcome {
    if let Err(message) = validate_recipe(recipe) {
        return failure("invalid_recipe", message);
    }
    let accepted = match input {
        RecipeInput::Image(_) => recipe.accepts(ExtractorInputKind::Image),
        RecipeInput::Files(_) => recipe.accepts(ExtractorInputKind::FileReferences),
    };
    if !accepted {
        return failure(
            "invalid_contract",
            "This Extractor does not accept that input.",
        );
    }
    let readiness = availability(recipe);
    if !readiness.is_available {
        return failure(
            "engine_unavailable",
            readiness
                .unavailable_reason
                .unwrap_or_else(|| "The Extractor is unavailable.".into()),
        );
    }
    let workspace = match crate::external_tools::PrivateWorkspace::create("extractor-recipe") {
        Ok(workspace) => workspace,
        Err(_) => {
            return failure(
                "workspace_error",
                "The extraction workspace could not be created.",
            )
        }
    };
    let staged_image = match input {
        RecipeInput::Image(bytes) => {
            let path = workspace.join("input.bin");
            if fs::write(&path, bytes).is_err() {
                return failure(
                    "workspace_error",
                    "The extraction input could not be staged.",
                );
            }
            Some(path)
        }
        RecipeInput::Files(_) => None,
    };
    let request_path = workspace.join("request.json");
    let request = match input {
        RecipeInput::Image(_) => serde_json::json!({
            "protocolVersion": 1,
            "input": { "kind": "image", "path": staged_image.as_ref() },
        }),
        RecipeInput::Files(paths) => serde_json::json!({
            "protocolVersion": 1,
            "input": {
                "kind": "file_references",
                "paths": paths.iter().take(crate::resource_limits::MAX_MEDIA_PROBE_FILES).collect::<Vec<_>>(),
            },
        }),
    };
    if serde_json::to_vec(&request)
        .ok()
        .and_then(|bytes| fs::write(&request_path, bytes).ok())
        .is_none()
    {
        return failure(
            "workspace_error",
            "The extraction request could not be staged.",
        );
    }

    let input_paths = match input {
        RecipeInput::Image(_) => staged_image.iter().cloned().collect::<Vec<_>>(),
        RecipeInput::Files(paths) => paths
            .iter()
            .take(crate::resource_limits::MAX_MEDIA_PROBE_FILES)
            .map(PathBuf::from)
            .collect::<Vec<_>>(),
    };
    let mut produced = Vec::new();
    let mut labels = Vec::new();
    let mut artifacts = HashMap::<(String, usize), PathBuf>::new();
    let mut failed_inputs = HashSet::new();
    let mut first_input_failure = None;
    for (step_index, step) in recipe.steps.iter().enumerate() {
        let Some(executable) = resolve_executable(&step.executable) else {
            return failure(
                "engine_unavailable",
                format!("{} is not installed.", executable_label(step)),
            );
        };
        let runs = step_runs(step, &input_paths);
        let isolates_input_failures = runs.len() > 1;
        for (run_index, input_path) in runs.into_iter().enumerate() {
            if isolates_input_failures && failed_inputs.contains(&run_index) {
                continue;
            }
            let extension = step
                .output_extension
                .as_deref()
                .map(|value| format!(".{value}"))
                .unwrap_or_default();
            let artifact_path =
                workspace.join(format!("step-{step_index}-{run_index}-artifact{extension}"));
            let stdout_path = workspace.join(format!("step-{step_index}-{run_index}.stdout"));
            let output = match fs::File::create(&stdout_path) {
                Ok(output) => output,
                Err(_) => {
                    return failure("workspace_error", "Extractor output could not be staged.")
                }
            };
            let arguments = match expand_arguments(
                &step.arguments,
                input_path,
                &request_path,
                &artifact_path,
                &artifacts,
                run_index,
                recipe,
            ) {
                Ok(arguments) => arguments,
                Err(message) => return failure("invalid_recipe", message),
            };
            let mut command = Command::new(&executable);
            command
                .args(arguments)
                .current_dir(workspace.join("."))
                .env_clear()
                .stdin(Stdio::null())
                .stdout(output)
                .stderr(Stdio::null());
            for name in ["PATH", "LANG", "LC_ALL", "SystemRoot", "WINDIR"] {
                if let Some(value) = std::env::var_os(name) {
                    command.env(name, value);
                }
            }
            let mut child = match command.spawn() {
                Ok(child) => child,
                Err(_) => {
                    return failure(
                        "engine_unavailable",
                        "The Extractor command could not be started.",
                    )
                }
            };
            let status = match crate::external_tools::wait_bounded(
                &mut child,
                Duration::from_secs(step.timeout_seconds),
            ) {
                Ok(status) => status,
                Err(crate::external_tools::ProcessWaitError::TimedOut) => {
                    return failure("engine_timeout", "The Extractor command timed out.")
                }
                Err(crate::external_tools::ProcessWaitError::Failed) => {
                    return failure("engine_failed", "The Extractor command failed.")
                }
            };
            if !status.success() {
                if isolates_input_failures {
                    failed_inputs.insert(run_index);
                    first_input_failure.get_or_insert_with(|| ExtractionFailure {
                        code: "engine_failed".into(),
                        message: "Extractor failed.".into(),
                    });
                    continue;
                }
                return failure("engine_failed", "Extractor failed.");
            }
            artifacts.insert((step.id.clone(), run_index), artifact_path.clone());
            let captured_path = match step.capture {
                ExtractorCapture::FileText => Some(&artifact_path),
                ExtractorCapture::StdoutText | ExtractorCapture::PastedJsonV1 => Some(&stdout_path),
                ExtractorCapture::Ignore => None,
            };
            let output = if let Some(captured_path) = captured_path {
                let size = captured_path
                    .metadata()
                    .map(|metadata| metadata.len())
                    .unwrap_or(0);
                if size > crate::resource_limits::MAX_OCR_TEXT_BYTES as u64 + 4_096 {
                    return failure(
                        "output_too_large",
                        "Extractor output exceeds the supported size limit.",
                    );
                }
                fs::read_to_string(captured_path).unwrap_or_default()
            } else {
                String::new()
            };
            match step.capture {
                ExtractorCapture::Ignore => {}
                ExtractorCapture::StdoutText | ExtractorCapture::FileText => {
                    if !output.trim().is_empty() {
                        produced.push(output.trim().to_string());
                    }
                }
                ExtractorCapture::PastedJsonV1 => {
                    match serde_json::from_str::<serde_json::Value>(&output) {
                        Ok(value) => {
                            match crate::content_extraction::parse_visual_label_json_fields(&value)
                            {
                                Ok((text, mut parsed_labels)) => {
                                    produced.extend(text);
                                    labels.append(&mut parsed_labels);
                                }
                                Err(message) => return failure("invalid_output", message),
                            }
                        }
                        Err(_) => {
                            return failure(
                                "invalid_output",
                                "The Extractor must return a JSON object.",
                            )
                        }
                    }
                }
            }
        }
    }
    labels = visual_labels_meeting_confidence(labels, recipe.minimum_visual_label_confidence);
    if produced.is_empty() && labels.is_empty() {
        first_input_failure
            .map(|failure| ExtractionOutcome::Failed { failure })
            .unwrap_or(ExtractionOutcome::NoOutput)
    } else {
        let text = produced.join("\n");
        if text.len() > crate::resource_limits::MAX_OCR_TEXT_BYTES {
            failure(
                "output_too_large",
                "Extracted text exceeds the supported size limit.",
            )
        } else {
            ExtractionOutcome::Produced { text, labels }
        }
    }
}

fn visual_labels_meeting_confidence(
    labels: Vec<crate::content_extraction::VisualLabel>,
    minimum_confidence_percent: u8,
) -> Vec<crate::content_extraction::VisualLabel> {
    let minimum_confidence = u16::from(minimum_confidence_percent) * 100;
    crate::content_extraction::normalize_visual_labels(labels)
        .into_iter()
        .filter(|label| {
            label
                .confidence_basis_points
                .is_none_or(|confidence| confidence >= minimum_confidence)
        })
        .collect()
}

fn step_runs<'a>(step: &ExtractorCommandStep, input_paths: &'a [PathBuf]) -> Vec<Option<&'a Path>> {
    let uses_singular_input = step.arguments.iter().any(|argument| {
        argument.contains("{input.path}") || argument.contains("{input.stagedPath}")
    });
    if step.mode == ExtractorStepMode::EachInput || uses_singular_input && input_paths.len() > 1 {
        input_paths
            .iter()
            .map(|path| Some(path.as_path()))
            .collect()
    } else {
        vec![input_paths.first().map(PathBuf::as_path)]
    }
}

fn resolve_executable(executable: &ExtractorExecutable) -> Option<PathBuf> {
    executable
        .path
        .as_deref()
        .map(PathBuf::from)
        .filter(|path| crate::external_tools::is_executable(path))
        .or_else(|| {
            executable.discover.iter().find_map(|name| {
                if name == crate::content_extraction::BUNDLED_EXTRACTOR_EXECUTABLE {
                    #[cfg(target_os = "macos")]
                    {
                        return std::env::current_exe().ok();
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        return None;
                    }
                }
                crate::external_tools::find_executable(name, &[])
            })
        })
}

fn executable_label(step: &ExtractorCommandStep) -> String {
    step.executable
        .discover
        .first()
        .cloned()
        .or_else(|| {
            step.executable
                .path
                .as_deref()
                .map(Path::new)
                .and_then(Path::file_name)
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| step.id.clone())
}

fn expand_arguments(
    arguments: &[String],
    input_path: Option<&Path>,
    request_path: &Path,
    output_path: &Path,
    artifacts: &HashMap<(String, usize), PathBuf>,
    run_index: usize,
    recipe: &ExtractorRecipe,
) -> Result<Vec<String>, String> {
    arguments
        .iter()
        .map(|argument| {
            let mut value = argument
                .replace("{request.path}", &request_path.to_string_lossy())
                .replace("{output.path}", &output_path.to_string_lossy());
            if value.contains("{output.base}") {
                let output_base = output_path.with_extension("");
                value = value.replace("{output.base}", &output_base.to_string_lossy());
            }
            if value.contains("{input.path}") || value.contains("{input.stagedPath}") {
                let path = input_path
                    .ok_or_else(|| "Extractor argument requires an input path".to_string())?;
                value = value
                    .replace("{input.path}", &path.to_string_lossy())
                    .replace("{input.stagedPath}", &path.to_string_lossy());
            }
            for resource in &recipe.resources {
                let token = format!("{{resource.{}.path}}", resource.id);
                if value.contains(&token) {
                    let path = resource.path.as_deref().ok_or_else(|| {
                        format!("Extractor resource {} is not configured", resource.id)
                    })?;
                    value = value.replace(&token, path);
                }
            }
            for prior_step in &recipe.steps {
                let token = format!("{{step.{}.output}}", prior_step.id);
                if value.contains(&token) {
                    let path = artifacts
                        .get(&(prior_step.id.clone(), run_index))
                        .or_else(|| artifacts.get(&(prior_step.id.clone(), 0)))
                        .ok_or_else(|| {
                            format!(
                                "Extractor argument references unavailable step {}",
                                prior_step.id
                            )
                        })?;
                    value = value.replace(&token, &path.to_string_lossy());
                }
            }
            if value.contains('{') || value.contains('}') {
                return Err("Extractor argument contains an unsupported placeholder".into());
            }
            Ok(value)
        })
        .collect()
}

fn unavailable(reason: impl Into<String>) -> crate::content_extraction::EngineAvailability {
    crate::content_extraction::EngineAvailability {
        is_available: false,
        unavailable_reason: Some(reason.into()),
    }
}

fn failure(code: &str, message: impl Into<String>) -> ExtractionOutcome {
    ExtractionOutcome::Failed {
        failure: ExtractionFailure {
            code: code.into(),
            message: message.into(),
        },
    }
}

fn validate_identifier(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        return Err(format!(
            "Extractor {label} identifiers require 1–64 letters, numbers, hyphens, or underscores"
        ));
    }
    Ok(())
}

fn validate_optional_path(value: Option<&str>, label: &str) -> Result<(), String> {
    if let Some(value) = value {
        if value.is_empty() || value.len() > 4_096 || value.contains('\0') {
            return Err(format!(
                "Extractor {label} paths require 1–4,096 characters"
            ));
        }
        if !Path::new(value).is_absolute() {
            return Err(format!("Extractor {label} paths must be absolute"));
        }
    }
    Ok(())
}

fn validate_arguments(arguments: &[String]) -> Result<(), String> {
    if arguments.len() > MAX_ARGUMENTS {
        return Err(format!(
            "Extractor commands support up to {MAX_ARGUMENTS} arguments"
        ));
    }
    for argument in arguments {
        if argument.len() > MAX_ARGUMENT_BYTES || argument.contains('\0') {
            return Err("Extractor command arguments exceed the supported limit".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipe() -> ExtractorRecipe {
        ExtractorRecipe {
            definition_version: EXTRACTOR_RECIPE_VERSION,
            accepts: vec![
                ExtractorInputKind::Image,
                ExtractorInputKind::FileReferences,
            ],
            accepted_file_formats: vec!["*".into()],
            minimum_visual_label_confidence: DEFAULT_MINIMUM_VISUAL_LABEL_CONFIDENCE,
            output: ExtractorOutputKind::SearchableText,
            steps: vec![ExtractorCommandStep {
                id: "extract".into(),
                executable: ExtractorExecutable {
                    path: None,
                    discover: vec!["example-extractor".into()],
                    version_arguments: vec!["--version".into()],
                },
                arguments: vec!["{input.path}".into()],
                mode: ExtractorStepMode::EachInput,
                capture: ExtractorCapture::StdoutText,
                output_extension: None,
                timeout_seconds: 60,
            }],
            resources: Vec::new(),
        }
    }

    #[test]
    fn legacy_recipes_accept_any_file_format() {
        let value = serde_json::json!({
            "definitionVersion": 1,
            "accepts": ["file_references"],
            "output": "searchable_text",
            "steps": recipe().steps,
            "resources": []
        });
        let parsed: ExtractorRecipe = serde_json::from_value(value).unwrap();
        assert_eq!(parsed.accepted_file_formats, ["*"]);
        assert_eq!(
            parsed.minimum_visual_label_confidence,
            DEFAULT_MINIMUM_VISUAL_LABEL_CONFIDENCE
        );
    }

    #[test]
    fn visual_label_confidence_defaults_to_a_conservative_floor() {
        let labels = vec![
            crate::content_extraction::VisualLabel {
                value: "dog".into(),
                confidence_basis_points: Some(8_000),
            },
            crate::content_extraction::VisualLabel {
                value: "terrier".into(),
                confidence_basis_points: Some(7_999),
            },
            crate::content_extraction::VisualLabel {
                value: "favorite".into(),
                confidence_basis_points: None,
            },
        ];

        let accepted =
            visual_labels_meeting_confidence(labels, DEFAULT_MINIMUM_VISUAL_LABEL_CONFIDENCE);

        assert_eq!(
            accepted
                .iter()
                .map(|label| label.value.as_str())
                .collect::<Vec<_>>(),
            ["dog", "favorite"]
        );
    }

    #[test]
    fn accepted_file_formats_are_bounded_and_unambiguous() {
        let mut candidate = recipe();
        candidate.accepted_file_formats = vec!["*".into(), "pdf".into()];
        assert!(validate_recipe(&candidate).is_err());
        candidate.accepted_file_formats = vec!["PDF".into()];
        assert!(validate_recipe(&candidate).is_err());
        candidate.accepted_file_formats = vec!["pdf".into(), "wav".into()];
        assert!(validate_recipe(&candidate).is_ok());
    }

    #[test]
    fn validates_multi_input_recipe_without_shell_strings() {
        let recipe = recipe();
        validate_recipe(&recipe).unwrap();
        assert!(recipe.accepts(ExtractorInputKind::Image));
        assert_eq!(recipe.hash().unwrap().len(), 64);
    }

    #[test]
    fn rejects_duplicate_inputs_and_discovery_paths() {
        let mut recipe = recipe();
        recipe.accepts.push(ExtractorInputKind::Image);
        assert!(validate_recipe(&recipe).is_err());
        recipe.accepts.pop();
        recipe.steps[0].executable.discover = vec!["/bin/tool".into()];
        assert!(validate_recipe(&recipe).is_err());
    }

    #[test]
    fn expands_step_artifacts_for_the_matching_input() {
        let mut recipe = recipe();
        recipe.steps.insert(
            0,
            ExtractorCommandStep {
                id: "prepare".into(),
                executable: ExtractorExecutable {
                    path: None,
                    discover: vec!["preparer".into()],
                    version_arguments: Vec::new(),
                },
                arguments: vec!["{input.path}".into(), "{output.path}".into()],
                mode: ExtractorStepMode::EachInput,
                capture: ExtractorCapture::Ignore,
                output_extension: Some("wav".into()),
                timeout_seconds: 30,
            },
        );
        let artifacts = HashMap::from([
            (
                ("prepare".to_string(), 0),
                PathBuf::from("/private/input-0.wav"),
            ),
            (
                ("prepare".to_string(), 1),
                PathBuf::from("/private/input-1.wav"),
            ),
        ]);
        let arguments = expand_arguments(
            &["{step.prepare.output}".into(), "{output.base}".into()],
            Some(Path::new("/input/two.m4a")),
            Path::new("/private/request.json"),
            Path::new("/private/transcript.txt"),
            &artifacts,
            1,
            &recipe,
        )
        .unwrap();
        assert_eq!(arguments, ["/private/input-1.wav", "/private/transcript"]);
    }

    #[test]
    fn singular_input_placeholders_fan_out_even_for_legacy_once_recipes() {
        let mut recipe = recipe();
        recipe.steps[0].mode = ExtractorStepMode::Once;
        let paths = vec![PathBuf::from("first.pdf"), PathBuf::from("second.pdf")];
        assert_eq!(step_runs(&recipe.steps[0], &paths).len(), 2);

        recipe.steps[0].arguments = vec!["{request.path}".into()];
        assert_eq!(step_runs(&recipe.steps[0], &paths).len(), 1);
    }
}
