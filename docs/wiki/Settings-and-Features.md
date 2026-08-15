# Settings and Features

Pasted can be a full workspace or a small clipboard history. **Settings → Functionality** provides global gates for Bins, Queue, pinning, protection, notes, Trash, Content Types, Sources, Transformations, Activity, CLI, OCR, Transcriptions, Revision History, content detection, HUD, and related tools.

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
- **Content Detection:** disabling it stops detector-driven classification and rescans. Detectors remain available while Content Types is enabled. Existing clips and Detector configuration are preserved.
- **OCR:** disabling it stops and cancels automatic image OCR and hides image Extractors. File transcription follows the separate Transcriptions feature. Completed extracted text remains with its clips; re-enabling OCR resumes eligible image backfill.
- **Transcriptions:** disabling it hides audio transcription controls and file-input Extractors. Completed transcripts remain stored; re-enabling restores the controls and Extractor configuration.
- **Transformations:** disabling it stops text workflows and Smart Action enrichment and hides Enrichers under Analysis.
- **Content Types:** disabling it hides semantic Content Type labels and calculated collections. Detectors may still classify clips using the preserved registry. Structural Clip Type remains visible.
- **Sources:** disabling it hides source metadata and calculated Source collections and stops icon resolution. Attribution remains stored so re-enabling Sources is reversible.
- **Insights:** disabling it hides library statistics. It does not change Analyzer execution or stored analysis results.
- **Notifications:** disabling the feature removes capture feedback. Clipboard capture itself continues.
- **Help:** disabling it hides the in-app documentation entry; it does not affect the external wiki.

Feature gates control visibility and future behavior. Factory Reset, permanent deletion, retention purges, and other destructive operations remain separately confirmed or explicitly described.

Other Settings pages cover:

- **General:** layout, zoom, row height, retention, sounds, startup, previews, and OS integration;
- **Hotkeys:** global shortcuts and platform permission status;
- **Intelligence:** detected and custom intelligence providers;
- **App Exclusions:** applications Pasted should not capture;
- **Storage:** database location, complete backup and restore, preflighted History and Organization transfer, Clip and Activity import/export, migration from supported clipboard managers, and Factory Reset;
- **About:** version, installation paths, signing, runtime, and CLI installation.

Appearance schemes use semantic theme tokens across the main app, HUD, menus, modals, Settings, and Tools pages.

## Content Analysis

**Settings → Analysis** shows the clip lifecycle beginning with Capture, followed by the four Analyzer passes. Capture assigns exactly one structural Clip Type—Text, Image, or Files—and records source attribution before Analysis. Optional surfaces follow Functionality: Source Attribution is hidden when Sources is disabled, Extractors remain for OCR or Transcriptions, Detectors remain for Content Detection or Content Types, and Enrichers remain for Transformations.

Content Types and Sources organize the library rather than running Analyzer participants. Disabling either feature hides its related presentation while preserving metadata. Clip Type remains visible because text, images, and file collections are structural capture representations rather than detected Content Types.

Inspectors and Enrichers have read-only managers for their practical input, output, runtime availability, and optional technical contracts. Extractors and ordered Detectors remain authorable, alongside the shared Content Type and Group registries. Extractors create searchable representations without replacing original clip content. IDs are stable: built-in Content Types and Groups can be renamed and reordered, Content Types can be assigned searchable icons, and custom entries can be archived. A custom Group must be empty before it can be archived or permanently deleted. Archiving a Content Type preserves existing clips and disables Detectors that would produce it. Registry metadata does not maintain revision history; changes are recorded in Activity, and **Reset…** recovers shipped metadata without removing custom entries.

**Rescan Clips** explicitly reapplies enabled Detectors to existing text clips. It leaves Image and Files Clip Types unchanged and reports how many clips were reclassified.

See [Content Analysis and Content Types](Detection-and-Types) for Extractors, Detector ordering, validation, testing, rescanning, and recovery details. See [Notifications and Capture Feedback](Notifications-and-Capture-Feedback) for capture confirmation settings and privacy behavior.
