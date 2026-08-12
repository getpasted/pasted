# Contributing to Pasted

Thank you for helping Pasted remember better.

## Start with the user outcome

For substantial changes, open or comment on an issue before investing in an implementation. Describe the problem, the observable outcome, and the acceptance criteria. Small fixes and documentation improvements can go directly to a pull request.

Security vulnerabilities must follow [SECURITY.md](SECURITY.md), not the public issue tracker. Support questions belong in the channels described by [SUPPORT.md](SUPPORT.md).

## Development setup

You will need:

- Node.js 22 or newer;
- the stable Rust toolchain;
- platform libraries required by Tauri 2;
- macOS for Apple Vision OCR and signed macOS packaging.

```sh
npm ci
npm run tauri dev
```

Linux contributors should also read the [Linux and SteamOS testing guide](docs/LINUX_STEAMOS_TESTING.md).

## Design and implementation rules

Read [AGENTS.md](AGENTS.md) before changing code. In particular:

- keep meaningful GUI and CLI capabilities on shared Rust domain services;
- preserve user data and make destructive behavior explicit and reversible where practical;
- never log clipboard contents, credentials, private paths, or unbounded IPC payloads;
- gate platform-specific behavior and provide an explicit graceful-failure path elsewhere;
- use Tailwind for structure and Pasted semantic theme primitives for visible colors and surfaces;
- do not increase CSS, parity, or architectural debt budgets to make a change pass;
- add regression tests for bugs and dangerous mutation paths.

If a feature intentionally ships on only the GUI or CLI, document why.

## Test before opening a pull request

Run the complete local gate:

```sh
npm run test:all
```

It covers Rust formatting, Clippy, frontend compilation, unit tests, IPC boundaries, security rules, feature gates, collection contracts, menu behavior, CLI parity, Transform behavior, activity events, CSS architecture, WCAG contrast, and platform chrome.

Platform-sensitive changes also need focused human testing on every affected environment. Never infer Windows or constrained Wayland success solely from a macOS build.

### Dependency license notices

Changes to `Cargo.lock`, `package-lock.json`, or the approved license policy require regenerating the checked-in third-party notice files:

```sh
cargo install --locked --features cli --version 0.9.1 cargo-about
npm run licenses:generate
npm run licenses:check
```

Do not add a newly encountered license to `about.toml` merely to make generation pass. Review its distribution and source-availability obligations first. The generator includes production npm packages, the supported-platform Rust graph, package copyright/NOTICE files, and complete selected license texts.

## Pull requests

- Keep each pull request focused enough to review and revert.
- Explain user-visible behavior and important implementation choices.
- Identify data migrations, destructive paths, permission changes, new network access, or new dependencies.
- Include screenshots or recordings for visible UI changes across relevant themes.
- Update the Wiki source in `docs/wiki/` when behavior or terminology changes.
- Update CLI help and parity tests when a shared capability changes.
- Do not commit secrets, local databases, signing material, generated release artifacts, or personal clipboard data.

AI-assisted contributions are welcome, but the submitter remains responsible for understanding, testing, and safely disclosing the resulting code.

## Commit and review expectations

Use clear, imperative commit messages. Reviewers may ask for narrower scope, additional tests, safer migrations, or a platform fallback before merging. A green build is required but is not a substitute for reviewing user-data and privacy consequences.

By contributing, you agree that your contribution is licensed under the repository's [MIT License](LICENSE).
