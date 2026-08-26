import { AlertTriangle, FilePenLine, LoaderCircle, Pin, ScanText, Shield, StickyNote, Trash2, Workflow } from 'lucide-react';

import type { useFeatures } from '../hooks/useFeatures';
import { useMinuteTick } from '../hooks/useMinuteTick';
import { translate } from '../localization/runtime';
import { localizedSourceName } from '../localization/presentation';
import type { Bin, ClipItem } from '../types';
import { structuralClipType } from '../utils/contentTypes';
import { dateTimeAttribute, formatFullDateTime, formatRelativeTime } from '../utils/date';
import { ClipBinSummary } from './ClipBinSummary';
import { ContentTypeIcon } from './ContentTypeIcon';
import { HighlightedClipText } from './HighlightedClipText';

type ClipCardFeatures = ReturnType<typeof useFeatures>;

export function ClipCardHeader({
  bins,
  clip,
  features,
  headerSpacingClass,
  headerTextClass,
  isTransforming,
  isTrashMode,
  noteSummary,
  primaryContentType,
  protectedByBin,
  queueIndex,
  searchQuery,
  transformError,
}: {
  bins: Bin[];
  clip: ClipItem;
  features: ClipCardFeatures;
  headerSpacingClass: string;
  headerTextClass: string;
  isTransforming: boolean;
  isTrashMode: boolean;
  noteSummary: string;
  primaryContentType: string;
  protectedByBin: boolean;
  queueIndex?: number;
  searchQuery?: string;
  transformError?: string;
}) {
  const relativeTimeNow = useMinuteTick();
  const showClipType = features.clipTypes
    || (features.types && (clip.content_types?.length ?? 0) > 0);
  return <>
    <div className={`clip-card-header flex items-center justify-between ${headerTextClass} ${headerSpacingClass}`}>
      <div className="flex items-center space-x-2">
        {showClipType && <div className="clip-type-icon theme-badge p-1 rounded border">
          <ContentTypeIcon
            type={features.types && (clip.content_types?.length ?? 0) > 0
              ? primaryContentType
              : structuralClipType(clip.content_type)}
            className="w-3.5 h-3.5 theme-text-muted"
          />
        </div>}
        {features.sources && <span className="font-medium theme-text-main truncate max-w-[120px]" title={localizedSourceName(clip.source)}>
          <HighlightedClipText text={localizedSourceName(clip.source)} query={searchQuery} field="source" />
        </span>}
      </div>
      <div className="clip-meta-row theme-text-subtle flex items-center text-[11px] font-mono">
        {features.transformations && isTransforming && <span
          role="status"
          aria-label={translate('component.clipCard.applyingTransform')}
          title={translate('component.clipCard.applyingTransform2')}
          className="clip-meta-item clip-meta-icon-only clip-transform-working"
        >
          <LoaderCircle className="clip-meta-icon animate-spin" />
        </span>}
        {features.transformations && !isTransforming && transformError && <span
          role="status"
          aria-label={translate('component.clipCard.transformFailed')}
          title={translate('component.clipCard.transformFailedTransformerror', { transformError })}
          className="clip-meta-item clip-meta-icon-only theme-danger-text"
        >
          <AlertTriangle className="clip-meta-icon" />
        </span>}
        {features.bins && <ClipBinSummary bins={bins} primaryBinId={clip.bin_id} />}
        {features.protection && clip.is_protected && <span
          role="img"
          aria-label={translate('component.clipCard.protectedClip')}
          title={clip.hotkey
            ? translate('component.clipCard.protectedByHotkey')
            : protectedByBin
              ? translate('component.clipCard.protectedByBin')
              : translate('component.clipCard.protected')}
          className="clip-meta-item clip-meta-icon-only clip-protected-accent"
        >
          <Shield className="clip-meta-icon" />
        </span>}
        {features.transformations && clip.is_transformed && <span
          role="img"
          aria-label={translate('component.clipCard.transformedClip')}
          title={translate('component.clipCard.transformed')}
          className="clip-meta-item clip-meta-icon-only transform-accent manual-transforms"
        >
          <Workflow className="clip-meta-icon" />
        </span>}
        {features.queue && queueIndex !== undefined && (queueIndex === 1
          ? <span className="clip-meta-item clip-queue-next elevation-control rounded-full font-mono font-extrabold animate-pulse">
            {translate('component.clipCard.nextUp1')}
          </span>
          : <span className="clip-meta-item clip-queue-position rounded-full font-mono font-semibold">
            {translate('component.clipCard.queuePosition', { position: queueIndex })}
          </span>)}
        {clip.content_type === 'image' && clip.text_content && <span
          role="img"
          aria-label={translate('component.clipCard.ocrTextAvailable')}
          title={translate('component.clipCard.ocrText')}
          className="clip-meta-item clip-meta-icon-only clip-ocr-accent"
        >
          <ScanText className="clip-meta-icon" />
        </span>}
        {noteSummary && <span title={translate('component.clipCard.notesNotesummary', { noteSummary })} className="clip-meta-item clip-meta-icon-only">
          <StickyNote className="clip-meta-icon clip-note-accent" />
        </span>}
        {features.pinning && clip.is_pinned && <span title={translate('component.clipCard.pinned')} className="clip-meta-item clip-meta-icon-only">
          <Pin className="clip-meta-icon pin-icon" />
        </span>}
        {isTrashMode && <span role="img" aria-label={translate('component.clipCard.clipInTrash')} title={translate('component.clipCard.inTrash')} className="clip-meta-item clip-meta-icon-only theme-status-danger-text">
          <Trash2 className="clip-meta-icon" />
        </span>}
        <time
          className="clip-meta-time"
          dateTime={dateTimeAttribute(clip.created_at)}
          title={formatFullDateTime(clip.created_at)}
        >
          {formatRelativeTime(clip.created_at, relativeTimeNow)}
        </time>
      </div>
    </div>
    {features.naming && clip.name && <div className="theme-named-text my-2.5 flex items-center space-x-2 text-xs font-semibold font-sans">
      {showClipType && <span className="clip-name-icon shrink-0 rounded border p-1">
        <FilePenLine className="h-3.5 w-3.5" />
      </span>}
      <span className="truncate" title={clip.name}>
        <HighlightedClipText text={clip.name} query={searchQuery} field="name" />
      </span>
    </div>}
  </>;
}
