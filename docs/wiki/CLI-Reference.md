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
pasted retention [--count <number|unlimited>] [--days <number|forever>]
                 [--trash-count <number|unlimited>] [--trash-days <number|forever>]
                 [--log-count <number|unlimited>] [--log-days <number|forever>] [--json]
pasted clear
```

`copy` accepts bounded stdin when text is omitted. `search` can reproduce the GUI's calculated Type and Source views with exact `--type` and `--source` filters; its structured records use the canonical `source` field and remain stable for scripts. `import` uses the source app's standard macOS location when no path is supplied, reads it without modification, and merges supported text while skipping duplicates. Paste and Pastebot may use a protected data-folder path instead of a database file. If necessary, Pasted raises the history limit so a successful import is not immediately trimmed. `retention` reads or updates the same count and age policies as Settings for active history, Trash, and Activity History. Each pair works independently; `unlimited` disables its count limit and `forever` disables its age limit. Pinned and protected active clips remain exempt, and protected clips are never auto-purged from Trash. `clear` permanently removes unpinned, unprotected active history.

## Clip actions

```text
pasted clip get <id> [--json]
pasted clip pin|unpin <id>... [--json]
pasted clip protect|unprotect <id>... [--json]
pasted clip trash|restore <id>... [--json]
pasted clip assign <bin-id|none> <id>... [--json]
```

Mutating commands report stable summaries and use explicit desired states rather than blind toggles.

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
pasted transform run <ref> [--text TEXT | --clip ID | --stdin] [--replace]
pasted operation list [--json]
pasted operation run <ref> [--text TEXT | --clip ID | --stdin] [--json]
pasted pipeline list [--json]
pasted pipeline run <ref> [--text TEXT | --clip ID | --stdin] [--json]
```

`--replace` requires `--clip ID` so Pasted can validate the expected input and create a revision. Operations and Pipelines are experimental in 1.0.

## Detection

```text
pasted registry list [--kind detector|operation|pipeline] [--all] [--json]
pasted registry enable|disable --kind detector|operation --ref <stable-ref> [--json]
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
pasted detector create --name <name> --type <type> --regex <pattern> [--json]
pasted detector update <id> [--name <name>] [--type <type>] [--regex <pattern>] [--validator <name|none>] [--priority <number>] [--enabled|--disabled] [--json]
pasted detector delete <id>
pasted detector restore-defaults
pasted detector rescan --yes [--json]
```

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
