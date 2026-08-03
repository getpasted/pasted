import React from 'react';
import { SequentialStatus } from '../types';
import { Disc, ArrowRightCircle, Layers } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';

interface SequentialQueueBarProps {
  status: SequentialStatus | null;
  onRefresh: () => void;
}

export const SequentialQueueBar: React.FC<SequentialQueueBarProps> = ({
  status,
  onRefresh,
}) => {
  const handlePopNext = async () => {
    try {
      await invoke('pop_sequential_paste');
      onRefresh();
    } catch (e) {
      console.error(e);
    }
  };

  const handlePasteAll = async () => {
    try {
      await invoke('paste_all_sequential');
      onRefresh();
    } catch (e) {
      console.error(e);
    }
  };

  const isActive = status?.is_active ?? false;
  const queue = status?.queue ?? [];

  return (
    <div className={`p-3 rounded-xl border transition-all ${
      isActive
        ? 'theme-card-selected bg-[#24202c] border-purple-500/50 shadow-xl'
        : 'theme-card-idle bg-[#212121] border-gray-700/80 shadow-md'
    }`}>
      {/* Header row */}
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-2 min-w-0">
          <div className={`p-1.5 rounded-lg border shrink-0 ${
            isActive ? 'bg-purple-900/50 text-purple-300 border-purple-500/40' : 'bg-[#181818] text-gray-400 border-gray-700'
          }`}>
            <Disc className={`w-3.5 h-3.5 ${isActive ? 'text-purple-400 animate-spin' : 'text-gray-400'}`} />
          </div>
          <h3 className="text-xs font-bold theme-title text-gray-100 truncate">Queue Controls</h3>
        </div>

        {isActive && (
          <span className="text-[9px] px-2 py-0.5 rounded-full bg-purple-500/20 text-purple-300 font-mono animate-pulse border border-purple-500/30 font-semibold shrink-0">
            RECORDING ACTIVE
          </span>
        )}
      </div>

      {/* Helper text */}
      <p className="text-[11px] theme-text-muted text-gray-400 leading-normal mt-2">
        Record with <kbd className="px-1 py-0.5 rounded bg-gray-800 font-mono text-[9px] border border-gray-700">⌥⇧C</kbd> / <kbd className="px-1 py-0.5 rounded bg-gray-800 font-mono text-[9px] border border-gray-700">⌘C</kbd> • Paste next with <kbd className="px-1 py-0.5 rounded bg-gray-800 font-mono text-[9px] border border-gray-700">⌥⇧X</kbd>
      </p>

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
              className="flex items-center space-x-1 px-2 py-1 rounded-lg bg-purple-900 hover:bg-purple-800 border border-purple-500/40 text-purple-200 text-[11px] font-semibold transition-colors cursor-pointer"
              title="Paste next queued item (⌥⇧X)"
            >
              <ArrowRightCircle className="w-3 h-3" />
              <span>Paste Next</span>
            </button>
            <button
              type="button"
              onClick={handlePasteAll}
              className="flex items-center space-x-1 px-2 py-1 rounded-lg bg-purple-600 hover:bg-purple-500 text-white text-[11px] font-semibold shadow transition-colors cursor-pointer"
              title="Combine all queued items and paste"
            >
              <Layers className="w-3 h-3" />
              <span>Paste All</span>
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
