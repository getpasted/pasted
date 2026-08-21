import React from 'react';
import { AlertTriangle, Check, ChevronDown, Copy, EyeOff, Files, Palette, Sparkles } from 'lucide-react';
import { getClipFilePaths, type ClipItem } from '../types';
import type { ColorFormats } from '../utils/color';
import { UI_COPY } from '../utils/uiCopy';
import { OverflowText } from './OverflowText';
import { SafeRasterImage } from './SafeRasterImage';
import { translate } from '../localization/runtime';
import { dateTimeAttribute, formatFullDateTime, formatRelativeTime } from '../utils/date';

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
  fileSearchableText: {
    extractorName: string;
    searchableText: string;
  } | null;
  extractionResults: ExtractionResult[];
  extractionHistory: ExtractionAttempt[];
  extractionHistoryHasMore: boolean;
  isExtractionHistoryLoading: boolean;
  isFileExtractionLoading: boolean;
  copiedFormat: string | null;
  isOcrLoading: boolean;
  ocrEnabled: boolean;
  transcriptionsEnabled: boolean;
  concealed?: boolean;
  concealedMask?: string;
  readOnly?: boolean;
  onColorChange: (value: string) => void;
  onCopyFormat: (label: string, value: string) => void;
  onRunOCR: () => void;
  onRunFileExtraction: () => void;
  onLoadExtractionHistory: (reset: boolean) => void;
}

export interface ExtractionAttempt extends ExtractionResult {
  runId: string;
  runAt: string;
}

export interface ExtractionResult {
  extractorRef: string;
  extractorName: string;
  engine: string;
  priority: number;
  duplicateOf?: string;
  outcome: 'produced' | 'no_output' | 'failed';
  text?: string;
  failure?: { code: string; message: string };
  updatedAt: string;
}

