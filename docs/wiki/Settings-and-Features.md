# Settings and Features

Pasted can be a full workspace or a small clipboard history. **Settings → Functionality** provides global gates for Bins, Queue, pinning, protection, notes, Trash, Types, Sources, Transformations, Activity Log, CLI, OCR, Revision History, content detection, Quick HUD, and related tools.

Disabling a feature hides its active UI and preserves existing data unless the setting explicitly describes destruction. Related settings are hidden when they cannot apply.

Other Settings pages cover:

- **General:** layout, zoom, row height, retention, sounds, startup, previews, and OS integration;
- **Hotkeys:** global shortcuts and platform permission status;
- **Connections:** detected and custom intelligence providers;
- **Blacklist:** applications Pasted should not capture;
- **Backup & Import:** export, merge import, and Factory Reset;
- **Debug:** schedulers and long-running background work;
- **About:** version, installation paths, signing, runtime, and CLI installation.

Appearance schemes use semantic theme tokens across the main app, HUD, menus, modals, Settings, and Tools pages.

## Detection and Types

**Settings → Detection** manages ordered detectors and the shared Type and Group registries. IDs are stable: built-in Types and Groups can be renamed and reordered, Types can be assigned searchable icons, and custom entries can be archived. A custom Group must be empty before it can be archived or permanently deleted. Archiving a Type preserves existing clips and disables detectors that would produce it. Registry metadata does not maintain revision history; changes are recorded in Activity Log, and **Restore Defaults** recovers the shipped metadata and detectors without removing custom entries.

**Rescan History** explicitly reapplies enabled detectors to existing text clips. It leaves image and file Types unchanged and reports how many clips were reclassified.
