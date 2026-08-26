import type { ClipItem } from '../types';
import { getClipFileSummary } from '../types';
import { localizedSourceName } from '../localization/presentation';
import { translate } from '../localization/runtime';
import { OverflowText } from './OverflowText';
import { concealedClipMask } from '../utils/concealedClipMask';

interface ClipDragPreviewProps {
  clip: ClipItem;
  x: number;
  y: number;
  batchCount: number;
  showSource: boolean;
  concealed: boolean;
}

export function ClipDragPreview({ clip, x, y, batchCount, showSource, concealed }: ClipDragPreviewProps) {
  const previewText = concealed
    ? concealedClipMask(clip)
    : clip.content_type === 'image'
    ? translate('app.imageClip')
    : clip.content_type === 'file'
      ? getClipFileSummary(clip)
      : clip.text_content || translate('app.emptyClip');

  return (
    <div
      data-testid="clip-drag-preview"
      className="clip-drag-preview fixed w-64 pointer-events-none rounded-xl border px-3 py-2.5"
      style={{ left: x + 14, top: y + 14, transform: 'rotate(1.5deg)' }}
    >
      {(showSource || batchCount > 1) && (
        <div className="theme-text-muted flex items-center justify-between gap-3 text-[10px]">
          {showSource && (
            <OverflowText
              text={localizedSourceName(clip.source)}
              className="theme-text-main truncate font-semibold"
            />
          )}
          {batchCount > 1 && (
            <span className="clip-drag-preview-count shrink-0 rounded-full px-2 py-0.5 font-bold">
              {translate('format.clipCount', { count: batchCount })}
            </span>
          )}
        </div>
      )}
      <OverflowText
        as="div"
        text={previewText}
        className={`theme-title mt-1.5 truncate font-mono text-xs ${concealed ? 'tracking-widest' : ''}`}
      />
    </div>
  );
}
