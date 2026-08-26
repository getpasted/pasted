# Changelog

Notable user-facing changes to Pasted are recorded here. The project follows semantic versioning after the stable `1.0.0` release.

## Unreleased

No changes yet.

## 1.0.0-rc.5 — 2026-08-26

This release candidate expands organization, recovery, local Analysis, and automation while hardening the shared contracts intended for 1.0:

- added multi-Bin organization with persistent per-Bin ordering, paste-by-ID clip hotkeys, explicit and inherited protection, and matching GUI, CLI, Activity, transfer, backup, Help, and localization behavior;
- added validated Full Backup and Full Restore for every durable Pasted-owned table plus interface state, including automatic pre-restore recovery snapshots;
- separated portable History and Organization transfer from Full Backup, added bounded preflight summaries, and added one-step restoration of every trashed clip;
- added seamless Activity loading, OpenTelemetry-shaped JSON archives, reporting CSV, safe deduplicating imports, and independent retention controls;
- bounded History, Trash, Search, and HUD queries with pagination, exact counts, incremental loading, and offscreen-card virtualization;
- unified Capture, Inspectors, Extractors, Classifiers, and Suggestions under one bounded Analysis lifecycle with shared GUI, CLI, and versioned privacy-safe JSON contracts;
- separated structural Clip Type, byte-verified File Format, semantic Content Type, and Source throughout Analysis, Insights, Search, Smart Bins, Functionality, Help, and documentation;
- added local Tesseract OCR, ffprobe and MediaInfo inspection, whisper.cpp transcription, and Apple Vision or cross-platform llama.cpp Visual Labels;
- made Visual Labels searchable and editable, preserved label edits in Clip Versions, and added shared minimum-confidence post-processing for shipped and custom label Extractors;
- unified shipped and custom Extractors under editable local recipes with guided setup, copyable commands, AI-assisted drafting and diagnosis, authoring history, reusable post-processing, and explicit expected no-output exit codes;
- made nonzero OCR status counts open Search for the matching clips and added stable clip-ID filtering to the GUI and CLI search contract;
- unified Search and Smart Bins around partial, case-insensitive Clip Type, Content Type, File Format, Source, and Visual Label matching with explicit operators and Functionality-aware behavior;
- added editable Content Types, Groups, and Classifiers with rescanning and safe migration of earlier classification preferences;
- made App Exclusions enforce independent text, image, file, and hotkey rules, and added private-browser capture exclusion for Safari, Chrome, Edge, Firefox, DuckDuckGo, and Brave;
- added confirmed, scoped Settings resets with exact change previews, shared defaults and validation, factory-reset coverage, and non-mutating `pasted settings reset <page> --dry-run --json` previews;
- added bounded external-history imports for Alfred, Pastebot, Pasta, Paste, CopyClip 2, Maccy, and Flycut;
- added welcome setup, reorganized Functionality settings, two new light themes, interactive capture previews, a global Notifications gate, and persistent workspace restoration;
- consolidated Transform storage and interfaces while refining manual editing, provider warnings, execution provenance, and deterministic GUI/CLI parity;
- normalized source metadata and stabilized screenshot, image-copy, file-reference, HUD, titlebar, and window-drag behavior; and
- added release-blocking license, advisory, mission-policy, dependency-review, notice, and source plus exact-artifact SPDX SBOM checks.

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
- the HUD and configurable global hotkeys;
- deterministic and intelligence-assisted Transforms;
- native macOS OCR and bounded file previews;
- local backup, import, reset, feature gates, themes, diagnostics, and the bundled `pasted` CLI;
- a signed and notarized universal macOS DMG;
- experimental Linux x86_64 AppImage and Windows x86_64 packages.

See the complete [1.0 release notes](docs/RELEASE_NOTES_1.0.0.md).
