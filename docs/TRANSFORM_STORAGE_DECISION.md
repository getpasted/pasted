# Transform storage decision for 1.0

Status: accepted and implemented before 1.0

## Decision

Pasted stores every reusable Transform in `saved_transforms`. The record’s
`authoring_kind` describes how it is edited:

- `intent` is a planned Transform and may contain deterministic, semantic, or
  mixed steps.
- `manual` is assembled directly from deterministic Operations and may have a
  hotkey.

Both forms use `transform:*` stable references, the same execution ledger,
provenance, Bin binding, automation foreign key, backup collection, library
registry, lifecycle facade, GUI, and `pasted transform` CLI.

## Pre-1.0 migration

Startup transactionally converts legacy `pipelines` and `pipeline_steps` rows
into manual `saved_transforms` plans, then rewrites every persisted dependency:

- Bin default Transforms;
- clip transformation provenance;
- transformation execution history;
- last-successful Transform settings; and
- Automations and their conditions.

If a legacy Pipeline ID collides with an existing Transform ID, the migrated
record receives a new ID and every dependent reference is rewritten through the
same temporary mapping. Only after all records and references are migrated are
the legacy tables removed. Any failure rolls the entire migration back.

Backup schema 10 exports all workflows in `saved_transforms`. Import still accepts the
legacy `pipelines` collection so a pre-migration backup remains recoverable.

## Compatibility boundary

`pipeline:*` references are accepted only as an input alias during the
pre-release transition and normalize immediately to `transform:*`. They are not
written by the canonical schema or execution paths. Internal names related to
the manual step editor can be renamed independently because they no longer
represent a distinct persistence model.
