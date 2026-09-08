The copycat keeps receipts.

Pasted 1.0.2 adds automatic Snapshots and fault-tolerant library recovery. When new clips arrive, Pasted creates a local recovery point once a day and keeps the five newest Snapshots by default. Snapshots can be created immediately, restored, deleted, or exported as Full Backups from the app or CLI.

If a moved or unavailable library cannot be opened, Pasted can recover from verified local data, preserve the failed files for diagnosis, and explain what happened in Storage. This release also fixes launch at login and keeps copy notifications independent from the main window on macOS. Existing App Lock credentials continue to work after the updated password-hashing dependency.

The primary build is a signed, notarized, and stapled universal app for macOS 13 or newer, on Apple Silicon and Intel.

**[Download Pasted 1.0.2](https://github.com/getpasted/pasted/releases/tag/v1.0.2)**

Or update from **Settings → About**, or with Homebrew:

```sh
brew upgrade --cask getpasted/tap/pasted
```

Linux x86_64 has an AppImage preview. Windows x86_64 has experimental unsigned builds. Every release includes checksums, updater signatures, third-party notices, and source plus exact-artifact SBOMs.

Tell us what happened in [Discussions](https://github.com/getpasted/pasted/discussions). File reproducible bugs in [Issues](https://github.com/getpasted/pasted/issues). Please leave private clipboard contents and credentials exactly where Pasted leaves them: with you.

Copy irresponsibly.

<!-- pasted-release:v1.0.2 -->
