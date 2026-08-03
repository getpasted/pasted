import { History, X } from 'lucide-react';
import type { ClipVersion } from '../types';
import { formatClipDateTime } from '../utils/date';

interface ClipRevisionHistoryProps {
  versions: ClipVersion[];
  isLoading: boolean;
  readOnly?: boolean;
  onClose: () => void;
  onRestore: (version: ClipVersion) => void;
}

export function ClipRevisionHistory({
  versions,
  isLoading,
  readOnly = false,
  onClose,
  onRestore,
}: ClipRevisionHistoryProps) {
  return (
    <section
      id="clip-revision-history-panel"
      className="clip-revision-history p-4 space-y-3 animate-in slide-in-from-bottom-2 duration-150"
      aria-label="Clip revision history"
    >
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-2">
          <History className="clip-revision-history-icon w-4 h-4" />
          <h4 className="clip-revision-history-title text-xs font-bold">Revision History</h4>
          <span className="clip-revision-history-count text-[10px] font-mono">
            {versions.length} {versions.length === 1 ? 'version' : 'versions'}
          </span>
        </div>
        <button type="button" onClick={onClose} aria-label="Close revision history" className="clip-revision-history-close p-1 rounded-full">
          <X className="w-3.5 h-3.5" />
        </button>
      </div>

      {isLoading ? (
        <p role="status" className="clip-revision-history-empty text-xs py-2">Loading revision history…</p>
      ) : versions.length === 0 ? (
        <p className="clip-revision-history-empty text-xs py-2">No past version snapshots recorded for this clip yet.</p>
      ) : (
        <div className="max-h-48 overflow-y-auto space-y-2 pr-1 custom-scrollbar">
          {versions.map((version, index) => (
            <div key={version.id || index} className="clip-revision-history-item p-2.5 rounded-lg flex items-center justify-between space-x-3 text-xs">
              <div className="min-w-0 flex-1">
                <div className="clip-revision-history-meta flex items-center space-x-2 text-[10px] font-mono mb-1">
                  <span>Version #{versions.length - index}</span>
                  <span>•</span>
                  <span>{formatClipDateTime(version.created_at)}</span>
                  <span>•</span>
                  <span>{version.text_content.length} chars</span>
                </div>
                <p className="clip-revision-history-preview text-xs font-mono truncate">{version.text_content}</p>
              </div>
              {!readOnly && (
                <button type="button" onClick={() => onRestore(version)} className="clip-revision-history-restore px-2.5 py-1 font-semibold rounded-md text-[11px] shrink-0 cursor-pointer">
                  Restore
                </button>
              )}
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
