import React from 'react';
import { Check, Copy, Files } from 'lucide-react';

import { translate } from '../localization/runtime';
import type { FileClipPreview } from './fileClipPreviewModel';
import { FileReferenceFooter } from './FileReferenceFooter';
import { SafeRasterImage } from './SafeRasterImage';
import { UI_COPY } from '../utils/uiCopy';

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
  emphasized = false,
  copiedFormat,
  onCopyFormat,
}: {
  label: string;
  value: string;
  copyLabel: string;
  emphasized?: boolean;
  copiedFormat: string | null;
  onCopyFormat: (label: string, value: string) => void;
}) {
  const viewportRef = React.useRef<HTMLDivElement>(null);
  useAutoHorizontalScroll(viewportRef, value, true);
  const selectAll = React.useCallback(() => {
    const viewport = viewportRef.current;
    const selection = window.getSelection();
    if (!viewport || !selection) return;
    const range = document.createRange();
    range.selectNodeContents(viewport);
    selection.removeAllRanges();
    selection.addRange(range);
  }, []);
  const handlePointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    if ((event.target as HTMLElement).closest('button')) return;
    const viewport = viewportRef.current;
    if (!viewport || document.activeElement === viewport) return;
    event.preventDefault();
    viewport.focus();
    selectAll();
  };

  return <div
    className={`file-copy-field flex min-h-11 min-w-0 items-center gap-1 px-1.5 py-1.5 ${emphasized ? 'theme-subtle-surface' : ''}`}
    onPointerDown={handlePointerDown}
  >
    <span className="theme-text-subtle w-12 shrink-0 pe-1 text-end text-[9px] font-semibold uppercase tracking-wide">{label}</span>
    <div
      ref={viewportRef}
      dir="ltr"
      tabIndex={0}
      className={`file-attribute-scroll theme-text-main min-w-0 flex-1 cursor-text overflow-x-auto whitespace-nowrap rounded-md px-2 py-1.5 font-mono outline-none select-text ${emphasized ? 'text-xs' : 'text-[11px]'}`}
      title={value}
      onFocus={selectAll}
    >
      {value}
    </div>
    <button
      type="button"
      onClick={() => onCopyFormat(copyLabel, value)}
      className="theme-icon-button theme-focusable ui-control-radius flex h-8 w-8 shrink-0 items-center justify-center border transition-colors"
      title={copiedFormat === copyLabel ? UI_COPY.copied : translate('component.clipPreviewContent.copyLabel', { label })}
    >
      {copiedFormat === copyLabel ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
    </button>
  </div>;
}

export function FileClipPreviewPanel({
  paths,
  previews,
  isLoading,
  copiedFormat,
  onCopyFormat,
  onRecheck,
}: {
  paths: string[];
  previews: FileClipPreview[];
  isLoading: boolean;
  copiedFormat: string | null;
  onCopyFormat: (label: string, value: string) => void;
  onRecheck: (index: number) => Promise<void>;
}) {
  const [recheckingIndexes, setRecheckingIndexes] = React.useState<ReadonlySet<number>>(new Set());
  const recheck = async (index: number) => {
    setRecheckingIndexes((current) => new Set(current).add(index));
    try {
      await onRecheck(index);
    } finally {
      setRecheckingIndexes((current) => {
        const next = new Set(current);
        next.delete(index);
        return next;
      });
    }
  };
  return <div className="theme-panel rounded-2xl border px-4 pb-4 pt-3">
    <div className="theme-title mb-2 flex min-h-8 items-center gap-2 text-xs font-semibold">
      <div className="flex min-w-0 items-center gap-2">
        <Files className="theme-status-info-text h-4 w-4 shrink-0" />
        <span>{paths.length === 1
          ? translate('component.clipPreviewContent.copiedFile')
          : translate('component.clipPreviewContent.lengthCopiedFiles', { length: paths.length })}</span>
      </div>
    </div>
    {paths.length > 0 && <div className={`grid gap-2 ${paths.length > 1 ? 'grid-cols-2' : 'grid-cols-1'}`}>
      {paths.map((path, index) => {
        const preview = previews.find((item) => item.index === index);
        const filename = path.split(/[\\/]/).pop() || path;
        return <article key={`${index}-${path}`} className="theme-surface min-w-0 overflow-hidden rounded-xl border">
          {(preview?.dataUrl || preview?.textContent) && <div className="theme-code-surface flex min-h-36 items-center justify-center border-b p-2">
            {preview.dataUrl ? <SafeRasterImage
              source={preview.dataUrl}
              alt={translate('common.previewOfName', { name: filename || translate('component.clipPreviewContent.copiedFileLowercase') })}
              className="max-h-72 w-full rounded-lg object-contain"
            /> : <pre className="theme-text-main overlay-scroll-region max-h-72 w-full overflow-auto whitespace-pre-wrap break-words p-2 font-mono text-xs">
              {preview.textContent}
            </pre>}
          </div>}
          <div>
            <FileCopyField
              label={translate('common.name')}
              value={filename}
              copyLabel={translate('component.clipPreviewContent.fileNameValue', { value: index + 1 })}
              copiedFormat={copiedFormat}
              onCopyFormat={onCopyFormat}
            />
            <div className="theme-divider border-t">
              <FileCopyField
                label={translate('component.clipPreviewContent.path')}
                value={path}
                copyLabel={translate('component.clipPreviewContent.filePathValue', { value: index + 1 })}
                emphasized
                copiedFormat={copiedFormat}
                onCopyFormat={onCopyFormat}
              />
            </div>
          </div>
          <FileReferenceFooter
            preview={preview}
            isChecking={(isLoading && !preview) || recheckingIndexes.has(index)}
            onRecheck={() => void recheck(index)}
          />
        </article>;
      })}
    </div>}
  </div>;
}
