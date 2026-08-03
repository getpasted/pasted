import { useCallback, useMemo, useRef, useState, type Dispatch, type SetStateAction } from 'react';
import type { Bin, ClipItem } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';

interface ClipDragPreview {
  clipId: number;
  x: number;
  y: number;
}

interface PinnedLayoutSnapshot {
  topById: Map<number, number>;
  heightById: Map<number, number>;
  firstTop: number;
  gap: number;
}

interface ClipBinDragInput {
  allClips: ClipItem[];
  setAllClips: Dispatch<SetStateAction<ClipItem[]>>;
  bins: Bin[];
  selectedClipIds: Set<number>;
  fetchClips: () => Promise<void>;
  assignClipToBin: (clipId: number, binId: number) => Promise<void>;
}

export function useClipBinDrag({
  allClips,
  setAllClips,
  bins,
  selectedClipIds,
  fetchClips,
  assignClipToBin,
}: ClipBinDragInput) {
  const [draggedClipId, setDraggedClipId] = useState<number | null>(null);
  const [pointerDropTargetBinId, setPointerDropTargetBinId] = useState<number | null>(null);
  const [clipDragPreview, setClipDragPreview] = useState<ClipDragPreview | null>(null);
  const [pinnedReorderOffsets, setPinnedReorderOffsets] = useState<Record<number, number>>({});
  const [isPinnedReorderSettling, setIsPinnedReorderSettling] = useState(false);
  const originalPinnedOrderRef = useRef<ClipItem[] | null>(null);
  const pinnedOrderPreviewRef = useRef<ClipItem[] | null>(null);
  const isPinnedPreviewActiveRef = useRef(false);
  const pinnedLayoutSnapshotRef = useRef<PinnedLayoutSnapshot | null>(null);
  const pinnedPreviewSignatureRef = useRef('');
  const pinnedDragGenerationRef = useRef(0);

  const disabledDropBinId = useMemo(() => {
    if (draggedClipId === null) return null;
    const draggedIds = selectedClipIds.size > 1 && selectedClipIds.has(draggedClipId)
      ? Array.from(selectedClipIds)
      : [draggedClipId];
    const draggedClips = allClips.filter((clip) => draggedIds.includes(clip.id));
    if (draggedClips.length !== draggedIds.length) return null;
    const currentBinId = draggedClips[0]?.bin_id ?? null;
    if (currentBinId === null || !draggedClips.every((clip) => clip.bin_id === currentBinId)) {
      return null;
    }
    return bins.find((bin) => bin.id === currentBinId && bin.bin_type !== 'tag')
      ? currentBinId
      : null;
  }, [allClips, bins, draggedClipId, selectedClipIds]);

  const getPointerDropTarget = useCallback((x: number, y: number) => {
    const target = document
      .elementFromPoint(x, y)
      ?.closest<HTMLElement>('[data-bin-drop-id]');
    if (!target) return null;
    const binId = Number(target.dataset.binDropId);
    return Number.isInteger(binId) && binId > 0 ? binId : null;
  }, []);

  const beginPinnedReorderPreview = useCallback((clipId: number) => {
    pinnedDragGenerationRef.current += 1;
    if (!allClips.find((clip) => clip.id === clipId)?.is_pinned) return;
    const pinnedIds = new Set(allClips.filter((clip) => clip.is_pinned).map((clip) => clip.id));
    const rendered = Array.from(document.querySelectorAll<HTMLElement>('[data-clip-list] [data-clip-id]'))
      .filter((element) => pinnedIds.has(Number(element.dataset.clipId)))
      .map((element) => {
        const id = Number(element.dataset.clipId);
        const rect = element.getBoundingClientRect();
        return { id, top: rect.top, height: rect.height, center: rect.top + rect.height / 2 };
      })
      .sort((left, right) => left.top - right.top);
    if (rendered.length !== pinnedIds.size) return;
    const measuredGaps = rendered.slice(0, -1).map((item, index) => (
      rendered[index + 1].top - item.top - item.height
    ));
    originalPinnedOrderRef.current = allClips;
    pinnedOrderPreviewRef.current = null;
    isPinnedPreviewActiveRef.current = false;
    pinnedLayoutSnapshotRef.current = {
      topById: new Map(rendered.map((item) => [item.id, item.top])),
      heightById: new Map(rendered.map((item) => [item.id, item.height])),
      firstTop: rendered[0]?.top ?? 0,
      gap: measuredGaps.length > 0
        ? measuredGaps.reduce((sum, gap) => sum + gap, 0) / measuredGaps.length
        : 0,
    };
    pinnedPreviewSignatureRef.current = '';
    setPinnedReorderOffsets({});
  }, [allClips]);

  const cancelPinnedReorderPreview = useCallback(() => {
    pinnedDragGenerationRef.current += 1;
    originalPinnedOrderRef.current = null;
    pinnedOrderPreviewRef.current = null;
    isPinnedPreviewActiveRef.current = false;
    pinnedLayoutSnapshotRef.current = null;
    pinnedPreviewSignatureRef.current = '';
    setPinnedReorderOffsets({});
  }, []);

  const updatePinnedReorderPreview = useCallback((x: number, y: number, clipId: number) => {
    const originalOrder = originalPinnedOrderRef.current;
    const layout = pinnedLayoutSnapshotRef.current;
    if (!originalOrder || !layout) return;
    if (getPointerDropTarget(x, y) !== null) {
      pinnedOrderPreviewRef.current = null;
      isPinnedPreviewActiveRef.current = false;
      if (pinnedPreviewSignatureRef.current !== '') {
        pinnedPreviewSignatureRef.current = '';
        setPinnedReorderOffsets({});
      }
      return;
    }
    const pointerElement = document.elementFromPoint(x, y);
    if (!pointerElement?.closest('[data-clip-list]')) {
      pinnedOrderPreviewRef.current = null;
      isPinnedPreviewActiveRef.current = false;
      if (pinnedPreviewSignatureRef.current !== '') {
        pinnedPreviewSignatureRef.current = '';
        setPinnedReorderOffsets({});
      }
      return;
    }

    const originalPinned = originalOrder.filter((clip) => clip.is_pinned);
    const draggedClip = originalPinned.find((clip) => clip.id === clipId);
    if (!draggedClip) return;
    const remainingPinned = originalPinned.filter((clip) => clip.id !== clipId);
    const draggedOriginalIndex = originalPinned.findIndex((clip) => clip.id === clipId);
    let insertionIndex = draggedOriginalIndex;
    remainingPinned.forEach((clip, remainingIndex) => {
      const originalIndex = originalPinned.findIndex((item) => item.id === clip.id);
      const top = layout.topById.get(clip.id);
      const height = layout.heightById.get(clip.id);
      if (top === undefined || height === undefined) return;
      if (originalIndex < draggedOriginalIndex && y <= top + height) {
        insertionIndex = Math.min(insertionIndex, remainingIndex);
      } else if (originalIndex > draggedOriginalIndex && y >= top) {
        insertionIndex = Math.max(insertionIndex, remainingIndex + 1);
      }
    });
    const reordered = [...remainingPinned];
    reordered.splice(insertionIndex, 0, draggedClip);
    const orderedWithRanks = reordered.map((clip, index) => ({ ...clip, pin_order: index }));
    const preview = [...orderedWithRanks, ...originalOrder.filter((clip) => !clip.is_pinned)];
    const differsFromOriginal = originalPinned.some((clip, index) => clip.id !== orderedWithRanks[index]?.id);
    const signature = differsFromOriginal ? orderedWithRanks.map((clip) => clip.id).join(',') : '';
    if (signature === pinnedPreviewSignatureRef.current) return;
    pinnedPreviewSignatureRef.current = signature;
    pinnedOrderPreviewRef.current = differsFromOriginal ? preview : null;
    isPinnedPreviewActiveRef.current = true;
    if (!differsFromOriginal) {
      setPinnedReorderOffsets({});
      return;
    }
    const offsets: Record<number, number> = {};
    let desiredTop = layout.firstTop;
    orderedWithRanks.forEach((clip) => {
      const originalTop = layout.topById.get(clip.id);
      if (originalTop !== undefined) {
        offsets[clip.id] = desiredTop - originalTop;
      }
      desiredTop += (layout.heightById.get(clip.id) ?? 0) + layout.gap;
    });
    setPinnedReorderOffsets(offsets);
  }, [getPointerDropTarget]);

  const finishClipPointerDrag = useCallback(async (x: number, y: number, clipId: number) => {
    const dragGeneration = pinnedDragGenerationRef.current;
    const binId = getPointerDropTarget(x, y);
    setPointerDropTargetBinId(null);
    setClipDragPreview(null);
    const pinnedOrderPreview = pinnedOrderPreviewRef.current;
    const isPinnedPreviewActive = isPinnedPreviewActiveRef.current;
    originalPinnedOrderRef.current = null;
    pinnedOrderPreviewRef.current = null;
    isPinnedPreviewActiveRef.current = false;
    pinnedLayoutSnapshotRef.current = null;
    pinnedPreviewSignatureRef.current = '';

    if (binId !== null) {
      setPinnedReorderOffsets({});
      await assignClipToBin(clipId, binId);
      return;
    }

    if (!isPinnedPreviewActive || !pinnedOrderPreview) {
      setPinnedReorderOffsets({});
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 90));
    if (pinnedDragGenerationRef.current !== dragGeneration) return;
    setIsPinnedReorderSettling(true);
    setAllClips(pinnedOrderPreview);
    setPinnedReorderOffsets({});
    requestAnimationFrame(() => {
      requestAnimationFrame(() => setIsPinnedReorderSettling(false));
    });
    const pinnedIds = pinnedOrderPreview.filter((clip) => clip.is_pinned).map((clip) => clip.id);

    try {
      await invoke('reorder_pinned_clips', { ids: pinnedIds });
    } catch (error) {
      console.error('Failed to save pin order:', error);
      void fetchClips();
    }
  }, [assignClipToBin, fetchClips, getPointerDropTarget, setAllClips]);

  return {
    draggedClipId,
    setDraggedClipId,
    pointerDropTargetBinId,
    setPointerDropTargetBinId,
    clipDragPreview,
    setClipDragPreview,
    disabledDropBinId,
    pinnedReorderOffsets,
    isPinnedReorderSettling,
    getPointerDropTarget,
    beginPinnedReorderPreview,
    updatePinnedReorderPreview,
    cancelPinnedReorderPreview,
    finishClipPointerDrag,
  };
}
