# Analysis architecture

Pasted runs derived-content work through one bounded Analyzer and exposes participant-specific results through parallel execution modules.

## Stable layers

- `analysis_contract.rs` owns representation names, the four ordered passes, participant contracts and run summaries, target kinds, failures, and clip-application state.
- `content_analysis.rs` owns scheduling and the in-memory Analysis context. Raw `AnalysisReport` values stay crate-internal.
- `extraction_execution.rs` and `detection_execution.rs` translate raw reports into stable Extractor and Detector results. GUI, CLI, and background services consume these results instead of interpreting scheduler state.
- Persistence accepts typed execution results. Preview and apply share the same serialized shape, and applied clip IDs come from shared application state rather than surface-specific JSON assembly.

## Adding an Inspector or Enricher

1. Add a typed `RepresentationKind` and bounded `AnalysisContext` field only when the participant has a concrete new input or output. Reuse an existing representation when its semantics match exactly.
2. Build an `AnalysisParticipant` with a stable reference, one `AnalysisPass`, deterministic priority, and explicit `requires` and `provides` lists. The scheduler runs it at most once and fails closed when declared output is missing.
3. Add a participant-specific execution module, such as `enrichment_execution.rs`. Translate `AnalysisReport` through `resolve_participant`, use `AnalysisTargetKind`, and expose privacy-safe `ParticipantRun` summaries.
4. Use `ClipApplication` for preview and apply identity. Add participant-specific persistence flags beside it only when callers need to distinguish derived records that were actually updated.
5. Route GUI, CLI, and background work through the same execution and application functions. Keep structured JSON stable and never synthesize application state in a surface adapter.
6. Add portable engine or participant fixtures so scheduling, failure, stale-input, and persistence behavior runs on every platform. Test real platform availability separately.

Participant results and Activity records must not include input content, credentials, sensitive file paths, or unbounded provider output. User-visible output may contain an explicitly requested derived result, but participant summaries and failures remain neutral and bounded.
