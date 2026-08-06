# macOS direct-download release

Pasted ships outside the Mac App Store as a signed, notarized, and stapled DMG. Release credentials stay in the macOS keychain or environment and must never be committed to the repository.

## One-time Apple setup

1. Confirm the Apple Developer Program membership is active. Pasted's permanent bundle identifier is `software.jjj.pasted`, under the developer-owned `jjj.software` namespace.
2. In Keychain Access, choose **Certificate Assistant > Request a Certificate From a Certificate Authority** and save the certificate request to disk.
3. In Apple Developer **Certificates, Identifiers & Profiles**, create a **Developer ID Application** certificate from that request. This certificate type is for direct distribution; an Apple Distribution certificate is for the App Store.
4. Download and open the `.cer` file. Verify that its certificate and private key appear together in the login keychain:

   ```sh
   security find-identity -v -p codesigning
   ```

5. Store notarization credentials in the login keychain. Apple ID authentication securely prompts for an app-specific password when `--password` is omitted:

   ```sh
   xcrun notarytool store-credentials "Pasted" \
     --apple-id "your-apple-id@example.com" \
     --team-id "YOURTEAMID"
   ```

   Alternatively, create an App Store Connect API key under **Users and Access > Integrations** and save the downloaded `.p8` file outside this repository. Apple only allows it to be downloaded once.

## Release credentials

The default local workflow uses the `Pasted` Keychain profile created above. Set `APPLE_KEYCHAIN_PROFILE` only when using a different profile name:

```sh
export APPLE_KEYCHAIN_PROFILE='Pasted'
```

Environment credentials remain supported for CI. The App Store Connect API-key route is preferred there:

```sh
export APPLE_SIGNING_IDENTITY='Developer ID Application: Your Name (TEAMID)'
export APPLE_API_ISSUER='issuer-uuid'
export APPLE_API_KEY='KEYID'
export APPLE_API_KEY_PATH='/absolute/private/path/AuthKey_KEYID.p8'
```

Tauri also supports Apple ID notarization with `APPLE_ID`, an app-specific `APPLE_PASSWORD`, and `APPLE_TEAM_ID`. Never put either credential set in files intended for sharing. Local builds should prefer the Keychain profile so secrets never enter shell history or project files.

## Build paths

Use an ad-hoc signature to exercise packaging on the current Mac without publishing anything:

```sh
npm run release:macos:local
```

Build the distributable artifact only after the Developer ID certificate and notarization credentials are available:

```sh
npm run release:macos
```

The release command runs the complete test suite, requires a Developer ID Application identity, and requires either the local Keychain profile or one complete environment credential set. With the Keychain profile, it signs the bundle, submits the DMG with `notarytool`, waits for Apple, staples the ticket, and verifies the result. Artifacts are written beneath `src-tauri/target/release/bundle/dmg/`.

Re-run verification independently with:

```sh
npm run release:macos:verify -- /absolute/path/Pasted_1.0.0_aarch64.dmg
```

## Clean-install acceptance test

Do this with the exact DMG intended for release, preferably on a second Mac or a clean macOS user account:

1. Transfer or download the DMG so macOS applies quarantine metadata.
2. Open it, drag Pasted to Applications, eject the disk image, and launch Pasted from Applications.
3. Confirm Gatekeeper opens it without an unidentified-developer warning.
4. Confirm the main window, menu-bar item, clipboard capture, global hotkeys, Accessibility guidance, launch-at-login behavior, file previews, OCR, and CLI installation.
5. Quit and relaunch; verify saved window position, settings, and clipboard history.
6. Check the artifact one final time with `npm run release:macos:verify -- /path/to/the.dmg` and retain its printed SHA-256 digest with the release notes.

The current local build host is Apple Silicon, so its default artifact is arm64. Supporting Intel Macs requires a separately installed Rust x86_64 target and a universal build; make that support decision explicitly before advertising system requirements.
