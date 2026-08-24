# Application architecture

Pasted exposes the same product capabilities through several adapters:

- the Tauri GUI command adapter in `src-tauri/src/commands.rs`;
- the `pasted` CLI adapter;
- the running-app bridge in `live_app.rs`;
- global hotkeys in `hotkey_manager.rs`;
- browser-mode development in `src/utils/tauri.ts`.

Adapters translate transport details only. Product validation, ordering, safety
limits, persistence, and mutation semantics belong in shared Rust application
services. Current shared entry points include Settings, clipboard actions,
Queue actions, manual Transforms, app locking, HUD window behavior, keyboard
shortcuts, platform capabilities, and paste automation.

## Contract boundaries

`contracts/app-events.json` is the canonical event-name registry. Run
`node scripts/generate-app-event-contracts.js` after changing it. The generated
Rust and TypeScript registries are checked in so native and frontend builds do
not require a generation step.

`shared/settings-contract.json` is the canonical persisted-settings registry.
It assigns every setting an owner, typed default, reset behavior, visibility,
mutation boundary, and validation policy. The frontend derives first-launch and
change-preview defaults from it, while `settings_contract.rs` provides the Rust
validation and repeatable page-reset API. Settings with dedicated domain work,
such as Security, Analysis, and Intelligence, declare that strategy explicitly;
factory reset remains a separate atomic deletion of the complete settings table.
The CLI uses the same reset plan for mutating resets and `--dry-run` previews.
`npm run test:architecture` rejects unregistered `AppSettings` fields, duplicate
ownership, incomplete reset defaults, or frontend/native contract drift.

The IPC audit requires every frontend invocation to have exactly one registered
Tauri command and an explicit browser implementation. Browser mode fails closed
for unknown commands.

Frontend code reaches durable capabilities through clients in `src/api`.
Components and hooks own presentation state; capability clients own command
names, request shapes, and response types. Settings, Clips, Bins, Activity,
Backup, Analysis, and Transforms have domain clients. Browser-mode behavior is
split into matching handlers under `src/mocks/browser` instead of growing one
parallel backend switch indefinitely. App state, manual Transforms, Operations,
and Queue behavior own their mock state inside those handlers so browser-mode
mutations follow the same cohesive boundaries as native capabilities.

Non-English locale catalogs load on demand. The localization snapshot exposes
catalog readiness, and application startup remains behind the splash until the
selected catalog, settings, and initial library data are all ready.

## Native application bootstrap

The native crate root is the GUI composition boundary. It configures Tauri
plugins, preserves the complete command registry for IPC auditing, and delegates
lifecycle behavior to three focused modules:

- `app_runtime.rs` initializes durable state and background services, handles
  second-instance activation and explicit exit, and applies resume-time locking;
- `app_windows.rs` owns restoration, platform presentation, focus and close
  events, and the atomic page-loaded/setup-ready reveal handshake;
- `app_tray.rs` owns tray and menu-bar icon loading, localized menu construction,
  feature-aware refreshes, and tray interactions.

Initialization order is part of the startup contract: configure hidden windows,
initialize and manage shared services, install native menus and monitoring,
register shortcuts, install the tray, then mark setup ready. The main window is
revealed only after both native setup and its webview page load are complete.

## Clipboard capture

The clipboard monitor owns polling and orchestration, but delegates deterministic
capture decisions to `clipboard_capture_policy.rs`. That policy owns source
attribution, screenshot and file-manager distinctions, composite image/file
selection, bounded image-file inspection, pasteboard change deduplication, and
recent-image coalescing. Payload persistence and user-facing feedback remain in
focused handlers under `clipboard_ingestion/`: Text owns Queue and Smart Bin
automation, Files owns preview prefetching, and Image owns bounded encoding and
OCR scheduling. Their shared context centralizes deduplication, App Exclusions,
capture suppression, source attribution, and privacy-safe feedback.

The policy is portable apart from its explicitly gated pasteboard generation
adapter. Platform clipboard reads stay in the monitor, while policy decisions
and payload preflight are covered by host-independent fixtures. The monitor is a
small coordinator for pause state, clipboard acquisition, policy resolution, and
handler dispatch.

## Compatibility boundaries

