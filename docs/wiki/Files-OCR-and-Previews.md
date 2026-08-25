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

The shipped **Media Metadata** Inspector uses an installed `ffprobe` executable when available and falls back to MediaInfo. Both engines normalize bounded audio and video facts from up to eight referenced files into the same aggregate container, codec, stream-count, and duration contract without returning file paths. Install FFmpeg with `brew install ffmpeg` or MediaInfo with `brew install mediainfo` on Homebrew systems, or use the corresponding distribution package on Linux. The Inspector remains registered when neither engine is installed and reports the missing runtime explicitly.

The built-in **File Format** Inspector reads a bounded prefix of each referenced file and identifies supported formats from byte signatures. Results are persisted as derived metadata, can include several formats for a multi-file clip, and remain separate from filename extensions. **Settings → Functionality → File Formats** controls scanning and presentation; **Analysis → Rescan Clips…** backfills existing file clips.

Media metadata is inspected live because referenced files can change outside the library. Structural metadata remains hash-bound and persistent; live engine results are not written into Activity or portable exports.

## Audio transcription

The shipped **Whisper Transcription** Extractor is a visible local recipe: FFmpeg prepares bounded audio, then an installed whisper.cpp `whisper-cli` executable writes searchable text with an explicitly selected local GGML model. Homebrew installations can use `brew install ffmpeg whisper-cpp`; model files remain a separate user-managed resource and are never downloaded automatically. Advanced Extractor settings expose both commands, their discovery or absolute executable paths, argv tokens, time limits, output handling, and the model resource.

**Transcriptions** under **Settings → Functionality** controls the shipped Whisper Extractor. It does not hide unrelated user-defined file Extractors such as PDF text readers.

Explicit transcription accepts bounded FLAC, MP3, OGG, WAV, M4A, and AAC file references. M4A and AAC audio is converted to a private temporary WAV with an installed FFmpeg executable before whisper.cpp runs. Applying a result stores searchable text and Extractor provenance without replacing the file clip's original path list. Search and smart `contains` rules include current hash-bound transcription text. A stale result cannot attach to a changed or removed clip.

## OCR

Apple Vision extracts searchable text from clipboard images and screenshots through the macOS Vision framework. Tesseract 5 provides an optional local alternative on macOS, Linux, and Windows. Install it with `brew install tesseract` on Homebrew systems or the distribution's `tesseract-ocr` package on Linux. Extractor settings show the framework or resolved executable location and detected version; an explicit Tesseract path can override automatic discovery.

Visual Labels use the same searchable extraction history without replacing the original image. Apple Vision Labels is built into macOS. **llama.cpp Labels** provides the cross-platform alternative through the official `llama-cli` multimodal interface and `ggml-org/SmolVLM-500M-Instruct-GGUF`. Install llama.cpp with `brew install llama.cpp`, `winget install llama.cpp`, or `conda install -c conda-forge llama.cpp`, then run the exact one-time model-download command shown in Extractor settings. Pasted runs extraction offline, requests bounded structured labels, normalizes duplicates, and applies the Extractor's 80% confidence floor.

- OCR is optional under **Settings → Functionality**.
- Re-enabling OCR resumes a hash-safe backfill of eligible images.
- Disabling OCR cancels background work; late results are discarded.
- Deleting or purging a clip also removes or excludes its OCR lifecycle state.
- The clip Inspector identifies the Extractor that produced the displayed OCR text. This provenance is also available in clip JSON as `ocr_extractor_ref`, `ocr_extractor_name`, and `ocr_engine_version`.
- Full Backup and History and Organization transfer round trips preserve completed OCR state.

Use **Settings → Analysis** or `pasted ocr status --json` to inspect background progress. Image extraction runs enabled, available `image → searchable_text` Extractors in priority order.

Manual extraction APIs and `pasted extractor run --apply` use the same hash-safe application result as background OCR. A result reports whether OCR text, file-backed searchable text, and derived classification were actually updated; stale or removed clips are rejected instead of being reported as applied.

## Visual Labels

Apple Vision Labels finds searchable subjects and objects locally on macOS. Labels use the same Extractor result, current-result storage, bounded scan history, hash-safe searchable-text application, and GUI/CLI execution paths as OCR. They appear as labels in the clip Inspector and can be matched directly with the `visual_label` Smart Bin condition.

The `pasted_json_v1` recipe protocol also accepts an optional `labels` array of `{ "value", "confidenceBasisPoints" }` objects. This provider-neutral contract lets a local llama.cpp wrapper or another image-recognition executable produce the same searchable, historical, and Smart Bin-compatible result without adding another storage path.
