# Platform Support

## macOS

**Supported:** macOS 13 or newer on Apple Silicon and Intel through one universal signed, notarized, stapled DMG.

Apple Vision OCR is macOS-only. Accessibility permission is needed for system-wide hotkeys and automatic Queue/Quick HUD paste, not ordinary capture.

## Linux

**Preview:** x86_64 AppImage, validated on SteamOS desktop mode.

Pasted uses native decorated windows on Linux. X11 and Wayland are detected at runtime. Clipboard capture works without focusing the app on the validated SteamOS setup. Desktop-wide shortcuts and target-aware paste may be restricted by a Wayland compositor or portal; Pasted reports that condition without consuming Queue items.

See [Linux and SteamOS testing](https://github.com/getpasted/pasted/blob/main/docs/LINUX_STEAMOS_TESTING.md).

## Windows

**Experimental:** unsigned x86_64 NSIS installer and portable executable.

Windows may identify the publisher as unknown. Smart App Control or organization policy can block unsigned applications. Stable Windows distribution is deferred until trusted signing and real-hardware acceptance are in place.

## What “graceful failure” means

Platform-specific features are gated. Unsupported hotkey, focus, OCR, or paste integrations must compile and return an explicit capability message; they must not silently claim success or discard data.

