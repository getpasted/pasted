import { AlertTriangle, CheckCircle2, RefreshCw } from 'lucide-react';

import { translate } from '../localization/runtime';
import type { FileClipPreview } from './fileClipPreviewModel';

const availabilityLabel = (availability: FileClipPreview['availability']) => {
  switch (availability) {
    case 'missing': return translate('component.fileClipPreviewPanel.fileUnavailable');
    case 'inaccessible': return translate('component.fileClipPreviewPanel.fileCannotBeAccessed');
    case 'unavailable': return translate('component.fileClipPreviewPanel.fileStatusUnavailable');
    default: return translate('component.fileClipPreviewPanel.fileAvailable');
  }
};

export function FileReferenceFooter({
  preview,
  isChecking,
  onRecheck,
}: {
  preview?: FileClipPreview;
  isChecking: boolean;
  onRecheck: () => void;
}) {
  const isUnavailable = preview && preview.availability !== 'available';
  const label = isChecking || !preview
    ? translate('component.fileClipPreviewPanel.checkingFile')
    : availabilityLabel(preview.availability);

  return <footer className="theme-divider theme-text-muted flex min-h-10 items-center justify-between gap-2 border-t px-3 py-1.5 text-[11px]">
    <span className="flex min-w-0 items-center gap-1.5" role="status">
      {isChecking || !preview
        ? <RefreshCw className="h-3.5 w-3.5 shrink-0 animate-spin" />
        : isUnavailable
          ? <AlertTriangle className="h-3.5 w-3.5 shrink-0" />
          : <CheckCircle2 className="h-3.5 w-3.5 shrink-0" />}
      <span className="truncate">{label}</span>
    </span>
    <button
      type="button"
      className="theme-ghost-button theme-focusable flex h-7 shrink-0 items-center gap-1.5 rounded-md px-2 font-semibold transition-colors disabled:cursor-wait disabled:opacity-60"
      disabled={isChecking}
      onClick={onRecheck}
    >
      <RefreshCw className={`h-3 w-3 ${isChecking ? 'animate-spin' : ''}`} />
      <span>{translate('component.fileClipPreviewPanel.recheck')}</span>
    </button>
  </footer>;
}
