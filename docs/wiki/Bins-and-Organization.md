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

Smart Bins match Clip Type, Source, Content Type, Capture Method, text content, or file details. Clip Type, Source, and Content Type choices follow their Functionality settings. They are computed views, so clips are not manually dropped into them.

Default first-launch Smart Bins are:

- Screenshots;
- Links & Web;
- Code Snippets.

## Ordering

Manual Bin order and per-Bin clip order persist across reloads, backups, and CLI access. Reordering uses the same collection-order contract as Queue and pinned clips.

## Bin Transforms

A Bin may run one saved Transform when a clip enters it. The original content and previous Bin can be recovered from Revision History when revisions are enabled.
