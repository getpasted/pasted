import { Clipboard, Disc, Pause, Search, Square } from 'lucide-react';
import { useState } from 'react';
import type { SequentialStatus } from '../types';
import type { ClipCollectionDefinition } from '../utils/clipCollections';
import { handleWindowDragDoubleClick, startWindowDrag } from '../utils/windowDrag';
import { translate } from '../localization/runtime';
import { ActionButton } from './AppDialogLayout';
import { ConfirmationDialog, type ConfirmationDialogRequest } from './ConfirmationDialog';
import { OverflowText } from './OverflowText';

interface ClipListHeaderProps {
  collection?: ClipCollectionDefinition;
  currentTab: string;
  searchTotalCount: number;
  ignoredAppStatus: { app_name: string; timestamp: number } | null;
  trashIsEmpty: boolean;
  onEmptyTrash: () => void | Promise<void>;
  clipboardPaused: boolean;
  onToggleClipboardPause: () => void;
  queueEnabled: boolean;
  queueStatus: SequentialStatus | null;
  onToggleQueue: () => void;
}

export function ClipListHeader({
  collection,
  currentTab,
  searchTotalCount,
  ignoredAppStatus,
  trashIsEmpty,
  onEmptyTrash,
  clipboardPaused,
  onToggleClipboardPause,
  queueEnabled,
  queueStatus,
  onToggleQueue,
}: ClipListHeaderProps) {
  const [emptyTrashRequest, setEmptyTrashRequest] = useState<ConfirmationDialogRequest | null>(null);
  const requestEmptyTrash = () => setEmptyTrashRequest({
    title: translate('app.emptyTrashConfirmation'),
    description: translate('app.emptyTrashDescription'),
    confirmLabel: translate('app.emptyTrash'),
    tone: 'danger',
    onConfirm: async () => {
      await onEmptyTrash();
      setEmptyTrashRequest(null);
    },
  });

  return (
    <>
    <div
      onMouseDown={startWindowDrag}
      onDoubleClick={handleWindowDragDoubleClick}
      className="h-[60px] border-b px-3 flex items-center justify-between col-list-header cursor-default titlebar-drag-handle shrink-0"
    >
      <div className="flex items-center space-x-2 titlebar-drag-handle min-w-0 flex-1 me-2">
        {collection?.icon === 'search'
          ? <Search className="theme-text-main w-4 h-4 titlebar-drag-handle shrink-0" />
          : <Clipboard className="theme-text-main w-4 h-4 titlebar-drag-handle shrink-0" />}
        <OverflowText as="h2" text={collection?.title ?? translate('collection.history')} className="theme-title text-xs font-bold uppercase tracking-wider titlebar-drag-handle truncate" />
        {currentTab === 'search' && (
          <span
            className="theme-badge min-w-5 rounded-md border px-1.5 py-0.5 text-center font-mono text-[10px] font-semibold"
            aria-label={translate('app.searchResultCount', { count: searchTotalCount })}
            title={translate('app.resultCount', { count: searchTotalCount })}
          >
            {searchTotalCount}
          </span>
        )}
      </div>
      <div className="flex items-center space-x-1.5 shrink-0">
        {ignoredAppStatus && (
          <span className="theme-status-danger text-[10px] px-2 py-0.5 rounded border font-mono flex items-center animate-in fade-in">
            {translate('app.ignoredApp', { name: ignoredAppStatus.app_name })}
          </span>
        )}
        {collection?.membership === 'trash' && (
          <ActionButton
            variant="danger"
            onClick={requestEmptyTrash}
            disabled={trashIsEmpty}
            className="shrink-0"
          >
            <span>{translate('app.emptyTrashEllipsis')}</span>
          </ActionButton>
        )}
        <button
          onClick={onToggleClipboardPause}
          className={`list-toolbar-button ui-control-radius flex h-7 w-7 items-center justify-center border transition-colors ${clipboardPaused ? 'is-warning' : ''}`}
          title={clipboardPaused ? translate('app.resumeHistory') : translate('app.pauseHistory')}
        >
          <Pause className={`w-4 h-4 ${clipboardPaused ? 'fill-current animate-pulse' : ''}`} strokeWidth={2.5} />
        </button>
        {queueEnabled && <button
          onClick={onToggleQueue}
          className={`list-toolbar-button ui-control-radius flex h-7 w-7 items-center justify-center border transition-colors ${queueStatus?.is_active ? 'is-queue-active' : ''}`}
          title={queueStatus?.is_active
            ? translate('app.stopQueueCount', { count: queueStatus.queue.length })
            : translate('app.startQueue')}
        >
          {queueStatus?.is_active
            ? <Square className="w-3.5 h-3.5 fill-current animate-pulse" strokeWidth={2.5} />
            : <Disc className="w-4 h-4 transition-colors" strokeWidth={2.5} />}
        </button>}
      </div>
    </div>
    <ConfirmationDialog request={emptyTrashRequest} onCancel={() => setEmptyTrashRequest(null)} />
    </>
  );
}
