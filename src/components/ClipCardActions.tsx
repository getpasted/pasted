import { ArrowRightCircle, Check, Copy, Eye, EyeOff, FilePenLine, MinusCircle, Pin, RotateCcw, Shield, ShieldOff, Trash, Trash2, X } from 'lucide-react';
import { useState, type MouseEvent } from 'react';

import type { useFeatures } from '../hooks/useFeatures';
import { translate } from '../localization/runtime';
import type { ClipItem } from '../types';
import type { ClipViewPolicy } from '../utils/clipViewPolicy';
import { clipDeleteLabel, UI_COPY } from '../utils/uiCopy';
import { FloatingActionStrip } from './FloatingActionStrip';

type ClipCardFeatures = ReturnType<typeof useFeatures>;

export function ClipCardActions({
  clip,
  concealmentEffective,
  features,
  isDragInProgress,
  isQueueMode,
  isTrashMode,
  onCopy,
  onDelete,
  onName,
  onPasteQueueItem,
  onPin,
  onPurgePermanently,
  onRemoveFromQueue,
  onRestore,
  onToggleConcealed,
  onToggleProtected,
  protectedByBin,
  protectionToggleDisabled,
  queueIndex,
  showActions,
  trashEnabled,
  viewPolicy,
}: {
  clip: ClipItem;
  concealmentEffective: boolean;
  features: ClipCardFeatures;
  isDragInProgress: boolean;
  isQueueMode: boolean;
  isTrashMode: boolean;
  onCopy: () => void;
  onDelete: (event?: MouseEvent) => void;
  onName?: () => void;
  onPasteQueueItem?: () => void;
  onPin: () => void;
  onPurgePermanently?: () => void;
  onRemoveFromQueue?: () => void;
  onRestore?: () => void;
  onToggleConcealed?: () => void;
  onToggleProtected?: () => void;
  protectedByBin: boolean;
  protectionToggleDisabled: boolean;
  queueIndex?: number;
  showActions: boolean;
  trashEnabled: boolean;
  viewPolicy: ClipViewPolicy;
}) {
  const [copied, setCopied] = useState(false);
  const handleCopy = (event: MouseEvent) => {
    event.stopPropagation();
    onCopy();
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return <FloatingActionStrip
    label={translate('component.clipCard.clipActions')}
    visible={showActions && !isDragInProgress}
  >
    <button
      onClick={handleCopy}
      className="floating-action-button"
      title={copied ? UI_COPY.copied : UI_COPY.copy}
    >
      {copied
        ? <Check className="w-3.5 h-3.5 theme-status-success-text" />
        : <Copy className="w-3.5 h-3.5" />}
    </button>
    {features.concealment && viewPolicy.canOrganize && onToggleConcealed && <button
      onClick={(event) => {
        event.stopPropagation();
        onToggleConcealed();
      }}
      className="floating-action-button is-warning"
      title={concealmentEffective
        ? translate('component.clipCard.revealSensitiveText')
        : translate('action.conceal')}
    >
      {concealmentEffective
        ? <Eye className="h-3.5 w-3.5" />
        : <EyeOff className="h-3.5 w-3.5" />}
    </button>}
    {features.naming && viewPolicy.canOrganize && onName && <button
      onClick={(event) => {
        event.stopPropagation();
        onName();
      }}
      className={`floating-action-button is-named ${clip.name ? 'is-active' : ''}`}
      title={clip.name ? translate('action.editName') : translate('action.nameClip')}
    >
      <FilePenLine className="h-3.5 w-3.5" />
    </button>}
    {isQueueMode || queueIndex !== undefined ? <>
      {onPasteQueueItem && <button
        onClick={(event) => {
          event.stopPropagation();
          onPasteQueueItem();
        }}
        className="floating-action-button is-accent"
        title={translate('component.clipCard.paste')}
      >
        <ArrowRightCircle className="h-3.5 w-3.5 rtl:-scale-x-100" />
      </button>}
      {onRemoveFromQueue && <button
        onClick={(event) => {
          event.stopPropagation();
          onRemoveFromQueue();
        }}
        className="floating-action-button is-danger"
        title={translate('component.clipCard.removeFromQueue')}
      >
        <MinusCircle className="w-3.5 h-3.5" />
      </button>}
    </> : isTrashMode ? <>
      <button
        onClick={(event) => {
          event.stopPropagation();
          onRestore?.();
        }}
        className="floating-action-button is-accent"
        title={UI_COPY.restore}
      >
        <RotateCcw className="w-3.5 h-3.5" />
      </button>
      <button
        onClick={(event) => {
          event.stopPropagation();
          onPurgePermanently?.();
        }}
        className="floating-action-button is-danger"
        title={UI_COPY.deletePermanently}
      >
        <Trash className="w-3.5 h-3.5" />
      </button>
    </> : <>
      {features.pinning && <button
        onClick={(event) => {
          event.stopPropagation();
          onPin();
        }}
        className={`floating-action-button ${clip.is_pinned ? 'is-success pin-icon' : ''}`}
        title={clip.is_pinned ? UI_COPY.unpin : UI_COPY.pin}
      >
        <Pin className="w-3.5 h-3.5" />
      </button>}
      {features.protection && onToggleProtected && <button
        onClick={(event) => {
          event.stopPropagation();
          onToggleProtected();
        }}
        disabled={protectionToggleDisabled}
        className={`floating-action-button ${clip.is_protected ? 'is-accent' : ''}`}
        title={clip.hotkey
          ? translate('component.clipCard.protectedByHotkey')
          : protectedByBin
            ? translate('component.clipPreview.protectedByBin')
            : clip.is_protected ? UI_COPY.unprotect : UI_COPY.protect}
      >
        {clip.is_protected && !protectionToggleDisabled
          ? <ShieldOff className="w-3.5 h-3.5" />
          : <Shield className="w-3.5 h-3.5" />}
      </button>}
      <button
        onClick={(event) => {
          event.stopPropagation();
          if (!clip.is_protected) onDelete(event);
        }}
        disabled={clip.is_protected}
        className={`floating-action-button ${clip.is_protected
          ? 'is-disabled cursor-not-allowed opacity-50'
          : 'is-danger'}`}
        title={clip.is_protected
          ? translate('component.clipCard.clipIsProtectedUnprotectFirstToDelete')
          : trashEnabled
            ? translate('component.clipCard.movetotrashOptionClickToDeletePermanently', { moveToTrash: UI_COPY.moveToTrash })
            : clipDeleteLabel({ trashEnabled })}
      >
        {trashEnabled ? <Trash2 className="w-3.5 h-3.5" /> : <X className="w-3.5 h-3.5" />}
      </button>
    </>}
  </FloatingActionStrip>;
}