“Transform” is the public product concept. “Pipeline” remains only where needed
to read pre-1.0 database records, migration fixtures, and storage APIs. New GUI
and application contracts use Manual Transform naming.

Database changes use ordered `NamedMigration` entries. Each entry and its
schema marker commit in one transaction. Additive column helpers propagate
SQLite failures and are safe to run repeatedly.

## Native ownership map

Native entry points follow one direction: an adapter translates its transport,
an application service owns product policy and orchestration, persistence owns
durable state and transactions, and a platform adapter contains operating-system
or external-runtime behavior. An empty platform cell means the capability is
portable Rust over SQLite; it does not permit an adapter to absorb product
behavior.

| Epic root | Adapter | Application service | Persistence | Platform boundary |
| --- | --- | --- | --- | --- |
| Native crate bootstrap (`lib.rs`) | `lib.rs` registers Tauri plugins and commands and composes lifecycle callbacks. | `app_runtime.rs` owns initialization order and run events; `app_windows.rs` owns reveal and window lifecycle; `app_tray.rs` owns tray behavior. | `db::DbState` is constructed by `app_runtime.rs`; bootstrap does not implement storage. | Tauri window, tray, menu, single-instance, and run-event APIs stay in the three `app_*` owners. |
| Settings contract (`shared/settings-contract.json`) | GUI settings clients and CLI Settings commands translate values and page identifiers. | `settings_contract.rs` owns defaults, validation, visibility, mutation ownership, and repeatable scoped reset plans; dedicated page services consume its defaults where applicable. | `settings_service.rs` applies direct setting changes atomically; `db/lifecycle.rs` independently owns destructive factory reset. | OS-facing reactions such as autostart, shortcuts, tray presentation, and window appearance remain in the native adapter after a validated mutation. |
| Clipboard monitor (`clipboard_monitor.rs`) | `clipboard_monitor.rs` polls the system clipboard and coordinates pause and acquisition state. | `clipboard_capture_policy.rs` owns deterministic selection and source policy; `clipboard_ingestion/{text,files,image}.rs` own payload-specific capture workflows. | Ingestion uses the focused `db/capture.rs` and clip owners through `DbState`; the monitor never writes clip rows directly. | `arboard`, active-application lookup, macOS pasteboard change markers, and Tauri capture feedback remain at the monitor/policy edges. |
| Extraction runtime (`content_extraction.rs`) | GUI `commands/{extraction,extractors}.rs` and CLI `cli/commands/{extractors,analyzer}.rs` translate requests. | `content_analysis.rs` schedules participants and `extraction_execution.rs` translates results; `content_extraction.rs` owns the cohesive extraction contract and definitions. | `db/extractors/` owns definitions and runtime configuration; `db/stored_analysis/{extractions,searchable_text,ocr}.rs` own derived results and lifecycle state. | `content_extraction/engine_runtime/{apple_vision,tesseract,whisper,custom_command,discovery}.rs` own executable discovery and engine execution. |
| Intelligence executor (`intelligence_executor.rs`) | GUI `commands/intelligence.rs` and CLI connection, Extractor, Suggestion, and Transform adapters map transport. | `intelligence_executor/{connections,planning,execution,extractor_authoring,saved_transforms}.rs` own selection, scheduling, authoring, and execution policy. | `db/intelligence_connections.rs`, `db/extractors/`, and `db/transforms/` own the durable definitions selected or produced by those workflows. | `intelligence_provider.rs` owns provider transport and `intelligence_scheduler.rs` owns bounded provider concurrency. |
| Transformation service (`transformation_service.rs`) | GUI Transformation and Manual Transform commands, CLI Transforms, hotkeys, clipboard actions, and the live-app bridge supply typed requests. | `transformation_service/{contracts,cancellation,compatibility,operations,orchestration}.rs` own execution semantics; compatibility is an entry-point adapter, not a second engine. | `db/transforms/` owns saved definitions, execution records, clip application, and provenance. | Paste destination activation and clipboard writes remain in `clipboard_actions.rs` and paste platform services, outside Transform execution. |
| Database schema (`db/schema.rs`) | `DbState` construction is the only activation entry; GUI and CLI do not call migrations directly. | `db/schema/canonical.rs` owns ordered activation and registered migrations own bounded upgrades. | `db/schema/{clips,content_compatibility,content_registry,extractors,organization,library_items,transformation_tables,migrations}.rs` own schema families. | `db/lifecycle.rs` owns SQLite opening and library-path modes; schema modules contain no GUI or native-picker behavior. |
| Database transfers (`db/transfers.rs`) | GUI Backup/Import commands and CLI `portability.rs` own pickers, files, and output formatting. | Transfer preflight and merge policy live with the transactional use case in `db/transfers/{library_validation,library_import,library_export,clip_transfer}.rs`. | Those same focused owners serialize, validate, and transactionally merge History and Organization data; `db/full_backups.rs` separately owns replacement snapshots. | Async native file pickers remain in GUI commands; filesystem reads and writes remain outside Tauri command-thread execution. |
| Database Transforms (`db/transforms.rs`) | Transformation adapters call `DbState` through stable domain methods and consume exported contracts from `db.rs`. | `transformation_service.rs` owns execution; `db/transforms/manual.rs` is bounded legacy/manual storage compatibility, not another execution service. | `db/transforms/{definitions,executions,applications,repository,operation_compatibility}.rs` own lifecycle, executions, atomic clip application and provenance, row decoding, and legacy operation fields. | No platform behavior belongs in Transform persistence. |
| Stored Analysis persistence (`db/stored_analysis.rs`) | Analyzer, Inspector, Extractor, OCR, and rescan adapters consume typed Analysis execution results. | Participant execution modules own result translation; stored Analysis methods only validate current clip identity and commit derived records. | `db/stored_analysis/{classifications,inspections,extractions,searchable_text,ocr}.rs` own their respective durable results and OCR lifecycle state. | Filesystem availability and media metadata remain live observations outside durable stored Analysis. |