function ExtractionCards({
  results,
  copiedFormat,
  onCopyFormat,
}: {
  results: ExtractionResult[];
  copiedFormat: string | null;
  onCopyFormat: (label: string, value: string) => void;
}) {
  const produced = results.filter((result) => result.outcome === 'produced' && result.text && !result.duplicateOf);

  return (
    <>
      {produced.map((result) => {
        const copyLabel = `${result.extractorName} text`;
        return (
          <article key={result.extractorRef} className="theme-surface overflow-hidden rounded-xl border">
            <header className="theme-divider flex min-h-10 items-center justify-between gap-2 border-b px-3 py-2">
              <p className="theme-text-main min-w-0 truncate text-xs font-semibold">{result.extractorName}</p>
              <button
                type="button"
                onClick={() => onCopyFormat(copyLabel, result.text || '')}
                className="theme-icon-button theme-focusable shrink-0 cursor-pointer rounded-lg border p-1.5 transition-colors"
                title={copiedFormat === copyLabel ? UI_COPY.copied : translate('component.clipPreviewContent.copyExtractedText')}
              >
                {copiedFormat === copyLabel ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
              </button>
            </header>
            <div className="p-3">
              <div dir="auto" className="theme-code-surface overlay-scroll-region max-h-60 overflow-y-auto whitespace-pre-wrap rounded-lg border p-3.5 font-mono text-xs leading-relaxed shadow-inner select-text">
                {result.text}
              </div>
            </div>
          </article>
        );
      })}
    </>
  );
}

function ExtractionActivity({
  history,
  hasMore,
  loading,
  onLoad,
}: {
  history: ExtractionAttempt[];
  hasMore: boolean;
  loading: boolean;
  onLoad: (reset: boolean) => void;
}) {
  const [expanded, setExpanded] = React.useState(false);
  const runs = history.reduce<Array<{ runId: string; runAt: string; attempts: ExtractionAttempt[] }>>((groups, attempt) => {
    const current = groups[groups.length - 1];
    if (current?.runId === attempt.runId) current.attempts.push(attempt);
    else groups.push({ runId: attempt.runId, runAt: attempt.runAt, attempts: [attempt] });
    return groups;
  }, []);

  const toggle = () => {
    const next = !expanded;
    setExpanded(next);
    if (next && history.length === 0) onLoad(true);
  };

  return (
    <footer className="theme-divider border-t text-xs">
      <div className="flex min-h-12 items-center justify-end p-2">
        <button
          type="button"
          className="theme-secondary-button theme-focusable flex items-center gap-1.5 rounded-lg border px-2.5 py-1.5 font-semibold transition-colors"
          onClick={toggle}
          aria-expanded={expanded}
        >
          <span>{translate('component.clipPreviewContent.details')}</span>
          <ChevronDown className={`h-3.5 w-3.5 transition-transform ${expanded ? 'rotate-180' : ''}`} />
        </button>
      </div>
      {expanded && (
        <div className="theme-text-muted theme-divider space-y-3 border-t px-4 py-3">
          {runs.map((run) => (
            <section key={run.runId} className="space-y-1.5">
              <time className="theme-text-subtle text-[10px]" dateTime={dateTimeAttribute(run.runAt)} title={formatFullDateTime(run.runAt)}>{formatRelativeTime(run.runAt)}</time>
              {run.attempts.map((result) => {
                const duplicateName = run.attempts.find((candidate) => candidate.extractorRef === result.duplicateOf)?.extractorName;
                return (
                  <div key={`${run.runId}:${result.extractorRef}`} className="flex items-start gap-2 py-1">
                    {result.outcome === 'failed' && <AlertTriangle className="theme-status-warning-text mt-0.5 h-3.5 w-3.5 shrink-0" />}
                    <div className="min-w-0 flex-1">
                      <div className="flex flex-wrap items-baseline justify-between gap-x-3 gap-y-0.5">
                        <p className="theme-text-main font-medium">{result.extractorName}</p>
                        <dl className="theme-text-subtle flex items-center gap-2 text-[10px]">
                          <div className="flex gap-1"><dt>{translate('common.priority')}</dt><dd>{result.priority}</dd></div>
                          <div className="flex gap-1"><dt>{translate('component.clipPreviewContent.engine')}</dt><dd dir="ltr">{result.engine}</dd></div>
                        </dl>
                      </div>
                      <p>{result.duplicateOf
                        ? duplicateName
                          ? translate('component.clipPreviewContent.sameTextAsName', { name: duplicateName })
                          : translate('component.clipPreviewContent.sameTextAsEarlierExtractor')
                        : result.outcome === 'failed'
                          ? result.failure?.message || translate('component.clipPreviewContent.extractorFailed')
                          : result.outcome === 'produced'
                            ? translate('component.clipPreviewContent.extractedTextSuccessfully')
                            : translate('component.clipPreviewContent.noTextFound')}</p>
                    </div>
                  </div>
                );
              })}
            </section>
          ))}
          {loading && <p>{translate('component.clipPreviewContent.loadingDetails')}</p>}
          {!loading && runs.length === 0 && <p>{translate('component.clipPreviewContent.noScanHistory')}</p>}
          {hasMore && !loading && (
            <button type="button" className="theme-text-main theme-focusable rounded-md" onClick={() => onLoad(false)}>
              {translate('component.clipPreviewContent.loadOlder')}
            </button>
          )}
        </div>
      )}
    </footer>
  );
}

function getOcrExtractorLabel(clip: ClipItem): string | null {
  if (clip.ocr_extractor_name) return clip.ocr_extractor_name;
  switch (clip.ocr_engine_version) {
    case 'macos-vision-v1': return 'Apple Vision OCR';
    case 'tesseract-cli-v1': return 'Tesseract OCR';
    case 'legacy': return 'Legacy OCR';
    default: return clip.ocr_engine_version || null;
  }
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
      <span className="theme-text-subtle w-9 shrink-0 ps-1 text-[9px] font-semibold uppercase tracking-wide">{label}</span>
      <div
        ref={viewportRef}
        dir="ltr"
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
        title={copiedFormat === copyLabel ? UI_COPY.copied : translate('component.clipPreviewContent.copyLabel', { label: label })}
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
  fileSearchableText,
  extractionResults,
  extractionHistory,
  extractionHistoryHasMore,
  isExtractionHistoryLoading,
  isFileExtractionLoading,
  copiedFormat,
  isOcrLoading,
  ocrEnabled,
  transcriptionsEnabled,
  concealed = false,
  concealedMask = '•••• ••••',
  readOnly = false,
  onColorChange,
  onCopyFormat,
  onRunOCR,
  onRunFileExtraction,
  onLoadExtractionHistory,
}: ClipPreviewContentProps) {
  const ocrExtractorLabel = getOcrExtractorLabel(clip);
  const filePaths = getClipFilePaths(clip);
  const hasProducedExtraction = extractionResults.some(
    (result) => result.outcome === 'produced' && result.text && !result.duplicateOf,
  );
  const [imageLoadingIndicatorClipId, setImageLoadingIndicatorClipId] = React.useState<number | null>(null);
  React.useEffect(() => {
    if (clip.content_type !== 'image' || resolvedImageBase64) return undefined;
    const timer = window.setTimeout(() => setImageLoadingIndicatorClipId(clip.id), 120);
    return () => window.clearTimeout(timer);
  }, [clip.content_type, clip.id, resolvedImageBase64]);
  const showImageLoadingIndicator = clip.content_type === 'image'
    && !resolvedImageBase64
    && imageLoadingIndicatorClipId === clip.id;

  if (concealed) {
    return (
      <div
        className="theme-status-warning flex min-h-40 items-center justify-center gap-3 rounded-2xl border p-6 font-mono shadow-inner select-none"
        role="status"
        aria-label={translate('collection.concealed')}
      >
        <EyeOff className="h-5 w-5 shrink-0" aria-hidden="true" />
        <span className="text-sm font-bold tracking-widest">{concealedMask}</span>
      </div>
    );
  }

  return (
    <>
        {clip.content_type === 'file' ? (
          <div className="space-y-4">
            <div className="theme-panel rounded-2xl border p-4 shadow-lg">
              <div className="theme-title mb-3 flex items-center gap-2 text-xs font-semibold">
                <Files className="theme-status-info-text h-4 w-4" />
                <span>{filePaths.length === 1 ? translate('component.clipPreviewContent.copiedFile') : translate('component.clipPreviewContent.lengthCopiedFiles', { length: filePaths.length })}</span>
              </div>
              {isFilePreviewLoading && (
                <div className="theme-text-muted mb-3 flex items-center justify-center gap-2 rounded-xl py-8 text-xs">
                  <Sparkles className="h-4 w-4 animate-spin" />
                  <span>{translate('component.clipPreviewContent.preparingFilePreview')}</span>
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
                              <SafeRasterImage
                                source={preview.dataUrl}
                                alt={translate('common.previewOfName', { name: filename || translate('component.clipPreviewContent.copiedFileLowercase') })}
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
                            label={translate('common.name')}
                            value={filename}
                            copyLabel={translate('component.clipPreviewContent.fileNameValue', { value: index + 1 })}
                            autoScroll
                            copiedFormat={copiedFormat}
                            onCopyFormat={onCopyFormat}
                          />
                          <div className="theme-divider border-t">
                            <FileCopyField
                              label={translate('component.clipPreviewContent.path')}
                              value={path}
                              copyLabel={translate('component.clipPreviewContent.filePathValue', { value: index + 1 })}
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

            {transcriptionsEnabled && <section className="theme-panel overflow-hidden rounded-xl border shadow-lg">
              <header className="theme-divider flex min-h-12 items-center justify-between gap-3 border-b px-4 py-2">
                <h3 className="theme-text-main text-xs font-semibold">{translate('component.clipPreviewContent.extractedText')}</h3>
                <div className="flex items-center space-x-1.5">
                  {fileSearchableText && (
                    <button
                      onClick={() => onCopyFormat('Extracted Text', fileSearchableText.searchableText)}
                      className="theme-icon-button theme-focusable cursor-pointer rounded-lg border p-1.5 transition-colors"
                      aria-label={copiedFormat === 'Extracted Text' ? UI_COPY.copied : translate('component.clipPreviewContent.copyExtractedText')}
                      title={copiedFormat === 'Extracted Text' ? UI_COPY.copied : translate('component.clipPreviewContent.copyExtractedText')}
                    >
                      <Copy className="h-3.5 w-3.5" />
                    </button>
                  )}
                  <button
                    onClick={onRunFileExtraction}
                    disabled={isFileExtractionLoading || readOnly}
                    className="theme-primary-button theme-focusable cursor-pointer rounded-lg border p-1.5 shadow transition-colors disabled:cursor-not-allowed disabled:opacity-40"
                    aria-label={readOnly ? translate('component.clipPreviewContent.restoreBeforeExtracting') : isFileExtractionLoading ? translate('component.clipPreviewContent.extractingText') : fileSearchableText || extractionResults.length > 0 ? translate('component.clipPreviewContent.extractAgain') : translate('component.clipPreviewContent.extractText')}
                    title={readOnly ? translate('component.clipPreviewContent.restoreBeforeExtracting') : isFileExtractionLoading ? translate('component.clipPreviewContent.extractingText') : fileSearchableText || extractionResults.length > 0 ? translate('component.clipPreviewContent.extractAgain') : translate('component.clipPreviewContent.extractText')}
                  >
                    <Sparkles className={`h-3.5 w-3.5 ${isFileExtractionLoading ? 'animate-spin' : ''}`} />
                  </button>
                </div>
              </header>
              {(hasProducedExtraction || fileSearchableText || extractionResults.length === 0) && <div className="space-y-3 p-4">
                {hasProducedExtraction ? (
                  <ExtractionCards results={extractionResults} copiedFormat={copiedFormat} onCopyFormat={onCopyFormat} />
                ) : fileSearchableText ? <>
                  <p className="theme-text-muted text-xs">{translate('component.clipPreviewContent.extractedByName', { name: fileSearchableText.extractorName })}</p>
                  <div dir="auto" className="theme-code-surface overlay-scroll-region max-h-60 overflow-y-auto whitespace-pre-wrap rounded-xl border p-3.5 font-mono text-xs leading-relaxed shadow-inner select-text">
                    {fileSearchableText.searchableText}
                  </div>
                </> : (
                  <p className="theme-text-muted text-xs italic">{translate('component.clipPreviewContent.runAnAvailableFileTextExtractorToCreateSearchableText')}</p>
                )}
              </div>}
              <ExtractionActivity key={clip.id} history={extractionHistory} hasMore={extractionHistoryHasMore} loading={isExtractionHistoryLoading} onLoad={onLoadExtractionHistory} />
            </section>}
          </div>
        ) : colorData ? (
          <div className="clip-color-inspector theme-panel p-6 rounded-2xl border shadow-2xl space-y-6">
            <div className="flex items-center justify-between">
              <div className="clip-content-accent flex items-center space-x-2 font-sans font-semibold text-xs">
                <Palette className="w-4 h-4" />
                <span>{translate('component.clipPreviewContent.colorInspectorAndSwatchCard')}</span>
              </div>
              <span className="theme-text-subtle text-[10px] font-mono">{translate('component.clipPreviewContent.wcagContrastRated')}</span>
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
                  title={translate('component.clipPreviewContent.pickColor')}
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
                { get label() { return translate('component.clipPreviewContent.tailwindBg'); }, val: colorData.tailwindBg },
              ].map((fmt) => (
                <button
                  key={fmt.label}
                  onClick={() => onCopyFormat(fmt.label, fmt.val)}
                  className="clip-format-button theme-surface flex items-center justify-between px-3 py-2 rounded-xl border text-xs group"
                >
                  <div className="flex flex-col text-start truncate pe-2">
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
                <span>{translate('component.clipPreviewContent.whiteText')}</span>
                <span className="text-[10px] opacity-80">({colorData.contrastWithWhite}:1)</span>
              </div>
              <div
                className="color-contrast-sample px-3 py-1.5 rounded-lg font-semibold flex items-center space-x-1.5 border"
                style={{ backgroundColor: colorData.hex, color: '#000000' }}
              >
                <span>{translate('component.clipPreviewContent.blackText')}</span>
                <span className="text-[10px] opacity-80">({colorData.contrastWithBlack}:1)</span>
              </div>
            </div>
          </div>
        ) : clip.content_type === 'image' ? (
          <div className="space-y-4 font-sans">
            <div className="image-preview-stage theme-panel flex flex-col items-center justify-center p-6 rounded-xl border shadow-inner">
              {resolvedImageBase64 ? (
                <SafeRasterImage
                  source={resolvedImageBase64}
                  alt={translate('component.clipPreviewContent.fullPreview')}
                  decoding="async"
                  className="max-h-96 object-contain rounded-lg shadow-2xl"
                />
              ) : (
                <div className="theme-text-muted flex items-center space-x-2 py-12">
                  {showImageLoadingIndicator && (
                    <>
                      <Sparkles className="clip-content-accent w-5 h-5 animate-spin" />
                      <span>{translate('component.clipPreviewContent.loadingImagePreview')}</span>
                    </>
                  )}
                </div>
              )}
            </div>

            <section className="ocr-panel theme-panel overflow-hidden rounded-xl border shadow-lg">
              <header className="theme-divider flex min-h-12 items-center justify-between gap-3 border-b px-4 py-2">
                <h3 className="theme-text-main text-xs font-semibold">{translate('component.clipPreviewContent.extractedText')}</h3>
                <div className="flex items-center space-x-1.5">
                  {clip.text_content && (
                    <button
                      onClick={() => onCopyFormat('Extracted Text', clip.text_content || '')}
                      className="theme-icon-button theme-focusable p-1.5 rounded-lg border transition-colors cursor-pointer"
                      title={copiedFormat === 'Extracted Text' ? UI_COPY.copied : translate('component.clipPreviewContent.copyExtractedText')}
                    >
                      <Copy className="w-3.5 h-3.5" />
                    </button>
                  )}
                  {ocrEnabled && <button
                    onClick={onRunOCR}
                    disabled={isOcrLoading || readOnly}
                    className="theme-primary-button theme-focusable p-1.5 rounded-lg border transition-colors shadow cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
                    title={readOnly ? translate('component.clipPreviewContent.restoreBeforeExtracting') : isOcrLoading ? translate('component.clipPreviewContent.extractingText') : clip.text_content || extractionResults.length > 0 ? translate('component.clipPreviewContent.extractAgain') : translate('component.clipPreviewContent.extractText')}
                  >
                    <Sparkles className={`w-3.5 h-3.5 ${isOcrLoading ? 'animate-spin' : ''}`} />
                  </button>}
                </div>
              </header>
              {(hasProducedExtraction || clip.text_content || extractionResults.length === 0) && <div className="space-y-3 p-4">
                {!hasProducedExtraction && clip.text_content && ocrExtractorLabel && (
                  <p className="theme-text-muted text-xs">{translate('component.clipPreviewContent.extractedByName', { name: ocrExtractorLabel })}</p>
                )}
                {hasProducedExtraction ? (
                  <ExtractionCards results={extractionResults} copiedFormat={copiedFormat} onCopyFormat={onCopyFormat} />
                ) : clip.text_content ? (
                  <div dir="auto" className="theme-code-surface overlay-scroll-region max-h-60 overflow-y-auto whitespace-pre-wrap rounded-xl border p-3.5 font-mono text-xs leading-relaxed shadow-inner select-text">
                    {clip.text_content}
                  </div>
                ) : (
                  <p className="theme-text-muted text-xs italic">
                    {ocrEnabled ? translate('component.clipPreviewContent.runOcrToRecognizeTextInThisImage') : translate('component.clipPreviewContent.ocrIsDisabledInSettingsFunctionality')}
                  </p>
                )}
              </div>}
              <ExtractionActivity key={clip.id} history={extractionHistory} hasMore={extractionHistoryHasMore} loading={isExtractionHistoryLoading} onLoad={onLoadExtractionHistory} />
            </section>
          </div>
        ) : (
          <div dir="auto" className="clip-text-content theme-surface p-4 rounded-xl border leading-relaxed overflow-x-auto whitespace-pre-wrap shadow-inner">
            {displayText}
          </div>
        )}
    </>
  );
}
