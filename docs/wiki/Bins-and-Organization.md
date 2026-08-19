# Bins and Organization

Bins are named collections of clips. They can have an emoji, a theme-safe text color, saved ordering, and an optional Transform.

## Manual Bins

Drag a clip onto a manual Bin or select one from the clip viewer. A clip has one primary manual/category Bin at a time; assigning another clears the old primary relationship.

When deleting a Bin, choose what happens to its clips:

- **No Bin** — keep clips and remove the Bin relationship;
- **Trash** — move affected clips to Trash;
- another manual Bin — reassign them atomically.

If the operation cannot finish, the Bin and clip relationships remain unchanged.

## Smart Bins

Smart Bins match Clip Type, Content Type, File Format, or Source. Choose a known value or enter a custom stable ID or name, then match it exactly with **is** or partially with **contains**. Multiple conditions can match any or all.

Each axis follows its Functionality setting. Disabling Clip Types, Content Types, File Formats, or Sources makes related conditions inactive without deleting the Smart Bin. Re-enable the feature to restore matching; rescan History to backfill derived Content Types and File Formats.

Smart Bins are computed views, so clips are not manually dropped into them. Older rules that used Capture Method, raw text, extensions, or file locations remain compatible but are no longer offered for new rules.

Default first-launch Smart Bins are:

- Images;
- Links and Web;
- Code Snippets.

The versioned rule shape used by the GUI, CLI, and portable transfer files is documented in [Smart Bin Rule Contract](Smart-Bin-Rule-Contract.md).

## Ordering

Manual Bin order and per-Bin clip order persist across reloads, backups, and CLI access. Reordering uses the same collection-order contract as Queue and pinned clips.

## Bin Transforms

A Bin may run one saved Transform when a clip enters it. The original content and previous Bin can be recovered from Revision History when revisions are enabled.
