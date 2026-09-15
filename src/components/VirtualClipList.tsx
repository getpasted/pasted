import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode, type RefObject } from 'react';

import { useVirtualClipViewport } from '../hooks/useVirtualClipViewport';
import type { ClipItem } from '../types';
import {
  createVirtualClipLayout,
  estimatedClipCardHeight,
  virtualClipIndexes,
} from '../utils/virtualClipList';
import { MeasuredVirtualClip } from './MeasuredVirtualClip';

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

export function VirtualClipList({
  clips,
  disabled = false,
  forcedClipIds = [],
  rowHeight,
  scrollRef,
  renderClip,
}: VirtualClipListProps) {
  const measuredSizesRef = useRef(new Map<number, number>());
  const measurementFrameRef = useRef<number | null>(null);
  const [measurementRevision, setMeasurementRevision] = useState(0);
  const clipIds = useMemo(() => clips.map((clip) => clip.id), [clips]);
  const estimatedSize = estimatedClipCardHeight(rowHeight);
  const layout = useMemo(
    () => createVirtualClipLayout(clipIds, measuredSizesRef.current, estimatedSize, CLIP_GAP),
    [clipIds, estimatedSize, measurementRevision],
  );
  const viewport = useVirtualClipViewport(scrollRef, disabled, layout.totalSize);

  useLayoutEffect(() => {
    measuredSizesRef.current.clear();
    setMeasurementRevision((revision) => revision + 1);
  }, [rowHeight]);

  useEffect(() => () => {
    if (measurementFrameRef.current !== null) cancelAnimationFrame(measurementFrameRef.current);
  }, []);

  const measureClip = useCallback((clipId: number, height: number) => {
    const previous = measuredSizesRef.current.get(clipId);
    if (previous !== undefined && Math.abs(previous - height) < 0.5) return;
    measuredSizesRef.current.set(clipId, height);
    if (measurementFrameRef.current !== null) return;
    measurementFrameRef.current = requestAnimationFrame(() => {
      measurementFrameRef.current = null;
      setMeasurementRevision((revision) => revision + 1);
    });
  }, []);

  if (disabled || layout.totalSize <= viewport.height + OVERSCAN_PX) {
    return <div role="list" className="space-y-2.5">
      {clips.map((clip, index) => <div key={clip.id} role="listitem">{renderClip(clip, index)}</div>)}
    </div>;
  }

  const indexById = new Map(clips.map((clip, index) => [clip.id, index]));
  const forcedIndexes = Array.from(new Set(forcedClipIds.flatMap((id) => {
    const index = indexById.get(id);
    return index === undefined ? [] : [index];
  })));
  const viewportIndexes = virtualClipIndexes(
    layout,
    viewport.scrollTop,
    viewport.height,
    OVERSCAN_PX,
  );
  const viewportIndexSet = new Set(viewportIndexes);
  const offscreenForcedIndexes = forcedIndexes.filter((index) => !viewportIndexSet.has(index));
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
    {viewportIndexes.length > 0 && <div
      className="absolute inset-x-0"
      style={{ transform: `translateY(${layout.positions[viewportIndexes[0]].start}px)` }}
    >
      {viewportIndexes.map((index) => {
      const clip = clips[index];
      return <MeasuredVirtualClip
        key={clip.id}
        clipId={clip.id}
        index={index}
        onMeasure={measureClip}
        totalCount={clips.length}
      >
        {renderClip(clip, index)}
      </MeasuredVirtualClip>;
      })}
    </div>}
    {offscreenForcedIndexes.map((index) => {
      const clip = clips[index];
      return <MeasuredVirtualClip
        key={clip.id}
        clipId={clip.id}
        index={index}
        onMeasure={measureClip}
        start={layout.positions[index].start}
        totalCount={clips.length}
      >
        {renderClip(clip, index)}
      </MeasuredVirtualClip>;
    })}
  </div>;
}
