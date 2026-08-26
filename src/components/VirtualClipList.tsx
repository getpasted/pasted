import { useCallback, useLayoutEffect, useMemo, useRef, useState, type ReactNode, type RefObject } from 'react';

import type { ClipItem } from '../types';
import {
  createVirtualClipLayout,
  estimatedClipCardHeight,
  virtualClipIndexes,
} from '../utils/virtualClipList';

const CLIP_GAP = 10;
const OVERSCAN_PX = 800;

interface VirtualClipListProps {
  clips: ClipItem[];
  disabled?: boolean;
  forcedClipIds?: number[];
  rowHeight: 'small' | 'medium' | 'large';
  scrollRef: RefObject<HTMLDivElement | null>;
  renderClip: (clip: ClipItem, index: number) => ReactNode;
}

function MeasuredClip({
  children,
  clipId,
  index,
  onMeasure,
  start,
  totalCount,
}: {
  children: ReactNode;
  clipId: number;
  index: number;
  onMeasure: (clipId: number, height: number) => void;
  start: number;
  totalCount: number;
}) {
  const ref = useRef<HTMLDivElement | null>(null);
  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) return undefined;
    const measure = () => onMeasure(clipId, element.getBoundingClientRect().height - CLIP_GAP);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [clipId, onMeasure]);
  return <div
    ref={ref}
    role="listitem"
    aria-posinset={index + 1}
    aria-setsize={totalCount}
    className="absolute inset-x-0 pb-2.5"
    style={{ transform: `translateY(${start}px)` }}
  >
    {children}
  </div>;
}

export function VirtualClipList({
  clips,
  disabled = false,
  forcedClipIds = [],
  rowHeight,
  scrollRef,
  renderClip,
}: VirtualClipListProps) {
  const measuredSizesRef = useRef(new Map<number, number>());
  const [measurementRevision, setMeasurementRevision] = useState(0);
  const [viewport, setViewport] = useState({ height: 800, scrollTop: 0 });
  const clipIds = useMemo(() => clips.map((clip) => clip.id), [clips]);
  const estimatedSize = estimatedClipCardHeight(rowHeight);
  const layout = useMemo(
    () => createVirtualClipLayout(clipIds, measuredSizesRef.current, estimatedSize, CLIP_GAP),
    [clipIds, estimatedSize, measurementRevision],
  );

  useLayoutEffect(() => {
    measuredSizesRef.current.clear();
    setMeasurementRevision((revision) => revision + 1);
  }, [rowHeight]);

  useLayoutEffect(() => {
    const element = scrollRef.current;
    if (!element || disabled) return undefined;
    let frame = 0;
    const update = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        setViewport({ height: element.clientHeight, scrollTop: element.scrollTop });
      });
    };
    update();
    element.addEventListener('scroll', update, { passive: true });
    const observer = new ResizeObserver(update);
    observer.observe(element);
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
      element.removeEventListener('scroll', update);
    };
  }, [disabled, scrollRef]);

  const measureClip = useCallback((clipId: number, height: number) => {
    const previous = measuredSizesRef.current.get(clipId);
    if (previous !== undefined && Math.abs(previous - height) < 0.5) return;
    measuredSizesRef.current.set(clipId, height);
    setMeasurementRevision((revision) => revision + 1);
  }, []);

  if (disabled) {
    return <div role="list" className="space-y-2.5">
      {clips.map((clip, index) => <div key={clip.id} role="listitem">{renderClip(clip, index)}</div>)}
    </div>;
  }

  const indexById = new Map(clips.map((clip, index) => [clip.id, index]));
  const forcedIndexes = forcedClipIds.flatMap((id) => {
    const index = indexById.get(id);
    return index === undefined ? [] : [index];
  });
  const indexes = virtualClipIndexes(
    layout,
    viewport.scrollTop,
    viewport.height,
    OVERSCAN_PX,
    forcedIndexes,
  );
  const firstViewportIndex = virtualClipIndexes(
    layout,
    viewport.scrollTop,
    viewport.height,
    0,
  )[0] ?? 0;

  return <div
    role="list"
    data-virtual-clip-list
    data-virtual-start-index={firstViewportIndex}
    className="relative"
    style={{ height: `${layout.totalSize}px` }}
  >
    {indexes.map((index) => {
      const clip = clips[index];
      return <MeasuredClip
        key={clip.id}
        clipId={clip.id}
        index={index}
        onMeasure={measureClip}
        start={layout.positions[index].start}
        totalCount={clips.length}
      >
        {renderClip(clip, index)}
      </MeasuredClip>;
    })}
  </div>;
}
