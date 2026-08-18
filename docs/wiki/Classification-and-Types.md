# Content Analysis and Content Types

Content Classification assigns semantic Content Types to new text clips for search, calculated collections, and Smart Bins. It runs locally through an ordered registry of regular expressions and optional built-in validators; no intelligence provider is involved.

Open **Settings → Analysis** to configure Analyzer participants. **Content Classification** controls whether Classifiers run; **Content Types** controls semantic labels, collections, and related library presentation without stopping classification.

The settings sequence begins with **Capture**. Clip Type assigns exactly one structural Text, Image, or Files value from the captured representation. Source Attribution records the associated application name; disabling **Sources** hides that capability and its presentation but retains attribution so the setting is reversible. Application icons are resolved only when displayed.

## Bounded analysis passes

Analysis uses a shared, non-destructive scheduler. Original clip representations enter an analysis context, and registered participants declare which representations they require and provide. Each participant runs at most once in one of four ordered passes:

1. **Inspect** reads structural information already available at capture.
2. **Extract** derives representations such as searchable image text.
3. **Classify** applies Classifiers to the text or representations now available.
4. **Suggest** proposes optional Smart Actions from the available analysis signals.

Within each pass, ready participants run in priority order. A participant blocked on a same-pass representation waits until a producer makes that input available; each participant still executes at most once, without recursion. Participants whose inputs remain missing after the pass settles are skipped. A participant that reports success without producing its declared output fails closed. Original clip content is never replaced by the scheduler. Operations remain separate because they are user-directed mutations rather than analysis participants.

The four passes share one versioned participant contract for target identity, requirements, outputs, outcomes, failures, and clip-application state. Capture, background work, and rescans stop after classification. Interactive requests may continue through suggestion, so optional expensive participants do not run implicitly when content is captured. Inspector, Extractor, and Classifier execution modules submit typed requests and translate the internal scheduler report into their domain-specific result without redefining those fields. Suggestion implementations follow the same boundary, so adding a pass participant does not require GUI or CLI code to interpret raw Analyzer state.

Text capture runs inspection and enabled classification in one Capture-policy request, then reuses that snapshot for the stored Content Type and content-hash-bound structural metadata. Focused history rescans and OCR application keep their narrower participant-specific mutation contracts.

The built-in Structure Inspector records content-free text counts, image dimensions, file item counts and extensions, and the derived clip origin. Stable results are fingerprinted and persisted; filesystem availability and total file size remain live observations. Clip Preview and `pasted inspector run` consume the same versioned result.

The built-in Smart Actions participant uses classification, structural metadata, and bounded content signals to suggest saved Transforms by stable reference. It runs only for interactive requests, never executes a suggestion, never mutates a clip, and never includes analyzed text in its result. Clip Preview and `pasted suggestion run` consume the same versioned result.

The Structure Inspector and Smart Actions do not have individual Settings switches. Structure is immutable and always available because Clip Preview depends on its bounded facts. Smart Actions is immutable and follows the Transformations feature under **Settings → Functionality**. Their read-only managers under **Settings → Analysis** show practical input/output relationships and runtime status, with stable references and contracts under Technical details for CLI and API use. Extractors and Classifiers remain authorable.

Every public Analysis result carries the same explicit `formatVersion`, policy, final pass, and privacy-safe participant summaries. Extractor and Classifier results retain their participant-specific outcome and application fields at the top level for stable GUI and CLI consumption.

## Extractors

Extractors create searchable representations from clip content without replacing the original. Every shipped and custom Extractor stores the same editable `recipe-v1` definition: accepted inputs, local commands, direct argv tokens, step artifacts, output capture, resources, time limits, priority, and enabled state. Apple Vision, Tesseract, and Whisper are shipped recipes. The settings header reports current local readiness.

Disabling **OCR** hides the shipped Apple Vision and Tesseract recipes. Disabling **Transcriptions** hides Whisper. User-defined image and file recipes remain manageable, so unrelated tools such as QR and PDF readers are not coupled to those switches.

Extractor input and output names are parsed through the same typed representation contract used by the Analysis scheduler. Unknown methods and unsupported contracts fail closed before execution, and active Extractor selection uses that shared contract rather than matching unrelated metadata strings.

Extraction runs every enabled, available Extractor that accepts the clip input, ordered by priority. Successful outputs are deduplicated and combined for search and later classification. Clip Preview shows each current successful result separately. Its Details footer keeps the complete per-clip scan history, including timestamps, successes, duplicate output, no-output attempts, and failures. Whisper requires both `whisper-cli` and a configured local GGML model file; no model is downloaded automatically.

Availability and execution are shared by app-driven extraction, manual runs, and the CLI. Every definition exposes its configured or automatically resolved runtime location, detected runtime version, dependencies, and revision. Internal execution-contract IDs remain read-only technical details. Tesseract and whisper.cpp executable paths can override automatic discovery; Apple Vision reports the macOS framework instead of inventing a path. Reset restores the current release's shipped definition, while upgrades preserve fields changed by the user.

AI authoring converts one description into the same reviewable local recipe available under Advanced settings. The original request and provider response remain local authoring history; AI is never part of runtime extraction. Without an enabled connection, Advanced remains the complete path. Pasted invokes recipe commands directly without a shell, clears unneeded environment variables, bounds input, output, steps, resources, and runtime, and removes its private workspace afterward. Custom definitions begin disabled.

