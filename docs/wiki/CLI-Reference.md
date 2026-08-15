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
pasted list [--limit N] [--offset N] [--bin ID | --pinned | --trash] [--json]
pasted search [query] [--type <type>] [--source <source>] [--trash] [--limit N] [--offset N] [--json]
pasted import sources [--json]
pasted import <alfred|pastebot|pasta|paste|copyclip|maccy|flycut> [history-file-or-folder] [--json]
pasted activity list [--limit N|--all] [--offset N] [--category VALUE] [--severity VALUE] [--event NAME] [--json]
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
                 [--log-count <number|unlimited>] [--log-days <number|forever>]
                 [--revision-count <number|unlimited>] [--json]
pasted settings list|get|set [arguments] [--json]
pasted recording status|pause|resume [--json]
pasted queue status|start|stop|add|remove|order|paste|paste-all [arguments] [--json]
pasted clear --yes [--json]
```

`copy` accepts bounded stdin when text is omitted. `list` and `search` provide bounded pagination; both can inspect Trash, while `list` can select a Bin or pinned clips. `search` reproduces Content Type and Source views with exact filters, and its structured records use the canonical `source` field. `import sources` reports supported managers and detected locations. `import` reads a source without modification and merges supported text while skipping duplicates. `retention` manages History, Trash, Activity History, and per-clip revision policies. `settings` reads or changes persisted values; app-bound visual or operating-system effects apply when the app observes the setting or next launches. `clear` requires `--yes` and permanently removes unpinned, unprotected clips from History.

`recording`, `queue`, `clip copy`, `clip paste`, and `ocr cancel` contact the running app through a bounded private request. Clipboard monitoring, Queue state, paste targeting, and cancellation therefore remain inside the process that owns them. These commands can launch Pasted when its executable is installed beside the CLI.

`activity list` exposes structured retained records to scripts. `activity export` writes every retained entry as OpenTelemetry-shaped JSON or analysis-friendly CSV; omitting the path writes to stdout. JSON archives include a versioned Pasted resource block and event timestamp, observed timestamp, event name, severity, body, and attributes. `activity import` accepts bounded JSON or CSV exports, validates the complete input, deduplicates records, applies the current Activity retention policy, and never replays imported actions. The file extension selects the format unless `--format` is supplied. `activity clear` permanently removes every retained entry and requires `--yes`.

`transfer export` writes the portable History and Organization JSON available under Settings → Storage → Export. `transfer inspect` performs the same bounded structural and referential preflight as import without changing saved data. `transfer import` validates the complete file before opening a write transaction, updates matching stable identities and content hashes, adds new items, and leaves unrelated data unchanged. The former `archive` command remains as a compatibility alias.

`clip export` and `clip import` are the CLI equivalents of selecting Clips under Settings → Storage → Export or choosing a Clips file under Import. JSON preserves complete clip records. CSV carries text-based rows for spreadsheet workflows. Imports validate the complete file before writing and skip existing content hashes.

`database location`, `database move`, and `database default` inspect or change the SQLite storage location. The former `library` command remains as a compatibility alias.

## Full backup and restore

```text
pasted backup create <path.pastedbackup> [--json]
pasted backup inspect <path.pastedbackup> [--json]
pasted backup restore <path.pastedbackup> --yes [--json]
```

Quit the graphical app before CLI restore. Full Backup uses SQLite’s online backup API to snapshot every durable Pasted-owned table. Full Restore validates the backup, migrates a temporary copy, creates a complete pre-restore recovery backup, and then replaces the active state. Provider and operating-system credentials and original files referenced by file clips remain external; saved references and paths are preserved.

## Clip actions

```text
pasted clip get <id> [--json]
pasted clip note <id> [--text <text> | --clear | --stdin] [--json]
pasted clip revisions <id> [--limit <n>] [--offset <n>] [--json]
pasted clip restore-revision <id> <revision-id> [--json]
pasted clip provenance <id> [--json]
pasted clip copy|paste <id> [--json]
pasted clip pin|unpin <id>... [--json]
pasted clip order-pinned <id>... [--json]
pasted clip protect|unprotect <id>... [--json]
pasted clip trash|restore <id>... [--json]
pasted clip restore-all [--json]
pasted clip purge <id>... --yes [--json]
pasted clip empty-trash --yes [--json]
pasted clip assign <bin-id|none> <id>... [--json]
```

Mutating commands report stable summaries and use explicit desired states rather than blind toggles. `restore-all` returns every trashed clip to History and reports the restored IDs in its structured result.

## Bins

```text
pasted bin list [--json]
pasted bin get <id> [--json]
pasted bin create --name <name> [--icon <icon>] [--color <color>] [--smart-rule-json <json>] [--transform <ref>] [--json]
pasted bin update <id> [--name <name>] [--icon <icon>] [--color <color>] [--smart-rule-json <json> | --clear-smart-rule] [--json]
pasted bin duplicate <id> [--name <name>] [--json]
pasted bin delete <id> [--disposition keep|trash|move] [--move-to <bin-id>] [--json]
pasted bin clips <bin-id> [--json]
pasted bin order <bin-id> <clip-id>... [--json]
pasted bin transform <id> <transform-ref|none> [--json]
pasted bin shortcut <id> <shortcut|none> [--json]
```

`bin order` replaces the complete saved order and rejects invalid/duplicate membership atomically.

## Transforms

```text
pasted transform list
pasted transform get <ref> [--json]
pasted transform plan [--intent <text> | --stdin] [--sample <text>] [--mode pinned|adaptive] [--connection <id>] [--json]
pasted transform test --plan-json <json> [--text <text> | --stdin] [--connection <id>] [--json]
pasted transform create --name <name> (--intent <text> | --plan-json <json> | --steps-json <json>) [--json]
pasted transform update <ref> [options] [--json]
pasted transform duplicate <ref> [--name <name>] [--json]
pasted transform delete <ref> [--json]
pasted transform run <ref> [--text TEXT | --clip ID | --stdin] [--apply] [--json]
pasted operation list [--json]
pasted operation get <ref> [--json]
pasted operation create --name <name> --type <type> [--config-json <json>] [--category <category>] [--json]
pasted operation update <ref> [options] [--json]
pasted operation duplicate <ref> [--name <name>] [--json]
pasted operation delete <ref> [--json]
pasted operation run <ref> [--text TEXT | --clip ID | --stdin] [--json]
pasted connection list [--json]
pasted connection get <id> [--json]
pasted connection detect [--json]
pasted connection create --name <name> --provider <kind> [--endpoint <value>] [--model <model>] [--credential-ref <ref>] [--json]
pasted connection update <id> [options] [--json]
pasted connection delete <id> [--json]
pasted connection order <id>... [--json]
```

`--apply` requires `--clip ID` so the expected input can be validated and a revision created. `--replace` remains an alias for compatibility. Intent planning uses the same bounded provider selection, credential references, and fallback behavior as the graphical composer. Operations are experimental in 1.0.

## Content Analysis

```text
pasted analyzer run [--text <text> | --clip <id> | --stdin] [--policy capture|background|interactive|rescan] [--extract] [--json]
pasted registry list [--kind capture|inspector|extractor|detector|enricher|operation|transform] [--all] [--json]
pasted registry enable|disable --kind extractor|detector|operation --ref <stable-ref> [--json]
pasted inspector list [--json]
pasted inspector get <ref> [--json]
pasted inspector run [--text <text> | --clip <id> | --stdin] [--apply] [--json]
pasted enricher list [--json]
pasted enricher get <ref> [--json]
pasted enricher run [--text <text> | --clip <id> | --stdin] [--json]
pasted extractor list [--json]
pasted extractor get <ref> [--json]
pasted extractor create [--name <name>] [--description <text>] [--engine <engine>] [--model <path>] [--input <contract>] [--output <contract>] [--priority <number>] [--enabled|--disabled] [--json]
pasted extractor update <ref> [options] [--json]
pasted extractor duplicate <ref> [--name <name>] [--json]
pasted extractor delete <ref> [--json]
pasted extractor run <ref> (--clip <id> | --file <path>) [--apply] [--json]
pasted extractor restore-defaults [--json]
pasted type list [--all] [--json]
pasted type create --id <id> --name <name> [--icon <icon>] [--group <group>] [--json]
pasted type update <id> [--name <name>] [--icon <icon>] [--group <group>] [--json]
pasted type archive <id> [--json]
pasted type restore <id> [--json]
pasted type restore-defaults [--json]
pasted type group-list [--all] [--json]
pasted type group-create --id <id> --name <name> [--order <number>] [--json]
pasted type group-update <id> [--name <name>] [--order <number>] [--json]
pasted type group-archive <id> [--json]
pasted type group-restore <id> [--json]
pasted type group-delete <id> [--json]
pasted type group-restore-defaults [--json]
pasted detector list [--json]
pasted detector get <ref> [--json]
pasted detector create --name <name> --type <type> --regex <pattern> [--json]
pasted detector update <ref> [--name <name>] [--type <type>] [--regex <pattern>] [--validator <name|none>] [--priority <number>] [--enabled|--disabled] [--json]
pasted detector duplicate <ref> [--name <name>] [--json]
pasted detector delete <ref> [--json]
pasted detector run <ref> [--text <text> | --clip <id> | --stdin] [--apply] [--json]
pasted detector restore-defaults [--json]
pasted detector rescan --yes [--json]
```

Registry JSON includes Capture definitions and each Analysis participant’s `analysisPass`, legacy `inputContract` and `outputContract` strings, typed `participantContract`, and `typeRelations`. The typed contract lists required and produced representations. The current `accepts` relations use the legacy `image` and `file` registry IDs to describe Clip Type applicability; `classifies_as` names the semantic Content Type produced by a Detector. Structure and Media Metadata Inspectors run in the inspect pass, Extractors run in the extract pass, Detectors run in the classify pass, and Smart Actions runs in the enrich pass. Every participant runs at most once after its declared inputs become available. Inspector runs preview by default; `--apply` persists content-hash-bound structural metadata for a clip. Built-in Inspectors and Enrichers are immutable. Extractor, Detector, and Transform management uses the lifecycle verbs appropriate to each asset.

`pasted analyzer run` returns one versioned preview of the applicable passes. Its JSON includes content-free structure, classification, Smart Action recommendations, and participant outcomes, but never original text, extracted text, image bytes, or file paths. Interactive policy includes enrichment when Transformations is enabled; capture, background, and rescan stop after classification. Image and file extraction are opt-in with `--extract` because OCR and transcription can be comparatively expensive. File references never enter text Detectors or Enrichers; only a produced searchable-text representation can feed later passes.

Whisper Transcription uses engine `whisper-cpp-cli-v1`. Configure a local GGML model with `pasted extractor update extractor:whisper-transcription --model /absolute/path/to/ggml-model.bin`. Use `--no-model` to clear it. Installing whisper.cpp or selecting a model never occurs implicitly. `pasted extractor run extractor:whisper-transcription --clip <id> --apply` stores hash-bound searchable text and provenance without replacing the clip's file references.

Inspector run JSON uses the versioned Analysis envelope. Structure reports origin, byte count, text counts, image dimensions, or file item count and extensions without returning the inspected clipboard content or paths. File availability, file/directory counts, and total size are returned separately as live observations and are not persisted. When ffprobe or MediaInfo is installed, file runs can also return live aggregate `mediaMetadata` containing container, codec, stream-count, and duration facts. Up to eight files are inspected with bounded execution and output; paths never appear in results.

Enricher run JSON uses the same versioned envelope and is always non-mutating. Smart Actions reports bounded content signals plus stable Transform references, names, revisions, and reasons. It does not return the analyzed text or execute a recommendation. Capture, background, and rescan policies stop before enrichment.

Extractor run JSON serializes the shared Extractor application result used by the app, CLI, and background OCR. It includes the shared `formatVersion`, policy, final pass, and privacy-safe `participants` fields alongside `targetKind`, `targetRef`, `outcome` (`produced`, `no_output`, or `failed`), `output`, `detectedType`, `matchedDetectorRef`, a structured `failure`, `appliedClipId`, `ocrUpdated`, and `classificationUpdated`. Preview results report no applied clip and both update flags as false. Applied runs claim the current clip by ID and content hash, and report an applied clip only after OCR state was persisted. A failed Extractor can still have `ocrUpdated: true` when its bounded failure code was successfully recorded for retry and diagnostics. Failures exit nonzero and never include input image or clipboard content.

Detector run JSON serializes the corresponding shared Detection application result. It includes the shared `formatVersion`, policy, final pass, and privacy-safe `participants` fields alongside `outcome` (`matched`, `no_match`, or `failed`), `matched`, `detectedType`, `matchedDetectorRef`, `failure`, and `appliedClipId`. Applied runs execute against the clip inside the database transaction, so the reported result and mutation cannot diverge. Participant summaries contain only stable references, passes, outcomes, and neutral failures—not analyzed text.

Content Type and Group IDs are stable. Built-in Content Types can be renamed, regrouped, and given a different icon, but cannot be archived; custom Content Types can be archived without reinterpreting historical clips. Custom Groups can be archived only when no Content Types use them. Archiving a custom Content Type disables Detectors that produce it. Detectors run in ascending priority order. Repeat `--regex` to provide alternatives. Shipped Detectors are editable and deletable; `restore-defaults` recovers their original definitions without removing custom Detectors. `rescan` explicitly reclassifies existing text clips while preserving Image and Files Clip Types.

## Maintenance

```text
pasted diagnostics [--json]
pasted licenses [--json]
pasted insights summary [--json]
pasted ocr status [--json]
pasted ocr scan [--clip <id>] [--json]
pasted ocr retry [--json]
pasted ocr cancel [--json]
pasted reset --yes [--json]
```

`licenses` remains available without a database and even when the optional clipboard-management CLI feature is disabled. `reset` is intentionally gated by `--yes`. Other commands respect feature settings and exit with an explicit explanation when a capability is disabled or unavailable.

`insights summary --json` keeps structural `clip_types`, bounded `file_formats`, and semantic `content_types` separate. Clip Type entries use `clip_type`; Content Type entries use `content_type`. File Formats are ordered by clip count and limited to the top 24 distinct extensions.

## Intentional app-only boundaries

Window, title-bar, dock, tray, cursor, emoji-picker, native-menu, preview-rendering, and operating-system permission-prompt commands remain graphical presentation behavior. Shortcut registration is owned by the running app; the CLI can persist shortcut settings and the app applies them when active or at launch. Provider scheduler cancellation remains process-local: a CLI Transform exits with its CLI process, while the app manages and cancels its own active jobs. Installation diagnostics remain available through `pasted diagnostics` without exposing internal presentation helpers.
