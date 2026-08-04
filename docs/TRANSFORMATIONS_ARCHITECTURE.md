# Transformations Architecture

This document defines the product and engineering contracts for Pasted's
Operations, Pipelines, and Automations. The goal is to make a single Operation
useful by itself while allowing the same Operation to participate in reusable,
trackable workflows.

## Product vocabulary

### Operation

An Operation is one directly runnable text transformation with one input and
one output. Examples include Uppercase, Format JSON, Regex Replace, or a custom
CLI command.

- Operations never require a Bin or Pipeline.
- Any Operation can be run once against a selected clip or test input.
- Built-in Operations have stable string keys defined by the application.
- Custom Operations have stable generated IDs and an explicit executor kind.
- An Operation owns its default configuration. A Pipeline step may override
  only fields declared overrideable by that Operation.

### Pipeline

A Pipeline is a named, ordered composition of Operation references.

- Pipeline steps reference Operations; they do not copy an Operation's type
  and configuration into an unrelated snapshot.
- Reordering steps changes execution order without changing Operation identity.
- Deleting an Operation used by a Pipeline is blocked until the dependency is
  removed or replaced.
- A Pipeline can be run manually, by shortcut, as a Bin default, or by an
  Automation.
- Pipeline edits preserve Pipeline identity and may create a revision used by
  execution history.

### Execution

Every direct Operation run and Pipeline run uses the same execution service.
An execution records:

- operation or pipeline identity and revision;
- source clip ID when one exists;
- trigger (`manual`, `shortcut`, `bin`, or `automation`);
- start time, duration, success/failure, and a safe error summary;
- input and output hashes rather than duplicated clipboard contents.

The raw captured clip is never destroyed by an execution. A caller may place
the output on the live system clipboard or save a derived clip, but the source
remains recoverable.

### Automation

An Automation connects a trigger to conditions and a Pipeline.

Initial triggers:

- after a new clip is captured;
- when a clip is copied or pasted from Pasted.

Initial actions:

- run a Pipeline;
- route the source or derived result to a Bin;
- add tags or set Pin/Protect state;
- optionally replace the live system clipboard with the result.

Automatic clipboard replacement is opt-in. Writes carry an ignore fingerprint
so Pasted does not recapture its own output and create a feedback loop.

### Bin

A Bin organizes clips. It is not a transformation type.

- Manual and Smart membership remain independent from transformations.
- Any Bin may optionally declare a default Pipeline used when its clips are
  copied or pasted.
- An Automation may route matching clips into a Bin.
- The unfinished mutually exclusive `Filter` Bin type is removed rather than
  becoming a third incompatible membership model.

## Operation kinds

### Built-in

Deterministic in-process transformations registered by stable key in Rust.
Each definition declares its display name, category, configuration schema, and
capabilities. Built-ins are not duplicated as one-step Pipelines.

### Regex

A user-defined pattern, replacement, flags, and match mode. Validation occurs
when saving and again before execution. Invalid patterns produce explicit
errors and never silently return the input.

### Shell / CLI

An explicit privileged Operation that sends clip text to stdin and reads stdout.

- Never inferred from an unknown Operation key.
- User-created Operations only; imported Operations begin disabled/untrusted.
- Visible warning and trust state in the editor.
- Configurable executable and arguments without implicit shell interpolation
  by default; a separate advanced shell-script mode may use the system shell.
- Enforced timeout, output-size limit, and captured exit status/stderr.
- A minimal, documented environment and working-directory policy.

### HTTP / API

An explicit network Operation with method, URL, headers, request template,
response selector, timeout, and output limit.

- Imported network Operations begin disabled/untrusted.
- Secrets are Keychain references, never raw values in SQLite, logs, or exports.
- Redirect and local-network policies are explicit.
- Failures return structured errors usable by Pipeline failure policy.

### AI

AI is an adapter over a configured provider or local CLI, not an unstructured
special case. An AI Operation declares a prompt template, provider/model
reference, timeout, and output contract. Provider credentials live outside the
Operation and are referenced by ID. Local-only providers and CLIs remain valid
options.

## Pipeline failure policy

The default is `stop`: the first failed step stops the Pipeline and preserves
the original input. Later additions may allow an explicit per-step `skip`
policy. Silent failure and implicit shell fallback are not supported.

## UI model

- **Operations** lists built-in and custom Operations. Selecting one opens a
  live sandbox; `Run on Clip` performs a direct one-shot execution.
- **Pipelines** lists named recipes and shows their ordered Operation steps.
- **Automations** lists enabled/disabled trigger rules and their destinations.
- Clip Preview offers Quick Operations and saved Pipelines without requiring a
  Bin assignment.
- Bin editing offers an optional default Pipeline as a separate behavior, not
  a Bin type.

## Migration direction

Pasted is pre-release, so the SQLite model should be replaced cleanly rather
than preserving ambiguous seeded duplication.

- Built-in Operation definitions move from seeded rows to a canonical Rust
  registry.
- Custom Operations remain persisted with stable IDs and typed config JSON.
- Pipelines use stable IDs and an ordered `pipeline_steps` relation.
- Automations and conditions use explicit relations and enabled/trust state.
- Execution records provide dependency and activity traceability.
- The 35 duplicated one-step default Filters are removed; their underlying
  built-in Operations remain available as Quick Operations.

