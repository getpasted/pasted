# Pasted contributor guidance

## GUI and CLI parity

Pasted aims to make its meaningful clipboard-management capabilities available through both the graphical app and the CLI.

- When adding or changing a user-facing capability, consider its GUI and CLI surfaces together.
- Prefer shared Rust domain services and data contracts so the GUI and CLI execute the same behavior instead of maintaining parallel implementations.
- Keep terminology, defaults, validation, safety limits, ordering, and error semantics consistent across both surfaces.
- CLI commands should support stable, scriptable output. Prefer a consistent structured-output option such as `--json` when results contain multiple fields.
- GUI-only presentation behavior does not require a CLI equivalent. Features that depend on a running app, operating-system integration, or interactive state may use an explicit live-app command path or remain deferred.
- If a meaningful capability ships on only one surface, document the intentional exception or leave a clearly scoped follow-up before treating the work as complete.
- Add parity-focused tests where practical, especially around shared mutations and structured output contracts.

## Localization and language parity

- Treat every locale registered in `src/locales/manifest.json` as a fully supported product surface. Any change to product-authored user-facing copy must update the canonical English catalog and every shipped locale in the same change; do not rely on the English runtime fallback to ship an incomplete translation.
- Put GUI copy, native menu copy, validation, notifications, accessibility labels, alt text, user-facing errors, and formatted count/date/list text behind stable localization keys. Keep intentional technical literals, stable identifiers, user data, commands, flags, paths, and external proper names untranslated as documented in `docs/LOCALIZATION.md`.
- Reuse an existing semantic message when the meaning and grammatical role are genuinely identical. Use separate keys when context, tone, plurality, capitalization, or future translation may differ; do not assemble translated sentences from independently translated fragments.
- Preserve interpolation placeholder names and plural-message shapes across every catalog. Use locale-appropriate plural categories and shared internationalized formatters rather than concatenating English punctuation or word order around translated values.
- Machine or local-model translations are drafts. Run the locale-specific editorial review scripts after draft generation, review new strings in their interface context, and preserve established product terminology—especially for destructive actions, privacy, security, backup and restore, platform instructions, Clips, Bins, and named destinations.
- Keep React, Rust-native menus, shipped registry metadata, and the locale manifest synchronized. When adding or renaming a localization key, update every consumer and catalog atomically and remove stale keys or audit exceptions rather than leaving hidden compatibility debt.
- Before treating user-facing copy work as complete, run `npm run test:i18n` and `npm run test:copy`; run `npm run build` and relevant Rust tests when runtime, native-menu, registry, formatter, or locale-selection behavior changes. Never weaken completeness checks or increase hardcoded-copy debt to make a change pass.
- Use logical inline-start and inline-end layout utilities and CSS properties for direction-sensitive spacing, alignment, borders, corners, and positioning. Physical left/right styling requires a documented operating-system or geometry exception in the RTL audit.
- Keep interface direction separate from user content. Clipboard text and user-defined labels use automatic direction; file paths, commands, code, and stable identifiers remain isolated or explicitly LTR as appropriate.
- Mirror navigation, disclosure, and directional-flow icons in RTL. Do not mirror semantic symbols whose meaning is direction-independent.

## Theme-safe styling

- Use Tailwind utilities for structure: layout, spacing, sizing, responsive behavior, and typography.
- Use Pasted semantic theme classes and CSS custom properties for colors, surfaces, borders, dividers, focus rings, and visual emphasis.
- A structural border utility such as `border`, `border-t`, or `border-b` never supplies its own color. Pair it on the same element with a semantic theme class or a component class that explicitly sets a semantic `border-color`; never allow it to fall back to `currentColor`. Dialog panels with borders must use `theme-panel` unless a documented semantic panel class owns the complete surface treatment.
- Do not introduce Tailwind's default palette utilities directly into user-facing UI. If a reusable semantic class does not exist, add one to the theme primitives instead of special-casing a component.
- Mechanical utilities such as `divide-y` must be paired with the corresponding semantic class, such as `theme-divide`.
- Keep the CSS architecture audit budgets ratcheting downward; never increase a debt budget to accommodate new styling.

## Architecture and audit ratchets

- Treat every source-size, dependency-boundary, hardcoded-copy, styling-debt, and similar audit threshold as a pre-edit constraint, not a failure to discover after implementation.
- Before adding code to a file governed by a ratchet, inspect the applicable audit script and the file's current measurement. Account for the planned change and leave reasonable headroom.
- If a governed file is at or near its limit, extract a cohesive capability or shared helper before adding the feature. Do not first grow the file past the limit and defer the extraction until the audit fails.
- Never raise a ratchet, debt allowance, exception count, or line limit to accommodate a change. Do not game line-count limits by collapsing otherwise readable code; use a real module boundary.
- Run the narrowest applicable audit immediately after the structural edit, then run the broader required suite once the focused ratchets pass.
- Recheck ratchets after every formatter, generator, test-fixture expansion, or follow-up fix that can change a governed file's measurement. Before committing or pushing, run `npm run test:architecture` and confirm every changed governed file remains within its existing threshold; a previously passing result from before the latest edit does not count.
- Before opening or updating a pull request or enabling auto-merge, run the same aggregate validation commands used by the applicable CI scopes—currently `npm run test:frontend` for frontend/audit changes and `npm run test:native` for Rust/native changes. Do not substitute a hand-picked subset or describe the branch as ready based only on narrower checks.

