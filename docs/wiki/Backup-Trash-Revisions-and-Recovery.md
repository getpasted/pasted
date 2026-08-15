# Backup, Trash, Revisions, and Recovery

## Trash

Trash is soft deletion. Trashed clips are read-only: Bin assignment, notes, Transforms, OCR mutation, and drag targets are disabled until restore. Trash may be disabled as a feature; existing trashed data remains intact.

Restore one clip from its Trash actions, or use **Settings → General → Trash → Restore Trashed Clips** to return every trashed clip to History in one operation. The equivalent scriptable command is `pasted clip restore-all [--json]`. Restoring does not recreate category Bin assignments removed when clips entered Trash; retained tag associations remain intact.

## Revision History

Revisions record content-changing actions and enough organization context to undo a Transform-plus-Bin move. Pin, protection, and other lightweight attributes are not a total rewind log.

- Preview a revision before restoring it.
- Restoring creates an inverse revision so the action can be reversed.
- Revision retention is configurable or unlimited.
- Disabling revisions preserves existing history but makes new edits irreversible.

## Full backup and restore

**Settings → Storage → Full Backup and Restore** creates an exact recovery point: a validated SQLite snapshot of all durable state owned by Pasted, plus saved interface/window state. This includes clips in History and Trash, stored clipboard images, all Bins and ordering, revisions, Activity, settings and hotkeys, blacklist rules, Copy Queue state, Transforms, Operations, automations, execution history, OCR state, Extractors, Detectors, derived Analysis classifications, Content Types, and intelligence connection setup.

Full Restore replaces the current state only after the selected backup passes format and SQLite integrity checks. Before replacement, Pasted creates another complete backup of the current state beside the active database. The CLI equivalents are `pasted backup create <path.pastedbackup>` and `pasted backup restore <path.pastedbackup> --yes`.

Copied screenshots and bitmap clips are stored in the database and are included. A copied file clip normally stores the original file’s path and metadata rather than another copy of the file, so the backup preserves that path but does not bundle the external file. API keys and passwords stay in Keychain, environment variables, or provider storage. Connection setup is restored and can use the same credential again when it is available. Derived preview caches are rebuilt when needed.

## History and organization transfer

**Settings → Storage → History and Organization** exports portable JSON for merging clip history and organization into another installation. It includes clips, Trash state, stored images, all Bins and ordering, Transforms, Operations, OCR state, Content Types, and Detectors. Import preflights the complete file, updates matches by stable identity or content hash, adds new items, and leaves unrelated data unchanged. Settings, Extractor configuration, derived Analysis classifications, Activity, revision snapshots, automations, intelligence connections, and credentials are not included.

Activity can be selected under **Settings → Storage → Export**. JSON and CSV both support a bounded, deduplicating round trip of inert audit records; CSV is also suitable for spreadsheet reporting. **Import** recognizes Activity exports automatically. Activity files do not contain clipboard contents, and importing one never replays the recorded actions. The current Activity retention limits still apply after import.

Transfer files include clips in History and Trash, notes, pins, protection, Bins and ordering, custom Operations, manual and assisted Transforms, Bin bindings, and OCR lifecycle metadata.

Imports are bounded and transactional. Unsupported or malformed data is rejected; a failure partway through leaves the destination unchanged.

## Factory Reset

Factory Reset removes local data and preferences after explicit confirmation, recreates starter Smart Bins, and relaunches. It is transactional: a database failure rolls back the reset rather than leaving partially cleared data.

Export before reset if any data matters.
