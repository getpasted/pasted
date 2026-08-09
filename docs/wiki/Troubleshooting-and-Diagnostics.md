# Troubleshooting and Diagnostics

Open **Settings → About** for version, installation path, data path, CLI path, signing, and runtime information. Use **Settings → Debug** for schedulers, Connections, OCR backfill, and other long-running work.

The CLI equivalent is:

```sh
pasted diagnostics --json
```

## Hotkeys do not work

- macOS: grant Accessibility to the installed Pasted app (or the active IDE/terminal during development).
- Wayland: the desktop may decline the global-shortcut portal request.
- Browser preview: system-wide shortcuts are unavailable by design.

## Paste Next or Quick HUD targets the wrong app

Focus the intended destination before opening Pasted/HUD. Pasted stores the last eligible external target and never targets itself. A failure keeps the clip queued.

## OCR is not offered

OCR applies to captured image content, not arbitrary file references. Confirm OCR is enabled, the item contains previewable image bytes, and the platform supports Apple Vision. Check `pasted ocr status --json`.

## A copied file cannot paste

Pasted stores a reference to its original path. If the file moved, was deleted, or is unavailable, restore it at that path or copy it again.

## Linux AppImage opens blank

Run it from a terminal and inspect WebKit/GTK output. SteamOS required host WebKit libraries and a compatible graphics backend; current packaging and validated workarounds are documented in the Linux testing guide.

## Recovery

Export before destructive troubleshooting. Backup import is a merge. Factory Reset is the last resort and permanently removes local data after confirmation.

