# Troubleshooting and Diagnostics

Open **Settings → About** for version, installation path, data path, CLI path, signing, and runtime information. OCR backfill progress and Extractor controls are under **Settings → Analysis**. Intelligence connection setup remains under **Settings → Connections**.

The CLI equivalent is:

```sh
pasted diagnostics --json
```

## Hotkeys do not work

- macOS: grant Accessibility to the installed Pasted app (or the active IDE/terminal during development).
- Wayland: the desktop may decline the global-shortcut portal request.
- Browser preview: system-wide shortcuts are unavailable by design.

## Paste Next or HUD targets the wrong app

Focus the intended destination before opening Pasted/HUD. Pasted stores the last eligible external target and never targets itself. A failure keeps the clip queued.

## OCR is not offered

OCR applies to captured image content, not arbitrary file references. Confirm OCR is enabled, the item contains previewable image bytes, and either Apple Vision or Tesseract OCR appears as available under **Settings → Analysis → Manage Extractors**. Install Tesseract 5 when a cross-platform engine is needed, then check `pasted extractor list --json` and `pasted ocr status --json`.

## Whisper Transcription is unavailable

Open **Settings → Analysis → Manage Extractors** and select Whisper Transcription. The settings header identifies the missing dependency; hover it for remediation. Install whisper.cpp, then configure an existing local GGML model in the Model field. Pasted does not download models automatically. `pasted extractor get extractor:whisper-transcription --json` reports the saved model path and current availability.

M4A and AAC transcription also requires FFmpeg. If those formats fail while WAV transcription works, install FFmpeg and try the extraction again.

## Capture feedback does not appear

Confirm **Notifications** is enabled under **Settings → Functionality**, then enable **Capture feedback** under **Settings → Notifications**. Skipped captures appear only when **Show skipped captures** is enabled. The feedback window uses the selected corner of the display currently containing the pointer.

Capture feedback is separate from clipboard capture: a clip can be saved successfully even when feedback is hidden. See [Notifications and Capture Feedback](Notifications-and-Capture-Feedback).

## A copied file cannot paste

Pasted stores a reference to its original path. If the file moved, was deleted, or is unavailable, restore it at that path or copy it again.

## Linux AppImage opens blank

Run it from a terminal and inspect WebKit/GTK output. SteamOS required host WebKit libraries and a compatible graphics backend; current packaging and validated workarounds are documented in the Linux testing guide.

## Recovery

Create a Full Backup before destructive troubleshooting. Full Restore replaces the current state and creates a recovery backup first; History and Organization import merges portable data. Factory Reset is the last resort and permanently removes local data after confirmation.
