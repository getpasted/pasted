Search without the scavenger hunt.

Pasted 1.1 makes large libraries feel small. History, Bins, types, and other collections now load in bounded pages, coordinate saved scroll restoration with their reveal, and keep recent pages warm for faster return trips. A bounded fallback keeps slow content from holding the interface indefinitely. File previews are bounded by concurrency, time, and memory, so missing external files cannot hold up an entire list.

Search now opens in the same focused modal whether the sidebar is expanded or collapsed. The current query is ready to edit immediately, filter helpers remain close at hand, progress appears as soon as a search starts, and Escape can clear the query or return to the previous collection. Search results remain paginated alongside the rest of the library.

The primary build is a signed, notarized, and stapled universal app for macOS 13 or newer, on Apple Silicon and Intel.

**[Download Pasted 1.1.0](https://github.com/getpasted/pasted/releases/tag/v1.1.0)**

[Read the full changelog](https://github.com/getpasted/pasted/blob/v1.1.0/CHANGELOG.md)

Or update from **Settings → About**, or with Homebrew:

```sh
brew upgrade --cask getpasted/tap/pasted
```

Linux x86_64 has an AppImage preview. Windows x86_64 has experimental unsigned builds. Every release includes checksums, updater signatures, third-party notices, and source plus exact-artifact SBOMs.

Tell us what happened in [Discussions](https://github.com/getpasted/pasted/discussions). File reproducible bugs in [Issues](https://github.com/getpasted/pasted/issues). Please leave private clipboard contents and credentials exactly where Pasted leaves them: with you.

Copy irresponsibly.

<!-- pasted-release:v1.1.0 -->
