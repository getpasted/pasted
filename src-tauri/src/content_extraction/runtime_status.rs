use std::path::Path;

use super::engine_runtime::{
    configured_or_discovered_executable, find_ffmpeg_executable, find_tesseract_executable,
    find_whisper_cpp_executable,
};
use super::*;

pub fn engine_availability(engine: &str) -> EngineAvailability {
    super::system_engine_registry().availability(engine)
}

pub fn engine_availability_for(
    engine: &str,
    executable_path: Option<&str>,
    model_path: Option<&str>,
) -> EngineAvailability {
    super::system_engine_registry().availability_for(
        engine,
        executable_path.map(Path::new),
        model_path.map(Path::new),
    )
}

pub fn inspect_extractor_runtime(extractor: &Extractor) -> ExtractorRuntimeStatus {
    if extractor.engine == RECIPE_ENGINE {
        crate::extractor_recipe::runtime_status(&extractor.recipe)
    } else {
        runtime_status_for(&extractor.engine, extractor.executable_path.as_deref())
    }
}

pub fn runtime_status_for(engine: &str, executable_path: Option<&str>) -> ExtractorRuntimeStatus {
    runtime_status_for_mode(engine, executable_path, true)
}

pub fn runtime_status_summary_for(
    engine: &str,
    executable_path: Option<&str>,
) -> ExtractorRuntimeStatus {
    runtime_status_for_mode(engine, executable_path, false)
}

fn runtime_status_for_mode(
    engine: &str,
    executable_path: Option<&str>,
    probe_versions: bool,
) -> ExtractorRuntimeStatus {
    let configured = executable_path.map(Path::new);
    match engine {
        APPLE_VISION_ENGINE => ExtractorRuntimeStatus {
            method: "system".into(),
            location: Some("macOS Vision framework".into()),
            version: probe_versions.then(apple_vision_runtime_version).flatten(),
            uses_automatic_discovery: false,
            dependencies: Vec::new(),
        },
        TESSERACT_ENGINE => command_runtime(
            configured_or_discovered_executable(configured, find_tesseract_executable),
            configured.is_none(),
            &["--version"],
            probe_versions,
            Vec::new(),
        ),
        WHISPER_CPP_ENGINE => command_runtime(
            configured_or_discovered_executable(configured, find_whisper_cpp_executable),
            configured.is_none(),
            &["--version"],
            probe_versions,
            vec![runtime_dependency(
                "FFmpeg",
                find_ffmpeg_executable(),
                &["-version"],
                "FFmpeg is not installed. M4A and AAC audio cannot be prepared.",
                probe_versions,
            )],
        ),
        CUSTOM_COMMAND_ENGINE => command_runtime(
            configured
                .filter(|path| crate::external_tools::is_executable(path))
                .map(Path::to_path_buf),
            false,
            &["--version"],
            probe_versions,
            Vec::new(),
        ),
        _ => ExtractorRuntimeStatus {
            method: "unregistered".into(),
            location: executable_path.map(str::to_string),
            version: None,
            uses_automatic_discovery: false,
            dependencies: Vec::new(),
        },
    }
}

fn command_runtime(
    path: Option<std::path::PathBuf>,
    uses_automatic_discovery: bool,
    version_arguments: &[&str],
    probe_versions: bool,
    dependencies: Vec<ExtractorRuntimeDependency>,
) -> ExtractorRuntimeStatus {
    let version = probe_versions
        .then(|| {
            path.as_deref()
                .and_then(|path| crate::external_tools::probe_version(path, version_arguments))
        })
        .flatten();
    ExtractorRuntimeStatus {
        method: "command".into(),
        location: path
            .as_deref()
            .map(|path| path.to_string_lossy().into_owned()),
        version,
        uses_automatic_discovery,
        dependencies,
    }
}

fn runtime_dependency(
    name: &str,
    path: Option<std::path::PathBuf>,
    version_arguments: &[&str],
    unavailable_reason: &str,
    probe_versions: bool,
) -> ExtractorRuntimeDependency {
    let is_available = path.is_some();
    let runtime = command_runtime(path, false, version_arguments, probe_versions, Vec::new());
    ExtractorRuntimeDependency {
        name: name.into(),
        location: runtime.location,
        version: runtime.version,
        is_available,
        unavailable_reason: (!is_available).then(|| unavailable_reason.into()),
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
