# Project Governance

Pasted is an open-source, project-led application maintained by Triple J Software, Inc. This document explains how decisions and repository authority work.

## Project leadership

The project owner is the final decision-maker for product direction, supported platforms, release readiness, security response, branding, and repository access. Maintainers are trusted contributors who may triage issues, review pull requests, manage releases, or administer specific project systems according to their granted permissions.

Maintainer access is earned through sustained, constructive participation and a demonstrated understanding of Pasted's user-data, privacy, cross-platform, and release-safety requirements. Access follows least privilege and may be narrowed or removed when responsibilities change.

## Decisions

Routine decisions happen in issues and pull requests. Larger or irreversible changes should begin with a written problem, intended outcome, alternatives, migration plan, and platform impact.

When priorities conflict, Pasted generally favors:

1. protecting clipboard data and user trust;
2. preserving reversibility and predictable local behavior;
3. maintaining a coherent, fast product over accumulating disconnected options;
4. shared GUI and CLI domain behavior;
5. explicit cross-platform limitations over silent partial success;
6. maintainable implementation over short-lived cleverness.

The project owner may decline, defer, or redirect a technically valid contribution when it does not fit the product direction or maintenance capacity.

## Reviews and merging

Changes to application code require passing CI and human review appropriate to their risk. Data migrations, destructive behavior, clipboard boundaries, process execution, signing, release workflows, and security controls receive additional scrutiny.

Automation—including Dependabot and Codex—may create branches, commits, and draft pull requests. Automation does not approve or merge its own work. Protected workflows, repository policy, secrets, and release authority remain human-controlled.

## Releases

Only designated maintainers may publish official Pasted releases. Release artifacts must follow the documented signing, notarization, checksum, and clean-install process. Experimental platform packages must be labeled honestly and cannot weaken the supported macOS release gate.

## Security and conduct

Security reports follow [SECURITY.md](SECURITY.md). Community behavior follows [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Confidential reports are handled by maintainers without a conflict of interest whenever practical.

## Changes to governance

Governance changes are proposed through a pull request and require approval from the project owner. The repository's MIT License preserves every contributor's right to fork the code if they prefer a different direction.

