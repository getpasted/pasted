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
pasted search <query>
pasted clear
```

`copy` accepts bounded stdin when text is omitted. `clear` permanently removes unpinned, unprotected active history.

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

## Maintenance

```text
pasted diagnostics [--json]
pasted ocr status [--json]
pasted ocr scan
pasted reset --yes [--json]
```

`reset` is intentionally gated by `--yes`. The CLI respects feature settings and exits with an explicit explanation when a capability is disabled or unavailable.

