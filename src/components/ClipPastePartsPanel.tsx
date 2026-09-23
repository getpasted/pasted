import { Check, Copy } from 'lucide-react';

import { translate } from '../localization/runtime';
import { contentTypeLabel } from '../utils/contentTypes';
import type { PastePartsAnalysis } from './pastePartModel';

function valueAtOffsets(source: string, start: number, end: number): string {
  return Array.from(source).slice(start, end).join('');
}

export function ClipPastePartsPanel({
  analysis,
  source,
  concealed,
  copiedPart,
  onCopy,
}: {
  analysis: PastePartsAnalysis;
  source: string;
  concealed: boolean;
  copiedPart: string | null;
  onCopy: (label: string, value: string) => void;
}) {
  if (analysis.parts.length === 0) return null;
  return <section className="theme-panel overflow-hidden rounded-xl border">
    <header className="theme-divider min-h-12 border-b px-4 py-3">
      <h3 className="theme-text-main text-xs font-semibold">
        {translate('component.clipPastePartsPanel.detectedContent')}
      </h3>
    </header>
    <div className="theme-divide divide-y px-4">
      {analysis.parts.map((part, index) => {
        const value = valueAtOffsets(source, part.startOffset, part.endOffset);
        const label = part.role || contentTypeLabel(part.kind);
        const copyKey = `Paste Part ${index}`;
        return <div key={`${part.startOffset}:${part.endOffset}:${part.kind}`} className="flex items-center gap-3 py-3">
          <div className="min-w-0 flex-1">
            <p className="theme-text-muted text-[10px] font-semibold uppercase tracking-wide">{label}</p>
            <p dir="auto" className="theme-text-main truncate text-xs">
              {concealed ? translate('component.clipPastePartsPanel.concealedValue') : value}
            </p>
          </div>
          <button
            type="button"
            disabled={concealed}
            onClick={() => onCopy(copyKey, value)}
            className="theme-icon-button theme-focusable ui-control-radius grid h-8 w-8 shrink-0 place-items-center border transition-colors disabled:cursor-not-allowed disabled:opacity-40"
            aria-label={translate('component.clipPreviewContent.copyLabel', { label })}
            title={translate('component.clipPreviewContent.copyLabel', { label })}
          >
            {copiedPart === copyKey ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
          </button>
        </div>;
      })}
    </div>
  </section>;
}
