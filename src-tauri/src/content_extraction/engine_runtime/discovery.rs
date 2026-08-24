use super::*;

pub(crate) fn configured_or_discovered_executable(
    configured: Option<&Path>,
    discover: impl FnOnce() -> Option<std::path::PathBuf>,
) -> Option<std::path::PathBuf> {
    match configured {
        Some(path) if crate::external_tools::is_executable(path) => Some(path.to_path_buf()),
        Some(_) => None,
        None => discover(),
    }
}

pub(crate) fn executable_availability(
    executable: Option<std::path::PathBuf>,
    unavailable_reason: &str,
) -> EngineAvailability {
    EngineAvailability {
        is_available: executable.is_some(),
        unavailable_reason: executable.is_none().then(|| unavailable_reason.into()),
    }
}

pub(crate) fn find_tesseract_executable() -> Option<std::path::PathBuf> {
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

pub(crate) fn find_whisper_cpp_executable() -> Option<std::path::PathBuf> {
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

pub(crate) fn find_ffmpeg_executable() -> Option<std::path::PathBuf> {
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
