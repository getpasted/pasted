import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { Bin } from '../types';

const BIN_ORDER_KEY = 'pasted_bin_order';

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
  const dragTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

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
    if (dragTimerRef.current) {
      clearTimeout(dragTimerRef.current);
      dragTimerRef.current = null;
    }
    setActiveDragBinId(null);
  }, []);

  const startBinDrag = useCallback((binId: number) => {
    if (isClipDragging) return;
    cancelBinDrag();
    dragTimerRef.current = setTimeout(() => {
      dragTimerRef.current = null;
      setActiveDragBinId(binId);
    }, 150);
  }, [cancelBinDrag, isClipDragging]);

  const moveDraggedBinBefore = useCallback((targetBinId: number) => {
    if (isClipDragging || activeDragBinId === null || activeDragBinId === targetBinId) return;
    const currentOrder = sortedBins.map((bin) => bin.id);
    const fromIndex = currentOrder.indexOf(activeDragBinId);
    const toIndex = currentOrder.indexOf(targetBinId);
    if (fromIndex === -1 || toIndex === -1) return;

    const nextOrder = [...currentOrder];
    const [moved] = nextOrder.splice(fromIndex, 1);
    nextOrder.splice(toIndex, 0, moved);
    setBinOrder(nextOrder);
    try {
      localStorage.setItem(BIN_ORDER_KEY, JSON.stringify(nextOrder));
    } catch {
      // Ordering remains valid for this session when browser storage is unavailable.
    }
  }, [activeDragBinId, isClipDragging, sortedBins]);

  useEffect(() => cancelBinDrag, [cancelBinDrag]);
  useEffect(() => {
    if (isClipDragging) cancelBinDrag();
  }, [cancelBinDrag, isClipDragging]);

  return {
    activeDragBinId,
    sortedBins,
    startBinDrag,
    cancelBinDrag,
    moveDraggedBinBefore,
  };
}
