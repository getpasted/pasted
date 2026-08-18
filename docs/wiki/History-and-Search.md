# History and Search

**History** contains clips that have not been moved to Trash. Trash remains separate so “everything you kept” and “everything you discarded” do not become the same list.

Pasted captures bounded representations of:

- plain and rich text;
- clipboard images and screenshots;
- copied image, text, media, PDF, and other file references;
- multiple files in their original selection order.

## Search

Search is its own persistent collection. Leaving Search does not erase the query, and returning restores the results.

Supported helpers include:

- `source:` — capture source;
- `type:` — Clip Type or any current Content Type;
- `has:note` — clips with notes;
- `is:pinned` — pinned clips;
- `is:protected` — protected clips;
- `regex:` — regular expression search.

An incomplete helper such as `source:` is treated as incomplete rather than “match everything.” Arrow keys navigate results. `Esc` closes the helper menu; on an empty search it returns to the previous collection.

OCR text and stored transcripts participate in ordinary text search without replacing the original image or file references.

Trashed search results are marked and remain chronologically commingled with active results so a Trash action does not reorder the whole result set.

## Retention

History, Trash, and Activity History each have independent count and age limits. A clip that exceeds either enabled History limit moves to Trash (or is purged when Trash is disabled). Trash purges its oldest eligible items when either Trash limit is exceeded, and Activity History removes its oldest entries by the same rule. A zero count means **Unlimited**, and a zero age means **Forever**. Pinned and protected clips are excluded from History retention, and protected clips are never auto-purged from Trash. Hard resource ceilings remain in force even when a user-facing limit is permissive.
