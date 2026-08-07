# Pasted

Pasted is an elegant privacy-focused clipboard manager and transformation app for macOS.

## What Pasted does

- Captures text, images, screenshots, PDFs, and copied file references in a searchable local history.
- Organizes clips with manual Bins, Smart Bins, pinning, protection, notes, Trash, and persistent per-collection ordering.
- Extracts searchable text from images with native macOS OCR.
- Previews common image files and the first page of copied PDFs without changing the copied file reference.
- Records copies into a persistent Queue, then pastes the next item or the whole Queue into the previously focused app.
- Builds reusable Transforms from deterministic Operations or an explicitly connected intelligence provider.
- Preserves revisions for content-changing actions and records important events in the Activity Log.
- Includes seven appearance choices: System, Cool, Dark, Warm, Vampire, Flux, and 808.
- Exposes powerful clipboard-management workflows through the bundled `pasted-cli` command.
- Lets each major feature be disabled for a simpler clipboard manager.

## Privacy and safety

Clipboard history, settings, revisions, previews, and activity data are stored locally in SQLite. **Pasted has no analytics or telemetry**.

Password managers and other sensitive apps are ignored by the default blacklist. Capture size, preview size, history retention, Trash retention, revisions, OCR, and content detection are bounded or configurable. Protected clips will remain, regardless of retention settings.

Pasted sends your private clip content outside the app only when you _explicitly_ run an intelligence-assisted transform through a connection you manually enabled. Connection credentials remain with the provider, operating system, or authenticated command-line tool; Pasted stores references rather than API keys.

## macOS requirements

- macOS 12 or newer
- Mac with Apple Silicon
- Accessibility permission for global hotkeys and automatic Queue/HUD pasting

## Default shortcuts

| Shortcut | Action |
| --- | --- |
| `⌥⇧V` | Open the Quick HUD |
| `⌥⇧C` | Start or stop recording copies into the Queue |
| `⌥⇧X` | Paste the next queued item |
| `1`–`9` | Paste the corresponding clip from the Quick HUD |
| `Esc` | Close the Quick HUD or an open menu |

Shortcuts can be changed or disabled in Settings.

## CLI

The app bundle includes `pasted-cli`. Install it from **Settings → About**, or add the bundled executable to your `PATH` manually:

```sh
sudo ln -s /Applications/Pasted.app/Contents/MacOS/pasted-cli /usr/local/bin/pasted-cli
```

The complete command reference is available inside **Help & Documentation → CLI Commands**. Commands that return structured records support stable JSON output where applicable.

## Development

Prerequisites:

- Node.js 18 or newer
- Rust 1.75 or newer
- macOS for native OCR and signed macOS packaging

```sh
npm install
npm run tauri dev
```

Run the complete test suite:

```sh
npm run test:all
```

## Releases and updates

Pasted 1.0 uses verified manual DMG updates for now. Download a newer signed DMG, and replace the application in `/Applications`.

Maintainers should follow [the macOS release guide](docs/MACOS_RELEASE.md) and the [1.0 release-candidate checklist](docs/RELEASE_CHECKLIST_1.0.0.md).

## License

Pasted is distributed under the [MIT License](LICENSE).
