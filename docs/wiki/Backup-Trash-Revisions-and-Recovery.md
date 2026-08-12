# Backup, Trash, Revisions, and Recovery

## Trash

Trash is soft deletion. Trashed clips are read-only: Bin assignment, notes, Transforms, OCR mutation, and drag targets are disabled until restore. Trash may be disabled as a feature; existing trashed data remains intact.

## Revision History

Revisions record content-changing actions and enough organization context to undo a Transform-plus-Bin move. Pin, protection, and other lightweight attributes are not a total rewind log.

- Preview a revision before restoring it.
- Restoring creates an inverse revision so the action can be reversed.
- Revision retention is configurable or unlimited.
- Disabling revisions preserves existing history but makes new edits irreversible.

## Backup and import

**Settings → Backup & Import** exports one JSON backup to a location you choose. Import merges by stable identities/content hashes and refreshes the live app.

Backups include active and trashed clips, notes, pins, protection, Bins and ordering, custom Operations, Pipelines, saved Transforms, Bin bindings, and OCR lifecycle metadata.

Imports are bounded and transactional. Unsupported or malformed data is rejected; a failure partway through leaves the destination unchanged.

## Factory Reset

Factory Reset removes the local Pasted library and preferences after explicit confirmation, recreates starter Smart Bins, and relaunches. It is transactional: a database failure rolls back the reset rather than leaving a half-empty library.

Export before reset if any data matters.

