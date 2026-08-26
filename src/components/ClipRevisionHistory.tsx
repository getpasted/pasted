import { useState } from 'react';
import { FolderInput, History, LoaderCircle, RotateCcw, Trash2, Workflow, X } from 'lucide-react';
import type { ClipVersion } from '../types';
import { dateTimeAttribute, formatFullDateTime, formatRelativeTime } from '../utils/date';
import { useMinuteTick } from '../hooks/useMinuteTick';
import { OverflowText } from './OverflowText';
import { translate } from '../localization/runtime';
import { DeleteClipVersionDialog } from './DeleteClipVersionDialog';

interface ClipRevisionHistoryProps {
  versions: ClipVersion[];
  versionCount?: number | null;
  isLoading: boolean;
  readOnly?: boolean;
  onClose: () => void;
  previewedVersionId?: number | null;
  restoringVersionId?: number | null;
  deletingVersionId?: number | null;
  hasMore?: boolean;
  isLoadingMore?: boolean;
  onPreview: (version: ClipVersion) => void;
  onPreviewStart: (version: ClipVersion) => void;
  onPreviewEnd: () => void;
  onLoadMore?: () => void;
  onRestore: (version: ClipVersion) => void;
  onDelete: (version: ClipVersion) => Promise<boolean>;
}