## Git and GitHub workflow

- Start each new effort from a clean worktree. If unrelated changes are present, preserve them and stop or isolate them; never stash, reset, overwrite, commit, or delete work merely to make the tree clean without confirming ownership and intent.
- Before creating a branch in the primary checkout, run `git fetch --prune origin`, switch to `main`, update it with `git pull --ff-only origin main`, and create a fresh `codex/<topic>` branch. When another worktree owns `main`, create the new worktree or branch directly from current `origin/main` instead of reusing a stale feature branch.
- Keep one effort per branch and pull request. Never continue new work on a branch whose pull request merged or closed, especially after a squash merge. Before pushing, inspect the diff against `origin/main` and confirm that the branch contains only the intended commits and files.
- Never force-push `main` or a shared branch. Use `--force-with-lease` only on an owned feature branch after a deliberate rebase and after resolving the exact remote branch state.
- Keep pull request titles and summaries aligned with the actual diff, list the validation that actually ran, and use issue-closing language such as `Closes #123` only when the change fully resolves that issue. Do not create, edit, label, close, or reopen GitHub issues unless the user has requested that state change.
- Inspect failed required checks before rerunning them. Fix failures caused by the branch, report genuinely external or flaky failures, and never bypass, weaken, or dismiss required checks merely to merge.
- After GitHub confirms a pull request merged, treat that confirmation as authorization to clean up its local feature branch: ensure the worktree is clean, switch away from the branch, confirm no linked worktree uses it, then delete the local branch and prune stale remote-tracking refs. Because squash-merged branches are not ancestors of `main`, do not rely only on `git branch --merged`; verify the pull request state first.
- Never delete an open, unmerged, unverified, currently checked-out, worktree-bound, or unpublished branch. If any branch contains commits or changes whose remote or pull-request status is uncertain, preserve it and investigate rather than guessing.

## Rust build artifact maintenance

- During substantial Tauri or Rust work, occasionally check the size of `src-tauri/target`; do not clean it after every routine build.
- If `src-tauri/target` exceeds 20 GiB, or the host is under meaningful disk pressure, run `cargo clean` from `src-tauri` after active builds and tests have finished.
- Treat the target directory as regenerable build output. Never include source, configuration, release artifacts outside `target`, Cargo registries, or unrelated project caches in this cleanup.
- Report that the next Rust build will require a full recompile and include the actual disk-space change when cleanup was prompted by disk pressure.

## Time and timestamp handling

- Store persisted instants as canonical UTC RFC 3339 strings ending in `Z`. Treat legacy SQLite `YYYY-MM-DD HH:MM:SS` values as UTC during a bounded migration; do not silently reinterpret them as local time.
- Validate and normalize timestamps at every import boundary before they can affect ordering, retention, deduplication, backup data, Activity, or Insights. Reject malformed values transactionally.
- Use local time only when presenting an instant or grouping records by a user-facing calendar day. Calendar summaries such as Insights must use the machine's local day boundary, including daylight-saving transitions, while their underlying stored timestamps remain UTC.
- Do not sort timestamp strings unless the data contract guarantees canonical form. Shared domain queries and GUI helpers must compare instants consistently, and GUI and CLI summaries must expose the same day semantics.
- Add deterministic tests around UTC/local midnight, positive and negative offsets, mixed legacy timestamp formats, imports, retention ordering, and structured plus human-readable CLI output whenever time-sensitive behavior changes.

## Activity records

- Treat Activity as a versioned, portable audit contract. Keep JSON exports aligned with the OpenTelemetry event shape used by Pasted: timestamp, observed timestamp, event name, severity text, body, attributes, and archive-level resource metadata.
- Use stable event names and structured attributes for category, outcome, and occurrence-specific metadata. Use `info`, `warn`, or `error` severity; do not invent priority when severity or outcome carries the intended meaning.
- Activity must never include clipboard contents, transformation input, credentials, or sensitive file details. Prefer bounded identifiers, counts, and neutral summaries.
- Imported Activity records are inert history. Validate and bound imports, deduplicate them transactionally, apply retention afterward, and never replay actions represented by imported events.

## Backup and restore

- Reserve “Full Backup” and “Full Restore” for a complete snapshot of every Pasted-owned durable database table plus meaningful persisted interface and window state. New durable tables are included automatically; add round-trip and table-coverage tests when the storage model changes.
- Validate backup integrity, format version, embedded interface state, and forward migrations before replacing the live library. Create a complete pre-restore recovery backup first and restore it if activation fails.
- Describe external boundaries explicitly. Provider and operating-system credentials remain in their credential stores, and original external files referenced by file clips remain external; preserve their references and paths.
- Call the portable JSON workflow “History and Organization” in the GUI and `transfer` in the CLI. It merges clips and their supported organizing definitions and must not be presented as a complete backup or replacement restore. Treat JSON as the file format, not as another named product concept.

