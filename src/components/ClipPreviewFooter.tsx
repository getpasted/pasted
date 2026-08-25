import { dateTimeAttribute, formatFullDateTime, formatRelativeTime } from '../utils/date';
import { useMinuteTick } from '../hooks/useMinuteTick';
import { translate } from '../localization/runtime';
import { getClipFilePaths, type ClipItem } from '../types';
import {
  formatFileSize,
  formatMediaDuration,
  type StructuralInspection,
} from './clipPreviewModel';

interface ClipPreviewFooterProps {
  clip: ClipItem;
  inspection: StructuralInspection | null;
  fileFormatsEnabled: boolean;
  revisionsEnabled: boolean;
  characterCount: number;
  wordCount: number;
  lineCount: number;
  versionCount: number | null;
  showHistory: boolean;
  onToggleHistory: () => void;
}

export function ClipPreviewFooter({
  clip,
  inspection,
  fileFormatsEnabled,
  revisionsEnabled,
  characterCount,
  wordCount,
  lineCount,
  versionCount,
  showHistory,
  onToggleHistory,
}: ClipPreviewFooterProps) {
  const relativeTimeNow = useMinuteTick();
  return (
    <div className="clip-preview-footer min-h-[55px] px-4 py-2.5 border-t flex text-[11px]">
      <div className="clip-preview-footer-stats">
        {clip.content_type === 'file' ? (
          <>
            <span className="clip-preview-footer-stat">
              <span>{translate('component.clipPreview.items')}</span>
              <strong>{inspection?.result.files?.itemCount ?? getClipFilePaths(clip).length}</strong>
            </span>
            <span className="clip-preview-footer-stat" title={inspection?.result.files?.extensions.join(', ') || translate('component.clipPreview.noFileExtensions')}>
              <span>{translate('component.clipPreview.fileExtensions')}</span>
              <strong>{inspection?.result.files ? (inspection.result.files.extensions.length > 2 ? translate('component.clipPreview.valueValue2', { value: inspection.result.files.extensions.slice(0, 2).join(', '), value2: inspection.result.files.extensions.length - 2 }) : inspection.result.files.extensions.join(', ') || '—') : '…'}</strong>
            </span>
            {fileFormatsEnabled && <span className="clip-preview-footer-stat" title={inspection?.fileFormats?.formats.map(({ mimeType }) => mimeType).join(', ')}>
              <span>{translate('component.clipPreview.fileFormats')}</span>
              <strong>{inspection?.fileFormats
                ? inspection.fileFormats.formats.map(({ format }) => format.toUpperCase()).join(', ') || '—'
                : '…'}</strong>
            </span>}
            <span className="clip-preview-footer-stat">
              <span>{translate('component.clipPreview.size')}</span>
              <strong>{inspection?.liveFileObservations ? (inspection.liveFileObservations.fileCount > 0 ? formatFileSize(inspection.liveFileObservations.totalSizeBytes) : '—') : '…'}</strong>
            </span>
            <span className="clip-preview-footer-stat">
              <span>{translate('component.clipPreview.available')}</span>
              <strong>{inspection?.liveFileObservations ? `${inspection.liveFileObservations.availableCount}/${inspection.result.files?.itemCount ?? 0}` : '…'}</strong>
            </span>
            {inspection?.mediaMetadata && <>
              <span className="clip-preview-footer-stat" title={inspection.mediaMetadata.containers.join(', ')}>
                <span>{translate('component.clipPreview.media')}</span>
                <strong>{inspection.mediaMetadata.mediaFileCount}</strong>
              </span>
              <span className="clip-preview-footer-stat" title={inspection.mediaMetadata.codecs.join(', ')}>
                <span>{translate('component.clipPreview.codecs')}</span>
                <strong>{inspection.mediaMetadata.codecs.slice(0, 2).join(', ') || '—'}</strong>
              </span>
              <span className="clip-preview-footer-stat">
                <span>{translate('component.clipPreview.duration')}</span>
                <strong>{formatMediaDuration(inspection.mediaMetadata.totalDurationMs)}</strong>
              </span>
            </>}
          </>
        ) : (
          <>
            <span className="clip-preview-footer-stat">
              <span>{translate('component.clipPreview.chars')}</span>
              <strong>{characterCount}</strong>
            </span>
            <span className="clip-preview-footer-stat">
              <span>{translate('component.clipPreview.words')}</span>
              <strong>{wordCount}</strong>
            </span>
            <span className="clip-preview-footer-stat">
              <span>{translate('component.clipPreview.lines')}</span>
              <strong>{lineCount}</strong>
            </span>
            {revisionsEnabled && <span className="clip-preview-footer-stat">
              <span>{translate('component.clipPreview.versions')}</span>
              <button
                type="button"
                onClick={onToggleHistory}
                className={`clip-revision-count ${showHistory ? 'is-active' : ''}`}
                title={versionCount === null ? translate('component.clipPreview.loadingVersions') : translate('component.clipPreview.viewVersions')}
                aria-label={versionCount === null ? translate('component.clipPreview.loadingClipVersionCount') : translate('component.clipPreview.viewCountClipVersions', { count: versionCount })}
                aria-expanded={showHistory}
                aria-controls="clip-revision-history-panel"
              >
                {versionCount ?? '…'}
              </button>
            </span>}
          </>
        )}
      </div>
      <div className="clip-preview-footer-captured">
        <span>{translate('component.clipPreview.captured')}</span>
        <time dateTime={dateTimeAttribute(clip.created_at)} title={formatFullDateTime(clip.created_at)}>
          {formatRelativeTime(clip.created_at, relativeTimeNow)}
        </time>
      </div>
    </div>
  );
}
