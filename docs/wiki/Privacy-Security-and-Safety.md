# Privacy, Security, and Safety

Pasted stores clipboard history, settings, previews, revisions, and Activity data locally in SQLite. It includes no analytics or telemetry.

The release gate checks production dependency licenses, known Rust advisories, package sources, telemetry SDK policy, and remote webview destinations. Tagged downloads include both a deterministic source dependency SPDX SBOM and an SPDX scan of the extracted platform payload.

## Capture privacy

- Password managers and sensitive apps are included in the default blacklist.
- The blacklist can be extended in Settings.
- Capture feedback is rendered locally by Pasted. It does not send clip contents, images, file names, or paths through operating-system notifications.
- Clipboard and IPC inputs are bounded by shared resource limits.
- Activity and error events record metadata, not clipboard contents, file contents, credentials, or transformation prompts.

Optional capture previews can show clip content in Pasted's own feedback window. Turn off **Show clip preview** under **Settings → Notifications** when screen sharing or working where an on-screen preview could be observed. See [Notifications and Capture Feedback](Notifications-and-Capture-Feedback).

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