## In-app voice and grammar

- Keep interface copy terse and non-redundant. Lead with information that is not already communicated by the destination, heading, label, control, numbering, or visible layout. Remove helper text that merely restates what the interface already shows.
- End initiating control labels with an ellipsis (`…`) when the action opens a modal, confirmation, native picker, or other follow-up input. Do not add an ellipsis to the final action inside that flow or to an immediate command.
- Write interface copy from inside the product. Do not describe Pasted as though an outside narrator is explaining the app when the current screen or control already supplies that context.
- Prefer concise, neutral, control-led phrasing in settings descriptions, status text, Help, and other Tools surfaces. Avoid unnecessary first- or second-person pronouns such as “we,” “our,” “you,” and “your.”
- Do not replace a redundant “Pasted” with an equally unnecessary pronoun. Restructure the sentence instead: for example, use “Launch automatically after logging into macOS,” not “Automatically launch Pasted…” or “Launch automatically when you log in…”.
- Do not end structural UI labels with colons. Settings names, headings, toggle labels, button labels, and stacked form labels stand on their own. Reserve colons for compact inline key–value metadata where the label and value form one expression, such as “Words: 42.”
- Use title case for named product destinations and navigation items, such as “Insights,” “Privacy and Capture,” and “Deletion and Recovery.” Keep short conjunctions such as “and” lowercase within title case. Use sentence case for subordinate headings, field labels, controls, actions, and tooltips. Preserve acronyms, proper names, and explicitly named product concepts where needed.
- Write descriptive helper copy as complete sentences with terminal punctuation. Loading or in-progress text may end with an ellipsis.
- Prefer “and” to ampersands in interface copy. Keep ampersands only when reproducing a literal external name, such as an operating-system settings path.
- Use the product name when it carries necessary meaning: product identity and About content, literal operating-system permission labels, application or CLI paths, and clearly scoped destructive actions such as “Reset Pasted.” Deliberate ownership language may remain where the user-centered wording is the point.
- Keep exceptions in the UI copy audit narrow, exact, and documented. New exceptions should explain why neutral in-app wording would be less clear.

## Native file and folder dialogs

- Any Tauri command that calls `blocking_pick_file`, `blocking_pick_files`, `blocking_save_file`, or `blocking_pick_folder` must be declared `async`. Never open a blocking native dialog from a synchronous `#[tauri::command]`; on macOS this blocks the app command thread and can beachball the application while the dialog is open.
- Follow the existing asynchronous picker command pattern: await the command from the GUI, treat cancellation as a normal `None` result, and keep any file inspection, copying, database work, or other blocking post-selection work inside `spawn_blocking`.
- Disable or otherwise settle repeated picker actions while a request is pending, preserve the parent dialog's state when the native dialog is cancelled, and surface inaccessible selections through the normal in-app error treatment.
- Add or extend an automated source-contract audit when introducing a native picker so the command cannot silently regress to a synchronous declaration.

## In-app dialogs and confirmations

- Use Pasted's shared modal API for every product-authored alert, confirmation, prompt, warning, or message. Use `AppDialog` for custom flows and `ConfirmationDialog` for standard confirmations, including dialogs nested inside another modal.
- Never use browser primitives such as `window.alert`, `window.confirm`, or `window.prompt`, Tauri message dialogs, operating-system message boxes, or equivalent OEM prompts for product-authored UI.
- Keep the parent modal and its draft state intact while a nested dialog is open. Cancellation must close only the nested dialog and must not mutate data or discard the parent state.
- Destructive confirmations must state the concrete consequence, use the appropriate warning or danger treatment, and keep the final confirming action free of an ellipsis.
- Native file and folder pickers and operating-system permission prompts are narrow exceptions because the operating system owns those interactions; follow the dedicated native-dialog rules above.
- Add or extend an automated source-contract audit whenever a prompt path is introduced or changed so vanilla or OEM message UI cannot silently return.

## Code Review Rules

### User data and reversibility

- Flag changes that can silently discard, orphan, or reinterpret clips, files, revisions, Bins, settings, or backup data. Require an explicit migration, bounded fallback, or user-confirmed destructive path.

### Clipboard privacy and safety

- Flag logging, notification, analytics, or error paths that expose clipboard contents, file paths, credentials, or transformation input. Clipboard reads and IPC payloads must remain bounded by the shared safety limits.

### Cross-platform behavior

- Platform-specific behavior must be gated and leave macOS, Windows, Linux X11, and constrained Wayland environments with a compiling, explicit success or graceful-failure path.
- Tests for platform-gated capabilities must not assume the host runner provides a particular operating-system implementation. Inject a portable test implementation or readiness fixture for shared scheduling, lifecycle, and persistence behavior, and test the real platform availability gate separately. Keep shared behavior tests running on every platform instead of hiding them behind platform-specific test configuration.
