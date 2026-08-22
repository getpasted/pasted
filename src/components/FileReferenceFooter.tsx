import { RefreshCw } from 'lucide-react';

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

interface FileReferenceFooterProps {
  preview?: FileClipPreview;
  isChecking: boolean;
  onRecheck: () => void;
}

export function FileReferenceFooter({ preview, isChecking, onRecheck }: FileReferenceFooterProps) {
  const label = isChecking || !preview
    ? translate('component.fileClipPreviewPanel.checkingFile')
    : availabilityLabel(preview.availability);

  return <footer className="theme-divider theme-text-muted flex min-h-11 items-center gap-1 border-t px-1.5 py-1.5 text-[11px]">
    <span className="theme-text-subtle w-12 shrink-0 pe-1 text-end text-[9px] font-semibold uppercase tracking-wide">{translate('component.fileClipPreviewPanel.status')}</span>
    <span className="min-w-0 flex-1 truncate px-2 py-1.5" role="status">{label}</span>
    <button
      type="button"
      className="theme-icon-button theme-focusable flex h-8 w-8 shrink-0 items-center justify-center rounded-md border transition-colors disabled:cursor-wait disabled:opacity-60"
      disabled={isChecking}
      onClick={onRecheck}
      aria-label={translate('component.fileClipPreviewPanel.recheck')}
      title={translate('component.fileClipPreviewPanel.recheck')}
    >
      <RefreshCw className={`h-3 w-3 ${isChecking ? 'animate-spin' : ''}`} />
    </button>
  </footer>;
}
