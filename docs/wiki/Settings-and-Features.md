# Settings and Features

Pasted can be a full workspace or a small clipboard history. **Settings → Functionality** provides global gates for App Lock, Bins, Queue, pinning, protection, notes, Trash, Clip Search, Clip Types, Content Types, Sources, Transformations, Activity, CLI, OCR, Transcriptions, Revision History, content classification, HUD, Hotkeys, and related tools.

Disabling a feature hides its active UI and preserves existing data unless the setting explicitly describes destruction. Related settings are hidden when they cannot apply.

## Presets

- **Simple** keeps the core clipboard-history experience and safety features visible.
- **Full** enables every available feature.
- **Custom** appears after individual feature choices no longer match a preset.

Changing presets does not erase clips or supporting records. Feature cards with an information indicator describe consequences that continue after the feature is hidden.

## Important feature interactions

- **Trash:** when enabled, ordinary deletion moves clips to recoverable Trash. Restore clips individually from Trash or restore every trashed clip from Settings → General. When disabled, new deletions are permanent. Existing trashed clips remain stored and become available again when Trash is re-enabled.
- **Revision History:** disabling it preserves existing revisions, but new edits and Transform replacements do not receive restorable snapshots.
- **Protection:** disabling the interface does not unprotect previously protected clips. Re-enable Protection to change them.
- **Bins:** disabling it hides manual and Smart Bin interfaces without deleting definitions, membership, ordering, or connected Transforms.
- **Clip Types:** disabling it hides structural Clip Type labels, sidebar collections, Insights summaries, and Clip Type Smart Bin matches. Capture continues assigning Text, Image, or Files internally so previews, copy behavior, Extractors, backups, and APIs remain correct.
- **File Formats:** disabling it stops format inspection, hides File Format collections and summaries, and suspends File Format Smart Bin matches. Existing results and rules remain stored.
- **Content Classification:** disabling it stops classifier-driven classification and rescans. Classifiers remain available while Content Types is enabled. Existing clips and Classifier configuration are preserved.
- **OCR:** disabling it stops automatic OCR and hides the shipped Apple Vision and Tesseract recipes. Unrelated custom image Extractors remain manageable. Completed extracted text remains with its clips; re-enabling OCR resumes eligible image backfill.
- **Transcriptions:** disabling it hides Whisper and audio transcription controls without hiding unrelated custom file Extractors. Completed transcripts remain stored.
- **Transformations:** disabling it stops text workflows and Smart Action suggestions and hides Suggestions under Analysis.
- **Hotkeys:** disabling it immediately unregisters every system-wide Pasted hotkey and hides hotkey configuration without deleting assignments. Re-enabling it restores eligible app, clip, Bin, and Transform hotkeys.
- **Content Types:** disabling it hides semantic Content Type labels and calculated collections and suspends Content Type Smart Bin matches. Classifiers may still classify clips using the preserved registry. Structural presentation follows the separate Clip Types setting.
- **Sources:** disabling it hides source metadata and calculated Source collections, stops icon resolution, and suspends Source Smart Bin matches. Attribution remains stored so re-enabling Sources is reversible.
- **Clip Search:** disabling it stops settled-query recording and hides Search plus the Search History settings page. Existing saved searches remain stored until they are removed after re-enabling Clip Search.
- **Insights:** disabling it hides library statistics. It does not change Analyzer execution or stored analysis results.
- **Notifications:** disabling the feature removes capture feedback. Clipboard capture itself continues.
- **App Lock:** disabling it immediately removes lock enforcement, hides Security, and clears the saved passphrase and authentication preferences. Timing and capture policies remain available when App Lock is enabled again.
- **Help:** disabling it hides the in-app documentation entry; it does not affect the external wiki.

Feature gates control visibility and future behavior. Factory Reset, permanent deletion, retention purges, and other destructive operations remain separately confirmed or explicitly described.

Other Settings pages cover:

- **General:** layout, zoom, row height, retention, sounds, startup, previews, and OS integration;
- **Search History:** when Clip Search is enabled, successful settled searches can be rerun, removed individually, or cleared together;
- **Hotkeys:** when enabled under Functionality, global hotkeys and platform permission status, including a configurable Lock Pasted action when App Lock is available;
- **Security:** when App Lock is enabled under Functionality, passphrase setup, system authentication, immediate lock, restart and sleep policies, inactivity auto-lock, and capture behavior while locked;
- **Intelligence:** detected and custom intelligence providers;
- **App Exclusions:** applications that should block selected text, image, file, or hotkey behavior;
- **Storage:** database location and detected volume-encryption status, complete backup and restore, preflighted History and Organization transfer, Clip and Activity import/export, migration from supported clipboard managers, and Factory Reset;
- **About:** version, installation paths, signing, runtime, and CLI installation.

## Scoped resets

General, Notifications, Hotkeys, Security, Analysis, App Exclusions, and Intelligence each provide a confirmed **Reset…** action. The confirmation lists the exact settings that differ from that page's shared defaults before anything is saved. Resetting one page does not reset unrelated pages, clips, Bins, or other library records.

The CLI uses the same setting registry, defaults, validation, sensitive-value policy, and page boundaries. Preview a reset without changing data with:

```sh
pasted settings reset <page> --dry-run --json
```

Remove `--dry-run` only after reviewing the returned changes. Factory Reset is a separate destructive workflow under Storage and is never implied by a page reset.

Appearance schemes use semantic theme tokens across the main app, HUD, menus, modals, Settings, and Tools pages.

## Content Analysis

**Settings → Analysis** shows the clip lifecycle beginning with Capture, followed by the four Analyzer passes. Capture assigns exactly one structural Clip Type—Text, Image, or Files—and records source attribution before Analysis. Each Clip Type then runs its applicable scanners: File Format inspection follows File Formats, shipped OCR and transcription recipes follow their switches, Classifiers follow Content Classification or Content Types, and Suggestions follow Transformations. Extractor management remains available for custom recipes.

Clip Types, File Formats, Content Types, and Sources organize the library. The sidebar keeps structural Text, Image, and Files filters separate from byte-verified File Formats and detected Content Types. Disabling a presentation feature hides its related interface while preserving original clip data; disabling File Formats also stops new format inspection.

Inspectors and Suggestions have read-only managers for their practical input, output, runtime availability, and optional technical contracts. Extractors and ordered Classifiers remain authorable, alongside the shared Content Type and Group registries. Extractors create searchable representations without replacing original clip content. IDs are stable: built-in Content Types and Groups can be renamed and reordered, Content Types can be assigned searchable icons, and custom entries can be archived. A custom Group must be empty before it can be archived or permanently deleted. Archiving a Content Type preserves existing clips and disables Classifiers that would produce it. Registry metadata does not maintain revision history; changes are recorded in Activity, and **Reset…** recovers shipped metadata without removing custom entries.

**Rescan Clips…** refreshes enabled Classifiers and File Format inspection for existing clips. It never changes original content or structural Clip Types.

See [Content Analysis and Content Types](Classification-and-Types) for Extractors, Classifier ordering, validation, testing, rescanning, and recovery details. See [Notifications and Capture Feedback](Notifications-and-Capture-Feedback) for capture confirmation settings and privacy behavior.
