import { Check, Copy, File, Files, Palette, ScanText, Sparkles } from 'lucide-react';
import { getClipFilePaths, type ClipItem } from '../types';
import type { ColorFormats } from '../utils/color';
import { UI_COPY } from '../utils/uiCopy';

interface ClipPreviewContentProps {
  clip: ClipItem;
  displayText: string;
  colorData: ColorFormats | null;
  resolvedImageBase64: string | null;
  filePreviews: Array<{ index: number; dataUrl: string; width: number; height: number }>;
  isFilePreviewLoading: boolean;
  copiedFormat: string | null;
  isOcrLoading: boolean;
  readOnly?: boolean;
  onColorChange: (value: string) => void;
  onCopyFormat: (label: string, value: string) => void;
  onRunOCR: () => void;
}

export function ClipPreviewContent({
  clip,
  displayText,
  colorData,
  resolvedImageBase64,
  filePreviews,
  isFilePreviewLoading,
  copiedFormat,
  isOcrLoading,
  readOnly = false,
  onColorChange,
  onCopyFormat,
  onRunOCR,
}: ClipPreviewContentProps) {
  return (
    <>
        {clip.content_type === 'file' ? (
          <div className="theme-panel rounded-2xl border p-4 shadow-lg">
            <div className="theme-title mb-3 flex items-center gap-2 text-xs font-semibold">
              <Files className="h-4 w-4 text-blue-400" />
              <span>{getClipFilePaths(clip).length === 1 ? 'Copied File' : `${getClipFilePaths(clip).length} Copied Files`}</span>
            </div>
            {isFilePreviewLoading && (
              <div className="theme-text-muted mb-3 flex items-center justify-center gap-2 rounded-xl py-8 text-xs">
                <Sparkles className="h-4 w-4 animate-spin" />
                <span>Preparing file preview…</span>
              </div>
            )}
            {filePreviews.length > 0 && (
              <div className={`mb-3 grid gap-2 ${filePreviews.length > 1 ? 'grid-cols-2' : 'grid-cols-1'}`}>
                {filePreviews.map((preview) => {
                  const path = getClipFilePaths(clip)[preview.index] ?? '';
                  return (
                    <figure key={`${preview.index}-${path}`} className="theme-code-surface overflow-hidden rounded-xl border">
                      <div className="flex min-h-36 items-center justify-center p-2">
                        <img
                          src={preview.dataUrl}
                          alt={`Preview of ${path.split(/[\\/]/).pop() || 'copied image'}`}
                          className="max-h-72 w-full rounded-lg object-contain"
                        />
                      </div>
                      <figcaption className="theme-divider theme-text-muted truncate border-t px-2.5 py-1.5 font-mono text-[10px]" title={path}>
                        {path.split(/[\\/]/).pop() || path}
                      </figcaption>
                    </figure>
                  );
                })}
              </div>
            )}
            <div className="theme-surface overflow-hidden rounded-xl border">
              {getClipFilePaths(clip).map((path, index) => (
                <div key={path} className={`flex items-center gap-2 px-3 py-2.5 ${index > 0 ? 'theme-divider border-t' : ''}`}>
                  <File className="theme-text-muted h-4 w-4 shrink-0" />
                  <span className="theme-text-main min-w-0 truncate font-mono text-xs" title={path}>{path}</span>
                </div>
              ))}
            </div>
          </div>
        ) : colorData ? (
          <div className="clip-color-inspector theme-panel p-6 rounded-2xl border shadow-2xl space-y-6">
            <div className="flex items-center justify-between">
              <div className="clip-content-accent flex items-center space-x-2 font-sans font-semibold text-xs">
                <Palette className="w-4 h-4" />
                <span>Color Inspector & Swatch Card</span>
              </div>
              <span className="theme-text-subtle text-[10px] font-mono">WCAG Contrast Rated</span>
            </div>

            <div className="flex items-center space-x-6">
              <div
                className="w-24 h-24 rounded-2xl border-2 border-white/20 shadow-2xl transition-[box-shadow,transform] duration-300 relative group shrink-0"
                style={{
                  backgroundColor: colorData.hex,
                  boxShadow: `0 12px 32px ${colorData.hex}44`,
                }}
              >
                <input
                  type="color"
                  value={colorData.hex}
                  onChange={(e) => onColorChange(e.target.value)}
                  className="absolute inset-0 opacity-0 cursor-pointer w-full h-full"
                  title="Pick Color"
                />
              </div>

              <div className="flex-1 space-y-2 font-sans">
                <div className="theme-title text-xl font-bold font-mono tracking-wider">
                  {colorData.hex.toUpperCase()}
                </div>
                <div className="theme-text-muted text-xs font-mono">
                  {colorData.rgb}
                </div>
                <div className="theme-text-muted text-xs font-mono">
                  {colorData.hsl}
                </div>
              </div>
            </div>

            {/* Formats Grid */}
            <div className="grid grid-cols-2 gap-2 font-sans">
              {[
                { label: 'HEX', val: colorData.hex },
                { label: 'RGB', val: colorData.rgb },
                { label: 'HSL', val: colorData.hsl },
                { label: 'Tailwind BG', val: colorData.tailwindBg },
              ].map((fmt) => (
                <button
                  key={fmt.label}
                  onClick={() => onCopyFormat(fmt.label, fmt.val)}
                  className="clip-format-button theme-surface flex items-center justify-between px-3 py-2 rounded-xl border text-xs group"
                >
                  <div className="flex flex-col text-left truncate pr-2">
                    <span className="theme-text-muted text-[10px] uppercase font-semibold">{fmt.label}</span>
                    <span className="theme-text-main font-mono truncate text-[11px]">{fmt.val}</span>
                  </div>
                  {copiedFormat === fmt.label ? (
                    <Check className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                  ) : (
                    <Copy className="theme-text-muted w-3.5 h-3.5 group-hover:text-current shrink-0 transition-colors" />
                  )}
                </button>
              ))}
            </div>

            {/* Contrast Ratio Preview */}
            <div className="theme-divider pt-2 border-t flex items-center justify-between text-xs font-sans">
              <div
                className="px-3 py-1.5 rounded-lg font-semibold flex items-center space-x-1.5 border border-white/10"
                style={{ backgroundColor: colorData.hex, color: '#ffffff' }}
              >
                <span>White Text</span>
                <span className="text-[10px] opacity-80">({colorData.contrastWithWhite}:1)</span>
              </div>
              <div
                className="px-3 py-1.5 rounded-lg font-semibold flex items-center space-x-1.5 border border-black/10"
                style={{ backgroundColor: colorData.hex, color: '#000000' }}
              >
                <span>Black Text</span>
                <span className="text-[10px] opacity-80">({colorData.contrastWithBlack}:1)</span>
              </div>
            </div>
          </div>
        ) : clip.content_type === 'image' ? (
          <div className="space-y-4 font-sans">
            <div className="image-preview-stage theme-panel flex flex-col items-center justify-center p-6 rounded-xl border shadow-inner">
              {resolvedImageBase64 ? (
                <img
                  src={resolvedImageBase64}
                  alt="Full Preview"
                  className="max-h-96 object-contain rounded-lg shadow-2xl"
                />
              ) : (
                <div className="theme-text-muted flex items-center space-x-2 py-12">
                  <Sparkles className="clip-content-accent w-5 h-5 animate-spin" />
                  <span>Loading image preview...</span>
                </div>
              )}
            </div>

            {/* OCR result card */}
            <div className="ocr-panel theme-panel p-4 rounded-xl border space-y-3 shadow-lg">
              <div className="flex items-center justify-between">
                <div className="clip-content-accent flex items-center space-x-2 font-semibold text-xs">
                  <ScanText className="w-4 h-4" />
                  <span>OCR Text</span>
                </div>

                <div className="flex items-center space-x-1.5">
                  {clip.text_content && (
                    <button
                      onClick={() => onCopyFormat('OCR Text', clip.text_content || '')}
                      className="theme-icon-button theme-focusable p-1.5 rounded-lg border transition-colors cursor-pointer"
                      title={copiedFormat === 'OCR Text' ? UI_COPY.copied : 'Copy OCR Text'}
                    >
                      <Copy className="w-3.5 h-3.5" />
                    </button>
                  )}
                  <button
                    onClick={onRunOCR}
                    disabled={isOcrLoading || readOnly}
                    className="theme-primary-button theme-focusable p-1.5 rounded-lg border transition-colors shadow cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
                    title={readOnly ? 'Restore Before OCR' : isOcrLoading ? 'Running OCR…' : clip.text_content ? 'Run OCR Again' : 'Run OCR'}
                  >
                    <Sparkles className={`w-3.5 h-3.5 ${isOcrLoading ? 'animate-spin' : ''}`} />
                  </button>
                </div>
              </div>

              {clip.text_content ? (
                <div className="theme-code-surface overlay-scroll-region p-3.5 border rounded-xl font-mono text-xs whitespace-pre-wrap leading-relaxed select-text shadow-inner max-h-60 overflow-y-auto">
                  {clip.text_content}
                </div>
              ) : (
                <p className="theme-text-muted text-xs italic">
                  Run OCR to recognize text in this image.
                </p>
              )}
            </div>
          </div>
        ) : (
          <div className="clip-text-content theme-surface p-4 rounded-xl border leading-relaxed overflow-x-auto whitespace-pre-wrap shadow-inner">
            {displayText}
          </div>
        )}
    </>
  );
}
