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

## Media metadata

The shipped **Media Metadata** Inspector uses an installed `ffprobe` executable to read bounded audio and video facts from up to eight referenced files. Results include aggregate container, codec, stream-count, and duration metadata without returning file paths. Install FFmpeg with `brew install ffmpeg` on Homebrew systems or the distribution's FFmpeg package on Linux. The Inspector remains registered when ffprobe is missing and reports its unavailable engine explicitly.

Media metadata is inspected live because referenced files can change outside the library. Structural metadata remains hash-bound and persistent; live ffprobe results are not written into Activity or portable exports.

## OCR

Apple Vision extracts searchable text from clipboard images and screenshots on macOS. Tesseract 5 provides an optional local alternative on macOS, Linux, and Windows. Install it with `brew install tesseract` on Homebrew systems or the distribution's `tesseract-ocr` package on Linux, then reopen or refresh Extractor settings.

- OCR is optional under **Settings → Functionality**.
- Re-enabling OCR resumes a hash-safe backfill of eligible images.
- Disabling OCR cancels background work; late results are discarded.
- Deleting or purging a clip also removes or excludes its OCR lifecycle state.
- The clip Inspector identifies the Extractor that produced the displayed OCR text. This provenance is also available in clip JSON as `ocr_extractor_ref`, `ocr_extractor_name`, and `ocr_engine_version`.
- Full Backup and History and Organization transfer round trips preserve completed OCR state.

Use **Settings → Analysis** or `pasted ocr status --json` to inspect background progress. OCR runs only when an available `image → searchable_text` Extractor is enabled.

Manual extraction in the clip preview and `pasted extractor run --apply` use the same hash-safe application result as background OCR. A result reports whether OCR text and derived classification were actually updated; stale or removed clips are rejected instead of being reported as applied.
