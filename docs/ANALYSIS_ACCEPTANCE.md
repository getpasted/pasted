# Analysis 1.0 acceptance

This matrix is the acceptance boundary for the frozen Pasted 1.0 Analysis contract. It complements portable automated tests with the native checks that require a real graphical app or operating-system framework.

## Participant surfaces

| Participant | GUI surface | CLI and API surface | 1.0 configuration boundary |
| --- | --- | --- | --- |
| Structure Inspector | Content-free statistics in Clip Preview | `pasted inspector` and `pasted registry --kind inspector` | Immutable and always available; no separate switch. |
| Image Text Extractor | Settings → Analysis → Extractors and OCR controls | `pasted extractor`, `pasted ocr`, and `pasted registry --kind extractor` | Definitions are manageable; engine availability remains platform-specific. |
| Detectors | Settings → Analysis → Detectors, Types, testing, and Rescan Clips | `pasted detector`, `pasted type`, and `pasted registry --kind detector` | Definitions, priority, enabled state, and supported validators are manageable. |
| Smart Actions Enricher | Contextual Smart Actions in Clip Preview | `pasted enricher` and `pasted registry --kind enricher` | Immutable; follows the Transformations feature and interactive policy. |
| Whole Analyzer | Clip Preview | `pasted analyzer run` and the shared Analysis API | Callers select a bounded policy and optional extraction; they do not invoke passes directly. |

Inspector and Enricher settings are intentionally absent. Inspector output is required by Clip Preview and other participants. Enricher output is useful only when Transformations can consume its stable recommendations, so the existing Transformations feature state is its user-facing control.

## Automated acceptance

Run from the repository root:

```sh
npm run test:all
cargo test --manifest-path src-tauri/Cargo.toml --test cli_integration
npm run bench:analysis -- --iterations 1000
```

The integration suite creates an isolated library and verifies Inspector preview and apply, Smart Action recommendation and non-mutation, Detector preview and apply, Extractor lifecycle and unavailable-engine behavior, whole-Analyzer policy bounds, canonical JSON fixtures, and content-free participant summaries. The portable engine fixtures cover produced, no-output, unavailable, failed, stale-input, and persistence outcomes on every platform. Platform smoke jobs compile the real native binaries separately.

## Native macOS acceptance

Use a high-contrast image containing a unique, non-sensitive sentence and an isolated test library. Confirm that `pasted extractor list --json` reports Apple Vision OCR as available, then run:

```sh
pasted extractor run extractor:apple-vision-ocr --file /absolute/path/to/test-image.png --json
```

The result must report `outcome: "produced"`, return the recognized sentence only in the explicitly requested Extractor output, continue through classification, and leave every application flag false in preview. The macOS availability-gate test separately verifies that the advertised Vision engine is linked into the native test binary.

Repeat through the graphical app with an image clip. Confirm OCR status completes, extracted text becomes searchable after relaunch, and a later Detector may classify the derived text without replacing the original image.

## Native Tesseract acceptance

Install Tesseract 5 from Homebrew or the operating-system package manager. Confirm that `pasted extractor list --json` reports Tesseract OCR as available, then run:

```sh
pasted extractor run extractor:tesseract-ocr --file /absolute/path/to/test-image.png --json
```

The result must satisfy the same produced-text, classification, preview, and privacy contract as Apple Vision. The portable test suite generates a bounded image and exercises the real Tesseract executable whenever it is installed; Linux validation installs `tesseract-ocr` so this native adapter cannot pass solely through a mock. With Tesseract absent, the shipped Extractor must remain visible and report an explicit unavailable reason without attempting another engine.

## GUI acceptance

1. Open Settings → Analysis. Confirm Extractors, Detectors, and OCR status are present and that there are no redundant Inspector or Enricher switches.
2. Select a text clip. Confirm Clip Preview shows character, word, and line statistics without an additional mutation or permission prompt.
3. Save a Transform whose name or Operations match a bounded Smart Action signal such as URL or JSON. Select a matching text clip and confirm the recommendation appears without executing automatically.
4. Select an image clip and run OCR explicitly. Confirm progress, success, no-text, failure, retry, and cancellation states settle rather than remaining indefinitely active.
5. Select a multi-file clip. Confirm item count, extensions, live availability, and total file size appear while file paths remain absent from Analysis JSON and Activity.
6. Disable Content Detection, OCR, and Transformations one at a time under Settings → Functionality. Confirm unrelated Analysis behavior remains usable and re-enabling each feature restores its surface without data loss.

## Freeze rule

Treat `contracts/analysis/v1` as the public compatibility source. A 1.0 change is complete only when shared Rust behavior, GUI consumption, CLI JSON, canonical fixtures, failure semantics, privacy checks, and this acceptance matrix agree. Incompatible contract changes move to a new version rather than reinterpreting version 1.
