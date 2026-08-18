# Analysis architecture

Pasted runs derived-content work through one bounded Analyzer and exposes participant-specific results through parallel execution modules.

## Version 1 contract

The serialized contracts under `contracts/analysis/v1`, their outcome and failure semantics, the four ordered passes, and the privacy boundary described here define Pasted 1.0. Before the stable release, contract changes must update GUI, CLI, fixtures, tests, and documentation atomically. After release, incompatible field, outcome, pass, mutation, or privacy changes require a new contract version; additive changes still require explicit cross-surface review.

Structure Inspection, Media Metadata, and Smart Actions are immutable built-in participants in 1.0. Structure Inspection is always available because Clip Preview depends on its bounded structural facts. The locally powered Media Metadata Inspector and Smart Actions run only under the interactive policy. Smart Actions also follows the user-facing Transformations feature state. These participants do not have redundant Settings switches. Their definitions and contracts remain inspectable through `pasted inspector`, `pasted suggestion`, and `pasted registry`.

## Stable layers

- `analysis_contract.rs` owns the shared result metadata, contract version, representation names, the four ordered passes, execution policies, participant contracts and run summaries, target kinds, failures, and clip-application state.
- `content_analysis.rs` owns typed Analysis requests, scheduling, and the in-memory Analysis context. Raw `AnalysisReport` values stay crate-internal.
- `analysis_execution.rs` translates a whole run into one content-free Analyzer snapshot. `inspection_execution.rs`, `extraction_execution.rs`, `classification_execution.rs`, and `suggestion_execution.rs` translate raw reports into stable participant-specific results. GUI, CLI, and background services consume these results instead of interpreting scheduler state.
- Persistence accepts typed execution results. Every public result carries shared version, policy, and final-pass metadata; preview and apply share the same serialized shape, and applied clip IDs come from shared application state rather than surface-specific JSON assembly.

Capture, background, and rescan policies stop after Classify. Interactive work may continue through Suggest, so optional expensive participants never run implicitly during capture. Callers choose a policy and available participants; they do not invoke scheduler passes directly.

Text capture submits one Capture-policy Analyzer request and reuses its classification matches and structural metadata during insertion. Persistence falls back to focused structural inspection only when the precomputed snapshot is unavailable or cannot be safely activated. Focused rescans and Extractor application remain participant-specific because their mutation contracts intentionally target only classification or extracted text.

The shipped `inspector:structure-v1` participant produces `structural_metadata` during the inspect pass. Stable, content-free facts are persisted against the clip content hash and a structural input fingerprint. Filesystem availability and size are live observations kept outside durable Analysis results. Full Backup includes the durable result table automatically; portable History and Organization transfer omits recomputable derived results.

The shipped `inspector:media-metadata-v1` participant consumes file references only during interactive inspection and produces bounded aggregate `media_metadata`. It prefers the `ffprobe-cli-v1` engine and falls back to `mediainfo-cli-v1`; both invoke a discovered executable directly with fixed arguments, a five-second total timeout, an eight-file ceiling, private output staging, and a bounded JSON parser. Participant identity and output semantics remain independent of the selected engine. Media metadata and external paths remain live and are not persisted.

The shipped `suggestion:smart-actions-v1` participant consumes analyzable text, classification, and structural metadata only for interactive requests. It returns bounded signals and stable saved-Transform references; it never returns input content, executes a Transform, or writes a clip. Clip Preview and `pasted suggestion run` use the same execution result.

Clip Preview and `pasted analyzer run` consume the whole-Analyzer snapshot. The snapshot reports clip kind, structural metadata, occurrence-level classification matches, content-free suggestions, participant outcomes, and only a boolean indicating whether searchable text became available. It never returns original text, extracted text, image bytes, file paths, or matched substrings. File references cannot be reinterpreted as analyzable text; only a successful Extractor may produce the searchable-text representation consumed by later passes. Automatic Clip Preview requests do not enable extraction; explicit GUI extraction or `pasted analyzer run --clip ID --extract` opts into potentially expensive OCR or transcription.

