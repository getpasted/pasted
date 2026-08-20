import { Pin, Trash2, X } from 'lucide-react';
import { translate } from '../localization/runtime';

interface ClipBatchActionBarProps {
  selectedCount: number;
  pinningEnabled: boolean;
  trashEnabled: boolean;
  onSetPinned: (pinned: boolean) => void;
  onTrash: () => void;
  onClearSelection: () => void;
}

export function ClipBatchActionBar({
  selectedCount,
  pinningEnabled,
  trashEnabled,
  onSetPinned,
  onTrash,
  onClearSelection,
}: ClipBatchActionBarProps) {
  return (
    <div className="batch-action-bar absolute bottom-4 left-1/2 -translate-x-1/2 border rounded-2xl px-3 py-1.5 shadow-2xl flex items-center space-x-2 text-[11px] whitespace-nowrap animate-in fade-in slide-in-from-bottom-2 duration-150 max-w-[calc(100%-1.5rem)] select-none">
      <span className="batch-action-count font-bold font-mono text-[11px] px-2 py-0.5 rounded-full border whitespace-nowrap shrink-0">
        {selectedCount}
      </span>
      <div className="batch-action-divider h-3.5 w-px shrink-0" />
      {pinningEnabled && <>
        <button
          onClick={() => onSetPinned(true)}
          className="batch-action-button flex items-center space-x-1 transition-colors font-medium cursor-pointer whitespace-nowrap shrink-0"
          title={translate('app.pinSelected')}
        >
          <Pin className="pin-icon w-3.5 h-3.5 shrink-0" />
          <span>{translate('action.pin')}</span>
        </button>
        <button
          onClick={() => onSetPinned(false)}
          className="batch-action-button flex items-center space-x-1 transition-colors font-medium cursor-pointer whitespace-nowrap shrink-0"
          title={translate('app.unpinSelected')}
        >
          <Pin className="theme-text-muted w-3.5 h-3.5 opacity-60 shrink-0" />
          <span>{translate('action.unpin')}</span>
        </button>
        <div className="batch-action-divider h-3.5 w-px shrink-0" />
      </>}
      <button
        onClick={onTrash}
        className="batch-action-button is-danger flex items-center space-x-1 transition-colors font-medium cursor-pointer whitespace-nowrap shrink-0"
        title={trashEnabled ? translate('app.moveSelectedToTrash') : translate('app.deleteSelectedPermanently')}
      >
        <Trash2 className="w-3.5 h-3.5 shrink-0" />
        <span>{translate('feature.trash.label')}</span>
      </button>
      <button
        onClick={onClearSelection}
        className="batch-action-button p-0.5 rounded-full transition-colors cursor-pointer shrink-0 ms-0.5"
        title={translate('app.deselect')}
      >
        <X className="w-3.5 h-3.5 shrink-0" />
      </button>
    </div>
  );
}
