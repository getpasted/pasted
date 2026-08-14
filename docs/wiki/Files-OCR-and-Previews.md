# Files, OCR, and Previews

Copied files remain file references. Pasted can display bounded previews without replacing the original clipboard representation.

Some screenshot tools publish a composite clipboard item containing both an image file reference and bitmap bytes. Pasted preserves explicit copies from Finder and other file managers as file references, but treats screenshot and otherwise ambiguous single-image composites as images so image paste and OCR remain available.

macOS does not expose a reliable clipboard-owner application for these payloads. Pasted labels a recognized composite capture as **Screenshot** instead of attributing it to whichever app happens to have focus. A screenshot-tool name is used only when the clipboard metadata provides a confident signal, such as a standard CleanShot filename.

When the file and bitmap representations arrive as separate clipboard updates, Pasted compares their bounded decoded RGBA content. An exact match is kept as one screenshot image rather than duplicated as an image and a file.

## File clips

- Multiple files preserve selection order.
- Name and path are selectable and copyable.
- Missing files fail explicitly instead of silently pasting unrelated data.
- File metadata replaces text-centric character/word/line metadata where appropriate.

## Previews

Pasted can cache bounded thumbnails for familiar image formats and the first page of PDFs. Preview settings control which safe types are shown and the maximum preview size. Large or unsupported files remain references.

## OCR

On macOS, Apple Vision can extract searchable text from clipboard images and screenshots.

- OCR is optional under **Settings → Functionality**.
- Re-enabling OCR resumes a hash-safe backfill of eligible images.
- Disabling OCR cancels background work; late results are discarded.
- Deleting or purging a clip also removes or excludes its OCR lifecycle state.
- Full Backup and History and Organization transfer round trips preserve completed OCR state.

Use **Settings → Analysis** or `pasted ocr status --json` to inspect background progress. OCR runs only when an available `image → searchable_text` Extractor is enabled.
