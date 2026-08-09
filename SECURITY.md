# Security Policy

Pasted handles clipboard contents, copied file references, local history, and optional transformation providers. We take vulnerabilities that could expose or corrupt that data seriously.

## Supported versions

Security fixes are applied to the latest published Pasted release line and the current release candidate. Development snapshots and older preview artifacts are not supported release channels.

| Version | Security support |
| --- | --- |
| Latest `1.0.x` release or release candidate | Supported |
| `main` development snapshots | Best effort; not recommended for sensitive data |
| Older preview builds | Unsupported; upgrade before reporting a version-specific issue |

## Report a vulnerability privately

**Do not open a public issue for a suspected vulnerability.** Use [GitHub private vulnerability reporting](https://github.com/getpasted/pasted/security/advisories/new) instead.

Include as much of the following as is safe:

- the affected Pasted version and operating system;
- the security impact and who could be affected;
- minimal, reproducible steps using synthetic clipboard data;
- whether the issue requires a particular setting, permission, provider, file type, or display server;
- relevant logs or crash output after removing clipboard contents, credentials, personal paths, and other secrets;
- any suggested mitigation or patch.

Never send real passwords, authentication tokens, private clipboard history, or someone else's data. If sensitive evidence is essential, describe it first and wait for a maintainer to arrange the safest way to share it.

## What belongs here

Examples include:

- clipboard contents escaping Pasted without an explicit user action;
- bypasses of the blacklist, size limits, feature gates, or protected-clip safeguards;
- arbitrary command, Transform, provider, or file execution without informed consent;
- path traversal, unsafe file previewing, backup tampering, or database injection;
- unauthorized access to local history, revisions, notes, files, settings, or backups;
- secrets or clipboard contents appearing in logs, diagnostics, notifications, or activity records;
- privilege escalation or unsafe use of accessibility, autostart, global-hotkey, tray, or paste automation permissions;
- compromised release signing, update metadata, GitHub Actions, or distributed artifacts;
- a vulnerable dependency with a credible impact on Pasted.

Ordinary bugs, crashes without a security consequence, feature requests, and documentation corrections belong in [GitHub Issues](https://github.com/getpasted/pasted/issues).

## What to expect

These are response goals, not a bug-bounty or service-level agreement:

- acknowledgement within 3 business days;
- initial severity and scope assessment within 7 business days;
- a status update at least every 14 days while remediation is active;
- coordinated disclosure after a fix or effective mitigation is available.

We may ask for more information, create a private fork for collaboration, request a CVE through GitHub, or publish a GitHub Security Advisory. Credit is offered when desired and appropriate.

## Safe research

Good-faith research is welcome when you:

- test only systems and data you own or are authorized to use;
- avoid privacy violations, persistence, service disruption, and destructive testing;
- stop after demonstrating the minimum access needed to establish impact;
- give us a reasonable opportunity to investigate and fix the issue before public disclosure;
- comply with applicable law.

Pasted will not pursue legal action against good-faith research that follows this policy. This project does not currently operate a paid bug-bounty program.

## Dependency advisories

Dependabot and RustSec findings are evaluated against Pasted's reachable code and supported platforms. An advisory may remain open when the patched dependency is incompatible with a required upstream framework. Such alerts are tracked rather than silently dismissed, and are upgraded when a compatible upstream path exists.

