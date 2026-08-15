# Pasted 1.0.0

Pasted 1.0 is the first direct-download release of the local, privacy-first clipboard manager for Apple Silicon and Intel Macs.

## Highlights

- Searchable history for text, images, screenshots, PDFs, and copied files
- Manual and Smart Bins with drag-and-drop assignment and ordering
- Pinning, protection, notes, Trash, revisions, Activity, Full Backup and Restore, and portable History and Organization import
- Local Analysis with structural and media inspection, Apple Vision or Tesseract OCR, whisper.cpp transcription, editable Detectors, and Smart Action recommendations
- Separate Clip Type, File Format, and Content Type summaries in Insights
- Persistent Copy Queue with target-aware Paste Next and Paste All
- Reusable deterministic and intelligence-assisted Transforms
- Configurable feature gates, hotkeys, retention, previews, content detection, and appearance
- Bundled `pasted` with shared native data and mutation contracts
- Signed, notarized, stapled, and Gatekeeper-verified DMG
- Release-blocking dependency license, advisory, mission-policy, and SPDX SBOM verification

## Privacy

Pasted stores its library locally and includes no analytics or telemetry. Intelligence-assisted Transforms contact a provider only when the user explicitly runs them through an enabled connection.

Production dependencies are checked against reviewed open-source license and source policies. Tagged releases include a deterministic dependency-graph SPDX SBOM plus an SPDX scan of each extracted platform payload.

## Updates

Version 1.0 uses manual DMG updates. Installing a newer Pasted application keeps the existing local library.

## Platform support

- **macOS 13 or newer:** supported through one signed, notarized universal DMG.
- **Linux x86_64:** AppImage preview validated on SteamOS; desktop integration varies by distribution and Wayland compositor.
- **Windows x86_64:** unsigned experimental installer and portable executable. Windows security policy may block them until signed distribution is available.
