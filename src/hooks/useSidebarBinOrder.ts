import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { Bin } from '../types';
import { useStableVerticalReorder } from './useStableVerticalReorder';
import { scheduleBackupClientStatePersistence } from '../utils/backupClientState';

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
  const [binOrder, setBinOrder] = useState<number[]>(readBinOrder);
  const binListRef = useRef<HTMLElement>(null);

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

  const commitBinOrder = useCallback((orderedIds: string[]) => {
    const nextOrder = orderedIds.map(Number).filter(Number.isInteger);
    setBinOrder(nextOrder);
    try {
      localStorage.setItem(BIN_ORDER_KEY, JSON.stringify(nextOrder));
      scheduleBackupClientStatePersistence();
    } catch {
      // Ordering remains valid for this session when browser storage is unavailable.
    }
  }, []);

  const {
    activeId,
    offsets,
    isSettling,
    isFinishing,
    startPointerReorder,
    consumeClickAfterDrag,
    cancel,
  } = useStableVerticalReorder({
    itemIds: sortedBins.map((bin) => String(bin.id)),
    containerRef: binListRef,
    onCommit: commitBinOrder,
    disabled: isClipDragging,
  });

  useEffect(() => {
    if (isClipDragging) cancel();
  }, [cancel, isClipDragging]);

  const numericOffsets = useMemo(() => Object.fromEntries(
    Object.entries(offsets).map(([id, offset]) => [Number(id), offset]),
  ), [offsets]);

  return {
    activeDragBinId: activeId === null ? null : Number(activeId),
    sortedBins,
    binListRef,
    binReorderOffsets: numericOffsets,
    isBinReorderSettling: isSettling,
    isBinReorderActive: activeId !== null || isFinishing || isSettling,
    startBinDrag: startPointerReorder,
    consumeBinDragClick: consumeClickAfterDrag,
  };
}
