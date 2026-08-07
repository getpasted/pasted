import React, { useCallback, useEffect, useState } from 'react';
import { QueuePasteTarget, SequentialStatus } from '../types';
import { Disc, ArrowRightCircle, Layers, AlertTriangle, CornerDownLeft } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';

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
    void invoke<QueuePasteTarget | null>('get_queue_paste_target')
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
    }
  };

  const isActive = status?.is_active ?? false;
  const queue = status?.queue ?? [];

  return (
    <div className={`queue-controls-card theme-card-idle p-3 rounded-xl border transition-[background-color,border-color,box-shadow] ${isActive ? 'is-active' : ''}`}>
      {/* Header row */}
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-2 min-w-0">
          <div className={`queue-controls-icon theme-surface p-1.5 rounded-lg border shrink-0 ${isActive ? 'is-active' : ''}`}>
            <Disc className={`w-3.5 h-3.5 ${isActive ? 'text-purple-400 animate-spin' : 'text-gray-400'}`} />
          </div>
          <h3 className="text-xs font-bold theme-title text-gray-100 truncate">Copy Queue</h3>
        </div>

        {isActive && (
          <span className="text-[9px] px-2 py-0.5 rounded-full bg-purple-500/20 text-purple-300 font-mono animate-pulse border border-purple-500/30 font-semibold shrink-0">
            RECORDING COPIES
          </span>
        )}
      </div>

      {/* Helper text */}
      <p className="text-[11px] theme-text-muted leading-normal mt-2">
        Toggle recording with <kbd className="theme-kbd px-1 py-0.5 rounded font-mono text-[9px] border">⌥⇧C</kbd>, then copy normally. Paste next with <kbd className="theme-kbd px-1 py-0.5 rounded font-mono text-[9px] border">⌥⇧X</kbd>.
      </p>

      {queue.length > 0 && (
        <div className="theme-text-muted mt-2 flex items-center gap-1.5 text-[10px]">
          <CornerDownLeft className="h-3 w-3 shrink-0" />
          <span>Next paste targets <strong className="theme-title font-semibold">{pasteTarget?.name ?? 'the previous app'}</strong></span>
        </div>
      )}

      {/* Action buttons row */}
      {queue.length > 0 && (
        <div className="mt-2.5 pt-2 border-t border-purple-500/20 flex items-center justify-between flex-wrap gap-2">
          <span className="text-xs font-mono font-bold text-purple-300 bg-purple-950/80 px-2 py-0.5 rounded border border-purple-500/30">
            {queue.length} in buffer
          </span>
          <div className="flex items-center space-x-1.5">
            <button
              type="button"
              onClick={handlePopNext}
              disabled={isPasting}
              className="flex items-center space-x-1 px-2 py-1 rounded-lg bg-purple-900 hover:bg-purple-800 border border-purple-500/40 text-purple-200 text-[11px] font-semibold transition-colors cursor-pointer"
              title="Paste Next (⌥⇧X)"
            >
              <ArrowRightCircle className="w-3 h-3" />
              <span>Paste Next</span>
            </button>
            <button
              type="button"
              onClick={handlePasteAll}
              disabled={isPasting}
              className="flex items-center space-x-1 px-2 py-1 rounded-lg bg-purple-600 hover:bg-purple-500 text-white text-[11px] font-semibold shadow transition-colors cursor-pointer"
              title="Combine and Paste"
            >
              <Layers className="w-3 h-3" />
              <span>Paste All</span>
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
