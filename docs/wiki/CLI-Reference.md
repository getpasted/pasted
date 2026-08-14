# CLI Reference

The app bundle includes the native `pasted` executable and uses the same SQLite domain services as the GUI.

Install it from **Settings → About**, or on macOS:

```sh
sudo ln -s /Applications/Pasted.app/Contents/MacOS/pasted /usr/local/bin/pasted
```

## History

```text
pasted copy "Hello"
cat server.log | pasted copy
pasted list [limit]
pasted search [query] [--type <type>] [--source <source>] [--json]
pasted import <alfred|pastebot|pasta|paste|copyclip|maccy|flycut> [history-file-or-folder] [--json]
pasted activity list [--limit N|--all] [--json]
pasted activity export [path] [--format json|csv]
pasted activity import <path> [--format json|csv] [--json]
pasted activity clear --yes [--json]
pasted transfer export <path.json> [--json]
pasted transfer inspect <path.json> [--json]
pasted transfer import <path.json> [--json]
pasted clip export [path] [--format json|csv]
pasted clip import <path> [--format json|csv] [--json]
pasted retention [--count <number|unlimited>] [--days <number|forever>]
                 [--trash-count <number|unlimited>] [--trash-days <number|forever>]
                 [--log-count <number|unlimited>] [--log-days <number|forever>] [--json]
pasted clear
```

`copy` accepts bounded stdin when text is omitted. `search` can reproduce the GUI's calculated Type and Source views with exact `--type` and `--source` filters; its structured records use the canonical `source` field and remain stable for scripts. `import` uses the source app's standard macOS location when no path is supplied, reads it without modification, and merges supported text while skipping duplicates. Paste and Pastebot may use a protected data-folder path instead of a database file. If necessary, Pasted raises the History limit so a successful import is not immediately trimmed. `retention` reads or updates the same count and age policies as Settings for History, Trash, and Activity History. Each pair works independently; `unlimited` disables its count limit and `forever` disables its age limit. Pinned and protected clips in History remain exempt, and protected clips are never auto-purged from Trash. `clear` permanently removes unpinned, unprotected clips from History.

`activity list` exposes structured retained records to scripts. `activity export` writes every retained entry as OpenTelemetry-shaped JSON or analysis-friendly CSV; omitting the path writes to stdout. JSON archives include a versioned Pasted resource block and event timestamp, observed timestamp, event name, severity, body, and attributes. `activity import` accepts bounded JSON or CSV exports, validates the complete input, deduplicates records, applies the current Activity retention policy, and never replays imported actions. The file extension selects the format unless `--format` is supplied. `activity clear` permanently removes every retained entry and requires `--yes`.

`transfer export` writes the portable History and Organization JSON available under Settings → Storage → Export. `transfer inspect` performs the same bounded structural and referential preflight as import without changing saved data. `transfer import` validates the complete file before opening a write transaction, updates matching stable identities and content hashes, adds new items, and leaves unrelated data unchanged. The former `archive` command remains as a compatibility alias.

`clip export` and `clip import` are the CLI equivalents of selecting Clips under Settings → Storage → Export or choosing a Clips file under Import. JSON preserves complete clip records. CSV carries text-based rows for spreadsheet workflows. Imports validate the complete file before writing and skip existing content hashes.

`database location`, `database move`, and `database default` inspect or change the SQLite storage location. The former `library` command remains as a compatibility alias.

## Full backup and restore

```text
pasted backup create <path.pastedbackup> [--json]
pasted backup restore <path.pastedbackup> --yes [--json]
```

Quit the graphical app before CLI restore. Full Backup uses SQLite’s online backup API to snapshot every durable Pasted-owned table. Full Restore validates the backup, migrates a temporary copy, creates a complete pre-restore recovery backup, and then replaces the active state. Provider and operating-system credentials and original files referenced by file clips remain external; saved references and paths are preserved.

## Clip actions

```text
pasted clip get <id> [--json]
pasted clip pin|unpin <id>... [--json]
pasted clip protect|unprotect <id>... [--json]
pasted clip trash|restore <id>... [--json]
pasted clip restore-all [--json]
pasted clip assign <bin-id|none> <id>... [--json]
```

