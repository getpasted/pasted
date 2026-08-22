import { StickyNote } from 'lucide-react';

import type { useFeatures } from '../hooks/useFeatures';
import { translate } from '../localization/runtime';
import type { ClipItem } from '../types';
import { concealedClipMask } from '../utils/concealedClipMask';
import { ClipFileThumbnail, ClipImageThumbnail } from './ClipCardThumbnails';
import { HighlightedClipText } from './HighlightedClipText';

type ClipCardFeatures = ReturnType<typeof useFeatures>;

export function ClipCardContent({
  clip,
  features,
  filePreviewMaxMb,
  filePreviewMode,
  imageMaxHeightClass,
  imagePlaceholderHeightClass,
  isSensitive,
  isSmall,
  lineClampClass,
  noteSummary,
  searchQuery,
}: {
  clip: ClipItem;
  features: ClipCardFeatures;
  filePreviewMaxMb: number;
  filePreviewMode: 'off' | 'safe' | 'all';
  imageMaxHeightClass: string;
  imagePlaceholderHeightClass: string;
  isSensitive: boolean;
  isSmall: boolean;
  lineClampClass: string;
  noteSummary: string;
  searchQuery?: string;
}) {
  return <>
    <div className={`theme-text-main ${clip.content_type === 'file' ? (isSmall ? 'text-[11px]' : 'text-xs') : lineClampClass} font-mono leading-relaxed break-all`}>
      {isSensitive ? <div
        className="theme-status-warning flex items-center rounded-lg border p-1.5 text-xs font-mono select-none"
        aria-label={translate('collection.concealed')}
      >
        <span className="tracking-widest font-bold">{concealedClipMask(clip)}</span>
      </div> : clip.content_type === 'image' ? <ClipImageThumbnail
        key={`${clip.id}:${clip.content_hash}`}
        clipId={clip.id}
        contentHash={clip.content_hash}
        maxHeightClass={imageMaxHeightClass}
        placeholderHeightClass={imagePlaceholderHeightClass}
      /> : clip.content_type === 'file' ? <ClipFileThumbnail
        key={`${clip.id}:${clip.content_hash}`}
        clip={clip}
        mode={filePreviewMode}
        maxSizeMb={filePreviewMaxMb}
        maxHeightClass={imageMaxHeightClass}
        placeholderHeightClass={imagePlaceholderHeightClass}
      /> : (clip.content_types ?? [clip.content_type]).includes('color') ? <div className="clip-thumbnail-stage flex items-center space-x-3 p-2 rounded border">
        <div
          className="theme-divider w-8 h-8 rounded border shadow"
          style={{ backgroundColor: clip.text_content || '#ffffff' }}
        />
        <span className="clip-note-accent font-mono text-xs">{clip.text_content}</span>
      </div> : <div className="relative flex items-center justify-between">
        <span>
          <HighlightedClipText text={clip.text_content || translate('component.clipCard.emptyItem')} query={searchQuery} field="content" />
        </span>
      </div>}
    </div>
    {features.notes && noteSummary && <div className="clip-note-summary mt-2 pt-1.5 border-t flex items-center space-x-1.5 text-[11px] font-sans italic">
      <StickyNote className="w-3 h-3 shrink-0" />
      <span className="truncate" title={noteSummary}>
        <HighlightedClipText text={noteSummary} query={searchQuery} field="note" />
      </span>
    </div>}
  </>;
}
