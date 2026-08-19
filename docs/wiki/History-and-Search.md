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

- `clip:` — structural Clip Type;
- `content:` — current Content Type;
- `format:` — verified File Format;
- `source:` — capture source;
- `has:note` — clips with notes;
- `is:pinned` — pinned clips;
- `is:protected` — protected clips;
- `is:trashed` — clips in Trash;
- `regex:` — regular expression search.

Clip Type, Content Type, File Format, and Source filters match complete values case-insensitively; ordinary search terms remain partial matches. The helpers follow their corresponding Functionality settings, and a disabled axis suspends its filters. An incomplete helper such as `source:` is treated as incomplete rather than “match everything.” Arrow keys navigate authoritative, paginated results from the local library, including clips older than the History page currently loaded in the app. `Esc` closes the helper menu; on an empty search it returns to the previous collection.

OCR text and stored transcripts participate in ordinary text search without replacing the original image or file references, including when `is:trashed` searches Trash. Extracted text is used for matching but is never added to Search result payloads.

Trashed search results are marked and remain chronologically commingled with active results so a Trash action does not reorder the whole result set.

## Retention

History, Trash, and Activity History each have independent count and age limits. A clip that exceeds either enabled History limit moves to Trash (or is purged when Trash is disabled). Trash purges its oldest eligible items when either Trash limit is exceeded, and Activity History removes its oldest entries by the same rule. A zero count means **Unlimited**, and a zero age means **Forever**. Pinned and protected clips are excluded from History retention, and protected clips are never auto-purged from Trash. Hard resource ceilings remain in force even when a user-facing limit is permissive.
