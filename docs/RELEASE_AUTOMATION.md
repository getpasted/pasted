# Desktop build and release automation

Pasted keeps its build workflows in this repository so the packaging definition is reviewed and versioned with the source it builds. Credentials never belong in the repository: store them in GitHub Environment secrets.

## Workflows

- **Desktop builds** runs on pull requests, pushes to `main`, and manual dispatches. It executes the complete test suite once, then produces credential-free macOS, Linux, and Windows test packages as workflow artifacts. The macOS artifact is ad-hoc signed and is only for testing.
- **Desktop release** runs manually as a packaging rehearsal or from a `vX.Y.Z` tag. Tag runs require a signed macOS package, preserve per-platform checksums, and assemble one draft GitHub Release for final human review.

Use GitHub Environments named `release-macos`, `release-linux`, and `release-publish`. Add required reviewers to the platform and publishing environments if the repository plan supports them. The Linux environment currently needs no secrets; it exists so Linux publishing can acquire an approval gate or GPG key later without changing the workflow shape.

## macOS secrets

Export the **Developer ID Application** certificate and private key from Keychain Access as a password-protected `.p12`, then store:

| Secret | Value |
| --- | --- |
| `APPLE_CERTIFICATE` | Single-line base64 of the `.p12`: `openssl base64 -A -in DeveloperID.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | Password chosen while exporting the `.p12` |
| `KEYCHAIN_PASSWORD` | A new random password used only for the ephemeral CI keychain |
| `APPLE_API_ISSUER` | App Store Connect API issuer UUID |
| `APPLE_API_KEY` | App Store Connect API key ID |
| `APPLE_API_PRIVATE_KEY` | Single-line base64 of `AuthKey_KEYID.p8`: `openssl base64 -A -in AuthKey_KEYID.p8` |

The runner imports these only into an ephemeral keychain, builds a universal Apple Silicon/Intel DMG, lets Tauri submit it for notarization, and verifies the Developer ID signature, Gatekeeper assessment, and stapled ticket before upload.

## Deferred Windows signing

Unsigned Windows packages remain available from **Desktop builds** for compatibility testing. Windows is intentionally excluded from public tagged releases until Pasted has a trusted code-signing certificate or Trusted Signing account. Adding Windows later must not weaken the signed macOS release gate.

## Cutting a release

1. Update the matching version in `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
2. Merge and let **Desktop builds** pass on `main`.
3. Create and push an annotated version tag, for example `git tag -a v1.0.0 -m "Pasted 1.0.0"` followed by `git push origin v1.0.0`.
4. Approve protected GitHub Environments if prompted.
5. Download and clean-install test the exact draft-release artifacts.
6. Edit release notes as needed, then publish the draft.

Publishing also exposes the generated `pasted.rb` release asset. The separate Homebrew tap pulls that public asset on its next scheduled run; see [Homebrew distribution](HOMEBREW.md). No release credential is shared with the tap.

Manual dispatch of **Desktop release** exercises native signing and packaging but intentionally does not create a GitHub Release because it has no immutable version tag.

## 1.0 platform matrix

- **macOS:** one universal DMG with native Apple Silicon and Intel binaries.
- **Linux:** one x86_64 AppImage. X11 and Wayland are detected at runtime rather than shipped as separate applications.
- **Windows:** unsigned x86_64 NSIS and MSI CI artifacts for testing; public installers are deferred until code signing is configured. Windows on ARM can use its x64 compatibility layer until a native ARM64 package has real hardware coverage.

Additional Linux package formats and native Linux/Windows ARM64 builds can be added based on demand. They should not multiply the initial release surface before Pasted has machines or repeatable environments that exercise them.
