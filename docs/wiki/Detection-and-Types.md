# Content Analysis, Detection, and Types

Content Detection classifies new text clips into Types used by search, calculated Type collections, and Smart Bins. It does not send content to an intelligence provider. Detection runs locally through an ordered registry of regular expressions and optional built-in validators.

Enable **Content Detection** and **Types** under **Settings → Functionality**, then open **Settings → Analysis**.

## Bounded analysis passes

Analysis uses a shared, non-destructive scheduler. Original clip representations enter an analysis context, and registered participants declare which representations they require and provide. Each participant runs at most once in one of four ordered passes:

1. **Inspect** reads structural information already available at capture.
2. **Extract** derives representations such as searchable image text.
3. **Classify** applies Detectors to the text or representations now available.
4. **Enrich** is reserved for optional, more expensive derived metadata.

Within each pass, ready participants run in priority order. A participant blocked on a same-pass representation waits until a producer makes that input available; each participant still executes at most once, without recursion. Participants whose inputs remain missing after the pass settles are skipped. A participant that reports success without producing its declared output fails closed. Original clip content is never replaced by the scheduler. Operations remain separate because they are user-directed mutations rather than analysis participants.

## Extractors

Extractors create searchable representations from clip content without replacing the original. Apple Vision OCR is the built-in image-to-text Extractor. It runs locally on macOS and appears as unavailable on Windows and Linux, where Apple Vision is not present. Extractor names, descriptions, priority, and enabled state can be managed under **Settings → Analysis → Manage Extractors**.

OCR scans use the first enabled, available Extractor with an `image` input and `searchable_text` output contract. The resulting text becomes available to the later classify pass during the same bounded run. This explicit boundary allows additional local or provider-backed engines without changing Detection or the stored OCR result model.

Engine availability and execution use one shared native registry for app-driven OCR, manual runs, and the CLI. Every engine returns a bounded typed outcome: produced text, no output, or a failure with a stable code and neutral message. Unknown engines remain stored but unavailable instead of falling through to another executable. Apple Vision is the only shipped engine adapter in this release.

Extractor failures remain distinct from valid no-text results throughout Analysis. Background and applied runs record a bounded lowercase ASCII failure code for retry and diagnostics, while CLI JSON reports `outcome` and a structured `failure` without including image or clipboard content. If attempt persistence fails, claimed OCR work returns to the pending state instead of remaining stuck as running.

## How detector matching works

Enabled detectors are evaluated in priority order; the lowest priority number runs first. Each detector defines:

- a display name and description;
- the Type assigned to a match;
- one or more regular expressions, where any expression may produce a candidate;
- an optional validator that rejects likely false positives;
- an enabled state for new clips.

Available validators include card and IBAN checksums, IP parsing, phone guardrails, environment-block recognition, and prose guardrails. A validator supplements the regular expression; it does not replace it.

Use the sample field and **Test** before saving a detector. Testing reports whether the current draft matches the sample without reclassifying history.

## Editing and recovering detectors

Built-in and custom detectors can be enabled, disabled, reordered by priority, duplicated, and edited. Deleting a shipped detector does not make it unrecoverable. **Reset to Default** restores the selected built-in draft, while **Restore Defaults** restores shipped Types and detectors and preserves custom entries.

Detector changes affect newly captured text. Existing clips keep their current Type until an explicit rescan.

## Rescan Clips

**Rescan Clips** reapplies the current enabled detector order to existing text clips. Confirm it only when you intend to reinterpret existing data because it can change:

- clip Types;
- Type collection results;
- Smart Bin membership;
- sensitive-content masking driven by classification.

Images and file clips are not reclassified. The completed operation reports how many text clips changed. Detector and Type registry edits are recorded in Activity when that feature is enabled, but registry metadata does not use clip Revision History.

The CLI equivalent requires explicit confirmation:

```sh
pasted detector rescan --yes --json
```

## Types and Groups

**Manage Types** opens the shared Type and Group registry. Type IDs are stable so saved searches, Smart Bins, CLI output, and historical clips can keep referring to the same concept even when its name, icon, or group changes.

- Built-in Types and Groups can be customized and later restored.
- Custom Types can be archived without changing historical clips.
- Archiving a Type disables detectors that would produce it.
- A custom Group must be empty before it can be archived or permanently deleted.
- Archived entries remain recoverable and are excluded from ordinary selection.

Disabling **Types** hides calculated Type collections. Disabling **Content Detection** stops new detector-based classification and hides detector management. Analysis remains available when OCR is enabled. Neither action deletes existing clips or registry data.

## CLI reference

The CLI can list and configure Extractors; list, create, update, delete, enable, and restore Detectors; manage Types and Groups; and inspect the shared processing registry. Use [`pasted extractor`, `pasted detector`, `pasted type`, and `pasted registry`](CLI-Reference#content-analysis) for scriptable access.
