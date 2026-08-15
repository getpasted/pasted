# Analysis 1.0 acceptance

This matrix is the acceptance boundary for the frozen Pasted 1.0 Analysis contract. It complements portable automated tests with the native checks that require a real graphical app or operating-system framework.

## Participant surfaces

| Participant | GUI surface | CLI and API surface | 1.0 configuration boundary |
| --- | --- | --- | --- |
| Capture | Settings → Analysis → read-only Manage Capture dialog | `pasted registry --kind capture` | Assigns one structural Clip Type and records source attribution before Analysis. Sources controls attribution presentation, not retained data. |
| Structure and Media Metadata Inspectors | Read-only Manage Inspectors dialog plus content-free facts in Clip Preview | `pasted inspector` and `pasted registry --kind inspector` | Immutable; no separate switches. Runtime availability is reported for Media Metadata. |
| Image Text Extractor | Settings → Analysis → Extractors and OCR controls | `pasted extractor`, `pasted ocr`, and `pasted registry --kind extractor` | Definitions are manageable; engine availability remains platform-specific. |
| File Text Extractor | Settings → Analysis → Extractors and persistent availability status | `pasted extractor`, the file-extraction API, and `pasted registry --kind extractor` | Definitions and local model paths are manageable; expensive transcription remains explicit. |
| Detectors | Settings → Analysis → Detectors, Content Types, testing, and Rescan Clips | `pasted detector`, `pasted type`, and `pasted registry --kind detector` | Definitions, priority, enabled state, and supported validators are manageable. |
| Smart Actions Enricher | Read-only Manage Enrichers dialog plus contextual Smart Actions in Clip Preview | `pasted enricher` and `pasted registry --kind enricher` | Immutable; follows the Transformations feature and interactive policy. |
| Whole Analyzer | Clip Preview | `pasted analyzer run` and the shared Analysis API | Callers select a bounded policy and optional extraction; they do not invoke passes directly. |

Inspector and Enricher definitions are inspectable but immutable in Settings. Their managers explain what each participant works with and provides, while technical details expose stable references and contracts for CLI and API use. Structure remains always available because Clip Preview depends on it. Smart Actions follows the existing Transformations feature state rather than adding a redundant switch.

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

## Native media metadata acceptance

With ffprobe installed, inspect a disposable audio or video file clip and confirm `pasted inspector list --json` reports `inspector:media-metadata-v1` with engine `ffprobe-cli-v1`. The interactive result must contain bounded aggregate `mediaMetadata` without a file path. Repeat on a system where ffprobe is absent and MediaInfo is installed; the same participant and output contract must report engine `mediainfo-cli-v1`. When neither runtime is installed, the participant remains visible and reports that either engine can satisfy it.

## Native whisper.cpp acceptance

Install whisper.cpp and obtain a local GGML model without placing either inside the repository. Configure the model and confirm availability:

```sh
pasted extractor update extractor:whisper-transcription --model /absolute/path/to/ggml-model.bin --json
pasted extractor run extractor:whisper-transcription --file /absolute/path/to/test-audio.wav --json
```

The preview must return only the explicitly requested bounded transcript, classify from that derived text when Detection is enabled, and leave application flags false. Repeat with a disposable file clip and `--clip <id> --apply`; `searchableTextUpdated` and `appliedClipId` must report success, the original file-reference payload must remain unchanged, and searching a unique transcript phrase must find the clip. With `whisper-cli` or the model missing, the shipped Extractor remains visible and reports which dependency is unavailable. No model download may begin.

## GUI acceptance

1. Open Settings → Analysis. Confirm Capture precedes the four participant groups. Manage Capture, Inspectors, and Enrichers must be read-only, show practical input/output relationships, keep technical contracts secondary, and add no redundant switches. In Manage Extractors, confirm the selected Extractor's concise availability remains visible in the settings header and full remediation appears in its tooltip.
2. Select a text clip. Confirm Clip Preview shows character, word, and line statistics without an additional mutation or permission prompt.
3. Save a Transform whose name or Operations match a bounded Smart Action signal such as URL or JSON. Select a matching text clip and confirm the recommendation appears without executing automatically.
4. Select an image clip and run OCR explicitly. Confirm progress, success, no-text, failure, retry, and cancellation states settle rather than remaining indefinitely active.
5. Select a multi-file clip. Confirm item count, extensions, live availability, and total file size appear while file paths remain absent from Analysis JSON and Activity.
6. Disable Sources, OCR, Transcriptions, Content Detection, Content Types, and Transformations one at a time under Settings → Functionality. Confirm each related surface hides without data loss, Capture and structural Clip Type remain visible, and unrelated Analysis behavior remains usable.
7. Open Insights with representative text, image, and multi-file clips. Confirm Clip Types, the top 24 File Formats, and Content Types use separate cards. Disabling Content Types hides only its semantic card; disabling Sources hides only source summaries.

## Freeze rule

Treat `contracts/analysis/v1` as the public compatibility source. A 1.0 change is complete only when shared Rust behavior, GUI consumption, CLI JSON, canonical fixtures, failure semantics, privacy checks, and this acceptance matrix agree. Incompatible contract changes move to a new version rather than reinterpreting version 1.
