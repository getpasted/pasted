# Settings and Features

Pasted can be a full workspace or a small clipboard history. **Settings → Functionality** provides global gates for Bins, Queue, pinning, protection, notes, Trash, Types, Sources, Transformations, Activity, CLI, OCR, Revision History, content detection, HUD, and related tools.

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
- **Content Detection:** disabling it stops detector-driven classification of new text clips and hides detector management. Analysis remains available when OCR is enabled. Existing clips are not reinterpreted.
- **OCR:** disabling it stops and cancels background OCR work. Completed OCR remains with its clips; re-enabling resumes eligible backfill.
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

**Settings → Analysis** summarizes all four Analyzer participant groups. Inspectors and Enrichers have read-only managers for their practical input, output, runtime availability, and optional technical contracts. Extractors and ordered Detectors remain authorable, alongside the shared Type and Group registries. Extractors create searchable representations without replacing original clip content. IDs are stable: built-in Types and Groups can be renamed and reordered, Types can be assigned searchable icons, and custom entries can be archived. A custom Group must be empty before it can be archived or permanently deleted. Archiving a Type preserves existing clips and disables Detectors that would produce it. Registry metadata does not maintain revision history; changes are recorded in Activity, and **Restore Defaults** recovers shipped metadata without removing custom entries.

**Rescan Clips** explicitly reapplies enabled detectors to existing text clips. It leaves image and file Types unchanged and reports how many clips were reclassified.

See [Content Analysis, Detection, and Types](Detection-and-Types) for Extractors, detector ordering, validation, testing, rescanning, and recovery details. See [Notifications and Capture Feedback](Notifications-and-Capture-Feedback) for capture confirmation settings and privacy behavior.
