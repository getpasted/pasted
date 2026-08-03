import { History, X } from 'lucide-react';
import type { ClipVersion } from '../types';
import { formatClipDateTime } from '../utils/date';

interface ClipRevisionHistoryProps {
  versions: ClipVersion[];
  isLoading: boolean;
  onClose: () => void;
  onRestore: (version: ClipVersion) => void;
}

export function ClipRevisionHistory({
  versions,
  isLoading,
  onClose,
  onRestore,
}: ClipRevisionHistoryProps) {
  return (
    <section className="p-4 bg-[#181818] border-b border-gray-700/80 space-y-3 animate-in slide-in-from-top-2 duration-150" aria-label="Clip revision history">
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-2">
          <History className="w-4 h-4 text-purple-400" />
          <h4 className="text-xs font-bold text-gray-200">Clip Revision History</h4>
          <span className="text-[10px] text-gray-400 font-mono">({versions.length} versions)</span>
        </div>
        <button type="button" onClick={onClose} aria-label="Close revision history" className="text-gray-400 hover:text-white p-1 rounded-full hover:bg-gray-800">
          <X className="w-3.5 h-3.5" />
        </button>
      </div>

      {isLoading ? (
        <p role="status" className="text-xs text-gray-500 py-2">Loading revision history…</p>
      ) : versions.length === 0 ? (
        <p className="text-xs text-gray-500 py-2">No past version snapshots recorded for this clip yet.</p>
      ) : (
        <div className="max-h-48 overflow-y-auto space-y-2 pr-1 custom-scrollbar">
          {versions.map((version, index) => (
            <div key={version.id || index} className="p-2.5 bg-[#222225] border border-gray-700/60 rounded-xl flex items-center justify-between space-x-3 text-xs">
              <div className="min-w-0 flex-1">
                <div className="flex items-center space-x-2 text-[10px] text-gray-400 font-mono mb-1">
                  <span>Version #{versions.length - index}</span>
                  <span>•</span>
                  <span>{formatClipDateTime(version.created_at)}</span>
                  <span>•</span>
                  <span>{version.text_content.length} chars</span>
                </div>
                <p className="text-xs text-gray-300 font-mono truncate">{version.text_content}</p>
              </div>
              <button type="button" onClick={() => onRestore(version)} className="px-2.5 py-1 bg-purple-600 hover:bg-purple-500 text-white font-semibold rounded-lg text-[11px] shrink-0 cursor-pointer shadow">
                Restore
              </button>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
