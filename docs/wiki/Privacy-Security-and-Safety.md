# Privacy, Security, and Safety

Pasted stores clipboard history, settings, previews, revisions, and Activity data locally in SQLite. It includes no analytics or telemetry.

The release gate checks production dependency licenses, known Rust advisories, package sources, telemetry SDK policy, and remote webview destinations. Tagged downloads include both a deterministic source dependency SPDX SBOM and an SPDX scan of the extracted platform payload.

## Capture privacy

- Common password managers and sensitive apps are included in App Exclusions by default.
- Text, image, file, and Pasted hotkey rules can be configured independently under **Settings → App Exclusions**. Blocking every content kind presents as an automatic capture pause; partial rules skip only the selected kinds.
- Native Wayland sessions do not expose the globally focused application, so App Exclusions cannot be enforced there.
- Capture feedback is rendered locally by Pasted. It does not send clip contents, images, file names, or paths through operating-system notifications.
- Clipboard and IPC inputs are bounded by shared resource limits.
- Activity and error events record metadata, not clipboard contents, file contents, credentials, or transformation prompts.

Optional capture previews can show clip content in Pasted's own feedback window. Turn off **Show clip preview** under **Settings → Notifications** when screen sharing or working where an on-screen preview could be observed. See [Notifications and Capture Feedback](Notifications-and-Capture-Feedback).

## App lock

Enable **App Lock** under **Settings → Functionality** to make Security available. **Settings → Security** can then require a Pasted passphrase before the graphical app loads clipboard history. Disabling the Functionality feature immediately removes lock enforcement and hides Security without deleting the saved passphrase or preferences. The app starts locked by default; **Lock after restart** can open it unlocked instead without changing the sleep or inactivity policies. Capture continues while the interface is locked by default. Turn off **Capture while locked** to discard clipboard changes instead; those changes are not saved to History or the Queue and do not start OCR, automations, Activity records, or capture notifications. Capture previews, the HUD, global Pasted shortcuts, live-app data commands, and data-bearing native menu actions are unavailable until unlock. Window management and Quit remain available.

Passphrases are stored only as salted Argon2 verifiers and may be any non-empty length. Native unlock uses LocalAuthentication for separately configured Touch ID and Apple Watch on macOS and Windows Hello on Windows; the operating system returns only success or failure and never shares biometric data with Pasted. Linux retains the complete passphrase path and reports system authentication as unavailable when the desktop session does not provide a supported authentication broker.

Lock Pasted and Unlock Pasted can be assigned under **Settings → Hotkeys** and default to `Alt+Shift+L` and `Alt+Shift+U`. Unlock Pasted brings the locked window forward and requests an enabled system authentication method; when none is available, it focuses the passphrase field. The shortcut never bypasses authentication.

App lock is a local privacy barrier, not database encryption. A user or process with access to the library file can still inspect it outside Pasted. Storage reports whether supported operating-system volume encryption was detected for the active library location. Full Backup is an unencrypted SQLite snapshot and includes the verifier and lock configuration so restored state retains the same interface protection; History and Organization transfer does not. The CLI exposes the same lock policies and a closed-app recovery reset for a forgotten passphrase. Recovery reset clears only App Lock credentials and authenticator preferences; it does not delete clips or unrelated settings.

## Intelligence

Clip content leaves Pasted only when you explicitly run an intelligence-assisted Transform through an enabled Connection. Pasted stores provider metadata and credential references; credentials remain with the operating system, provider, or authenticated CLI.

## Data integrity

- SQL uses bound parameters for untrusted content.
- Destructive and multi-record operations use transactions.
- Protected clips survive automated destructive retention.
- Queue items are consumed only after successful paste.
- Full Restore validates before replacement and creates a recovery backup first. History and Organization import and Factory Reset roll back on simulated mid-operation failures.
- Revisions are scoped to their owning clip.

The complete automated coverage matrix is maintained in [`docs/SAFETY_TEST_MATRIX.md`](https://github.com/getpasted/pasted/blob/main/docs/SAFETY_TEST_MATRIX.md).

## Reporting a vulnerability

Do not disclose suspected vulnerabilities or sensitive clipboard data in a public issue. Read the repository's [Security Policy](https://github.com/getpasted/pasted/security/policy), then use [GitHub private vulnerability reporting](https://github.com/getpasted/pasted/security/advisories/new).
