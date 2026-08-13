# Building, Contributing, and Releasing

## Development

Requirements:

- Node.js 22 or newer;
- Rust 1.75 or newer;
- platform packages required by Tauri/WebKitGTK;
- macOS for Apple Vision OCR and signed macOS packaging.

```sh
npm install
npm run tauri dev
npm run test:all
```

The complete gate runs formatting, Clippy, frontend build, Rust unit tests, IPC parity, security, dependency license/mission policy, source-SBOM freshness, feature, collection, menu, CLI, Transform, Activity Log, platform, CSS architecture, and WCAG audits. Hosted builds additionally run dependency review, `cargo-deny`, and exact-payload SBOM inspection.

## Contribution rules

- Keep meaningful GUI and CLI capabilities on shared Rust domain services.
- Use semantic theme classes for color, surfaces, borders, focus, and emphasis.
- Preserve or explicitly migrate clips, files, revisions, Bins, settings, and backups.
- Never expose clipboard contents, file paths, credentials, or prompts in logs/analytics.
- Gate platform-specific behavior and provide an explicit graceful-failure path.

## Protected branch workflow

Create a short-lived branch from current `main`, commit and push there, and open a pull request. Direct pushes to `main` are blocked. A change can merge only after dependency review, dependency policy, the complete validation suite, and native macOS, Linux, and Windows compile smoke checks pass, with review conversations resolved. Full release-mode packages and exact-artifact SBOMs are built from the merged `main` revision. A second approval is encouraged but is not required while Pasted has one active maintainer. Protection bypasses are for documented recovery from a GitHub or repository incident, never for routine changes.

Dependabot checks npm, Cargo, and GitHub Actions weekly. The scheduled **Dependency policy** workflow independently refreshes RustSec findings and enforces license policy, notice/SBOM freshness, and advisory-exception deadlines even when the source tree has not changed.

See the [contribution guide](https://github.com/getpasted/pasted/blob/main/CONTRIBUTING.md), [project governance](https://github.com/getpasted/pasted/blob/main/GOVERNANCE.md), [`AGENTS.md`](https://github.com/getpasted/pasted/blob/main/AGENTS.md), and the [issue tracker](https://github.com/getpasted/pasted/issues).

## Releases

- [Release automation](https://github.com/getpasted/pasted/blob/main/docs/RELEASE_AUTOMATION.md)
- [macOS release guide](https://github.com/getpasted/pasted/blob/main/docs/MACOS_RELEASE.md)
- [1.0 release checklist](https://github.com/getpasted/pasted/blob/main/docs/RELEASE_CHECKLIST_1.0.0.md)
- [1.0 release notes](https://github.com/getpasted/pasted/blob/main/docs/RELEASE_NOTES_1.0.0.md)

Only the protected release workflow creates the installable signed/notarized macOS DMG. Linux and Windows artifacts never weaken the macOS signing gate.
