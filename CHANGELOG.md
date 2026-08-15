# Changelog

Notable user-facing changes to Pasted are recorded here. The project follows semantic versioning after the stable `1.0.0` release.

## Unreleased

- added validated Full Backup and Full Restore across Settings and the CLI, covering every durable database table plus saved interface/window state and creating a pre-restore recovery snapshot automatically.
- consolidated reporting exports under Storage and added History and Organization preflight summaries in the GUI and `pasted transfer` CLI workflow.
- added one-step recovery for every trashed clip through Settings and `pasted clip restore-all`, with shared mutation summaries and Activity logging.
- added seamless Activity history loading plus versioned OpenTelemetry-shaped JSON archives, round-trip reporting CSV, safe deduplicating imports, and matching CLI commands.
- clarified the portable History and Organization scope and renamed clips-only exports so they cannot be mistaken for Full Backups.
- bounded clip, Trash, and HUD queries with incremental loading, exact collection counts, and offscreen card virtualization.
- renamed Activity clearing consistently across the GUI and CLI, and tightened Insights language to its active-library scope.
- unified Capture, Inspectors, Extractors, Classifiers, and Suggestions under one bounded Analysis lifecycle with matching GUI, CLI, and versioned JSON contracts.
- added Tesseract OCR, ffprobe and MediaInfo inspection, and whisper.cpp transcription with explicit local dependency and model configuration.
- separated structural Clip Type, referenced File Format, and semantic Content Type throughout Analysis, Insights, Functionality, Help, and documentation.
- aligned Reset, Content Type, Insights, and Help navigation language across dialogs, native menus, CLI messages, and architecture documentation.
- made App Exclusions enforce independent text, image, file, and shortcut rules, with reliable focused-app identity on macOS, Windows, and X11.
- unified the Functionality, Notifications, and App Exclusions footer guidance and tightened their Settings copy and row layout.

## 1.0.0-rc.5 — 2026-08-12

This release candidate expands library organization, capture feedback, and customization while hardening shared data contracts:

- added release-blocking open-source policy checks, expiring RustSec exceptions, pull-request dependency review, and source plus exact-artifact SPDX SBOMs;
- added multi-Bin clip organization with persistent per-Bin ordering and matching GUI and CLI behavior;
- introduced editable content types, groups, and classifiers, including rescanning and safe migration of earlier classification preferences;
- added bounded external-history imports for Maccy, Pastebot, Paste, Pasta, CopyClip, and Flycut;
- added welcome setup, reorganized feature settings, two new light themes, interactive capture previews, and a global notification gate;
- consolidated Transform storage and interfaces while refining editor workflows, provider warnings, and execution provenance;
- normalized clip source metadata and fixed screenshot and image-copy semantics without reinterpreting existing clip identity;
- restored the last active page, selected clip, sidebar state, and navigation sections across launches, with a configurable Startup View preference;
- improved clip-list rendering, pin feedback, macOS titlebar and window-drag behavior, and in-app help; and
- added in-app open-source license access and complete third-party notice artifacts for distributed packages.

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
