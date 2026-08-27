# Transforms and Intelligence

A **Transform** is a reusable intent: describe the result, let Pasted build a validated plan, preview it, and save it for clips or Bins.

## Deterministic and semantic work

- Deterministic steps are local, replayable Operations such as trimming, case conversion, encoding, extraction, or regex replacement.
- Semantic steps run through a Connection you explicitly enabled, such as an authenticated local CLI or local model.

Not every Transform uses AI. Pasted records which Transform ran, whether intelligence was involved, and the resulting revision/provenance.

## Connections

**Settings → Intelligence** detects supported local tools without reading their credentials. Enable and order connections to define priority and fallback. Pasted stores connection metadata and credential references, not API keys.

On compatible Macs, discovery also includes Apple Intelligence through the on-device `SystemLanguageModel`. It requires no endpoint or credential, keeps requests on the device, and remains unavailable until macOS reports the model assets ready.

Structured provider results use Pasted's bounded JSON Schema 2020-12 subset: objects, arrays, strings, numbers, integers, booleans, nullability, enumerations, required properties, closed objects, item-count limits, and numeric ranges. Unsupported constraints fail before execution, and every provider's result is validated against the original schema before Pasted accepts it.

## Running a Transform

Transforms can be previewed from the clip Workflow menu, run in the Playground, invoked from a Bin, or run through the CLI. Replacing clip content creates a revision when Revision History is enabled.

## Advanced Operations and Manual Transforms

Operations and deterministic manual Transforms remain available under **Advanced** for compatibility and power-user workflows. Legacy pipeline identifiers and execution contracts remain stable, but the editor experience is experimental in 1.0.

Architecture details live in the repository’s [Transformations documentation](https://github.com/getpasted/pasted/blob/main/docs/TRANSFORMATIONS.md).
