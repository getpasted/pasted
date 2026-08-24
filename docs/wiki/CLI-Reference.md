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
pasted search [query] [--clip <type>] [--content <type>] [--format <format>] [--source <source>] [--trash] [--limit N] [--offset N] [--json]
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
pasted settings list|get|set|reset [arguments] [--dry-run] [--json]
pasted app-lock status|enable|change-passphrase|disable|lock|unlock [--stdin] [--json]
pasted app-lock idle <never|1m|5m|1h|8h> [--stdin] [--json]
pasted app-lock lock-on-sleep <on|off> [--stdin] [--json]
pasted app-lock lock-on-restart <on|off> [--stdin] [--json]
pasted app-lock capture-while-locked <on|off> [--stdin] [--json]
pasted app-lock system-auth <on|off> [--stdin] [--json]
pasted app-lock apple-watch <on|off> [--stdin] [--json]
pasted app-lock reset --yes [--json]
pasted recording status|pause|resume [--json]
pasted queue status|start|stop|add|remove|order|paste|paste-all [arguments] [--json]
pasted clear --yes [--json]
```

`settings reset <page>` uses the same scoped defaults as the corresponding Settings footer. Supported pages are `general`, `notifications`, `hotkeys`, `app-exclusions`, `security`, `analysis`, and `intelligence`. Add `--dry-run` to inspect the effective changes without saving them. Security accepts `--stdin` when an actual reset requires App Lock authentication. Structured output reports the reset scope, whether it was a dry run, and the effective setting changes.

`pasted search` uses the same query grammar, fuzzy case-insensitive collection-axis filters, Functionality gates, chronological ordering, and extracted-text index as Search in the app and Quick HUD. It is unavailable when Clip Search is disabled under Functionality; background indexing continues. `--clip`, `--content`, `--format`, and `--source` combine with helpers in the query. `--limit` accepts 1–500 items per page. `--json` returns `{ "schemaVersion": 1, "items", "totalCount", "limit", "offset" }`; each item keeps the stable snake-case Clip fields, and extracted OCR or transcript text is not returned. Offset pages reflect the current library, so restart at offset 0 after mutating clips between requests.

`copy` accepts bounded stdin when text is omitted. `list` and `search` provide bounded pagination; both can inspect Trash, while `list` can select a Bin or pinned clips. `search` filters Clip Type, Content Type, File Format, and Source with case-insensitive partial matching. Its structured records expose canonical `content_type`, `content_types`, `file_formats`, and `source` fields. A disabled Functionality axis cannot be used as a search filter. `import sources` reports supported managers and detected locations. `import` reads a source without modification and merges supported text while skipping duplicates. `retention` manages History, Trash, Activity History, and per-clip revision policies. `settings` reads or changes persisted values; app-bound visual or operating-system effects apply when the app observes the setting or next launches. `clear` requires `--yes` and permanently removes unpinned, unprotected clips from History.

App-lock mutations read the passphrase from a hidden terminal prompt or bounded stdin with `--stdin`; the passphrase is never accepted as a command-line argument. `change-passphrase --stdin` accepts exactly two lines: the current passphrase followed by the new passphrase. `lock` and `unlock` contact the running app. Enabling `system-auth` or `apple-watch` verifies that the operating-system method is available; the operating-system prompt appears when that method is used to unlock. Live availability can change while a configured method remains enabled, such as while a paired Watch is locked or out of range. App-lock commands are unavailable when App Lock is disabled under Functionality; `pasted settings set enableAppLock true` restores the feature. While app lock is enabled, other CLI commands require a valid `PASTED_APP_LOCK_PASSPHRASE` in their process environment. `status --json` reports the stable `enabled`, `systemAuthEnabled`, `systemAuthAvailable`, `systemAuthLabel`, `appleWatchEnabled`, `appleWatchAvailable`, `idleMinutes`, `lockOnSleep`, `lockOnRestart`, and `captureWhileLocked` fields without exposing the verifier.

`app-lock disable` requires the current passphrase and removes the passphrase verifier plus every system-authentication preference. If the passphrase is unavailable, quit Pasted and run `pasted app-lock reset --yes`. Recovery reset does not require the passphrase because App Lock protects the interface rather than encrypting the database. It disables App Lock, removes its verifier and authenticator preferences, preserves timing and capture policies, records a local Activity event, and does not delete clips or unrelated settings. The command refuses to reset a running app.

`recording`, `queue`, `clip copy`, `clip paste`, and `ocr cancel` contact the running app through a bounded private request. Clipboard monitoring, Queue state, paste targeting, and cancellation therefore remain inside the process that owns them. These commands can launch Pasted when its executable is installed beside the CLI.

`activity list` exposes structured retained records to scripts. `activity export` writes every retained entry as OpenTelemetry-shaped JSON or analysis-friendly CSV; omitting the path writes to stdout. JSON archives include a versioned Pasted resource block and event timestamp, observed timestamp, event name, severity, body, and attributes. `activity import` accepts bounded JSON or CSV exports, validates the complete input, deduplicates records, applies the current Activity retention policy, and never replays imported actions. The file extension selects the format unless `--format` is supplied. `activity clear` permanently removes every retained entry and requires `--yes`.

`transfer export` writes the portable History and Organization JSON available under Settings → Storage → Export. `transfer inspect` performs the same bounded structural and referential preflight as import without changing saved data. `transfer import` validates the complete file before opening a write transaction, updates matching stable identities and content hashes, adds new items, and leaves unrelated data unchanged. The former `archive` command remains as a compatibility alias.

`clip export` and `clip import` are the CLI equivalents of selecting Clips under Settings → Storage → Export or choosing a Clips file under Import. JSON preserves complete clip records. CSV carries text-based rows for spreadsheet workflows. Imports validate the complete file before writing and skip existing content hashes.

`database location`, `database protection`, `database move`, and `database default` inspect or change SQLite storage. `database protection` reports `protected`, `notDetected`, or `unknown` for the volume containing the active database; it never treats an unavailable operating-system check as proof that encryption is off. The former `library` command remains as a compatibility alias.

## Full backup and restore

```text
pasted backup create <path.pastedbackup> [--json]
pasted backup inspect <path.pastedbackup> [--json]
pasted backup restore <path.pastedbackup> --yes [--json]
```

Quit the graphical app before CLI restore. Full Backup uses SQLite’s online backup API to create an unencrypted snapshot of every durable Pasted-owned table. Protect the resulting file with an encrypted archive or encrypted storage when needed. Full Restore validates the backup, migrates a temporary copy, creates a complete pre-restore recovery backup, and then replaces the active state. Provider and operating-system credentials and original files referenced by file clips remain external; saved references and paths are preserved.

## Clip actions

```text
pasted clip get <id> [--json]
pasted clip note <id> [--text <text> | --clear | --stdin] [--json]
pasted clip revisions <id> [--limit <n>] [--offset <n>] [--json]
pasted clip restore-revision <id> <revision-id> [--json]
pasted clip provenance <id> [--json]
pasted clip copy|paste <id> [--json]
pasted clip hotkey <id> <hotkey|none> [--json]
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
pasted bin hotkey <id> <hotkey|none> [--json]
pasted bin protect <id> <on|off> [--json]
```

`bin order` replaces the complete saved order and rejects invalid/duplicate membership atomically. `bin protect` controls inherited protection for manual Bins and Tags; Smart Bins cannot confer protection. `clip hotkey` assigns a paste-by-ID hotkey and explicitly protects the clip. Clearing the hotkey does not remove that protection. Hotkey mutations require Hotkeys under Functionality; use `pasted settings set enableHotkeys true` to re-enable them. Disabling Hotkeys preserves assignments. A running app stops dispatching them immediately and releases their system registrations when its settings refresh or the app next launches.

Smart Bin rules use the version 1 `clip_type`, `content_type`, `file_format`, and `source` collection axes. Conditions support case-insensitive `is` and `contains` operators and combine with `match: "any"` or `match: "all"`.

```sh
pasted bin create --name "Safari Links" \
  --smart-rule-json '{"version":1,"conditions":[{"type":"content_type","operator":"is","value":"link"},{"type":"source","operator":"contains","value":"Safari"}],"match":"all"}' \
  --json
