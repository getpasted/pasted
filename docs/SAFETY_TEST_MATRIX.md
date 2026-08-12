# Safety and reversibility test matrix

Pasted handles clipboard contents, file references, local history, and destructive library operations. The automated suite treats those paths as data-integrity boundaries rather than ordinary UI behavior.

Run the complete gate with:

```sh
npm run test:all
```

## Automated guarantees

| Boundary | Covered behavior |
| --- | --- |
| SQLite input safety | Untrusted clip text and metadata remain bound values and cannot become executable SQL. |
| Schema migration | Legacy Bin and pre-release Transform schemas migrate without dropping existing records; partial pre-release schemas merge safely. |
| Factory Reset | A successful reset removes user state and recreates valid first-launch defaults. A simulated mid-reset database failure rolls back every deletion. |
| Backup import | Backups preserve active and trashed clips, notes, protection, pins, Bins, ordering, Transforms, and completed OCR state. Unsupported schemas and simulated mid-import failures leave the destination unchanged. |
| Trash and retention | Trashed clips become read-only, leave active collections, remain recoverable, and are not silently counted as active history. Protected clips survive destructive retention. |
| Revisions | Content-changing actions create bounded snapshots, disabled history preserves old revisions, and a revision belonging to one clip cannot be restored onto another. |
| OCR | OCR results are content-hash checked, late results are discarded after deletion or feature disablement, backfill resumes without reprocessing, and OCR state follows Trash, restore, purge, and backup lifecycles. |
| Queue and automatic paste | Queue order persists, failed target restoration does not consume items, clipboard writes are marked internal, and internal writes are excluded from captured history. |
| File previews | File and PDF reads are bounded, missing files fail explicitly, preview caches are keyed safely, and CSV export neutralizes spreadsheet formulas. |
| CLI parity | Static audits require GUI and CLI mutations to route through shared Rust services, keep documented commands synchronized, and preserve structured `--json` contracts. |
| Platform integration | macOS, Windows, Linux X11, and constrained Wayland paths must compile and either succeed or return an explicit capability failure without consuming queued data. |
| Dependency trust | Rust and npm licenses must match reviewed policy; forbidden telemetry dependencies, remote webview connections, untrusted package sources, stale notices/SBOMs, and expired advisory exceptions fail the gate. |
| Release artifacts | Each packaged payload receives a Syft-generated SPDX SBOM and license-policy audit in addition to the deterministic 418-component source dependency SBOM. |

## Human release checks

Some guarantees depend on the operating system and cannot be made credible by unit tests alone. The release candidate checklist therefore still requires clean-machine checks for Gatekeeper, Accessibility permission, global shortcuts, target-aware paste, launch at login, tray/Dock behavior, OCR permission, and actual DMG installation.

See [`RELEASE_CHECKLIST_1.0.0.md`](RELEASE_CHECKLIST_1.0.0.md) for the acceptance run and [`RELEASE_AUTOMATION.md`](RELEASE_AUTOMATION.md) for CI credential boundaries.
