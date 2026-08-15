# Pasted

Pasted is the private, local clipboard workspace for copycats: humans, scripts, automations, and agents working from one shared library.

**No cloud account. No off-device telemetry. No subscription.**

The [Pasted Wiki](https://github.com/getpasted/pasted/wiki) covers installation, everyday workflows, recovery, CLI automation, platform support, privacy, and troubleshooting.

## What Pasted does

- Captures text, images, screenshots, PDFs, and copied file references in a searchable local history.
- Organizes clips with manual Bins, Smart Bins, Content Types, Sources, pinning, protection, notes, Trash, and persistent per-collection ordering.
- Analyzes clips locally through bounded Inspect, Extract, Classify, and Enrich passes, with native macOS OCR, optional Tesseract OCR, ffprobe or MediaInfo media metadata, local whisper.cpp transcription, editable Detectors, content-free structural metadata, and Smart Action recommendations.
- Previews common image files and the first page of copied PDFs without changing the copied file reference.
- Records copies into a persistent Queue, then pastes the next item or the whole Queue into the previously focused app.
- Builds reusable Transforms from deterministic Operations or an explicitly connected intelligence provider.
- Preserves revisions for content-changing actions, records important events in Activity, and summarizes the active library locally in Insights.
- Creates complete, validated Full Backups and separately exports portable History and Organization JSON for merging into another library.
- Includes seven appearance choices: System, Cool, Dark, Warm, Vampire, Flux, and 808.
- Exposes powerful clipboard-management workflows through the bundled `pasted` command.
- Imports supported text history from Alfred, Pastebot, Pasta, Paste, CopyClip 2, Maccy, and Flycut without changing the source library.
- Lets each major feature be disabled for a simpler clipboard manager.

## Privacy and safety

Clipboard history, settings, revisions, previews, and activity data are stored locally in SQLite. **Pasted has no analytics or telemetry**.

Password managers and other sensitive apps are ignored by the default blacklist. Capture size, preview size, history retention, Trash retention, revisions, OCR, and content detection are bounded or configurable. Protected clips will remain, regardless of retention settings.

Pasted sends your private clip content outside the app only when you _explicitly_ run an intelligence-assisted transform through a connection you manually enabled. Connection credentials remain with the provider, operating system, or authenticated command-line tool; Pasted stores references rather than API keys.

Analyzer snapshots and participant summaries are deliberately content-free. They can report bounded structure, a detected Content Type, participant outcomes, and recommended Transform references, but never return original text, extracted text, image bytes, or file paths. An explicit Extractor preview may return its bounded derived text to the caller that requested it.

The Analysis settings sequence shows Capture ahead of the Analyzer. Capture assigns one structural Clip Type and records source attribution; optional source presentation and icon resolution follow the Sources setting.

## The Copycat Covenant

- **No cloud account.** The core clipboard workspace stays local and works without a hosted identity.
- **No off-device telemetry.** Usage insights stay local; Pasted does not report how copycats use the app or CLI.
- **No subscription.** Pasted will not rent your clipboard back to you. Financial support is an endorsement, never a feature unlock.
- **Every copycat welcome.** The GUI and CLI use the same local library so people and the tools they direct share one source of context.

## Platform support

Pasted 1.0 supports macOS 13 or newer on Apple Silicon and Intel. Accessibility permission is required for global hotkeys and automatic Queue and HUD pasting. Linux is available as an x86_64 AppImage preview, and Windows builds are unsigned and experimental. See [Platform Support](docs/wiki/Platform-Support.md) for the explicit capability matrix.

## Default shortcuts

| Shortcut | Action |
| --- | --- |
| `⌥⇧V` | Open the HUD |
| `⌥⇧C` | Start or stop recording copies into the Queue |
| `⌥⇧X` | Paste the next queued item |
| `1`–`9` | Paste the corresponding clip from the HUD |
| `Esc` | Close the HUD or an open menu |

Shortcuts can be changed or disabled in Settings.

## CLI

The app bundle includes `pasted`. Install it from **Settings → About**, or add the bundled executable to your `PATH` manually:

```sh
sudo ln -s /Applications/Pasted.app/Contents/MacOS/pasted /usr/local/bin/pasted
```

The GUI and CLI use the same local library and shared domain services. The complete command reference is available inside **Help → CLI Commands** and in the [CLI Reference](docs/wiki/CLI-Reference.md). Commands that return structured records support stable JSON output where applicable.

```sh
pasted list --limit 20 --json
pasted search --type link --json
pasted analyzer run --stdin --json
pasted insights summary --json
pasted backup create Pasted.pastedbackup --json
pasted transfer export Pasted-history.json --json
```

The Analyzer command returns one versioned, privacy-safe preview across the applicable Inspector, Extractor, Detector, and Enricher participants. Focused commands such as `pasted inspector`, `pasted extractor`, `pasted detector`, and `pasted enricher` expose the same contracts for automation and diagnostics.

Managed or scripted installs can bypass the first-run walkthrough by launching the graphical app with `--skip-welcome`. On macOS:

```sh
open -a Pasted --args --skip-welcome
```

## Development

Prerequisites:

- Node.js 22 or newer
- Rust 1.75 or newer
- macOS for native OCR and signed macOS packaging; optional FFmpeg or MediaInfo supplies cross-platform media inspection, FFmpeg prepares M4A/AAC audio, and whisper.cpp plus a local GGML model supplies local audio transcription

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

## Community

- Read [CONTRIBUTING.md](CONTRIBUTING.md) before proposing or implementing a change.
- Use [SUPPORT.md](SUPPORT.md) to choose the right public help channel.
- Report vulnerabilities privately according to [SECURITY.md](SECURITY.md).
- Participation is governed by the [Code of Conduct](CODE_OF_CONDUCT.md).
- Project authority and automated-contribution boundaries are described in [GOVERNANCE.md](GOVERNANCE.md).
- User-visible release history is maintained in the [Changelog](CHANGELOG.md).

## License

Pasted is distributed under the [MIT License](LICENSE).