```

Use `pasted type list --json` for registered Content Type IDs and `pasted insights summary --json` for observed Clip Types, File Formats, and Sources. The corresponding Functionality setting must be enabled for an axis to match. Invalid rule shapes are rejected. See [Smart Bin Rule Contract](Smart-Bin-Rule-Contract.md) for bounds and compatibility behavior.

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
pasted registry list [--kind capture|inspector|extractor|classifier|suggestion|operation|transform] [--all] [--json]
pasted registry enable|disable --kind extractor|classifier|operation --ref <stable-ref> [--json]
pasted inspector list [--json]
pasted inspector get <ref> [--json]
pasted inspector run [--text <text> | --clip <id> | --stdin] [--apply] [--json]
pasted inspector rescan --yes [--json]
pasted suggestion list [--json]
pasted suggestion get <ref> [--json]
pasted suggestion run [--text <text> | --clip <id> | --stdin] [--json]
pasted extractor list [--json]
pasted extractor get <ref> [--json]
pasted extractor create [--name <name>] [--description <text>] (--recipe <recipe.json> | --prompt <request>) [--format <format>]... [--connection <id>] [--priority <number>] [--enabled|--disabled] [--json]
pasted extractor update <ref> --recipe <recipe.json> [--format <format>]... [options] [--json]
pasted extractor propose --prompt <request> [--connection <id>] [--json]
pasted extractor history <ref> [--json]
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
pasted classifier list [--json]
pasted classifier get <ref> [--json]
pasted classifier create --name <name> --type <type> --regex <pattern> [--json]
pasted classifier update <ref> [--name <name>] [--type <type>] [--regex <pattern>] [--validator <name|none>] [--priority <number>] [--enabled|--disabled] [--json]
pasted classifier duplicate <ref> [--name <name>] [--json]
pasted classifier delete <ref> [--json]
pasted classifier run <ref> [--text <text> | --clip <id> | --stdin] [--apply] [--json]
pasted classifier restore-defaults [--json]
pasted classifier rescan --yes [--json]
```

