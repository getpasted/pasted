# Changelog

Notable user-facing changes to Pasted are recorded here. The project follows semantic versioning after the stable `1.0.0` release.

## Unreleased

## 1.0.0-rc.4 — 2026-08-10

- renamed Backup settings to Storage and added a safe, reversible SQLite library-location workflow;
- added GUI and CLI controls to inspect, move, or restore the default library location;
- preserved source libraries as recovery copies and added bounded validation, rollback, and relocation tests; and
- introduced reusable, dismissible toast notifications for storage and backup feedback.

## 1.0.0-rc.3 — 2026-08-10

- restored the branded macOS DMG background and icon layout in CI-built releases; and
- added release verification that rejects DMGs missing their Finder presentation metadata.

## 1.0.0-rc.2 — 2026-08-09

This release candidate hardens the intended 1.0 build after RC1 testing:

- strengthened clipboard plain-text sanitization and bounded destructive-data paths;
- added rollback tests for factory reset, backup import, and cross-clip revision restoration;
- refreshed frontend, Rust, and GitHub Actions dependencies;
- added unsigned experimental Windows artifacts alongside the Linux preview;
- added the comprehensive GitHub Wiki and automated Wiki publishing; and
- completed repository security, contribution, support, conduct, and review policies.

## 1.0.0-rc.1 — 2026-08-08

The first public release candidate includes:

- local clipboard history for text, images, screenshots, PDFs, and file references;
- History search, Bins, Smart Bins, Queue, pinning, protection, notes, Trash, revisions, and activity logging;
- the HUD and configurable global shortcuts;
- deterministic and intelligence-assisted Transforms;
- native macOS OCR and bounded file previews;
- local backup, import, reset, feature gates, themes, diagnostics, and the bundled `pasted` CLI;
- a signed and notarized universal macOS DMG;
- experimental Linux x86_64 AppImage and Windows x86_64 packages.

See the complete [1.0 release notes](docs/RELEASE_NOTES_1.0.0.md).
