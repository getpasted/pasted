import { FolderInput, History, LoaderCircle, RotateCcw, Workflow, X } from 'lucide-react';
import type { ClipVersion } from '../types';
import { formatClipDateTime } from '../utils/date';

interface ClipRevisionHistoryProps {
  versions: ClipVersion[];
  isLoading: boolean;
  readOnly?: boolean;
  onClose: () => void;
  previewedVersionId?: number | null;
  restoringVersionId?: number | null;
  onPreview: (version: ClipVersion) => void;
  onRestore: (version: ClipVersion) => void;
}

export function ClipRevisionHistory({
  versions,
  isLoading,
  readOnly = false,
  onClose,
  previewedVersionId = null,
  restoringVersionId = null,
  onPreview,
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
          {versions.map((version, index) => {
            const isRestoring = restoringVersionId === version.id;
            const restoreInProgress = restoringVersionId !== null;
            return (
              <div
                key={version.id || index}
                role="button"
                tabIndex={restoreInProgress ? -1 : 0}
                aria-pressed={previewedVersionId === version.id}
                aria-busy={isRestoring}
                onClick={() => { if (!restoreInProgress) onPreview(version); }}
                onKeyDown={(event) => {
                  if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault();
                    if (!restoreInProgress) onPreview(version);
                  }
                }}
                className={`clip-revision-history-item p-2.5 rounded-lg text-xs cursor-pointer ${previewedVersionId === version.id ? 'is-previewing' : ''} ${isRestoring ? 'is-restoring' : ''}`}
              >
                <div className="min-w-0 flex-1">
                  {version.action_label && (
                    <div className="theme-text-main mb-1.5 flex items-center gap-1.5 text-[11px] font-semibold">
                      {version.restores_organization
                        ? <FolderInput className="h-3.5 w-3.5 shrink-0" />
                        : <Workflow className="h-3.5 w-3.5 shrink-0" />}
                      <span className="min-w-0 leading-snug">{version.action_label}</span>
                    </div>
                  )}
                  <div className="clip-revision-history-meta mb-2">
                    <strong>#{versions.length - index},</strong>
                    <time dateTime={version.created_at}>{formatClipDateTime(version.created_at)},</time>
                    <span>{version.text_content.length} chars,</span>
                    <span>{version.restores_organization ? 'Content + Bin' : 'Content'}</span>
                  </div>
                  <p className="clip-revision-history-preview text-xs font-mono truncate">{version.text_content}</p>
                </div>
                {!readOnly && (
                  <button
                    type="button"
                    disabled={restoreInProgress}
                    onClick={(event) => { event.stopPropagation(); onRestore(version); }}
                  className="clip-revision-history-restore px-2.5 py-1 font-semibold rounded-md text-[11px] cursor-pointer"
                  aria-label={`Restore revision ${versions.length - index}`}
                  title="Restore this revision"
                >
                    {isRestoring
                      ? <LoaderCircle className="h-4 w-4 animate-spin" />
                      : <RotateCcw className="h-4 w-4" />}
                </button>
                )}
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}