Registry JSON includes Capture definitions and each Analysis participant’s `analysisPass`, legacy `inputContract` and `outputContract` strings, typed `participantContract`, and `typeRelations`. The typed contract lists required and produced representations. The current `accepts` relations use the legacy `image` and `file` registry IDs to describe Clip Type applicability; `classifies_as` names the semantic Content Type produced by a Classifier. Structure, File Format, and Media Metadata Inspectors run in the inspect pass, Extractors run in the extract pass, Classifiers run in the classify pass, and Smart Actions runs in the suggest pass. Every participant runs at most once after its declared inputs become available. Inspector runs preview by default; `--apply` persists content-hash-bound results for a clip. Built-in Inspectors and Suggestions are immutable. Extractor, Classifier, and Transform management uses the lifecycle verbs appropriate to each asset.

`pasted analyzer run` returns one versioned preview of the applicable passes. Its JSON includes content-free structure, classification, Smart Action suggestions, and participant outcomes, but never original text, extracted text, image bytes, or file paths. Interactive policy includes suggestion when Transformations is enabled; capture, background, and rescan stop after classification. Image and file extraction are opt-in with `--extract` because OCR and transcription can be comparatively expensive. File references never enter text Classifiers or Suggestions; only a produced searchable-text representation can feed later passes.

Every Extractor now stores the same versioned `recipe-v1` document. Recipes declare one or both input kinds, local executable discovery or absolute paths, argv tokens, time limits, resources, step artifacts, and how searchable text is captured. Commands run directly without a shell, in a private workspace with a reduced environment and bounded input, output, and runtime. Supported placeholders are `{input.path}`, `{input.stagedPath}`, `{request.path}`, `{output.path}`, `{output.base}`, `{step.ID.output}`, and `{resource.ID.path}`. A step can capture standard output, a generated text file, Pasted protocol JSON, or nothing. New custom Extractors remain disabled unless `--enabled` is explicit.

Automatic scans, rescans, and whole-Analyzer extraction run every enabled, available Extractor compatible with the clip in priority order. Successful outputs are deduplicated and combined into searchable text. A targeted `pasted extractor run REF` still runs only the requested Extractor.

`extractor propose` asks an enabled Intelligence connection to draft this deterministic local recipe; it does not run tools or install software. `extractor create --prompt` drafts and saves in one command. The original request, provider/model identity, structured response, and timestamps are retained locally as reviewable authoring history. Runtime extraction never contacts AI. Use `--recipe` for the complete no-AI path; [`poppler-pdf-extractor.json`](../examples/poppler-pdf-extractor.json) is a directly runnable example.

