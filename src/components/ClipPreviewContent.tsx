import React from 'react';
import { Check, Copy, Files, Palette, ScanText, Sparkles } from 'lucide-react';
import { getClipFilePaths, type ClipItem } from '../types';
import type { ColorFormats } from '../utils/color';
import { UI_COPY } from '../utils/uiCopy';
import { OverflowText } from './OverflowText';

interface ClipPreviewContentProps {
  clip: ClipItem;
  displayText: string;
  colorData: ColorFormats | null;
  resolvedImageBase64: string | null;
  filePreviews: Array<{
    index: number;
    dataUrl: string | null;
    textContent: string | null;
    width: number | null;
    height: number | null;
  }>;
  isFilePreviewLoading: boolean;
  copiedFormat: string | null;
  isOcrLoading: boolean;
  ocrEnabled: boolean;
  readOnly?: boolean;
  onColorChange: (value: string) => void;
  onCopyFormat: (label: string, value: string) => void;
  onRunOCR: () => void;
}

function useAutoHorizontalScroll(
  ref: React.RefObject<HTMLDivElement | null>,
  value: string,
  enabled: boolean,
) {
  React.useEffect(() => {
    const viewport = ref.current;
    if (!viewport || !enabled || window.matchMedia('(prefers-reduced-motion: reduce)').matches) return undefined;
    let frame = 0;
    let direction = 1;
    let lastTime = performance.now();
    let holdUntil = lastTime + 900;
    const animate = (time: number) => {
      const maximum = Math.max(0, viewport.scrollWidth - viewport.clientWidth);
      const paused = viewport.matches(':hover') || viewport.contains(document.activeElement);
      if (maximum > 3 && !paused && time >= holdUntil) {
        const elapsed = Math.min(40, time - lastTime);
        viewport.scrollLeft += direction * 58 * (elapsed / 1000);
        if (viewport.scrollLeft >= maximum - 0.5) {
          viewport.scrollLeft = maximum;
          direction = -1;
          holdUntil = time + 900;
        } else if (viewport.scrollLeft <= 0.5) {
          viewport.scrollLeft = 0;
          direction = 1;
          holdUntil = time + 900;
        }
      }
      lastTime = time;
      frame = requestAnimationFrame(animate);
    };
    frame = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(frame);
  }, [enabled, ref, value]);
}