export function ClipRevisionHistory({
  versions,
  versionCount = null,
  isLoading,
  readOnly = false,
  onClose,
  previewedVersionId = null,
  restoringVersionId = null,
  deletingVersionId = null,
  hasMore = false,
  isLoadingMore = false,
  onPreview,
  onPreviewStart,
  onPreviewEnd,
  onLoadMore,
  onRestore,
  onDelete,
}: ClipRevisionHistoryProps) {
  const relativeTimeNow = useMinuteTick();
  const [versionToDelete, setVersionToDelete] = useState<ClipVersion | null>(null);
  const mutationInProgress = restoringVersionId !== null || deletingVersionId !== null;
  return <>
    <section
      id="clip-revision-history-panel"
      className="clip-revision-history p-4 space-y-3 animate-in slide-in-from-bottom-2 duration-150"
      aria-label={translate('component.clipRevisionHistory.clipRevisionHistory')}
    >
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-2">
          <History className="clip-revision-history-icon w-4 h-4" />
          <h4 className="clip-revision-history-title text-xs font-bold">{translate('component.clipRevisionHistory.versionHistory')}</h4>
          <span className="clip-revision-history-count text-[10px] font-mono">
            {translate('format.versionCount', { count: versionCount ?? versions.length })}
          </span>
        </div>
        <button type="button" onClick={onClose} aria-label={translate('component.clipRevisionHistory.closeRevisionHistory')} className="clip-revision-history-close p-1 rounded-full">
          <X className="w-3.5 h-3.5" />
        </button>
      </div>

      {isLoading ? (
        <p role="status" className="clip-revision-history-empty text-xs py-2">{translate('component.clipRevisionHistory.loadingVersionHistory')}</p>
      ) : versions.length === 0 ? (
        <p className="clip-revision-history-empty text-xs py-2">{translate('component.clipRevisionHistory.noVersionsYet')}</p>
      ) : (
        <div className="max-h-48 overflow-y-auto space-y-2 pe-1 custom-scrollbar">
          {versions.map((version, index) => {
            const isRestoring = restoringVersionId === version.id;
            const isDeleting = deletingVersionId === version.id;
            const actionLabel = version.action_kind === 'original'
              ? translate('component.clipRevisionHistory.original')
              : version.action_kind === 'extraction'
                ? translate('component.clipRevisionHistory.beforeExtractingAgain')
                : version.action_kind === 'visual_label_edit'
                  ? translate('component.clipRevisionHistory.beforeEditingVisualLabels')
                  : version.is_current
                    ? translate('component.clipRevisionHistory.current')
                : version.action_label;
            const isPreviewing = version.is_current
              ? previewedVersionId === null
              : previewedVersionId === version.id;
            return (
              <div
                key={version.id || index}
                role="button"
                tabIndex={mutationInProgress ? -1 : 0}
                aria-pressed={isPreviewing}
                aria-busy={isRestoring || isDeleting}
                onMouseEnter={() => { if (!mutationInProgress) onPreviewStart(version); }}
                onMouseLeave={onPreviewEnd}
                onFocus={() => { if (!mutationInProgress) onPreviewStart(version); }}
                onBlur={(event) => {
                  if (!event.currentTarget.contains(event.relatedTarget)) onPreviewEnd();
                }}
                onClick={() => { if (!mutationInProgress) onPreview(version); }}
                onKeyDown={(event) => {
                  if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault();
                    if (!mutationInProgress) onPreview(version);
                  }
                }}
                className={`clip-revision-history-item p-2.5 rounded-lg text-xs cursor-pointer ${isPreviewing ? 'is-previewing' : ''} ${isRestoring || isDeleting ? 'is-mutating' : ''}`}
              >
                <div className="min-w-0 flex-1">
                  {actionLabel && (
                    <div className="theme-text-main mb-1.5 flex items-center gap-1.5 text-[11px] font-semibold">
                      {version.restores_organization
                        ? <FolderInput className="h-3.5 w-3.5 shrink-0" />
                        : <Workflow className="h-3.5 w-3.5 shrink-0" />}
                      <span className="min-w-0 leading-snug">{actionLabel}</span>
                    </div>
                  )}
                  <div className="clip-revision-history-meta mb-2">
                    <strong>#{versions.length - index},</strong>
                    <time
                      dateTime={dateTimeAttribute(version.created_at)}
                      title={formatFullDateTime(version.created_at)}
                    >
                      {formatRelativeTime(version.created_at, relativeTimeNow)},
                    </time>
                    <span>{translate('format.characterCount', { count: version.text_content.length })}</span>
                    <span>{version.restores_organization ? translate('component.clipRevisionHistory.contentBin') : translate('component.clipRevisionHistory.content')}</span>
                  </div>
                  <OverflowText as="p" text={version.text_content} className="clip-revision-history-preview text-xs font-mono truncate" />
                </div>
                {!readOnly && !version.is_current && (
                  <div className="floating-action-strip clip-revision-history-actions flex items-center gap-1 rounded-lg border p-1" aria-label={translate('component.clipRevisionHistory.versionActions')}>
                    <button
                      type="button"
                      disabled={mutationInProgress}
                      onClick={(event) => { event.stopPropagation(); onRestore(version); }}
                      className="floating-action-button is-accent disabled:cursor-not-allowed disabled:opacity-40"
                      aria-label={translate('component.clipRevisionHistory.restoreVersionValue', { value: versions.length - index })}
                      title={translate('component.clipRevisionHistory.restoreVersion')}
                    >
                      {isRestoring ? <LoaderCircle className="h-4 w-4 animate-spin" /> : <RotateCcw className="h-4 w-4" />}
                    </button>
                    {!version.is_original && <button
                      type="button"
                      disabled={mutationInProgress}
                      onClick={(event) => { event.stopPropagation(); setVersionToDelete(version); }}
                      className="floating-action-button is-danger disabled:cursor-not-allowed disabled:opacity-40"
                      aria-label={translate('component.clipRevisionHistory.deleteVersion')}
                      title={translate('component.clipRevisionHistory.deleteVersion')}
                    >
                      {isDeleting ? <LoaderCircle className="h-4 w-4 animate-spin" /> : <Trash2 className="h-4 w-4" />}
                    </button>}
                  </div>
                )}
              </div>
            );
          })}
          {hasMore && (
            <button
              type="button"
              disabled={isLoadingMore || mutationInProgress}
              onClick={onLoadMore}
              className="clip-revision-history-load-more w-full rounded-lg border px-3 py-2 text-[11px] font-semibold"
            >
              {isLoadingMore
                ? <><LoaderCircle className="h-3.5 w-3.5 animate-spin" /> {translate('component.clipRevisionHistory.loadingOlder')}</>
                : translate('component.clipRevisionHistory.loadOlderVersions')}
            </button>
          )}
        </div>
      )}
    </section>
    <DeleteClipVersionDialog
      deleting={versionToDelete?.id === deletingVersionId}
      version={versionToDelete}
      onCancel={() => { if (deletingVersionId === null) setVersionToDelete(null); }}
      onConfirm={async () => {
        if (versionToDelete && await onDelete(versionToDelete)) setVersionToDelete(null);
      }}
    />
  </>;
}
