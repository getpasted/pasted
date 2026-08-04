# Transformations Competitive Roadmap

This comparison grounds Pasted's transformation roadmap in proven workflows
while identifying a coherent product space that existing clipboard managers,
text expanders, and general automation tools do not cover together.

## What established tools teach us

| Product | Existing strength | Lesson for Pasted |
| --- | --- | --- |
| Pastebot | Named filters chain one or more filter types, provide live before/after preview, run from the main or quick-paste UI, and support filter-specific and last-filter shortcuts. Filters and pasteboards remain separate. | Preserve the fast filter-on-paste workflow. Pipelines should be named ordered recipes, while Bins remain optional organization. |
| Alfred | Clipboard history values can be addressed by offset and fed into hotkey-driven workflows, variables, files, and other outputs. | Treat clipboard text as one possible execution input and make Operations/Pipelines composable from headless workflows. |
| Raycast | Dynamic placeholders expose clipboard, selected text, arguments, snippets, dates, browser context, and chained modifiers. Extensions and AI commands add new actions. | Add typed execution inputs and interactive parameters rather than assuming every Operation consumes only the latest clipboard. Keep AI behind the same adapter contract as other Operations. |
| TextExpander | Snippets support fill-in text/areas/options, optional sections, nesting, clipboard insertion, and JavaScript, AppleScript, or shell-backed expansion. | Pipeline Operations should be able to declare typed parameters. A run may prompt for missing values before execution, and reusable definitions should compose without copying their implementation. |
| Keyboard Maestro | Filters work on clipboards or variables and can be mixed with scripts and broader macro actions. | Keep the transformation engine independent from storage. Inputs and outputs should be explicit values that callers may send to clipboard, clip history, a Bin, a file, or another step. |
| CopyQ | Clipboard-change commands, a JavaScript-like scripting API, and a broad CLI allow automation and external control. | Make the Pasted CLI a first-class client of the same safe execution service. Offer clipboard-change Automations without making unrestricted scripting the implicit default. |

## Primary sources

