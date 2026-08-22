import React from 'react';

export function useSidebarHoverState(isClipDragging: boolean, isBinReorderActive: boolean) {
  const [isPostDragHoverSuppressed, setIsPostDragHoverSuppressed] = React.useState(false);
  const [hoveredControl, setHoveredControl] = React.useState<string | null>(null);
  const wasClipDraggingRef = React.useRef(false);
  const wasBinReorderingRef = React.useRef(false);
  const isPointerOverRef = React.useRef(false);
  const lastPointerRef = React.useRef<{ x: number; y: number } | null>(null);
  const isAnyDrag = isClipDragging || isBinReorderActive;
  const isHoverMuted = isAnyDrag || isPostDragHoverSuppressed;

  React.useLayoutEffect(() => {
    if (isAnyDrag) setHoveredControl(null);
    if (wasClipDraggingRef.current && !isClipDragging) {
      setIsPostDragHoverSuppressed(isPointerOverRef.current);
    } else if (wasBinReorderingRef.current && !isBinReorderActive) {
      setIsPostDragHoverSuppressed(false);
      const pointer = lastPointerRef.current;
      if (isPointerOverRef.current && pointer) {
        const frame = requestAnimationFrame(() => {
          const control = document
            .elementFromPoint(pointer.x, pointer.y)
            ?.closest<HTMLElement>('[data-sidebar-hover-key]');
          setHoveredControl(control?.dataset.sidebarHoverKey ?? null);
        });
        wasClipDraggingRef.current = isClipDragging;
        wasBinReorderingRef.current = isBinReorderActive;
        return () => cancelAnimationFrame(frame);
      }
    }
    wasClipDraggingRef.current = isClipDragging;
    wasBinReorderingRef.current = isBinReorderActive;
  }, [isAnyDrag, isBinReorderActive, isClipDragging]);

  const onPointerEnter = () => {
    isPointerOverRef.current = true;
  };

  const onPointerMove = (event: React.PointerEvent<HTMLElement>) => {
    lastPointerRef.current = { x: event.clientX, y: event.clientY };
    if (isHoverMuted) {
      if (hoveredControl !== null) setHoveredControl(null);
      return;
    }
    const control = (event.target as HTMLElement).closest<HTMLElement>('[data-sidebar-hover-key]');
    const nextKey = control && event.currentTarget.contains(control)
      ? control.dataset.sidebarHoverKey ?? null
      : null;
    if (nextKey !== hoveredControl) setHoveredControl(nextKey);
  };

  const onPointerLeave = () => {
    isPointerOverRef.current = false;
    lastPointerRef.current = null;
    setHoveredControl(null);
    if (!isAnyDrag) setIsPostDragHoverSuppressed(false);
  };

  return {
    hoveredControl,
    isHoverMuted,
    onPointerEnter,
    onPointerMove,
    onPointerLeave,
  };
}