function FileCopyField({
  label,
  value,
  copyLabel,
  autoScroll = false,
  emphasized = false,
  copiedFormat,
  onCopyFormat,
}: {
  label: string;
  value: string;
  copyLabel: string;
  autoScroll?: boolean;
  emphasized?: boolean;
  copiedFormat: string | null;
  onCopyFormat: (label: string, value: string) => void;
}) {
  const viewportRef = React.useRef<HTMLDivElement>(null);
  useAutoHorizontalScroll(viewportRef, value, autoScroll);
  const selectAll = React.useCallback(() => {
    const viewport = viewportRef.current;
    if (!viewport) return;
    const selection = window.getSelection();
    if (!selection) return;
    const range = document.createRange();
    range.selectNodeContents(viewport);
    selection.removeAllRanges();
    selection.addRange(range);
  }, []);

  const handleFieldPointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    if ((event.target as HTMLElement).closest('button')) return;
    const viewport = viewportRef.current;
    if (!viewport || document.activeElement === viewport) return;
    event.preventDefault();
    viewport.focus();
    selectAll();
  };

  return (
    <div
      className={`file-copy-field flex min-h-10 min-w-0 items-center gap-1 px-1.5 py-0.5 ${emphasized ? 'theme-subtle-surface' : ''}`}
      onPointerDown={handleFieldPointerDown}
    >
      <span className="theme-text-subtle w-9 shrink-0 pl-1 text-[9px] font-semibold uppercase tracking-wide">{label}</span>
      <div
        ref={viewportRef}
        tabIndex={0}
        className={`file-attribute-scroll theme-text-main min-w-0 flex-1 cursor-text overflow-x-auto whitespace-nowrap rounded-md px-2 py-1.5 font-mono outline-none select-text ${emphasized ? 'text-xs' : 'text-[11px]'}`}
        title={value}
        onFocus={() => selectAll()}
      >
        {value}
      </div>
      <button
        type="button"
        onClick={() => onCopyFormat(copyLabel, value)}
        className="theme-icon-button theme-focusable flex h-8 w-8 shrink-0 items-center justify-center rounded-md border transition-colors"
        title={copiedFormat === copyLabel ? UI_COPY.copied : `Copy ${label}`}
      >
        {copiedFormat === copyLabel ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
      </button>
    </div>
  );
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
  ocrEnabled,
  readOnly = false,
  onColorChange,
  onCopyFormat,
  onRunOCR,
}: ClipPreviewContentProps) {
  const filePaths = getClipFilePaths(clip);
  const [imageLoadingIndicatorClipId, setImageLoadingIndicatorClipId] = React.useState<number | null>(null);
  React.useEffect(() => {
    if (clip.content_type !== 'image' || resolvedImageBase64) return undefined;
    const timer = window.setTimeout(() => setImageLoadingIndicatorClipId(clip.id), 120);
    return () => window.clearTimeout(timer);
  }, [clip.content_type, clip.id, resolvedImageBase64]);
  const showImageLoadingIndicator = clip.content_type === 'image'
    && !resolvedImageBase64
    && imageLoadingIndicatorClipId === clip.id;

  return (
    <>
        {clip.content_type === 'file' ? (
          <div className="theme-panel rounded-2xl border p-4 shadow-lg">
            <div className="theme-title mb-3 flex items-center gap-2 text-xs font-semibold">
              <Files className="theme-status-info-text h-4 w-4" />
              <span>{filePaths.length === 1 ? 'Copied File' : `${filePaths.length} Copied Files`}</span>
            </div>
            {isFilePreviewLoading && (
              <div className="theme-text-muted mb-3 flex items-center justify-center gap-2 rounded-xl py-8 text-xs">
                <Sparkles className="h-4 w-4 animate-spin" />
                <span>Preparing file preview…</span>
              </div>
            )}
            {filePaths.length > 0 && (
              <div className={`grid gap-2 ${filePaths.length > 1 ? 'grid-cols-2' : 'grid-cols-1'}`}>
                {filePaths.map((path, index) => {
                  const preview = filePreviews.find((item) => item.index === index);
                  const filename = path.split(/[\\/]/).pop() || path;
                  return (
                    <article key={`${index}-${path}`} className="theme-surface min-w-0 overflow-hidden rounded-xl border">
                      {preview && (
                        <div className="theme-code-surface flex min-h-36 items-center justify-center border-b p-2">
                          {preview.dataUrl ? (
                            <img
                              src={preview.dataUrl}
                              alt={`Preview of ${filename || 'copied file'}`}
                              className="max-h-72 w-full rounded-lg object-contain"
                            />
                          ) : (
                            <pre className="theme-text-main overlay-scroll-region max-h-72 w-full overflow-auto whitespace-pre-wrap break-words p-2 font-mono text-xs">
                              {preview.textContent}
                            </pre>
                          )}
                        </div>
                      )}
                      <div>
                        <FileCopyField
                          label="Name"
                          value={filename}
                          copyLabel={`File Name ${index + 1}`}
                          autoScroll
                          copiedFormat={copiedFormat}
                          onCopyFormat={onCopyFormat}
                        />
                        <div className="theme-divider border-t">
                          <FileCopyField
                            label="Path"
                            value={path}
                            copyLabel={`File Path ${index + 1}`}
                            autoScroll
                            emphasized
                            copiedFormat={copiedFormat}
                            onCopyFormat={onCopyFormat}
                          />
                        </div>
                      </div>
                    </article>
                  );
                })}
              </div>
            )}
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
                className="theme-divider w-24 h-24 rounded-2xl border-2 shadow-2xl transition-[box-shadow,transform] duration-300 relative group shrink-0"
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
                    <OverflowText text={fmt.val} className="theme-text-main font-mono truncate text-[11px]" />
                  </div>
                  {copiedFormat === fmt.label ? (
                    <Check className="theme-status-success-text w-3.5 h-3.5 shrink-0" />
                  ) : (
                    <Copy className="theme-text-muted w-3.5 h-3.5 group-hover:text-current shrink-0 transition-colors" />
                  )}
                </button>
              ))}
            </div>

            {/* Contrast Ratio Preview */}
            <div className="theme-divider pt-2 border-t flex items-center justify-between text-xs font-sans">
              <div
                className="color-contrast-sample px-3 py-1.5 rounded-lg font-semibold flex items-center space-x-1.5 border"
                style={{ backgroundColor: colorData.hex, color: '#ffffff' }}
              >
                <span>White Text</span>
                <span className="text-[10px] opacity-80">({colorData.contrastWithWhite}:1)</span>
              </div>
              <div
                className="color-contrast-sample px-3 py-1.5 rounded-lg font-semibold flex items-center space-x-1.5 border"
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
                  decoding="async"
                  className="max-h-96 object-contain rounded-lg shadow-2xl"
                />
              ) : (
                <div className="theme-text-muted flex items-center space-x-2 py-12">
                  {showImageLoadingIndicator && (
                    <>
                      <Sparkles className="clip-content-accent w-5 h-5 animate-spin" />
                      <span>Loading image preview...</span>
                    </>
                  )}
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
                  {ocrEnabled && <button
                    onClick={onRunOCR}
                    disabled={isOcrLoading || readOnly}
                    className="theme-primary-button theme-focusable p-1.5 rounded-lg border transition-colors shadow cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
                    title={readOnly ? 'Restore Before OCR' : isOcrLoading ? 'Running OCR…' : clip.text_content ? 'Run OCR Again' : 'Run OCR'}
                  >
                    <Sparkles className={`w-3.5 h-3.5 ${isOcrLoading ? 'animate-spin' : ''}`} />
                  </button>}
                </div>
              </div>

              {clip.text_content ? (
                <div className="theme-code-surface overlay-scroll-region p-3.5 border rounded-xl font-mono text-xs whitespace-pre-wrap leading-relaxed select-text shadow-inner max-h-60 overflow-y-auto">
                  {clip.text_content}
                </div>
              ) : (
                <p className="theme-text-muted text-xs italic">
                  {ocrEnabled ? 'Run OCR to recognize text in this image.' : 'OCR is disabled in Settings → Functionality.'}
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
