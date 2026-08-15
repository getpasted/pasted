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

## Theme-safe styling

- Use Tailwind utilities for structure: layout, spacing, sizing, responsive behavior, and typography.
- Use Pasted semantic theme classes and CSS custom properties for colors, surfaces, borders, dividers, focus rings, and visual emphasis.
- A structural border utility such as `border`, `border-t`, or `border-b` never supplies its own color. Pair it on the same element with a semantic theme class or a component class that explicitly sets a semantic `border-color`; never allow it to fall back to `currentColor`. Dialog panels with borders must use `theme-panel` unless a documented semantic panel class owns the complete surface treatment.
- Do not introduce Tailwind's default palette utilities directly into user-facing UI. If a reusable semantic class does not exist, add one to the theme primitives instead of special-casing a component.
- Mechanical utilities such as `divide-y` must be paired with the corresponding semantic class, such as `theme-divide`.
- Keep the CSS architecture audit budgets ratcheting downward; never increase a debt budget to accommodate new styling.

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

## Code Review Rules

### User data and reversibility

- Flag changes that can silently discard, orphan, or reinterpret clips, files, revisions, Bins, settings, or backup data. Require an explicit migration, bounded fallback, or user-confirmed destructive path.

### Clipboard privacy and safety

- Flag logging, notification, analytics, or error paths that expose clipboard contents, file paths, credentials, or transformation input. Clipboard reads and IPC payloads must remain bounded by the shared safety limits.

### Cross-platform behavior

- Platform-specific behavior must be gated and leave macOS, Windows, Linux X11, and constrained Wayland environments with a compiling, explicit success or graceful-failure path.
- Tests for platform-gated capabilities must not assume the host runner provides a particular operating-system implementation. Inject a portable test implementation or readiness fixture for shared scheduling, lifecycle, and persistence behavior, and test the real platform availability gate separately. Keep shared behavior tests running on every platform instead of hiding them behind platform-specific test configuration.
