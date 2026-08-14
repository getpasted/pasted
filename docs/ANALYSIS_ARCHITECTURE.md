# Analysis architecture

Pasted runs derived-content work through one bounded Analyzer and exposes participant-specific results through parallel execution modules.

## Stable layers

- `analysis_contract.rs` owns the shared result metadata, contract version, representation names, the four ordered passes, execution policies, participant contracts and run summaries, target kinds, failures, and clip-application state.
- `content_analysis.rs` owns typed Analysis requests, scheduling, and the in-memory Analysis context. Raw `AnalysisReport` values stay crate-internal.
- `analysis_execution.rs` translates a whole run into one content-free Analyzer snapshot. `inspection_execution.rs`, `extraction_execution.rs`, `detection_execution.rs`, and `enrichment_execution.rs` translate raw reports into stable participant-specific results. GUI, CLI, and background services consume these results instead of interpreting scheduler state.
- Persistence accepts typed execution results. Every public result carries shared version, policy, and final-pass metadata; preview and apply share the same serialized shape, and applied clip IDs come from shared application state rather than surface-specific JSON assembly.

Capture, background, and rescan policies stop after classification. Interactive work may continue through enrichment, so optional expensive participants never run implicitly during capture. Callers choose a policy and available participants; they do not invoke scheduler passes directly.

Text capture submits one Capture-policy Analyzer request and reuses its classification and structural metadata during insertion. Persistence falls back to focused structural inspection only when the precomputed snapshot is unavailable or cannot be safely activated. Focused rescans and OCR application remain participant-specific because their mutation contracts intentionally target only classification or extracted text.

The shipped `inspector:structure-v1` participant produces `structural_metadata` during the inspect pass. Stable, content-free facts are persisted against the clip content hash and a structural input fingerprint. Filesystem availability and size are live observations kept outside durable Analysis results. Full Backup includes the durable result table automatically; portable History and Organization transfer omits recomputable derived results.

The shipped `enricher:smart-actions-v1` participant consumes analyzable text, classification, and structural metadata only for interactive requests. It returns bounded signals and stable saved-Transform references; it never returns input content, executes a Transform, or writes a clip. Clip Preview and `pasted enricher run` use the same execution result.

Clip Preview and `pasted analyzer run` consume the whole-Analyzer snapshot. The snapshot reports clip kind, structural metadata, classification, content-free recommendations, participant outcomes, and only a boolean indicating whether searchable text became available. It never returns original text, OCR text, image bytes, or file paths. File clips intentionally schedule inspection only, preventing serialized file-reference metadata from being reinterpreted as analyzable text. Automatic Clip Preview requests do not enable extraction; `pasted analyzer run --clip ID --extract` is the explicit potentially expensive image path.

## Adding an Inspector or Enricher

1. Add a typed `RepresentationKind` and bounded `AnalysisContext` field only when the participant has a concrete new input or output. Reuse an existing representation when its semantics match exactly.
2. Build an `AnalysisParticipant` with a stable reference, one `AnalysisPass`, deterministic priority, and explicit `requires` and `provides` lists. The scheduler runs it at most once and fails closed when declared output is missing.
3. Add a participant-specific execution module, such as `enrichment_execution.rs`. Translate `AnalysisReport` through `resolve_participant`, use `AnalysisTargetKind`, and expose privacy-safe `ParticipantRun` summaries.
4. Use `ClipApplication` for preview and apply identity. Add participant-specific persistence flags beside it only when callers need to distinguish derived records that were actually updated.
5. Route GUI, CLI, and background work through the same execution and application functions. Keep structured JSON stable and never synthesize application state in a surface adapter.
6. Add portable engine or participant fixtures so scheduling, failure, stale-input, and persistence behavior runs on every platform. Test real platform availability separately.

Participant results and Activity records must not include input content, credentials, sensitive file paths, or unbounded provider output. User-visible output may contain an explicitly requested derived result, but participant summaries and failures remain neutral and bounded.