The shipped Apple Vision, Tesseract, and Whisper definitions use the same recipe runner. Apple Vision invokes the explicit bundled Pasted bridge, Tesseract invokes `tesseract`, and Whisper declares an FFmpeg preparation step followed by `whisper-cli` plus a required local GGML model resource. Installing commands or selecting resources never occurs implicitly. `pasted extractor run extractor:whisper-transcription --clip <id> --apply` stores hash-bound searchable text and provenance without replacing file references.

Inspector run JSON uses the versioned Analysis envelope. Structure reports origin, byte count, text counts, image dimensions, or file item count and filename extensions without returning the inspected clipboard content or paths. The File Format Inspector reads bounded file signatures, persists verified `fileFormats`, and never guesses from an extension. `inspector rescan --yes` backfills current file clips and reports missing external references as `missingCount`, not failures. File availability, file/directory counts, and total size are returned separately as live observations and are not persisted. When ffprobe or MediaInfo is installed, file runs can also return live aggregate `mediaMetadata` containing container, codec, stream-count, and duration facts. Up to eight files are inspected with bounded execution and output; paths never appear in results.

Suggestion run JSON uses the same versioned envelope and is always non-mutating. Smart Actions reports bounded content signals plus stable Transform references, names, revisions, and reasons. It does not return the analyzed text or execute a suggestion. Capture, background, and rescan policies stop before suggestion.

Extractor run JSON serializes the shared Extractor application result used by the app, CLI, and background OCR. It includes the shared `formatVersion`, policy, final pass, and privacy-safe `participants` fields alongside `targetKind`, `targetRef`, `outcome` (`produced`, `no_output`, or `failed`), `output`, `classificationMatches`, a structured `failure`, `appliedClipId`, `ocrUpdated`, and `classificationUpdated`. Each classification match identifies its Classifier and Content Type plus bounded character offsets, without returning the matched text. Preview results report no applied clip and both update flags as false. Applied runs claim the current clip by ID and content hash, and report an applied clip only after extracted text and classifications were persisted. A failed Extractor can still have `ocrUpdated: true` when its bounded failure code was successfully recorded for retry and diagnostics. Failures exit nonzero and never include input image or clipboard content.

Classifier run JSON serializes the corresponding shared Classification application result. It includes the shared `formatVersion`, policy, final pass, and privacy-safe `participants` fields alongside `outcome` (`matched`, `no_match`, or `failed`), `matched`, distinct `contentTypes`, occurrence-level `matches`, `failure`, and `appliedClipId`. Classifiers can report multiple Content Types and multiple occurrences of each type. Applied runs execute against the clip inside the database transaction, so the reported result and mutation cannot diverge. Participant summaries and matches never include analyzed text.

Content Type and Group IDs are stable. Built-in Content Types can be renamed, regrouped, and given a different icon, but cannot be archived; custom Content Types can be archived without reinterpreting historical clips. Custom Groups can be archived only when no Content Types use them. Archiving a custom Content Type disables Classifiers that produce it. Classifiers run in ascending priority order, but priority orders matches instead of selecting a single winner. Repeat `--regex` to provide alternatives. Shipped Classifiers are editable and deletable; `restore-defaults` recovers their original definitions without removing custom Classifiers. `rescan` explicitly rebuilds Content Type occurrences while preserving each clip's Text, Image, or Files Clip Type.

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

`insights summary --json` keeps structural `clip_types`, verified `file_formats`, and semantic `content_types` separate. Clip Type entries use `clip_type`; Content Type entries use `content_type`. File Formats come from bounded byte-signature inspection, are ordered by clip count, and are limited to the top 24 values.

## Intentional app-only boundaries

Window, title-bar, dock, tray, cursor, emoji-picker, native-menu, preview-rendering, and operating-system permission-prompt commands remain graphical presentation behavior. Hotkey registration is owned by the running app; the CLI can persist hotkey settings and the app applies them when active or at launch. Provider scheduler cancellation remains process-local: a CLI Transform exits with its CLI process, while the app manages and cancels its own active jobs. Installation diagnostics remain available through `pasted diagnostics` without exposing internal presentation helpers.
