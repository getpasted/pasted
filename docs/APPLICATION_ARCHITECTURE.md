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

The IPC audit requires every frontend invocation to have exactly one registered
Tauri command and an explicit browser implementation. Browser mode fails closed
for unknown commands.

Frontend code reaches durable capabilities through clients in `src/api`.
Components and hooks own presentation state; capability clients own command
names, request shapes, and response types.

## Compatibility boundaries

“Transform” is the public product concept. “Pipeline” remains only where needed
to read pre-1.0 database records, migration fixtures, and storage APIs. New GUI
and application contracts use Manual Transform naming.

Database changes use ordered `NamedMigration` entries. Each entry and its
schema marker commit in one transaction. Additive column helpers propagate
SQLite failures and are safe to run repeatedly.

## Testing boundaries

Application workflows that touch the system clipboard or paste destination
separate the commit decision from operating-system ports. Portable tests assert
that failed writes or target activation never consume Queue entries. Platform
availability is tested separately from shared mutation behavior.

`npm run test:architecture`, `npm run test:ipc`, and `npm run test:cli` protect
these boundaries. `npm run test:all` remains the completion gate.