## Type applicability

Capture establishes three distinct axes. Every clip has one structural Clip Type—Text, Image, or Files. A Files clip may expose several File Formats because it can reference several files. Analysis may add several semantic Content Types and occurrences without rewriting either structural identity or original content. Version 1 persistence binds those matches to the analyzed content hash, representation, Classifier, and character offsets.

The shared registry models applicability as typed edges. Every Analyzer item exposes a `participantContract` containing accepted and produced representations. Its `typeRelations` identify direct edges: `accepts` currently uses the legacy `image` and `file` registry IDs to describe Clip Type applicability, while `classifies_as` connects each Classifier to the semantic Content Type it produces. GUI and CLI consumers can visualize participant → representation → classification relationships without interpreting display names or engine IDs. This legacy wire naming is frozen for the version 1 contract; user-facing surfaces must keep Clip Type and Content Type distinct.

Extractor definitions use one `recipe-v1` runtime contract across shipped and custom entries. A recipe records accepted representations, direct local command steps, absolute or discovered executables, argv tokens, step artifacts, output capture, resources, time limits, scheduling state, and revision. The runner never invokes a shell or AI. It clears unneeded environment variables, uses a private workspace, validates placeholders and resources, bounds execution and output, and removes staging afterward. Runtime status resolves every command and resource into visible readiness.

AI is an optional recipe authoring surface. It receives the user description and strict schema, then returns the same recipe editable under Advanced settings; it cannot inspect files, run tools, install dependencies, or participate in extraction. Original prompts, provider/model identity, responses, structured proposals, and canonical UTC timestamps are retained locally in authoring sessions linked to immutable recipe revisions. These tables are included in Full Backup and excluded from Activity. The CLI accepts recipe JSON, offers the same AI drafting path, and exposes authoring history. Shipped recipes carry versioned baselines so upgrades can update untouched defaults while Reset restores the current release definition.

## Adding an Inspector or Suggestion participant

1. Add a typed `RepresentationKind` and bounded `AnalysisContext` field only when the participant has a concrete new input or output. Reuse an existing representation when its semantics match exactly.
2. Build an `AnalysisParticipant` with a stable reference, one `AnalysisPass`, deterministic priority, and explicit `requires` and `provides` lists. The scheduler runs it at most once and fails closed when declared output is missing.
3. Add a participant-specific execution module, such as `suggestion_execution.rs`. Translate `AnalysisReport` through `resolve_participant`, use `AnalysisTargetKind`, and expose privacy-safe `ParticipantRun` summaries.
4. Use `ClipApplication` for preview and apply identity. Add participant-specific persistence flags beside it only when callers need to distinguish derived records that were actually updated.
5. Route GUI, CLI, and background work through the same execution and application functions. Keep structured JSON stable and never synthesize application state in a surface adapter.
6. Treat the privacy-safe examples in `contracts/analysis/v1` as the public serialized contract for the Analyzer and its participant surfaces. Rust serialization tests and CLI integration tests consume them directly, and the frontend parity audit checks its mocks against their fields.
7. Add portable engine or participant fixtures so scheduling, failure, stale-input, and persistence behavior runs on every platform. Test real platform availability separately.

Participant results and Activity records must not include input content, credentials, sensitive file paths, or unbounded provider output. User-visible output may contain an explicitly requested derived result, but participant summaries and failures remain neutral and bounded.

See [Analysis failure and parity matrix](ANALYSIS_FAILURE_MATRIX.md) for the shared outcome, mutation, and surface semantics.

See [Analysis performance baselines](ANALYSIS_PERFORMANCE.md) for the opt-in release-mode harness and regression policy.

See [Analysis 1.0 acceptance](ANALYSIS_ACCEPTANCE.md) for the release acceptance matrix across GUI, CLI, capture, and platform-specific extraction.
