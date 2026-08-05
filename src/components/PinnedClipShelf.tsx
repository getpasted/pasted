import { useEffect, useRef, useState, type CSSProperties, type WheelEvent } from 'react';
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
  const [displayedClips, setDisplayedClips] = useState(stackedClips);
  const displayedClipsRef = useRef(stackedClips);
  const [leavingClipIds, setLeavingClipIds] = useState<Set<number>>(() => new Set());
  const removalTimers = useRef(new Map<number, ReturnType<typeof setTimeout>>());
  const stackedSignature = stackedClipIds.join(',');

  useEffect(() => {
    const nextIds = new Set(stackedSignature.split(',').filter(Boolean).map(Number));
    const nextStackedClips = clips.filter((clip) => nextIds.has(clip.id));
    const currentIds = new Set(displayedClipsRef.current.map((clip) => clip.id));
    const additions = nextStackedClips.filter((clip) => !currentIds.has(clip.id));
    if (additions.length > 0) {
      const nextDisplayed = [...displayedClipsRef.current, ...additions];
      displayedClipsRef.current = nextDisplayed;
      setDisplayedClips(nextDisplayed);
    }

    for (const clip of displayedClipsRef.current) {
      if (nextIds.has(clip.id) || removalTimers.current.has(clip.id)) continue;

      setLeavingClipIds((current) => new Set(current).add(clip.id));
      const timer = setTimeout(() => {
        const nextDisplayed = displayedClipsRef.current.filter((item) => item.id !== clip.id);
        displayedClipsRef.current = nextDisplayed;
        setDisplayedClips(nextDisplayed);
        setLeavingClipIds((current) => {
          const next = new Set(current);
          next.delete(clip.id);
          return next;
        });
        removalTimers.current.delete(clip.id);
      }, 360);
      removalTimers.current.set(clip.id, timer);
    }

    for (const clip of nextStackedClips) {
      const timer = removalTimers.current.get(clip.id);
      if (!timer) continue;
      clearTimeout(timer);
      removalTimers.current.delete(clip.id);
      setLeavingClipIds((current) => {
        const next = new Set(current);
        next.delete(clip.id);
        return next;
      });
    }
  }, [clips, stackedSignature]);

  useEffect(() => () => {
    for (const timer of removalTimers.current.values()) clearTimeout(timer);
  }, []);

  const shownClips = displayedClips.slice(0, 5);
  const remainingClips = Math.max(0, displayedClips.length - shownClips.length);
  const overflowIsLeaving = remainingClips > 0
    && displayedClips.slice(5).every((clip) => leavingClipIds.has(clip.id));
  const visible = displayedClips.length > 0;
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
            className={`pinned-clip-shelf-card ${leavingClipIds.has(clip.id) ? 'is-leaving' : ''} ${remainingClips === 0 && index === shownClips.length - 1 ? 'is-stack-tail' : ''} ${selectedClipId === clip.id ? 'is-selected' : ''}`}
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
            className={`pinned-clip-shelf-card pinned-clip-shelf-overflow is-stack-tail ${overflowIsLeaving ? 'is-leaving' : ''}`}
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
