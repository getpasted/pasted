# Desktop build and release automation

Pasted keeps its build workflows in this repository so the packaging definition is reviewed and versioned with the source it builds. Credentials never belong in the repository: store them in GitHub Environment secrets.

## Workflows

- **Desktop builds** runs on pull requests, pushes to `main`, and manual dispatches. Draft pull requests run the inexpensive dependency review and Rust policy gates while awaiting automated or human review. Marking a pull request ready runs the complete test suite, then compiles the native library, GUI, CLI, and test harnesses on macOS, Linux, and Windows only after primary validation succeeds. After merge, the workflow builds full distributables, creates exact-payload SPDX SBOMs, and uploads credential-free Linux and Windows test packages. The macOS universal CLI and DMG build in parallel; their ad-hoc intermediates expire after one day because Gatekeeper would reject them.
- **Dependency policy** runs every Monday after the Dependabot update windows and on manual dispatch. It re-evaluates RustSec data, advisory-exception expiry dates, licenses, notices, sources, mission policy, and source-SBOM freshness even when no repository change triggers the ordinary build.
- **Refresh Dependabot compliance artifacts** runs only for same-repository Dependabot changes to npm or Cargo manifests and lockfiles. It installs npm packages without lifecycle scripts, regenerates the checked-in notices and source SBOM, and commits only those three canonical artifacts back to the update branch. The writable workflow rejects any other pull-request or generated file before it acts.
- **Desktop release** is the only source of an installable macOS DMG. Its `Pasted-release-macOS` artifact is Developer ID signed, submitted to Apple, stapled, verified, extracted for an artifact-level SBOM, and audited before upload. It runs manually as a packaging rehearsal or from a `vX.Y.Z` tag; tag runs preserve per-platform checksums and SPDX SBOMs, include signed updater payloads, include explicitly experimental unsigned Windows packages, and assemble one draft GitHub Release for final human review.
- **Updater feed** runs only after a versioned GitHub Release is published. It verifies the release's detached updater signatures and renders one static manifest from immutable versioned assets. Every release advances the prerelease channel; a stable version also advances the stable channel. Draft artifacts can be tested manually without becoming discoverable, but an end-to-end channel update necessarily uses a published prerelease.

Updater channel releases are mutable manifest pointers, not application-source releases. Their tags target the current default branch so the repository-scoped `GITHUB_TOKEN` can create each channel without the unavailable Workflows permission; `latest.json` still points only to immutable, versioned release assets and signatures.

## Protected `main` workflow

`main` is a protected integration branch. Start from an up-to-date `main`, create a short-lived branch, commit and push there, then open a draft pull request for early automated or human review. Resolve actionable feedback before marking it ready. The ready pull request must pass **Review dependency changes**, **Verify Rust dependency policy**, **Validate**, and the macOS, Linux, and Windows smoke jobs before merge. Full release-mode packaging and exact-artifact SBOM audits run from the merged `main` revision. Review conversations must be resolved. Direct pushes, force-pushes, and branch deletion are blocked.

The repository does not require a second approval while it has one active maintainer; doing so would make ordinary maintenance impossible. Add a required approval when a second regular reviewer is available. Administrators retain emergency recovery access through GitHub, but a protection bypass is reserved for an actual GitHub or repository incident. Preserve the source branch, document why the bypass was necessary, run the complete checks as soon as service returns, and revert immediately if they do not pass.

## Dependency and artifact policy

`about.toml` and `dependency-policy.json` are reviewed allowlists, not catalogs to expand automatically. `cargo-deny` rejects unapproved Rust licenses, unknown registries or Git sources, wildcard requirements, and unacknowledged RustSec findings. Advisory exceptions require a reason and an expiration date; expiration fails the ordinary local gate even if the upstream dependency has not yet moved.

The checked-in `THIRD_PARTY_SBOM.spdx.json` describes the complete supported-target Rust runtime, production npm graph, and shipped installer-tool components. Because statically linked packages cannot always be reconstructed from a desktop executable, each platform job also extracts the exact DMG, AppImage, or NSIS payload and scans it with pinned Syft. The source and artifact SBOMs are complementary and ship with tagged releases. Artifact audits require platform-specific application and CLI file evidence, inspect every package Syft can identify, and fail unknown package licenses unless the package is an explicitly reviewed Pasted application or packaging record; forbidden license families always fail.

