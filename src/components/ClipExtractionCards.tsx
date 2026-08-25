import { Check, Copy } from 'lucide-react';

import { translate } from '../localization/runtime';
import { UI_COPY } from '../utils/uiCopy';
import type { EffectiveVisualLabels, ExtractionResult } from './clipPreviewModel';
import { VisualLabelEditor } from './VisualLabelEditor';

export function ClipExtractionCards({
  results,
  copiedFormat,
  onCopyFormat,
  visualLabels,
  readOnly,
  onAddVisualLabel,
  onRemoveVisualLabel,
  onResetVisualLabels,
}: {
  results: ExtractionResult[];
  copiedFormat: string | null;
  onCopyFormat: (label: string, value: string) => void;
  visualLabels: EffectiveVisualLabels | null;
  readOnly: boolean;
  onAddVisualLabel: (label: string) => void | Promise<void>;
  onRemoveVisualLabel: (label: string) => void | Promise<void>;
  onResetVisualLabels: () => void | Promise<void>;
}) {
  const produced = results.filter((result) => result.outcome === 'produced' && (result.text || result.labels?.length) && !result.duplicateOf);
  return <>{produced.map((result) => {
    const copyLabel = `${result.extractorName} text`;
    const copyValue = result.labels?.length && visualLabels
      ? visualLabels.labels.map((label) => label.value).join('\n')
      : result.text || '';
    const labelValues = new Set(result.labels?.map((label) => label.value.toLocaleLowerCase()));
    const description = result.text?.split('\n')
      .filter((line) => !labelValues.has(line.toLocaleLowerCase()))
      .join('\n');
    return (
      <article key={result.extractorRef} className="theme-surface overflow-hidden rounded-xl border">
        <header className="theme-divider flex min-h-10 items-center justify-between gap-2 border-b px-3 py-2">
          <p className="theme-text-main min-w-0 truncate text-xs font-semibold">{result.extractorName}</p>
          <button type="button" onClick={() => onCopyFormat(copyLabel, copyValue)} className="theme-icon-button theme-focusable shrink-0 cursor-pointer rounded-lg border p-1.5 transition-colors" title={copiedFormat === copyLabel ? UI_COPY.copied : translate('component.clipPreviewContent.copyExtractedText')}>
            {copiedFormat === copyLabel ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
          </button>
        </header>
        <div className="p-3">
          {result.labels?.length && visualLabels ? (
            <div className="space-y-3">
              <VisualLabelEditor visualLabels={visualLabels} readOnly={readOnly} onAdd={onAddVisualLabel} onRemove={onRemoveVisualLabel} onReset={onResetVisualLabels} />
              {description && <div dir="auto" className="theme-code-surface whitespace-pre-wrap rounded-lg border p-3.5 text-xs leading-relaxed select-text">{description}</div>}
            </div>
          ) : (
            <div dir="auto" className="theme-code-surface overlay-scroll-region max-h-60 overflow-y-auto whitespace-pre-wrap rounded-lg border p-3.5 font-mono text-xs leading-relaxed shadow-inner select-text">{result.text}</div>
          )}
        </div>
      </article>
    );
  })}</>;
}
