# Analysis failure and parity matrix

Analysis distinguishes absence, valid empty results, failures, and rejected mutations. GUI, CLI, capture, rescan, and background callers consume the same execution results; surface adapters may change presentation or process exit status, but must not reinterpret an outcome.

| Condition | Contract result | Mutation | Surface behavior |
| --- | --- | --- | --- |
| Pass excluded by policy | No participant run is reported. | None. | Capture, background, and rescan stop after `classify`; interactive work may continue through `enrich`. |
| Participant lacks a required representation | Participant outcome is `missing_input`. A focused participant result resolves this to failure code `missing_input`. | None. | GUI, CLI, and background callers preserve the same participant summary. |
| Extractor finds no text | Extractor outcome is `no_output`; `failure` and `output` are null. | Applied runs may record successful OCR completion with no text. | This is a valid result, not an engine failure. |
| Detector finds no match | Detector outcome is `no_match`; `matched` is false and classification fields are null. | Applied runs retain the plain-text classification. | This is a valid result and exits successfully in the CLI. |
| Participant fails | Outcome is `failed` with a stable code and neutral bounded message. Partial derived context is discarded. | No derived output is applied. Extractor attempt state may record the bounded failure code. | CLI Extractor runs emit the structured result and exit nonzero; GUI and background callers receive the same result. |
| Declared output is absent | Scheduler converts the apparent success to `failed` with code `contract_violation`. | None. | Every caller fails closed before interpreting participant-specific fields. |
| Participant is unexpectedly absent | Focused resolution returns `failed` with code `missing_participant`. | None. | Indicates an internal scheduling/registry mismatch, never a valid empty result. |
| Input or source metadata exceeds a safety bound | Analysis is rejected before participant work. Inspector uses code `input_too_large`; other focused entry points return their bounded validation error. Source metadata shares the Analysis contract's 1,024-byte limit. | None. | GUI commands reject; CLI exits nonzero; background work does not enqueue unbounded payloads. |
| Apply target changed or disappeared | The shared application service rejects the stale content hash or missing clip. | None; `appliedClipId` is never reported. | GUI, CLI, and background extraction share the same hash-safe persistence path. |
| Persistence fails after a claim | The operation returns an error and resets claimed OCR work when applicable. | Transaction rolls back or the work returns to pending. | No surface may report a successful application from inferred state. |

The canonical examples in `contracts/analysis/v1` lock representative bounded-policy, no-match, missing-input, and failed JSON shapes. Portable scheduler and execution tests cover the remaining rows; platform availability is tested separately from shared behavior.