The Windows artifact allowlist also names the four NSIS stock plug-ins embedded by the installer (covered by NSIS's zlib/libpng license) and Tauri's Apache-2.0 `nsis_tauri_utils` plug-in. Syft identifies those temporary installer records by filename but cannot recover their licenses from the DLL metadata, so each is reviewed and allowed by exact name rather than by a broad pattern.

Mission policy is separate from copyright licensing. The dependency audit blocks known telemetry SDKs, undeclared network-capable direct dependencies, and remote webview CSP destinations. User-configured intelligence providers and permission-declared operations remain explicit user actions; Insights remains an on-device SQLite query and is guarded as such.

Use GitHub Environments named `release-macos`, `release-linux`, `release-windows`, and `release-publish`. Add required reviewers to the platform and publishing environments if the repository plan supports them. Each platform environment receives the same updater signing secrets; Apple credentials remain scoped to macOS.

## Updater signing secrets

Generate one long-lived Tauri updater key pair on a trusted offline or maintainer-owned machine:

```sh
npm run tauri signer generate -- -w /absolute/private/path/pasted-updater.key
```

Back up the private key and password outside GitHub. Store the private key and public key exactly as generated:

| Secret | Value |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | Complete private updater key, or its absolute path for a local release |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password protecting that private key |
| `PASTED_UPDATER_PUBLIC_KEY` | Complete matching public key embedded into every release build |

The workflow fails closed if any value is missing. Rotating this key requires a transition release signed by the old key and containing the new public key; losing the private key without such a release strands installed versions on their current channel.

## macOS secrets

Export the **Developer ID Application** certificate and private key from Keychain Access as a password-protected `.p12`, then store:

| Secret | Value |
| --- | --- |
| `APPLE_CERTIFICATE` | Single-line base64 of the `.p12`: `openssl base64 -A -in DeveloperID.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | Password chosen while exporting the `.p12` |
| `KEYCHAIN_PASSWORD` | A new random password used only for the ephemeral CI keychain |
| `APPLE_ID` | Apple Account email associated with the Developer Program membership |
| `APPLE_PASSWORD` | App-specific password created for Pasted CI; never the Apple Account password |
| `APPLE_TEAM_ID` | Developer Team ID (`46SE2P5ZAH`) |

The runner imports the certificate only into an ephemeral keychain, builds a universal Apple Silicon/Intel app and DMG, lets Tauri notarize and staple the app, then separately notarizes and staples the outer disk image. The release gate verifies the Developer ID signature, Gatekeeper assessment, universal architectures, and both stapled tickets before upload. App Store Connect API credentials can replace the Apple Account credentials later, but they are intentionally not a 1.0 prerequisite.

Pasted ships both the private `pasted-app` GUI executable and the public `pasted` CLI. Before Tauri bundles the universal app, `scripts/build-macos-universal-cli.sh` builds both CLI architectures and merges them with `lipo`; the release workflow then signs that nested CLI before Tauri signs the enclosing app. This keeps the bundled CLI and standalone release artifact universal, notarizable, and covered by the Developer ID signature.

## Experimental Windows distribution

Unsigned Windows packages are available from **Desktop builds** for compatibility testing and from tagged releases as explicitly experimental downloads. The release includes an x86_64 NSIS installer, a portable executable, and a SHA-256 manifest. Windows may identify their publisher as unknown; Smart App Control or organization-managed policies can block unsigned applications entirely. Windows must not be presented as a frictionless stable download until Pasted has a trusted code-signing certificate or Trusted Signing account. Its unsigned release job remains independent of—and must never weaken—the signed macOS release gate.

## Cutting a release

1. Update the matching version in `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
2. Regenerate notices and the source SBOM, then merge through a protected pull request after dependency review, dependency policy, and **Desktop builds** pass.
3. Create and push an annotated version tag, for example `git tag -a v1.0.0-rc.1 -m "Pasted 1.0.0 RC 1"` followed by `git push origin v1.0.0-rc.1`.
4. Approve protected GitHub Environments if prompted.
5. Download and clean-install test the exact draft-release artifacts.
6. Exercise a signed update from the previous published channel build and verify that a tampered signature is rejected. Because RC5 has no updater, manually install the first updater-bearing RC and prove it can update to a later RC or final candidate before publishing stable 1.0.
7. For a stable release, update `docs/RELEASE_ANNOUNCEMENT.md` with a concise standalone announcement in Pasted's marketing voice and replace its `pasted-release` marker with the exact version tag. RC releases do not publish Discussions.
8. Replace the generated changelog-only body with complete release notes, then publish the draft as a pre-release for an RC tag or a full release for a final tag. Publication advances the matching updater feed. Stable publication also creates the version-matched post in the GitHub Discussions **Announcements** category.

Every published release must be useful without following another link. Include a short introduction, user-facing highlights, the supported download matrix, verification or signing expectations, the issue-reporting link, and the full changelog link. The changelog link supplements these details; it never replaces them.

Use human-friendly release titles such as `Pasted 1.0.0 RC4`. Keep SemVer identifiers such as `v1.0.0-rc.4`, package versions, and generated artifact filenames in their machine-friendly lowercase form.

Publishing also exposes the generated `pasted.rb` release asset. The separate Homebrew tap pulls that public asset on its next scheduled run; see [Homebrew distribution](HOMEBREW.md). No release credential is shared with the tap.

Stable announcement publication is fail-closed and idempotent. The workflow requires `docs/RELEASE_ANNOUNCEMENT.md` to contain exactly one marker matching the published tag, such as `<!-- pasted-release:v1.1.0 -->`; a missing or stale marker fails the job. A rerun finds that marker in recent Discussions and reuses the existing post instead of creating a duplicate. Keep the announcement useful on its own, retain the site's concise and lightly irreverent voice, and link to the immutable versioned release.

Manual dispatch of **Desktop release** exercises native signing and packaging but intentionally does not create a GitHub Release because it has no immutable version tag.

## 1.0 platform matrix

- **macOS:** one universal DMG with native Apple Silicon and Intel binaries.
- **Linux:** one x86_64 AppImage. X11 and Wayland are detected at runtime rather than shipped as separate applications.
- **Windows:** unsigned x86_64 NSIS installer and portable executable, published as experimental downloads with checksums. Windows on ARM can use its x64 compatibility layer until a native ARM64 package has real hardware coverage. Stable, warning-free distribution remains deferred until code signing is configured.

Additional Linux package formats and native Linux/Windows ARM64 builds can be added based on demand. They should not multiply the initial release surface before Pasted has machines or repeatable environments that exercise them.