Mutating commands report stable summaries and use explicit desired states rather than blind toggles. `restore-all` returns every trashed clip to History and reports the restored IDs in its structured result.

## Bins

```text
pasted bin list [--json]
pasted bin clips <bin-id> [--json]
pasted bin order <bin-id> <clip-id>... [--json]
```

`bin order` replaces the complete saved order and rejects invalid/duplicate membership atomically.

## Transforms

```text
pasted transform list
pasted transform get <ref> [--json]
pasted transform create --name <name> (--plan-json <json> | --steps-json <json>) [--json]
pasted transform update <ref> [options] [--json]
pasted transform duplicate <ref> [--name <name>] [--json]
pasted transform delete <ref> [--json]
pasted transform run <ref> [--text TEXT | --clip ID | --stdin] [--apply] [--json]
pasted operation list [--json]
pasted operation run <ref> [--text TEXT | --clip ID | --stdin] [--json]
```

`--apply` requires `--clip ID` so the expected input can be validated and a revision created. `--replace` remains an alias for compatibility. Operations are experimental in 1.0.

## Content Analysis

```text
pasted registry list [--kind extractor|detector|operation|transform] [--all] [--json]
pasted registry enable|disable --kind extractor|detector|operation --ref <stable-ref> [--json]
pasted extractor list [--json]
pasted extractor get <ref> [--json]
pasted extractor create [--name <name>] [--description <text>] [--engine <engine>] [--input <contract>] [--output <contract>] [--priority <number>] [--enabled|--disabled] [--json]
pasted extractor update <ref> [options] [--json]
pasted extractor duplicate <ref> [--name <name>] [--json]
pasted extractor delete <ref> [--json]
pasted extractor run <ref> (--clip <id> | --file <path>) [--apply] [--json]
pasted extractor restore-defaults
pasted type list [--all] [--json]
pasted type create --id <id> --name <name> [--icon <icon>] [--group <group>] [--json]
pasted type update <id> [--name <name>] [--icon <icon>] [--group <group>] [--json]
pasted type archive <id>
pasted type restore <id>
pasted type restore-defaults
pasted type group-list [--all] [--json]
pasted type group-create --id <id> --name <name> [--order <number>] [--json]
pasted type group-update <id> [--name <name>] [--order <number>] [--json]
pasted type group-archive <id>
pasted type group-restore <id>
pasted type group-delete <id>
pasted type group-restore-defaults
pasted detector list [--json]
pasted detector get <ref> [--json]
pasted detector create --name <name> --type <type> --regex <pattern> [--json]
pasted detector update <ref> [--name <name>] [--type <type>] [--regex <pattern>] [--validator <name|none>] [--priority <number>] [--enabled|--disabled] [--json]
pasted detector duplicate <ref> [--name <name>] [--json]
pasted detector delete <ref> [--json]
pasted detector run <ref> [--text <text> | --clip <id> | --stdin] [--apply] [--json]
pasted detector restore-defaults
pasted detector rescan --yes [--json]
```

Registry JSON includes each analysis participant’s `analysisPass`, `inputContract`, and `outputContract`. Extractors run in the extract pass and Detectors run in the classify pass. Every participant runs at most once after its declared inputs become available. Extractor, Detector, and Transform management uses the same core verbs: `list`, `get`, `create`, `update`, `duplicate`, `delete`, and `run`. Runs preview by default; `--apply` explicitly mutates a clip.

Type and Group IDs are stable. Built-in Types can be renamed, regrouped, and given a different icon, but cannot be archived; custom Types can be archived without reinterpreting historical clips. Custom Groups can be archived only when no Types use them. Archiving a custom Type disables detectors that produce it. Detectors run in ascending priority order. Repeat `--regex` to provide alternatives. Shipped detectors are editable and deletable; `restore-defaults` recovers their original definitions without removing custom detectors. `rescan` explicitly reclassifies existing text clips with the current enabled detector order while preserving image and file types.

## Maintenance

```text
pasted diagnostics [--json]
pasted licenses [--json]
pasted ocr status [--json]
pasted ocr scan
pasted reset --yes [--json]
```

`licenses` remains available without a database and even when the optional clipboard-management CLI feature is disabled. `reset` is intentionally gated by `--yes`. Other commands respect feature settings and exit with an explicit explanation when a capability is disabled or unavailable.
