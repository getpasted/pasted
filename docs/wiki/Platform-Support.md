# Platform Support

Optional whisper.cpp transcription, FFmpeg audio preparation, ffprobe or MediaInfo inspection, and Tesseract OCR use locally installed executables. Availability is reported under **Settings → Analysis** and through the corresponding CLI registry commands; no engine or model is downloaded automatically.

## macOS

**Supported:** macOS 13 or newer on Apple Silicon and Intel through one universal signed, notarized, stapled DMG.

Apple Vision OCR is built in on macOS. Optional Tesseract 5 OCR is detected from standard Homebrew locations or the executable path. Accessibility permission is needed for system-wide hotkeys and automatic Queue/HUD paste, not ordinary capture.

## Linux

**Preview:** x86_64 AppImage, validated on SteamOS desktop mode.

Pasted uses native decorated windows on Linux. X11 and Wayland are detected at runtime. Clipboard capture works without focusing the app on the validated SteamOS setup. Desktop-wide hotkeys and target-aware paste may be restricted by a Wayland compositor or portal; Pasted reports that condition without consuming Queue items.

Local image OCR is available when Tesseract 5 is installed, normally through the distribution's `tesseract-ocr` package.

See [Linux and SteamOS testing](https://github.com/getpasted/pasted/blob/main/docs/LINUX_STEAMOS_TESTING.md).

## Windows

**Experimental:** unsigned x86_64 NSIS installer and portable executable.

Windows may identify the publisher as unknown. Smart App Control or organization policy can block unsigned applications. Stable Windows distribution is deferred until trusted signing and real-hardware acceptance are in place.

Local image OCR is available when a Tesseract installation can be found in its standard Program Files location or on the executable path.

## What “graceful failure” means

Platform-specific features are gated. Unsupported hotkey, focus, OCR, or paste integrations must compile and return an explicit capability message; they must not silently claim success or discard data.
