import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { Bin } from '../types';

const BIN_ORDER_KEY = 'pasted_bin_order';

interface BinLayoutSnapshot {
  topById: Map<number, number>;
  heightById: Map<number, number>;
  centersWithoutDragged: number[];
  firstTop: number;
  gap: number;
}

function readBinOrder() {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(BIN_ORDER_KEY) || '[]');
    if (!Array.isArray(parsed)) return [];
    return Array.from(new Set(parsed.filter((id): id is number => Number.isInteger(id) && id > 0)));
  } catch {
    return [];
  }
}

export function useSidebarBinOrder(bins: Bin[], isClipDragging: boolean) {
  const [activeDragBinId, setActiveDragBinId] = useState<number | null>(null);
  const [binOrder, setBinOrder] = useState<number[]>(readBinOrder);
  const [binReorderOffsets, setBinReorderOffsets] = useState<Record<number, number>>({});
  const [isBinReorderSettling, setIsBinReorderSettling] = useState(false);
  const dragTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const activeDragBinIdRef = useRef<number | null>(null);
  const dragOriginOrderRef = useRef<number[] | null>(null);
  const previewOrderRef = useRef<number[] | null>(null);
  const layoutSnapshotRef = useRef<BinLayoutSnapshot | null>(null);
  const previewSignatureRef = useRef('');
  const dragGenerationRef = useRef(0);

  const sortedBins = useMemo(() => {
    if (binOrder.length === 0) return bins;
    const positions = new Map(binOrder.map((id, index) => [id, index]));
    return [...bins].sort((left, right) => {
      const leftIndex = positions.get(left.id);
      const rightIndex = positions.get(right.id);
      if (leftIndex === undefined && rightIndex === undefined) return 0;
      if (leftIndex === undefined) return 1;
      if (rightIndex === undefined) return -1;
      return leftIndex - rightIndex;
    });
  }, [binOrder, bins]);

  const cancelBinDrag = useCallback(() => {
    dragGenerationRef.current += 1;
    if (dragTimerRef.current) {
      clearTimeout(dragTimerRef.current);
      dragTimerRef.current = null;
    }
    activeDragBinIdRef.current = null;
    dragOriginOrderRef.current = null;
    previewOrderRef.current = null;
    layoutSnapshotRef.current = null;
    previewSignatureRef.current = '';
    setActiveDragBinId(null);
    setBinReorderOffsets({});
  }, []);

  const finishBinDrag = useCallback(async () => {
    if (dragTimerRef.current) {
      clearTimeout(dragTimerRef.current);
      dragTimerRef.current = null;
    }
    const draggedId = activeDragBinIdRef.current;
    if (draggedId === null) return;
    const nextOrder = previewOrderRef.current;
    const generation = dragGenerationRef.current;
    activeDragBinIdRef.current = null;
    dragOriginOrderRef.current = null;
    previewOrderRef.current = null;
    layoutSnapshotRef.current = null;
    previewSignatureRef.current = '';
    setActiveDragBinId(null);

    if (!nextOrder) {
      setBinReorderOffsets({});
      return;
    }

    await new Promise((resolve) => setTimeout(resolve, 115));
    if (dragGenerationRef.current !== generation) return;
    setIsBinReorderSettling(true);
    setBinOrder(nextOrder);
    setBinReorderOffsets({});
    try {
      localStorage.setItem(BIN_ORDER_KEY, JSON.stringify(nextOrder));
    } catch {
      // Ordering remains valid for this session when browser storage is unavailable.
    }
    requestAnimationFrame(() => {
      requestAnimationFrame(() => setIsBinReorderSettling(false));
    });
  }, []);

  const startBinDrag = useCallback((binId: number) => {
    if (isClipDragging) return;
    cancelBinDrag();
    const generation = dragGenerationRef.current;
    dragTimerRef.current = setTimeout(() => {
      dragTimerRef.current = null;
      if (dragGenerationRef.current !== generation) return;
      const currentOrder = sortedBins.map((bin) => bin.id);
      const rendered = Array.from(document.querySelectorAll<HTMLElement>('[data-bin-order-id]'))
        .map((element) => {
          const id = Number(element.dataset.binOrderId);
          const rect = element.getBoundingClientRect();
          return { id, top: rect.top, height: rect.height, center: rect.top + rect.height / 2 };
        })
        .filter((item) => currentOrder.includes(item.id))
        .sort((left, right) => left.top - right.top);
      if (rendered.length !== currentOrder.length) return;
      const measuredGaps = rendered.slice(0, -1).map((item, index) => (
        rendered[index + 1].top - item.top - item.height
      ));
      activeDragBinIdRef.current = binId;
      dragOriginOrderRef.current = currentOrder;
      layoutSnapshotRef.current = {
        topById: new Map(rendered.map((item) => [item.id, item.top])),
        heightById: new Map(rendered.map((item) => [item.id, item.height])),
        centersWithoutDragged: rendered.filter((item) => item.id !== binId).map((item) => item.center),
        firstTop: rendered[0]?.top ?? 0,
        gap: measuredGaps.length > 0
          ? measuredGaps.reduce((sum, gap) => sum + gap, 0) / measuredGaps.length
          : 0,
      };
      setActiveDragBinId(binId);
    }, 150);
  }, [cancelBinDrag, isClipDragging, sortedBins]);

  const moveDraggedBinToPosition = useCallback((pointerY: number) => {
    const draggedId = activeDragBinIdRef.current;
    const originalOrder = dragOriginOrderRef.current;
    const layout = layoutSnapshotRef.current;
    if (isClipDragging || draggedId === null || !originalOrder || !layout) return;
    const remainingOrder = originalOrder.filter((id) => id !== draggedId);
    const insertionIndex = layout.centersWithoutDragged.filter((center) => pointerY >= center).length;
    const nextOrder = [...remainingOrder];
    nextOrder.splice(insertionIndex, 0, draggedId);
    const differsFromOriginal = originalOrder.some((id, index) => id !== nextOrder[index]);
    const signature = differsFromOriginal ? nextOrder.join(',') : '';
    if (signature === previewSignatureRef.current) return;
    previewSignatureRef.current = signature;
    previewOrderRef.current = differsFromOriginal ? nextOrder : null;
    if (!differsFromOriginal) {
      setBinReorderOffsets({});
      return;
    }
    const offsets: Record<number, number> = {};
    let desiredTop = layout.firstTop;
    nextOrder.forEach((id) => {
      const originalTop = layout.topById.get(id);
      if (originalTop !== undefined) offsets[id] = desiredTop - originalTop;
      desiredTop += (layout.heightById.get(id) ?? 0) + layout.gap;
    });
    setBinReorderOffsets(offsets);
  }, [isClipDragging]);

  useEffect(() => cancelBinDrag, [cancelBinDrag]);
  useEffect(() => {
    if (isClipDragging) cancelBinDrag();
  }, [cancelBinDrag, isClipDragging]);

  return {
    activeDragBinId,
    sortedBins,
    binReorderOffsets,
    isBinReorderSettling,
    startBinDrag,
    finishBinDrag,
    cancelBinDrag,
    moveDraggedBinToPosition,
  };
}
