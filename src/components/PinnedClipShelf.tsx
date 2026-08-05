import type { CSSProperties, WheelEvent } from 'react';
import { FileText, Image as ImageIcon, Pin } from 'lucide-react';
import type { ClipItem } from '../types';
import { getClipFileSummary } from '../types';

interface PinnedClipShelfProps {
  clips: ClipItem[];
  stackedClipIds: number[];
  selectedClipId?: number;
  onSelect: (clip: ClipItem) => void;
  onRevealAll: () => void;
  onWheel: (event: WheelEvent<HTMLDivElement>) => void;
}

function clipSummary(clip: ClipItem) {
  if (clip.content_type === 'file') return getClipFileSummary(clip);
  if (clip.content_type === 'image') return clip.text_content?.trim() || 'Image';
  return clip.text_content?.replace(/\s+/g, ' ').trim() || 'Empty clip';
}

export function PinnedClipShelf({
  clips,
  stackedClipIds,
  selectedClipId,
  onSelect,
  onRevealAll,
  onWheel,
}: PinnedClipShelfProps) {
  const stackedIds = new Set(stackedClipIds);
  const stackedClips = clips.filter((clip) => stackedIds.has(clip.id));
  const shownClips = stackedClips.slice(0, 5);
  const remainingClips = Math.max(0, stackedClips.length - shownClips.length);
  const visible = stackedClips.length > 0;
  if (clips.length === 0) return null;

  return (
    <div
      className={`pinned-clip-shelf ${visible ? 'is-visible' : ''}`}
      aria-hidden={!visible}
      onWheel={onWheel}
    >
      <div
        className="pinned-clip-shelf-stack"
        aria-label={`${stackedClips.length} stacked pinned ${stackedClips.length === 1 ? 'clip' : 'clips'}`}
      >
        {shownClips.map((clip, index) => (
          <button
            type="button"
            key={clip.id}
            tabIndex={visible ? 0 : -1}
            className={`pinned-clip-shelf-card ${remainingClips === 0 && index === shownClips.length - 1 ? 'is-stack-tail' : ''} ${selectedClipId === clip.id ? 'is-selected' : ''}`}
            style={{
              '--pinned-shelf-index': index,
              '--pinned-shelf-depth': shownClips.length - index,
            } as CSSProperties}
            onClick={() => onSelect(clip)}
          >
            <span className="pinned-clip-shelf-icon" aria-hidden="true">
              {clip.content_type === 'image' ? <ImageIcon /> : clip.content_type === 'file' ? <FileText /> : <Pin />}
            </span>
            <span className="pinned-clip-shelf-source">{clip.source_app}</span>
            <span className="pinned-clip-shelf-summary">{clipSummary(clip)}</span>
            {index === 0 && (
              <span className="pinned-clip-shelf-count">{stackedClips.length}</span>
            )}
          </button>
        ))}
        {remainingClips > 0 && (
          <button
            type="button"
            tabIndex={visible ? 0 : -1}
            className="pinned-clip-shelf-card pinned-clip-shelf-overflow is-stack-tail"
            style={{
              '--pinned-shelf-index': 5,
              '--pinned-shelf-depth': 0,
            } as CSSProperties}
            onClick={onRevealAll}
          >
            <Pin aria-hidden="true" />
            <span>+{remainingClips} more pinned</span>
          </button>
        )}
      </div>
    </div>
  );
}
