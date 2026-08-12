## What changed

<!-- Describe the user-visible outcome and the implementation at a useful review level. -->

## Why

<!-- Link the issue and explain the problem this solves. -->

Closes #

## Verification

- [ ] `npm run test:all` passes.
- [ ] I waited for the required dependency-policy, validation, macOS, Linux, and Windows checks before merge.
- [ ] I added or updated tests for changed behavior.
- [ ] I manually tested the affected operating systems and themes, or documented what remains untested.
- [ ] I updated `docs/wiki/`, CLI help, and other documentation where behavior or terminology changed.

## Safety and parity

- [ ] This does not silently discard, orphan, or reinterpret user data.
- [ ] Destructive or migration behavior is explicit, bounded, and tested.
- [ ] Logs, errors, IPC, and screenshots do not expose clipboard contents, credentials, or private paths.
- [ ] Platform-specific behavior has a compiling success or graceful-failure path on macOS, Windows, Linux X11, and constrained Wayland.
- [ ] Meaningful GUI and CLI surfaces remain consistent, or the intentional exception is documented.
- [ ] New visible styling uses semantic theme primitives and works across Pasted themes.
- [ ] New or changed dependencies have reviewed licenses, network/telemetry implications, notices, and SBOM entries.

## Visual evidence

<!-- Add before/after screenshots or a short recording for UI changes. Remove this section when not applicable. -->
