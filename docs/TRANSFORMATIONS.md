# Transformations

Transformations is Pasted's unified system for changing text as it moves into, through, or out of the app.

The primary product model is intentionally small:

```text
Input -> Transform -> Destination
                  ^
               Trigger
```

## Vocabulary

- **Transform**: a reusable workflow. It may contain locally replayable Operations, AI-assisted steps, or both.
- **Operation**: an experimental deterministic building block, such as Uppercase or Regex Replace.
- **Manual Transform**: a Transform assembled directly from deterministic Operations.
- **Trigger**: the moment a Transform runs.
- **Input**: the content supplied to the execution.
- **Destination**: what receives the output.

Transforms are the supported workflow. Operations remain the lower-level,
experimental building blocks. Planned and manually built Transforms share one
table, `transform:*` identity, execution service, backup contract, and lifecycle.
Any remaining “Pipeline” or “Filter” names are pre-release input or editor
adapters rather than product or persistence terminology.

The pre-1.0 storage decision and migration boundary are documented in
[Transform storage decision for 1.0](./TRANSFORM_STORAGE_DECISION.md).

## CLI lifecycle contract

`pasted transform` is the scriptable surface for both authoring forms:

- `list` and `get` inspect canonical Transform definitions.
- `create` accepts exactly one of `--plan-json` or `--steps-json`.
- `update` preserves the existing authoring form and stable reference.
- `duplicate` preserves the authoring form but creates a new stable reference.
- `delete` accepts a `transform:*` stable reference.
- `run --apply --clip ID` records a clip revision and durable provenance for
  saved and manually built Transforms.

Structured lifecycle output is available with `--json`.

The standalone CLI persists hotkey changes through the shared database. If
the GUI is already running, its native global-hotkey registration is refreshed
on the next app launch; other lifecycle changes are visible after the normal
library refresh.

## Inputs

The same Transform must be able to accept:

- A selected Pasted clip.
- The current system clipboard.
- Selected text from the frontmost application, when macOS permissions allow it.
- Newly captured clipboard content.
- Explicit text passed by the CLI, URL scheme, plugin, or API.
- A previous Transform step's output.

Input is immutable for an execution. Pasted should retain enough provenance to explain where it came from without silently replacing the original clip.

## Triggers

### Manual

- Run from the clip viewer or context menu.
- Run from the Transformations playground.
- Run from the HUD or command palette.

### Copy

- Transform a clip, place the result on the system clipboard, and optionally create a derived clip.
- Preserve the source clip unless the user explicitly chooses replacement.

### Paste

- Transform and paste into the frontmost application without first mutating stored history.
- Support “Paste with last Transform” and a direct hotkey for any favorite manually built Transform.

### Capture

- Match a captured clip by app, content type, text rule, or Bin.
- Run automatically, then keep the original, keep the result, or keep both according to the Automation.
- Route the result to a Bin as a separate destination decision.

### External

- Global hotkey.
- CLI/stdin.
- URL scheme or local API.
- Plugin or integration event.

## Destinations

Every execution chooses one explicit destination:

- Preview only.
- Copy result.
- Paste result.
- Create a derived clip.
- Replace a clip, with revision history.
- Assign or route to a Bin.
- Return to the caller, such as stdout or a plugin.

Destinations may be composed only when the UI states each side effect clearly. “Copy and paste” is a convenience action, not an implicit mutation of history.

## Interaction rules

1. The playground always runs without side effects.
2. Manual actions default to non-destructive output.
3. Replacing a stored clip always records a revision.
4. Automatic capture rules must expose whether they keep the source, result, or both.
5. Remote, AI, HTTP, and command Operations display their trust boundary before first use.
6. Every execution records its Transform or Operation version, trigger, destination, duration, and outcome.
7. A failed automatic transformation never destroys or hides the captured source.

## Hotkey and last-used contract

- A manually built Transform becomes the last-used Transform only after a successful execution.
- Failed Operation or Transform execution never replaces that reference.
- Deleting the referenced Transform makes the next last-Transform action fail
  explicitly and clears the stale reference.
- Per-Transform hotkeys transform the current clipboard and paste the result.
- `copyLastPipelineHotkey` transforms the current clipboard and leaves the
  result on the clipboard.
- `pasteLastPipelineHotkey` transforms the current clipboard, updates it, and
  pastes into the frontmost application.
- `openTransformationsHotkey` opens the Transformations workspace.

## Competitor lessons

- Pastebot makes filtering a late-bound choice before copy or paste, with hotkeys for the last or a specific filter.
- Alfred models work as explicit triggers, inputs, actions, utilities, and outputs, including clipboard-history placeholders.
- Raycast treats clipboard entries and selected text as inputs, supports alternate paste formats, dynamic placeholders, scripts, and AI commands.
- TextExpander runs at expansion time and combines stored text with clipboard variables, fill-ins, cursor placement, nesting, and scripts.

Pasted should combine these capabilities around clipboard-native language instead of recreating a general visual automation canvas.

## Delivery sequence

1. Complete manual preview, copy-result, and paste-result actions on the shared execution service.
2. Add per-Transform hotkeys and “last used Transform” behavior.
3. Remove remaining pre-release Pipeline terminology from internal editor and command adapters.
4. Add capture-time Automations with source-preservation guarantees.
5. Add selected-text input and lightweight fill-ins.
6. Enable capability-scoped command, HTTP, and AI Operations.

## References

- [Pastebot Filters](https://tapbots.com/pastebot/help/05_filters/)
- [Alfred Workflows](https://www.alfredapp.com/help/workflows/)
- [Alfred clipboard history in Workflows](https://www.alfredapp.com/help/features/clipboard/accessing-clipboard-history/)
- [Raycast Clipboard History](https://manual.raycast.com/clipboard-history)
- [Raycast Dynamic Placeholders](https://manual.raycast.com/dynamic-placeholders)
- [Raycast AI Commands](https://manual.raycast.com/ai/ai-commands)
- [TextExpander advanced Snippet elements](https://textexpander.com/learn/using/snippets/advanced-snippet-elements/more-advanced-functions)
