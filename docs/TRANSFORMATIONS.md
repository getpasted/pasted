# Transformations

Transformations is Pasted's unified system for changing text as it moves into, through, or out of the app.

The product model is intentionally small:

```text
Input -> Pipeline -> Destination
             ^
          Trigger
```

## Vocabulary

- **Operation**: one reusable transformation, such as Uppercase, Regex Replace, an AI prompt, an HTTP request, or a local command.
- **Pipeline**: an ordered composition of one or more Operations.
- **Trigger**: the moment a Pipeline runs.
- **Input**: the content supplied to the Pipeline.
- **Destination**: what receives the output.

The canonical Rust and SQLite APIs use Pipeline terminology. Any remaining
“Filter” names are temporary frontend adapter names only; they are not product
or backend domain terminology.

## Inputs

The same Pipeline must be able to accept:

- A selected Pasted clip.
- The current system clipboard.
- Selected text from the frontmost application, when macOS permissions allow it.
- Newly captured clipboard content.
- Explicit text passed by the CLI, URL scheme, plugin, or API.
- A previous Pipeline step's output.

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
- Support “Paste with last Pipeline” and a direct shortcut for any favorite Pipeline.

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
6. Every execution records Pipeline, Operation versions, trigger, destination, duration, and outcome.
7. A failed automatic transformation never destroys or hides the captured source.

## Shortcut and last-used contract

- A Pipeline becomes the last-used Pipeline only after a successful execution.
- Failed Operation or Pipeline execution never replaces that reference.
- Deleting the referenced Pipeline makes the next last-Pipeline action fail
  explicitly and clears the stale reference.
- Per-Pipeline shortcuts transform the current clipboard and paste the result.
- `copyLastPipelineHotkey` transforms the current clipboard and leaves the
  result on the clipboard.
- `pasteLastPipelineHotkey` transforms the current clipboard, updates it, and
  pastes into the frontmost application.
- `openTransformationsHotkey` opens the Transformations workspace.

## Competitor lessons

- Pastebot makes filtering a late-bound choice before copy or paste, with shortcuts for the last or a specific filter.
- Alfred models work as explicit triggers, inputs, actions, utilities, and outputs, including clipboard-history placeholders.
- Raycast treats clipboard entries and selected text as inputs, supports alternate paste formats, dynamic placeholders, scripts, and AI commands.
- TextExpander runs at expansion time and combines stored text with clipboard variables, fill-ins, cursor placement, nesting, and scripts.

Pasted should combine these capabilities around clipboard-native language instead of recreating a general visual automation canvas.

## Delivery sequence

1. Complete manual preview, copy-result, and paste-result actions on the shared execution service.
2. Add per-Pipeline hotkeys and “last used Pipeline” behavior.
3. Add CLI stdin/stdout execution using stable Pipeline and Operation identifiers.
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