- [Pastebot filters](https://tapbots.com/pastebot/help/05_filters/)
- [Pastebot quick paste](https://tapbots.com/pastebot/help/06_quick_paste_menu/)
- [Alfred clipboard workflow placeholders](https://www.alfredapp.com/help/features/clipboard/accessing-clipboard-history/)
- [Raycast dynamic placeholders and modifiers](https://manual.raycast.com/dynamic-placeholders)
- [Raycast extensions](https://manual.raycast.com/extensions)
- [TextExpander fill-in syntax](https://textexpander.com/learn/using/snippets/snippet-fill-ins/advanced-fill-in-syntax)
- [TextExpander scripting](https://textexpander.com/learn/using/snippets/scripting-textexpander)
- [Keyboard Maestro clipboard filters](https://www.keyboardmaestro.com/documentation-keyboardmaestro/7/filters.html)
- [CopyQ command line](https://copyq.readthedocs.io/en/latest/command-line.html)
- [CopyQ scripting API](https://copyq.readthedocs.io/en/stable/scripting-api.html)

## Built-in transformation coverage

Pastebot's step menu exposes Find and Replace, Extract, Change Case, Smart
Punctuation, Convert to Plain Text, Line Operations, Sort, Quote Text, Wrap
Line, Base64, Hex, URL, HTML Entity, Shell Script, External Shell Script, and
Translate. Its bundled Create List recipe also demonstrates the most reusable
part of its model: Quote Text accepts before/after content and can apply it to
each line, so common formats can be composed instead of each becoming a new
executor.

Pasted now covers the local, deterministic portion of that set through stable
built-ins. Quote Text supports configurable before/after content and per-line
application; typed extractors cover URLs, email addresses, phone numbers, IP
addresses, and numbers; and HTML paragraph/list builders provide convenient
one-step forms of common recipes. The registry also includes Alfred's compact
utility set (reverse text, strip diacritics, and strip non-alphanumeric) and
Raycast's JSON-string and percent-encoding modifiers.

Two Pastebot entries intentionally remain privileged adapters rather than
ordinary built-ins:

- External Shell Script belongs to the trusted CLI/extension executor, with an
  executable plus argument list preferred over an interpolated shell command.
- Translate belongs to an HTTP, local model, or provider-backed Operation with
  an explicit connection and network permission.

Wrap Line remains a local follow-up until its exact contract is chosen (visual
column width, word-boundary width, or prefix/suffix wrapping). Prefix/suffix
wrapping itself is already covered by Quote Text.

Pastebot also demonstrates an `Apply to Each Line` switch at the step level.
Pasted supports that behavior for Quote Text today. Generalizing it to every
Operation should add an explicit execution-scope field to `pipeline_steps`
rather than smuggling a wrapper through operation-specific configuration.

## Pasted's opportunity

Pasted should combine the strongest parts of these products without becoming a
general-purpose visual programming environment:

1. Pastebot's immediate preview-and-paste ergonomics.
2. Raycast and TextExpander's contextual, parameterized inputs.
3. Alfred and Keyboard Maestro's composability.
4. CopyQ's automation and CLI reach.
5. Pasted-specific raw-original preservation, Bins, revisions, and execution
   history.

The differentiator is one inspectable engine shared by GUI actions, keyboard
shortcuts, Automations, and CLI calls. Every privileged action is explicit and
every automatic execution can be traced back to its trigger and definition.

## Capability roadmap

### Foundation: predictable direct transformations

- Canonical built-in Operation registry with stable keys and categories.
- Direct one-shot Operation execution on arbitrary text or a selected clip.
- Named Pipelines referencing ordered Operations.
- Live input/output/error preview.
- Stable identity, dependency-safe edits/deletes, and revision-aware execution.
- Explicit failure behavior; no silent pass-through or implicit shell fallback.

### Context and parameters

- Typed inputs: literal text, stdin, current clipboard, historical clip ID,
  selected text, and previous Pipeline output.
- Typed parameters: text, multiline text, secret, number, boolean, and options.
- Default values and optional run-time prompts.
- Template values such as current date/time, source application, clip metadata,
  and prior outputs.
- Named outputs so later steps can reference more than one value without
  relying on hidden global clipboard state.

### Automation

- Capture and use/paste triggers.
- Conditions over source app, content type, text, Bin, tags, and metadata.
- Actions for Pipeline execution, Bin routing, tags, Pin/Protect, clipboard
  replacement, and derived-clip creation.
- Dry-run mode showing which Automations would match and what they would do.
- Priority and explicit conflict behavior when multiple Automations match.
- Loop fingerprints, timeouts, output caps, and raw-original preservation.

### Extensibility

- Regex Operations with validated structured config.
- CLI Operations with executable/argument mode, plus separately trusted
  advanced shell-script mode.
- HTTP Operations with Keychain-backed secrets and structured response paths.
- AI Operations through configured providers or local CLIs, with provider and
  model references separated from prompt definitions.
- Imported privileged Operations and Automations disabled until reviewed.

## CLI contract

The existing `pasted-cli` should become a stable headless surface over the same
domain and execution service, not a second implementation that edits SQLite
directly.

Planned command shape:

```text
pasted operation list [--json]
pasted operation show <operation-ref> [--json]
pasted operation run <operation-ref> [--text TEXT | --clip ID | --stdin]

pasted pipeline list [--json]
pasted pipeline show <pipeline-ref> [--json]
pasted pipeline validate <pipeline-ref>
pasted pipeline run <pipeline-ref> [--text TEXT | --clip ID | --stdin]

pasted automation list [--json]
pasted automation test <automation-ref> [--text TEXT | --clip ID | --stdin]
pasted automation enable|disable <automation-ref>
```

CLI rules:

- stdin input and stdout output compose naturally with Unix pipelines;
- diagnostics go to stderr and failures use documented non-zero exit codes;
- `--json` returns stable machine-readable envelopes;
- execution defaults to no clipboard mutation unless an explicit flag requests
  it;
- privileged Operations honor the same trust and secret policies as the GUI;
- the CLI asks the running app/service to execute when necessary rather than
  racing it through direct SQLite writes.

Example:

```sh
git diff | pasted pipeline run review-summary --stdin
pasted operation run format-json --clip 142
pasted pipeline run clean-url --stdin --copy < urls.txt
```

## Deliberate non-goals

- Pasted does not initially need a free-form node graph.
- Arbitrary unknown identifiers never imply executable code.
- Automatic clipboard replacement is never enabled merely by importing a rule.
- AI is not a parallel product surface; it is one privileged Operation adapter.
- Bins do not become a second Pipeline or Automation schema.
