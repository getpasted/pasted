# Detection and Types

Content Detection classifies new text clips into Types used by search, calculated Type collections, and Smart Bins. It does not send content to an intelligence provider. Detection runs locally through an ordered registry of regular expressions and optional built-in validators.

Enable **Content Detection** and **Types** under **Settings → Functionality**, then open **Settings → Detection**.

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

## Rescan History

**Rescan History** reapplies the current enabled detector order to existing text clips. Confirm it only when you intend to reinterpret existing data because it can change:

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

Disabling **Types** hides calculated Type collections. Disabling **Content Detection** stops new detector-based classification and hides the Detection settings page. Neither action deletes existing clips or registry data.

## CLI reference

The CLI can list, create, update, delete, enable, and restore detectors; manage Types and Groups; and inspect the shared processing registry. Use [`pasted detector`, `pasted type`, and `pasted registry`](CLI-Reference#detection) for scriptable access.
