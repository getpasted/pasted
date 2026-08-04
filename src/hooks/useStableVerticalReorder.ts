import { useCallback, useEffect, useRef, useState, type PointerEvent as ReactPointerEvent, type RefObject } from 'react';

interface LayoutSnapshot {
  topById: Map<string, number>;
  heightById: Map<string, number>;
  firstTop: number;
  gap: number;
}

interface StableVerticalReorderOptions {
  itemIds: string[];
  containerRef: RefObject<HTMLElement | null>;
  onCommit: (orderedIds: string[]) => void;
  transitionMs?: number;
  disabled?: boolean;
}

interface PointerGesture {
  pointerId: number;
  itemId: string;
  startX: number;
  startY: number;
  active: boolean;
}

function nearestScrollContainer(element: HTMLElement | null): HTMLElement | null {
  let ancestor = element?.parentElement ?? null;
  while (ancestor) {
    const overflowY = window.getComputedStyle(ancestor).overflowY;
    if (/^(auto|scroll|overlay)$/.test(overflowY) && ancestor.scrollHeight > ancestor.clientHeight) {
      return ancestor;
    }
    ancestor = ancestor.parentElement;
  }
  return document.scrollingElement instanceof HTMLElement ? document.scrollingElement : null;
}

export function useStableVerticalReorder({
  itemIds,
  containerRef,
  onCommit,
  transitionMs = 100,
  disabled = false,
}: StableVerticalReorderOptions) {
  const [activeId, setActiveId] = useState<string | null>(null);
  const [offsets, setOffsets] = useState<Record<string, number>>({});
  const [isSettling, setIsSettling] = useState(false);
  const [isFinishing, setIsFinishing] = useState(false);
  const itemIdsRef = useRef(itemIds);
  const onCommitRef = useRef(onCommit);
  const activeIdRef = useRef<string | null>(null);
  const originalOrderRef = useRef<string[] | null>(null);
  const previewOrderRef = useRef<string[] | null>(null);
  const layoutRef = useRef<LayoutSnapshot | null>(null);
  const previewSignatureRef = useRef('');
  const generationRef = useRef(0);
  const gestureRef = useRef<PointerGesture | null>(null);
  const removeListenersRef = useRef<(() => void) | null>(null);
  const suppressClickRef = useRef(false);
  itemIdsRef.current = itemIds;
  onCommitRef.current = onCommit;

  const resetPreview = useCallback(() => {
    activeIdRef.current = null;
    originalOrderRef.current = null;
    previewOrderRef.current = null;
    layoutRef.current = null;
    previewSignatureRef.current = '';
    setActiveId(null);
    setOffsets({});
    setIsFinishing(false);
  }, []);

  const cancel = useCallback(() => {
    generationRef.current += 1;
    gestureRef.current = null;
    removeListenersRef.current?.();
    resetPreview();
  }, [resetPreview]);

  const begin = useCallback((itemId: string) => {
    const container = containerRef.current;
    const currentOrder = itemIdsRef.current;
    if (!container || currentOrder.length < 2 || !currentOrder.includes(itemId)) return false;
    generationRef.current += 1;
    const rendered = Array.from(container.querySelectorAll<HTMLElement>('[data-stable-reorder-id]'))
      .map((element) => {
        const id = element.dataset.stableReorderId ?? '';
        const rect = element.getBoundingClientRect();
        return { id, top: rect.top, height: rect.height };
      })
      .filter((item) => currentOrder.includes(item.id))
      .sort((left, right) => left.top - right.top);
    if (rendered.length !== currentOrder.length) return false;
    const measuredGaps = rendered.slice(0, -1).map((item, index) => (
      rendered[index + 1].top - item.top - item.height
    ));
    activeIdRef.current = itemId;
    originalOrderRef.current = [...currentOrder];
    previewOrderRef.current = null;
    layoutRef.current = {
      topById: new Map(rendered.map((item) => [item.id, item.top])),
      heightById: new Map(rendered.map((item) => [item.id, item.height])),
      firstTop: rendered[0]?.top ?? 0,
      gap: measuredGaps.length > 0
        ? measuredGaps.reduce((sum, gap) => sum + gap, 0) / measuredGaps.length
        : 0,
    };
    previewSignatureRef.current = '';
    setActiveId(itemId);
    setOffsets({});
    setIsFinishing(false);
    return true;
  }, [containerRef]);

  const update = useCallback((pointerY: number) => {
    const draggedId = activeIdRef.current;
    const originalOrder = originalOrderRef.current;
    const layout = layoutRef.current;
    if (!draggedId || !originalOrder || !layout) return;
    const draggedOriginalIndex = originalOrder.indexOf(draggedId);
    const remainingOrder = originalOrder.filter((id) => id !== draggedId);
    let insertionIndex = draggedOriginalIndex;
    remainingOrder.forEach((id, remainingIndex) => {
      const originalIndex = originalOrder.indexOf(id);
      const top = layout.topById.get(id);
      const height = layout.heightById.get(id);
      if (top === undefined || height === undefined) return;
      if (originalIndex < draggedOriginalIndex && pointerY <= top + height) {
        insertionIndex = Math.min(insertionIndex, remainingIndex);
      } else if (originalIndex > draggedOriginalIndex && pointerY >= top) {
        insertionIndex = Math.max(insertionIndex, remainingIndex + 1);
      }
    });
    const nextOrder = [...remainingOrder];
    nextOrder.splice(insertionIndex, 0, draggedId);
    const differs = originalOrder.some((id, index) => id !== nextOrder[index]);
    const signature = differs ? nextOrder.join('\u001f') : '';
    if (signature === previewSignatureRef.current) return;
    previewSignatureRef.current = signature;
    previewOrderRef.current = differs ? nextOrder : null;
    if (!differs) {
      setOffsets({});
      return;
    }
    const nextOffsets: Record<string, number> = {};
    let desiredTop = layout.firstTop;
    nextOrder.forEach((id) => {
      const originalTop = layout.topById.get(id);
      if (originalTop !== undefined) nextOffsets[id] = desiredTop - originalTop;
      desiredTop += (layout.heightById.get(id) ?? 0) + layout.gap;
    });
    setOffsets(nextOffsets);
  }, []);

  const finish = useCallback(async () => {
    const draggedId = activeIdRef.current;
    if (!draggedId) return;
    const nextOrder = previewOrderRef.current;
    const generation = generationRef.current;
    activeIdRef.current = null;
    originalOrderRef.current = null;
    previewOrderRef.current = null;
    layoutRef.current = null;
    previewSignatureRef.current = '';
    setActiveId(null);
    if (!nextOrder) {
      setOffsets({});
      return;
    }
    setIsFinishing(true);
    await new Promise((resolve) => setTimeout(resolve, transitionMs + 15));
    if (generationRef.current !== generation) return;
    const scrollContainer = nearestScrollContainer(containerRef.current);
    const settledScrollTop = scrollContainer?.scrollTop;
    const preserveScrollPosition = () => {
      if (scrollContainer && settledScrollTop !== undefined) scrollContainer.scrollTop = settledScrollTop;
    };
    setIsSettling(true);
    onCommitRef.current(nextOrder);
    setOffsets({});
    requestAnimationFrame(() => {
      preserveScrollPosition();
      requestAnimationFrame(() => {
        preserveScrollPosition();
        setIsSettling(false);
        setIsFinishing(false);
      });
    });
  }, [containerRef, transitionMs]);

  const startPointerReorder = useCallback((itemId: string, event: ReactPointerEvent) => {
    if (disabled || event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    removeListenersRef.current?.();
    const gesture: PointerGesture = {
      pointerId: event.pointerId,
      itemId,
      startX: event.clientX,
      startY: event.clientY,
      active: false,
    };
    gestureRef.current = gesture;

    const removeListeners = () => {
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', handlePointerEnd);
      window.removeEventListener('pointercancel', handlePointerCancel);
      window.removeEventListener('keydown', handleKeyDown);
      removeListenersRef.current = null;
    };
    const handlePointerMove = (pointerEvent: PointerEvent) => {
      const current = gestureRef.current;
      if (!current || current.pointerId !== pointerEvent.pointerId) return;
      if (!current.active && Math.hypot(
        pointerEvent.clientX - current.startX,
        pointerEvent.clientY - current.startY,
      ) >= 6) {
        current.active = begin(current.itemId);
      }
      if (!current.active) return;
      pointerEvent.preventDefault();
      update(pointerEvent.clientY);
    };
    const handlePointerEnd = (pointerEvent: PointerEvent) => {
      const current = gestureRef.current;
      if (!current || current.pointerId !== pointerEvent.pointerId) return;
      gestureRef.current = null;
      removeListeners();
      if (current.active) {
        suppressClickRef.current = true;
        setTimeout(() => {
          suppressClickRef.current = false;
        }, 0);
        void finish();
      }
    };
    const handlePointerCancel = (pointerEvent: PointerEvent) => {
      const current = gestureRef.current;
      if (!current || current.pointerId !== pointerEvent.pointerId) return;
      cancel();
    };
    const handleKeyDown = (keyEvent: KeyboardEvent) => {
      if (keyEvent.key !== 'Escape' || !gestureRef.current?.active) return;
      keyEvent.preventDefault();
      keyEvent.stopPropagation();
      cancel();
    };
    removeListenersRef.current = removeListeners;
    window.addEventListener('pointermove', handlePointerMove, { passive: false });
    window.addEventListener('pointerup', handlePointerEnd);
    window.addEventListener('pointercancel', handlePointerCancel);
    window.addEventListener('keydown', handleKeyDown);
  }, [begin, cancel, disabled, finish, update]);

  useEffect(() => () => {
    generationRef.current += 1;
    removeListenersRef.current?.();
  }, []);

  useEffect(() => {
    if (activeId || isFinishing) document.documentElement.classList.add('is-stable-reordering');
    else document.documentElement.classList.remove('is-stable-reordering');
    return () => document.documentElement.classList.remove('is-stable-reordering');
  }, [activeId, isFinishing]);

  const consumeClickAfterDrag = useCallback(() => {
    if (!suppressClickRef.current) return false;
    suppressClickRef.current = false;
    return true;
  }, []);

  return {
    activeId,
    offsets,
    isSettling,
    isFinishing,
    startPointerReorder,
    consumeClickAfterDrag,
    cancel,
  };
}
