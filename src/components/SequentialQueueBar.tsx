import React, { useCallback, useEffect, useState } from 'react';
import { QueuePasteTarget, SequentialStatus } from '../types';
import { Disc, ArrowRightCircle, Layers, AlertTriangle, CornerDownLeft } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { translate } from '../localization/runtime';

interface SequentialQueueBarProps {
  status: SequentialStatus | null;
  onRefresh: () => void;
}

export const SequentialQueueBar: React.FC<SequentialQueueBarProps> = ({
  status,
  onRefresh,
}) => {
  const [error, setError] = useState('');
  const [isPasting, setIsPasting] = useState(false);
  const [pasteTarget, setPasteTarget] = useState<QueuePasteTarget | null>(null);
  const refreshPasteTarget = useCallback(() => {
    void invoke<QueuePasteTarget>('get_queue_paste_target')
      .then(setPasteTarget)
      .catch(() => setPasteTarget(null));
  }, []);

  useEffect(() => {
    refreshPasteTarget();
    window.addEventListener('focus', refreshPasteTarget);
    return () => window.removeEventListener('focus', refreshPasteTarget);
  }, [refreshPasteTarget]);

  const handlePopNext = async () => {
    setError('');
    setIsPasting(true);
    try {
      await invoke('pop_sequential_paste');
      onRefresh();
    } catch (e) {
      console.error(e);
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setIsPasting(false);
      refreshPasteTarget();
    }
  };

  const handlePasteAll = async () => {
    setError('');
    setIsPasting(true);
    try {
      await invoke('paste_all_sequential');
      onRefresh();
    } catch (e) {
      console.error(e);
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setIsPasting(false);
      refreshPasteTarget();
    }
  };

  const isActive = status?.is_active ?? false;
  const queue = status?.queue ?? [];
  const canPasteAutomatically = pasteTarget?.automaticPasteAvailable === true;

  return (
    <div className={`queue-controls-card theme-card-idle p-3 border transition-[background-color,border-color,box-shadow] ${isActive ? 'is-active' : ''}`}>
      {/* Header row */}
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-2 min-w-0">
          <div className={`queue-controls-icon theme-surface p-1.5 rounded-lg border shrink-0 ${isActive ? 'is-active' : ''}`}>
            <Disc className={`w-3.5 h-3.5 ${isActive ? 'theme-status-info-text animate-spin' : 'theme-text-muted'}`} />
          </div>
          <h3 className="text-xs font-bold theme-title truncate">{translate('component.sequentialQueueBar.copyQueue')}</h3>
        </div>

        {isActive && (
          <span className="queue-recording-badge ui-pill text-[9px] px-2 py-0.5 font-mono animate-pulse border font-semibold shrink-0">
            {translate('component.sequentialQueueBar.recordingCopies')}
          </span>
        )}
      </div>

      {/* Helper text */}
      <p className="text-[11px] theme-text-muted leading-normal mt-2">
        {translate('component.sequentialQueueBar.shortcutDescription', { toggleShortcut: '⌥⇧C', pasteShortcut: '⌥⇧X' })}
      </p>

      {queue.length > 0 && canPasteAutomatically && (
        <div className="theme-text-muted mt-2 flex min-w-0 items-center gap-1.5 text-[10px]">
          <CornerDownLeft className="h-3 w-3 shrink-0" />
          <span className="flex min-w-0 items-baseline gap-1">
            <span className="shrink-0">{translate('component.sequentialQueueBar.nextPasteTargets')}</span>
            <strong className="theme-title truncate font-semibold" title={pasteTarget.name}>{pasteTarget.name}</strong>
          </span>
        </div>
      )}

      {queue.length > 0 && pasteTarget && !canPasteAutomatically && (
        <div className="theme-status-warning mt-2 flex items-start gap-1.5 rounded-lg border px-2 py-1.5 text-[10px] leading-relaxed">
          <AlertTriangle className="mt-px h-3 w-3 shrink-0" />
          <span>{pasteTarget.unavailableReason}</span>
        </div>
      )}

      {/* Action buttons row */}
      {queue.length > 0 && (
        <div className="queue-controls-footer mt-2.5 pt-2 border-t flex items-center justify-between flex-wrap gap-2">
          <span className="queue-count-badge text-xs font-mono font-bold px-2 py-0.5 rounded border">
            {translate('component.sequentialQueueBar.bufferCount', { count: queue.length })}</span>
          <div className="flex items-center space-x-1.5">
            <button
              type="button"
              onClick={handlePopNext}
              disabled={isPasting || !canPasteAutomatically}
              className="queue-action-secondary ui-control-radius flex items-center space-x-1 px-2 py-1 border text-[11px] font-semibold transition-colors cursor-pointer"
              title={translate('component.sequentialQueueBar.pasteNextX')}
            >
              <ArrowRightCircle className="h-3 w-3 rtl:-scale-x-100" />
              <span>{translate('component.sequentialQueueBar.pasteNext')}</span>
            </button>
            <button
              type="button"
              onClick={handlePasteAll}
              disabled={isPasting || !canPasteAutomatically}
              className="queue-action-primary ui-control-radius flex items-center space-x-1 px-2 py-1 border text-[11px] font-semibold shadow transition-colors cursor-pointer"
              title={translate('component.sequentialQueueBar.combineAndPaste')}
            >
              <Layers className="w-3 h-3" />
              <span>{translate('component.sequentialQueueBar.pasteAll')}</span>
            </button>
          </div>
        </div>
      )}
      {error && (
        <div role="alert" className="theme-status-danger mt-2.5 flex items-start gap-2 rounded-lg border px-2.5 py-2 text-[11px] leading-relaxed">
          <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
          <span>{error}</span>
        </div>
      )}
    </div>
  );
};
