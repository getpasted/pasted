# Bins and Organization

Bins are named collections of clips. They can have an emoji, a theme-safe text color, saved ordering, and an optional Transform.

## Manual Bins

Drag a clip onto a manual Bin or use the multi-select Bin picker in the clip viewer. A clip can belong to several manual Bins at once. The picker adds and removes individual memberships without disturbing the others, and the selected clip shows its complete Bin list.

Older libraries may still carry one primary-Bin compatibility pointer, but it does not limit current organization: persisted membership records are authoritative and manual Bins can overlap freely. The GUI, CLI, Full Backup, History and Organization transfer, Revision History, Search, and Activity use those same assignment records.

When deleting a Bin, choose what happens to its clips:

- **No Bin** — keep clips and remove the Bin relationship;
- **Trash** — move affected clips to Trash;
- another manual Bin — reassign them atomically.

If the operation cannot finish, the Bin and clip relationships remain unchanged.

Manual Bins can protect every assigned clip without changing any clip's explicit protection setting. Inherited protection applies immediately on assignment and disappears when the clip leaves its last protecting Bin, unless the clip is also explicitly protected or has an assigned hotkey. When several Bins protect a clip, it remains protected until it leaves all of them. The Protected collection and `is:protected` search include explicit, hotkey, and inherited protection. Smart Bins cannot confer protection because their membership is computed dynamically.

## Smart Bins

Smart Bins match Clip Type, Content Type, File Format, or Source. Choose a known value or enter a custom stable ID or name, then match it exactly with **is** or partially with **contains**. Multiple conditions can match any or all.

Each axis follows its Functionality setting. Disabling Clip Types, Content Types, File Formats, or Sources makes related conditions inactive without deleting the Smart Bin. Re-enable the feature to restore matching; rescan History to backfill derived Content Types and File Formats.

Smart Bins are computed views, so clips are not manually dropped into them. Older rules that used Capture Method, raw text, extensions, or file locations remain compatible but are no longer offered for new rules.

Default first-launch Bins are:

- **Projects**, a manual Bin for organizing clips into a collection;
- **From Browsers**, a Smart Bin that combines clips captured from common browsers.

These defaults demonstrate manual and calculated organization without duplicating the Clip Type and Content Type collections in the main navigation.

The versioned rule shape used by the GUI, CLI, and portable transfer files is documented in [Smart Bin Rule Contract](Smart-Bin-Rule-Contract.md).

## Ordering

Manual Bin order and per-Bin clip order persist across reloads, backups, and CLI access. Reordering uses the same collection-order contract as Queue and pinned clips.

## Bin Transforms

A Bin may run one saved Transform when a clip enters it. The original content and previous Bin can be recovered from Revision History when revisions are enabled.

## Clip hotkeys

Assign a global hotkey from the selected clip's detail view to paste that exact clip into the last external target. Assigning a hotkey explicitly protects the clip. Removing the hotkey leaves protection in place; remove the hotkey before explicitly unprotecting the clip.