`content_extraction.rs` is the deliberate size exception in this epic. It is a
cohesive contract and definition module: serialized Extractor definitions,
recipe compatibility, bounded shared types, and the engine trait must remain
reviewable together. It is not the engine registry and does not own process
execution. The small registry in `content_extraction/engine_runtime.rs` composes
the platform engine adapters, while each adapter owns its executable discovery
or invocation. New engine implementation belongs under `engine_runtime/`, not
in the contract module.

The database integration root now composes focused persistence owners under
`src-tauri/src/db`. Schema activation, transfers, Transforms, stored Analysis,
clip lifecycle, organization, Settings, retention, and lifecycle operations
belong in their domain modules rather than returning to the integration root.

The GUI command adapter follows the same shape under `src-tauri/src/commands`.
Activity, App Lock, Queue, retention, and Library Storage commands are registered
from focused adapter modules; their Tauri request mapping should not return to
the command integration root. App Lock and Queue adapters delegate product
behavior to the same application services used by hotkeys, the CLI, and the
live-app bridge. Library Storage keeps native folder selection asynchronous and
delegates relocation and recovery behavior to the shared storage services.

The CLI integration root delegates capability-specific argument mapping and
output formatting to modules under `src-tauri/src/cli/commands`.
The root owns startup, feature gating, dispatch, and help output only. Activity,
analysis, App Lock, Bins, Clips, connections, Extractors, history, live-app
controls, maintenance, Operations, portability, registry metadata, retention,
Settings, storage, Suggestions, and Transforms have focused adapters. Shared
argument parsing and presentation helpers are split by domain beside them. CLI
contract audits read the complete module tree so moving an adapter cannot
accidentally remove its GUI-parity checks. Architecture ratchets cap both the
integration root and every individual adapter or support module.

## Testing boundaries

Application workflows that touch the system clipboard or paste destination
separate the commit decision from operating-system ports. Portable tests assert
that failed writes or target activation never consume Queue entries. Platform
availability is tested separately from shared mutation behavior.

`npm run test:architecture`, `npm run test:ipc`, and `npm run test:cli` protect
these boundaries. `npm run test:all` remains the completion gate.

The architecture audit also ratchets the remaining large integration modules.
They may shrink as capabilities move out, but growing past their recorded line
budgets requires extracting a boundary rather than adding more orchestration.
