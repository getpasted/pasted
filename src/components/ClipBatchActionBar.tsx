import { Eye, Pin, PinOff, RotateCcw, ShieldOff, Trash2, X } from 'lucide-react';
import { UI_COPY } from '../utils/uiCopy';
import { translate } from '../localization/runtime';
import type { ClipBatchCollectionAction } from './clipBatchActionModel';

interface ClipBatchActionBarProps {
  selectedCount: number;
  pinningEnabled: boolean;
  trashEnabled: boolean;
  trashMode: boolean;
  collectionAction?: ClipBatchCollectionAction;
  onSetPinned: (pinned: boolean) => void;
  onUnprotect: () => void;
  onReveal: () => void;
  onTrash: () => void;
  onRestore: () => void;
  onDeletePermanently: () => void;
  onClearSelection: () => void;
}

export function ClipBatchActionBar({
  selectedCount,
  pinningEnabled,
  trashEnabled,
  trashMode,
  collectionAction,
  onSetPinned,
  onUnprotect,
  onReveal,
  onTrash,
  onRestore,
  onDeletePermanently,
  onClearSelection,
}: ClipBatchActionBarProps) {
  const actionLayoutClassName = 'flex h-6 w-6 shrink-0 cursor-pointer items-center justify-center rounded-md transition-colors';
  const trashLabel = trashEnabled ? translate('app.moveSelectedToTrash') : translate('app.deleteSelectedPermanently');
  return (
    <div className="batch-action-bar absolute bottom-4 left-1/2 -translate-x-1/2 border rounded-2xl px-3 py-1.5 flex items-center space-x-2 text-[11px] whitespace-nowrap animate-in fade-in slide-in-from-bottom-2 duration-150 max-w-[calc(100%-1.5rem)] select-none">
      <span className="batch-action-count font-bold font-mono text-[11px] px-2 py-0.5 rounded-full border whitespace-nowrap shrink-0">
        {selectedCount}
      </span>
      <div className="batch-action-divider h-3.5 w-px shrink-0" />
      {collectionAction === 'unpin' && <>
        <button onClick={() => onSetPinned(false)} className={`batch-action-button ${actionLayoutClassName}`} title={translate('app.unpinSelected')} aria-label={translate('app.unpinSelected')}>
          <PinOff className="w-3.5 h-3.5 shrink-0" />
        </button>
        <div className="batch-action-divider h-3.5 w-px shrink-0" />
      </>}
      {collectionAction === 'unprotect' && <>
        <button onClick={onUnprotect} className={`batch-action-button is-accent ${actionLayoutClassName}`} title={UI_COPY.unprotect} aria-label={UI_COPY.unprotect}>
          <ShieldOff className="w-3.5 h-3.5 shrink-0" />
        </button>
        <div className="batch-action-divider h-3.5 w-px shrink-0" />
      </>}
      {collectionAction === 'reveal' && <>
        <button onClick={onReveal} className={`batch-action-button is-warning ${actionLayoutClassName}`} title={translate('component.clipCard.revealSensitiveText')} aria-label={translate('component.clipCard.revealSensitiveText')}>
          <Eye className="w-3.5 h-3.5 shrink-0" />
        </button>
        <div className="batch-action-divider h-3.5 w-px shrink-0" />
      </>}
      {collectionAction === 'restore' && <>
        <button onClick={onRestore} className={`batch-action-button is-success ${actionLayoutClassName}`} title={UI_COPY.restore} aria-label={UI_COPY.restore}>
          <RotateCcw className="w-3.5 h-3.5 shrink-0" />
        </button>
        <div className="batch-action-divider h-3.5 w-px shrink-0" />
      </>}
      {pinningEnabled && !collectionAction && <>
        <button
          onClick={() => onSetPinned(true)}
          className={`batch-action-button ${actionLayoutClassName}`}
          title={translate('app.pinSelected')}
          aria-label={translate('app.pinSelected')}
        >
          <Pin className="pin-icon w-3.5 h-3.5 shrink-0" />
        </button>
        <button
          onClick={() => onSetPinned(false)}
          className={`batch-action-button ${actionLayoutClassName}`}
          title={translate('app.unpinSelected')}
          aria-label={translate('app.unpinSelected')}
        >
          <PinOff className="theme-text-muted w-3.5 h-3.5 opacity-60 shrink-0" />
        </button>
        <div className="batch-action-divider h-3.5 w-px shrink-0" />
      </>}
      {trashMode ? <button onClick={onDeletePermanently} className={`batch-action-button is-danger ${actionLayoutClassName}`} title={translate('app.deleteSelectedPermanently')} aria-label={translate('app.deleteSelectedPermanently')}>
          <X className="w-3.5 h-3.5 shrink-0" />
        </button> : <button
          onClick={onTrash}
          className={`batch-action-button is-danger ${actionLayoutClassName}`}
          title={trashLabel}
          aria-label={trashLabel}
        >
          {trashEnabled ? <Trash2 className="w-3.5 h-3.5 shrink-0" /> : <X className="w-3.5 h-3.5 shrink-0" />}
        </button>}
      <button
        onClick={onClearSelection}
        className="batch-action-button p-0.5 rounded-full transition-colors cursor-pointer shrink-0 ms-0.5"
        title={translate('app.deselect')}
        aria-label={translate('app.deselect')}
      >
        <X className="w-3.5 h-3.5 shrink-0" />
      </button>
    </div>
  );
}