Every engine returns a bounded typed outcome: produced text, no output, or a failure with a stable code and neutral message. Tesseract, FFmpeg, whisper.cpp, and custom commands run with direct arguments in private temporary workspaces, have fixed execution limits, bound staged input and extracted text, and remove staged output after every attempt. Whisper accepts up to eight local FLAC, MP3, OGG, WAV, M4A, or AAC references per run; M4A and AAC preparation requires FFmpeg. If an accelerated whisper.cpp run fails, the same bounded run is retried once with its CPU backend.

Extractor failures remain distinct from valid no-text results throughout Analysis. A shared application result normalizes produced text, valid no-text results, typed failures, and authoritative persistence flags before background OCR, GUI commands, or CLI commands inspect the outcome. Structured results report `appliedClipId`, `ocrUpdated`, `searchableTextUpdated`, and `classificationUpdated` alongside the Analysis fields. File transcription is stored as hash-bound searchable text with Extractor provenance while the original file-reference payload remains unchanged.

Background OCR, manual GUI extraction, CLI Extractor application, and CLI rescans execute and persist Analyzer results through the same application service. User-initiated application claims the clip by ID and content hash before persistence, while background work uses the same result contract after its queue claim. The shared path records OCR state with identical failure and stale-clip semantics and never reports `appliedClipId` for a rejected stale result. Derived classification persistence is best-effort: a classification write failure never reinterprets successfully stored OCR text as an extraction failure, and a later analysis run can rebuild the derived metadata.

## How classifier matching works

Enabled classifiers are evaluated in priority order; the lowest priority number runs first. Each classifier defines:

- a display name and description;
- the Content Type assigned to a match;
- one or more regular expressions, where any expression may produce a candidate;
- an optional validator that rejects likely false positives;
- an enabled state for new clips.

Available validators include card and IBAN checksums, IP parsing, phone guardrails, environment-block recognition, and prose guardrails. A validator supplements the regular expression; it does not replace it.

Use the sample field and **Test** before saving a classifier. Testing reports whether the current draft matches the sample without reclassifying history.

Classifier previews, new text capture, explicit application, and history rescans consume the same typed Analysis result. It distinguishes `matched`, `no_match`, and `failed`, carries the matched Content Type and Classifier reference, and includes bounded participant summaries without including analyzed text. Applying a Classifier runs inside the clip mutation transaction, so the returned result describes the content and Classifier definition that were actually applied.

## Editing and recovering classifiers

Built-in and custom classifiers can be enabled, disabled, reordered by priority, duplicated, and edited. Deleting a shipped classifier does not make it unrecoverable. **Reset to Default** restores the selected built-in draft. **Reset…** restores shipped Content Types and Classifiers while preserving custom entries.

Classifier changes affect newly captured text. Existing clips keep their current Content Type until an explicit rescan.

## Rescan Clips

**Rescan Clips** reapplies the current enabled classifier order to existing text clips. Confirm it only when you intend to reinterpret existing data because it can change:

- clip Content Types;
- Content Type collection results;
- Smart Bin membership;
- sensitive-content masking driven by classification.

Images and file clips are not reclassified. The completed operation reports how many text clips changed, remained unchanged, or failed Analysis. Classifier and Content Type registry edits are recorded in Activity when that feature is enabled, but registry metadata does not use clip Revision History.

If Analysis itself fails for a clip, the rescan leaves that clip's existing Content Type unchanged instead of silently reclassifying it as plain text.

The CLI equivalent requires explicit confirmation:

```sh
pasted classifier rescan --yes --json
```

## Content Types and Groups

Clip Type, File Format, and Content Type are separate concepts. Every clip has exactly one structural Clip Type: Text, Image, or Files. A Files clip may contain several referenced files and therefore several File Formats. Classifiers may eventually associate several semantic Content Types with one clip; the current persistence contract retains one winning classification until the multi-match schema migration is complete. Insights presents these categories separately rather than combining them.

**Manage Content Types…** opens the shared Content Type and Group registry. Content Type IDs are stable so saved searches, Smart Bins, CLI output, and historical clips can keep referring to the same concept even when its name, icon, or group changes.

- Built-in Content Types and Groups can be customized and later reset.
- Custom Content Types can be archived without changing historical clips.
- Archiving a Content Type disables Classifiers that would produce it.
- A custom Group must be empty before it can be archived or permanently deleted.
- Archived entries remain recoverable and are excluded from ordinary selection.

Disabling **Content Types** hides semantic labels, calculated collections, and Content Type summaries without stopping classification. Structural Clip Type remains visible. Disabling **Content Classification** stops classifier-based classification and rescans. The Classifier manager remains available when either feature is enabled. Neither action deletes existing clips or registry data.

## CLI reference

The CLI can preview the whole Analyzer; run and inspect built-in Inspectors and Suggestions; list and configure Extractors; list, create, update, delete, enable, and restore Classifiers; manage Content Types and Groups; and inspect the shared processing registry. `pasted registry list --kind capture --json` exposes Clip Type and Source Attribution alongside the processing definitions. Use [`pasted analyzer`, `pasted inspector`, `pasted suggestion`, `pasted extractor`, `pasted classifier`, `pasted type`, and `pasted registry`](CLI-Reference#content-analysis) for scriptable access.
