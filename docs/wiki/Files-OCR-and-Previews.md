# Files, OCR, and Previews

Copied files remain file references. Pasted can display bounded previews without replacing the original clipboard representation.

## File clips

- Multiple files preserve selection order.
- Name and path are selectable and copyable.
- Missing files fail explicitly instead of silently pasting unrelated data.
- File metadata replaces text-centric character/word/line metadata where appropriate.

## Previews

Pasted can cache bounded thumbnails for familiar image formats and the first page of PDFs. Preview settings control which safe types are shown and the maximum preview size. Large or unsupported files remain references.

## OCR

On macOS, Apple Vision can extract searchable text from clipboard images and screenshots.

- OCR is optional under **Settings → Features**.
- Re-enabling OCR resumes a hash-safe backfill of eligible images.
- Disabling OCR cancels background work; late results are discarded.
- Deleting or purging a clip also removes or excludes its OCR lifecycle state.
- Backup/import preserves completed OCR state.

Use **Settings → Debug** or `pasted ocr status --json` to inspect background progress.

