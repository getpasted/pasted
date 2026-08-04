# UI Styling Audit

This checklist records the styling debt found during the multi-scheme and glass-material work. It separates visual-system debt from the related Filters/Operations product refactor so fixes can be made in a deliberate order.

## Audit snapshot

- `src/App.css` is now a nine-line ordered manifest over eight responsibility-based modules in `src/styles/`; the split preserves the original cascade exactly.
- Literal colors in `src/App.css` dropped from 532 to 349.
- `!important` baseline: 441; current enforced budget: 95.
- Cool/Warm compatibility-selector baseline: 269; current enforced budget: 1 (the shared light token declaration).
- Utility-coupled selectors and hard-coded JSX surfaces are both budgeted at 0.
- Only 18 of 30 React component files use a semantic theme primitive.
- The original undefined `--bg-preview-header` reference is fixed; the token audit now guards against regressions.
- The automated contrast audit checks selected color pairs, but does not render components or detect missing tokens and cascade failures.

## P0 — correctness and cascade hazards

- [x] Replace the undefined `--bg-preview-header` references with an intentional semantic material role.
- [x] Remove Clip Preview selectors based on child position or incidental utility classes, including `.col-preview > div:first-child`, `div.rounded-xl`, and `div.p-3`.
- [x] Make semantic component classes authoritative so Vampire and Flux do not fall through to dark hard-coded surfaces.
- [x] Remove the Cool/Warm repair layer. New schemes are implemented through tokens rather than scheme-specific component overrides.
- [x] Replace generic element repair selectors (`div.h-screen.overflow-y-auto`, global headings, generic inputs, and color utility selectors) with component roles.

## P1 — hard-coded surface islands

- [x] Unify Filter/Operation sandboxes, headers, and editor surfaces under shared transform-workspace primitives. Deeper product behavior remains a dedicated project.
- [x] Migrate `FilterEditorModal` and `OperationEditorModal` from parallel hard-coded palettes to one shared editor shell.
- [x] Migrate `BinModal` and its tabs, fields, rule builder, footer, and buttons to dialog/input/tab primitives.
- [x] Finish `ClipPreviewContent` theme coverage for color, image, and OCR modes.
- [x] Migrate the Quick HUD from a fixed dark palette to HUD-specific material tokens. Product redesign remains a dedicated project.
- [x] Remove remaining hard-coded sidebar surfaces and icon blues; use navigation and accent roles.
- [x] Consolidate note input, cancel, save, action, row, and viewer surfaces shared by `ClipPreview` and `ClipNoteRow`.
- [x] Convert Smart Actions from a fixed cyan island to scheme-aware contextual recommendation roles.
- [x] Introduce a compact semantic header for standalone Tools pages and migrate Analytics & Insights.
- [x] Move Settings navigation into the compact sticky header and apply the shared icon treatment to standalone Tools pages.
- [x] Convert the clear-history, delete-bin, and clip-note dialogs to a single semantic dialog shell without Cool/Warm repair selectors.

## P1 — shared system gaps

- [x] Add semantic tokens for overlays, menus, code surfaces, destructive/warning/success states, focus rings, and secondary accents.
- [x] Define a single named layer scale for sticky, drag, popover, menu, modal, and critical UI.
- [x] Replace broad `transition-all` usage with property-specific transitions; add `prefers-reduced-motion` behavior.
- [x] Add `color-scheme` metadata per scheme so native controls and browser/Tauri chrome agree with the active palette.
- [x] Replace non-standard `overflow-y: overlay`, remove permanent `will-change`, and keep backdrop blur on structural glass layers instead of stacking it again on nested headers/toolbars.
- [x] Scope selection rules to application shells and explicit overlays while preserving the nested cursor inheritance required to prevent WebKit pointer jitter.

## P2 — cleanup and maintainability

- [x] Split `App.css` by responsibility: foundation, layout/chrome, reusable theme primitives, clip/sidebar UI, accessibility, tools/editors, preview/notes, and finishing utilities.
- [x] Remove or verify unused rules such as `tools-section-tab`, `col-list-scroll-area`, `clip-content-body`, `clip-text-render`, and legacy menu/navigation helpers.
- [x] Review suspected duplicate structural blocks; remaining repeated selectors intentionally separate surface tint, blur ownership, and accessibility fallbacks.
- [x] Replace broad radius selectors such as `.settings-page > div > div[class~="rounded-2xl"]` with explicit panel roles.
- [x] Move intentional fixed-color content (code samples, image checkerboards, contrast samples) into named opt-out primitives so it is distinguishable from accidental theme debt.

## Testing gaps

- [x] Add a token audit that scans every CSS module, fails on undefined custom properties or broken module imports, and prevents the `!important` budget from increasing.
- [ ] Add rendered smoke tests for every scheme across the main window, Tools pages, dialogs, menus, HUD, and editor modals.
- [x] Add a hard-coded-surface budget so new arbitrary dark backgrounds cannot silently enter theme-aware components.
- [ ] Add interaction screenshots for hover, selected, drag, disabled, focus, modal, and context-menu states.

## Filters & Operations product refactor

These are functional issues exposed while auditing the styling, not CSS-only defects.

- [ ] Replace pipeline category inference. It currently compares category label words with operation type identifiers, so many category pills cannot produce correct results.
- [ ] Create one canonical operation-category model. `OperationEditorModal` uses “Structure & Tags” while seeded operations use “Structure & Formatting”.
- [ ] Share sandbox state, result/error presentation, cards, category navigation, and CRUD action patterns between pipelines and operations.
- [x] Remove the imperative `openCreateRef` bridge between `FilterManager` and `OperationsManager`; creation now lives with each collection toolbar instead of reaching into child state from the page header.
- [ ] Keep operation counts synchronized after CRUD instead of refreshing primarily when the active subtab changes.
- [ ] Define the intended relationship clearly: Operations are reusable atomic steps; Pipelines are ordered compositions of operation IDs/configurations.
- [ ] Add tests for category membership, pipeline composition, operation deletion dependencies, ordering, duplication, import/export, and live sandbox errors.

## Recommended attack order

1. Token audit and missing semantic roles.
2. Shared dialog/menu/editor shells.
3. Filters & Operations domain/controller refactor.
4. Clip preview and HUD hard-coded islands.
5. Delete the compatibility repair layer and dead CSS.
6. Split the stylesheet once responsibilities are stable.
